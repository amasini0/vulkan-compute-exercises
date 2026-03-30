use anyhow::{anyhow, Result};
use ash::vk;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::slice::from_raw_parts_mut;
use vk_mem::{self, Alloc};

/// Represents a memory buffer that can be used
pub struct Buffer<'a, T> {
    allocator: &'a vk_mem::Allocator,
    handle: vk::Buffer,
    memory: RefCell<vk_mem::Allocation>,
    size: usize,
    phantom: PhantomData<T>,
}

impl<'a, T> Buffer<'a, T> {
    /// Creates a new buffer of given size and type, and allocates the required memory using
    /// the provided allocator.
    pub fn new_shared(
        allocator: &'a vk_mem::Allocator,
        size: usize,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size((size * size_of::<T>()) as vk::DeviceSize)
            .usage(usage);

        let memory_flags = vk::MemoryPropertyFlags::DEVICE_LOCAL
            | vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT;

        let allocation_create_info = vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
            usage: vk_mem::MemoryUsage::Auto,
            required_flags: memory_flags,
            ..Default::default()
        };

        let (handle, memory) =
            unsafe { allocator.create_buffer(&buffer_create_info, &allocation_create_info) }?;

        Ok(Buffer::<T> {
            allocator,
            handle,
            memory: RefCell::new(memory),
            size,
            phantom: PhantomData::default(),
        })
    }

    pub fn handle(&self) -> vk::Buffer {
        return self.handle;
    }

    pub fn map(&self) -> Result<*const T> {
        match unsafe {
            self.allocator
                .map_memory(self.memory.borrow_mut().deref_mut())
        } {
            Ok(ptr) => Ok(ptr.cast::<T>().cast_const()),
            Err(e) => Err(anyhow!("{:#}", e)),
        }
    }

    pub fn map_mut(&self) -> Result<*mut T> {
        match unsafe {
            self.allocator
                .map_memory(self.memory.borrow_mut().deref_mut())
        } {
            Ok(ptr) => Ok(ptr.cast::<T>()),
            Err(e) => Err(anyhow!("{:#}", e)),
        }
    }

    pub fn map_slice(&self) -> Result<&[T]> {
        match self.map() {
            Ok(ptr) => {
                let slice = unsafe { std::slice::from_raw_parts(ptr, self.size) };
                Ok(slice)
            }
            Err(e) => Err(e),
        }
    }

    pub fn map_slice_mut(&self) -> Result<&mut [T]> {
        match self.map_mut() {
            Ok(ptr) => {
                let slice = unsafe { from_raw_parts_mut(ptr, self.size) };
                Ok(slice)
            }
            Err(e) => Err(e),
        }
    }

    pub fn unmap(&self) {
        unsafe {
            self.allocator
                .unmap_memory(self.memory.borrow_mut().deref_mut())
        };
    }
}

impl<'a, T> Drop for Buffer<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.allocator
                .destroy_buffer(self.handle, self.memory.borrow_mut().deref_mut());
        }
    }
}
