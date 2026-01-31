use ash::{Device, Entry, Instance, vk};
use std::ops::Deref;

/// Handles for GPU context-related objects
pub struct Context<'a> {
    pub(super) _entry: &'a Entry,
    pub instance: Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub queue: vk::Queue,
    pub command_pool: vk::CommandPool,
}

impl<'a> Drop for Context<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

impl<'a> Deref for Context<'a> {
    type Target = Device;
    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
