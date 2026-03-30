use anyhow::{anyhow, Result};
use ash::{vk, Device, Entry, Instance};
use std::ffi::{c_char, CStr};
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

/// Sets up a GPU compute context on the first available physical device that supports it.
/// If successful, returns a Context structure containing all Vulkan context-related objects.
pub fn setup_compute_context<'a>(
    entry: &'a Entry,
    app_name: &CStr,
    api_version: u32,
    instance_extensions: &[*const c_char],
    device_extensions: &[*const c_char],
) -> Result<Context<'a>> {
    // Create a unique instance. For that, prepare an app info with the given name and API version.
    // It should enable all extensions provided in extensions_instance.
    let instance = {
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name)
            .api_version(api_version);
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(instance_extensions);
        unsafe { entry.create_instance(&create_info, None)? }
    };

    // Select a physical device. It should be a device that supports AT LEAST the given API version.
    // Furthermore, it should have at least one queue family that supports both COMPUTE and TRANSFER.
    let devices = unsafe { instance.enumerate_physical_devices()? };

    let find_suitable_queue_family =
        |device: vk::PhysicalDevice| -> Option<(vk::PhysicalDevice, usize)> {
            let num_families =
                unsafe { instance.get_physical_device_queue_family_properties2_len(device) };
            let mut queue_families = vec![vk::QueueFamilyProperties2::default(); num_families];
            unsafe {
                instance.get_physical_device_queue_family_properties2(
                    device.clone(),
                    &mut queue_families,
                )
            };

            for (qfam_idx, qfam_props2) in queue_families.iter().enumerate() {
                let flags = qfam_props2.queue_family_properties.queue_flags;
                if flags.contains(vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER) {
                    return Some((device.clone(), qfam_idx));
                }
            }
            None
        };

    let (physical_device, qfam_idx) = devices
        .into_iter()
        .filter_map(find_suitable_queue_family)
        .next()
        .ok_or(anyhow!("No suitable device found"))?;

    // Create a unique (logical) device from the physical device.
    // The device should be created with a single queue from the family you identified above. The
    // queue priority should be 1.0.
    // The device should enable all extensions listed in extensions_device.
    let (device, queue) = {
        let queue_priorities = [1.0f32; 1];
        let queue_create_info = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(qfam_idx as u32)
            .queue_priorities(&queue_priorities); 1];

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_info)
            .enabled_extension_names(device_extensions);

        let device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(|_| anyhow!("Failed to create logical device"))?
        };

        let queue = unsafe { device.get_device_queue(qfam_idx as u32, 0) };

        (device, queue)
    };

    // Create a unique command pool. Use the queue family index you found above.
    // Make sure that buffers allocated from this command pool can be reset (i.e., recorded multiple times).
    // This is achieved by adding the vk::CommandPoolCreateFlagBits::eResetCommandBuffer flag to the
    // command pool create info.
    let command_pool = {
        let create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(qfam_idx as u32);

        unsafe { device.create_command_pool(&create_info, None)? }
    };

    Ok(Context {
        _entry: entry,
        instance,
        physical_device,
        device,
        queue,
        command_pool,
    })
}
