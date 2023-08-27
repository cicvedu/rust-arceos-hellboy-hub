//! Simple memory allocation.
//!
//! TODO: more efficient

// use core::alloc::Layout;
// use core::num::NonZeroUsize;

// use crate::{AllocResult, BaseAllocator, ByteAllocator};

// pub struct SimpleByteAllocator;

// impl SimpleByteAllocator {
//     pub const fn new() -> Self {
//         Self {}
//     }
// }

// impl BaseAllocator for SimpleByteAllocator {
//     fn init(&mut self, _start: usize, _size: usize) {
//         todo!();
//     }

//     fn add_memory(&mut self, _start: usize, _size: usize) -> AllocResult {
//         todo!();
//     }
// }

// impl ByteAllocator for SimpleByteAllocator {
//     fn alloc(&mut self, _layout: Layout) -> AllocResult<NonZeroUsize> {
//         todo!();
//     }

//     fn dealloc(&mut self, _pos: NonZeroUsize, _layout: Layout) {
//         todo!();
//     }

//     fn total_bytes(&self) -> usize {
//         todo!();
//     }

//     fn used_bytes(&self) -> usize {
//         todo!();
//     }

//     fn available_bytes(&self) -> usize {
//         todo!();
//     }
// }

/********************************************************** */

use super::{AllocError, AllocResult, BaseAllocator, ByteAllocator};
use core::alloc::Layout;
use core::num::NonZeroUsize;
use slab_allocator::Heap;

/// A byte-granularity memory allocator based on the [slab allocator].
///
/// [slab allocator]: ../slab_allocator/index.html
pub struct SimpleByteAllocator {
    inner: Option<Heap>,
}

impl SimpleByteAllocator {
    /// Creates a new empty `SimpleByteAllocator`.
    pub const fn new() -> Self {
        Self { inner: None }
    }

    fn inner_mut(&mut self) -> &mut Heap {
        self.inner.as_mut().unwrap()
    }

    fn inner(&self) -> &Heap {
        self.inner.as_ref().unwrap()
    }
}

impl BaseAllocator for SimpleByteAllocator {
    fn init(&mut self, start: usize, size: usize) {
        self.inner = unsafe { Some(Heap::new(start, size)) };
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        unsafe {
            self.inner_mut().add_memory(start, size);
        }
        Ok(())
    }
}

impl ByteAllocator for SimpleByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonZeroUsize> {
        self.inner_mut()
            .allocate(layout)
            .map(|addr| NonZeroUsize::new(addr).unwrap())
            .map_err(|_| AllocError::NoMemory)
    }

    fn dealloc(&mut self, pos: NonZeroUsize, layout: Layout) {
        unsafe { self.inner_mut().deallocate(pos.get(), layout) }
    }

    fn total_bytes(&self) -> usize {
        self.inner().total_bytes()
    }

    fn used_bytes(&self) -> usize {
        self.inner().used_bytes()
    }

    fn available_bytes(&self) -> usize {
        self.inner().available_bytes()
    }
}
