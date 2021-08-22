#[allow(mutable_transmutes)]

use libc::{c_void, free, malloc, size_t};
use std::cell::{Cell, RefCell};
use std::intrinsics;
use std::mem::transmute;
use std::rc::Rc;
use std::{mem, ptr};

struct Slab<'a, T> {
    parent: &'a SlabAllocator<T>, // Would RC be better to get rid of 'a? How to?
    ptr: *mut T,
}

impl<'a, T> Slab<'a, T> {
    fn borrow<'r>(&'r self) -> &'r T {
        unsafe { &*self.ptr }
    }

    fn borrow_mut<'r>(&'r mut self) -> &'r mut T {
        unsafe { &mut *self.ptr }
    }
}

impl<'a, T: Eq> Eq for Slab<'a, T> {}

impl<'a, T: PartialEq> PartialEq for Slab<'a, T> {
    fn eq(&self, other: &Slab<T>) -> bool {
        self.borrow() == other.borrow()
    }
}

impl<'a, T> Drop for Slab<'a, T> {
    fn drop(&mut self) {
        self.parent.free(self.ptr);
    }
}

#[derive(Clone)]
pub struct SlabBox<'a, T>(Rc<RefCell<Slab<'a, T>>>);

impl<'a, T> SlabBox<'a, T> {
    #[inline(always)]
    pub fn borrow<'r>(&'r self) -> &'r T {
        let SlabBox(ref rc) = *self;
        unsafe { transmute(rc.borrow().borrow()) }
    }

    #[inline(always)]
    pub fn borrow_mut<'r>(&'r mut self) -> &'r mut T {
        let SlabBox(ref rc) = *self;
        unsafe { transmute(rc.borrow_mut().borrow_mut()) }
    }
}

#[derive(Debug)]
pub struct SlabAllocator<T> {
    items: Vec<*mut T>,    // holds pointers to allocations
    alloc: Cell<usize>,    // number of outstanding items
    capacity: Cell<usize>, // number of items pre-allocated (valid in items)
    chunks: Vec<*mut T>,   // holds pointers to each chunk for freeing
}

impl<T> SlabAllocator<T> {
    pub fn new(initial_size: usize) -> SlabAllocator<T> {
        let mut allocator = SlabAllocator {
            items: Vec::with_capacity(initial_size),
            alloc: Cell::new(0),
            capacity: Cell::new(0),
            chunks: Vec::with_capacity(20),
        };

        allocator.expand(initial_size);
        allocator
    }

    // pre-allocates and additional new_items and adds them to the end of
    // self.items, increasing self.capacity with the new size
    fn expand(&mut self, new_items: usize) {
        unsafe {
            let memory = malloc((mem::size_of::<T>() * new_items) as size_t) as *mut T;
            assert!(!memory.is_null());

            self.chunks.push(memory);
            for i in 0..new_items as isize {
                self.items.push(memory.offset(i));
            }
        }

        self.capacity.set(self.capacity.get() + new_items);
    }

    pub unsafe fn dirty_alloc<'r>(&'r self) -> SlabBox<'r, T> {
        self.all_alloc(None)
    }

    pub fn alloc<'r>(&'r self, value: T) -> SlabBox<'r, T> {
        self.all_alloc(Some(value))
    }

    fn all_alloc<'r>(&'r self, value: Option<T>) -> SlabBox<'r, T> {
        let (alloc, capacity) = (self.alloc.get(), self.capacity.get());
        if alloc >= capacity {
            unsafe {
                // is there a safe/better way to do something like this?
                let mut_self: &mut SlabAllocator<T> = transmute(self);
                mut_self.expand(capacity * 2 - capacity);
            }
        }

        let ptr: *mut T = *self.items.get(alloc).unwrap();
        match value {
            Some(val) => unsafe {
                ptr::write(&mut *ptr, val);
            },
            None => { /* Giving back dirty value. */ }
        }

        self.alloc.set(alloc + 1);
        let slab = Slab {
            parent: self,
            ptr: ptr,
        };
        SlabBox(Rc::new(RefCell::new(slab)))
    }

    fn free(&self, ptr: *mut T) {
        let alloc = self.alloc.get();
        if alloc <= 0 {
            panic!("Over-freeing....somehow");
        }

        self.alloc.set(alloc - 1);
        unsafe {
            // Dropping if needed
            // if intrinsics::needs_drop::<T>() {
            //     //let ty = intrinsics::get_tydesc::<T>();
            //     let ty = intrinsics::type_id::<T>();
            //     (ty.drop_glue)(ptr as i8);
                
            // }

            // Letting an immutable slice be mutable, unsafely
            let items: &mut [*mut T] = transmute(self.items.as_slice());
            items[alloc - 1] = ptr;
        }
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.alloc.get(), self.capacity.get())
    }
}

impl<T> Drop for SlabAllocator<T> {
    fn drop(&mut self) {
        for chunk in self.chunks.iter() {
            unsafe {
                free(*chunk as *mut c_void);
            }
        }
    }
}
