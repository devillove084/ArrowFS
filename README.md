RustFS
======

ArrowFS is a virtual file system written completely in Rust.

Directory Structure
-------------------
* bench/
  * bench.rs _The benchmarks._

* libbench/lib.rs _The benchmarking library._

* libslab/lib.rs _The slab allocator library._

* src/
  * directory.rs _Insert/Remove/Get directory method implementations._
  * file.rs _FileHandle implementation and structure definitions._
  * inode.rs _Inode structure and implementation._
  * proc.rs _Proc structure (which wraps everything) and implementation._
