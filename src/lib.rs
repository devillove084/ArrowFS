#![feature(core_intrinsics)]
//#![recursion_limit="256"]
#![recursion_limit = "512"]
#[allow(mutable_transmutes)]

mod directory;
mod file;
mod inode;
mod slab;

use crate::file::File::{DataFile, Directory, EmptyFile};
use crate::file::{File, FileHandle};
use directory::DirectoryHandle;
pub use file::Whence;
pub use inode::Inode;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type FileDescriptor = isize;

pub const O_RDONLY: u32 = 1 << 0;
pub const O_WRONLY: u32 = 1 << 1;
pub const O_RDWR: u32 = 1 << 2;
pub const O_NONBLOCK: u32 = 1 << 3;
pub const O_APPEND: u32 = 1 << 4;
pub const O_CREAT: u32 = 1 << 5;

pub struct Proc<'r> {
    cwd: File<'r>,
    fd_table: HashMap<FileDescriptor, FileHandle<'r>>,
    fds: Vec<FileDescriptor>,
}

impl<'r> Proc<'r> {
    pub fn new() -> Proc<'r> {
        Proc {
            cwd: File::new_dir(None),
            fd_table: HashMap::new(),
            fds: (0..(256 - 2)).map(|i| 256 - i).collect(),
        }
    }

    #[inline(always)]
    fn extract_fd(fd_opt: &Option<FileDescriptor>) -> FileDescriptor {
        match fd_opt {
            &Some(fd) => fd,
            &None => panic!("Error in FD allocation."),
        }
    }

    pub fn open(&mut self, path: &'r str, flags: u32) -> FileDescriptor {
        let lookup = self.cwd.get(path);
        let file = match lookup {
            Some(f) => f,
            None => {
                if (flags & O_CREAT) != 0 {
                    // FIXME: Fetch from allocator
                    let rcinode = Rc::new(RefCell::new(Box::new(Inode::new())));
                    let file = File::new_data_file(rcinode);
                    self.cwd.insert(path, file.clone());
                    file
                } else {
                    EmptyFile
                }
            }
        };

        match file {
            DataFile(_) => {
                let fd = Proc::extract_fd(&self.fds.pop());
                let handle = FileHandle::new(file);
                self.fd_table.insert(fd, handle);
                fd
            }
            Directory(_) => -1,
            EmptyFile => -2,
        }
    }

    pub fn read(&self, fd: FileDescriptor, dst: &mut [u8]) -> usize {
        let handle = self.fd_table.get(&fd).expect("fd does not exist");
        handle.read(dst)
    }

    pub fn write(&mut self, fd: FileDescriptor, src: &[u8]) -> usize {
        let handle = self.fd_table.get_mut(&fd).expect("fd does not exist");
        handle.write(src)
    }

    pub fn seek(&mut self, fd: FileDescriptor, o: isize, whence: Whence) -> usize {
        let handle = self.fd_table.get_mut(&fd).expect("fd does not exist");
        handle.seek(o, whence)
    }

    pub fn close(&mut self, fd: FileDescriptor) {
        self.fd_table.remove(&fd);
        self.fds.push(fd);
    }

    pub fn unlink(&mut self, path: &'r str) {
        self.cwd.remove(path);
    }
}
