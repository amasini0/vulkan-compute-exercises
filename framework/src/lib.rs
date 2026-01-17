use anyhow::{Error, Result};
use ash::{Device, Entry, Instance, vk};
use std::ffi::{CStr, c_char};
use std::fs::File;

/// Handles for most relevant Vulkan objects
pub struct VulkanObjects {
    pub instance: Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub queue: vk::Queue,
    pub command_pool: vk::CommandPool,
}

/// Handles for pipeline objects
#[derive(Debug)]
pub struct Pipeline {
    pub handle: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub cache: vk::PipelineCache,
    pub shader_module: vk::ShaderModule,
}

fn load_shader(source_file: &str) -> Result<Vec<u32>> {
    // Open file stream
    let mut file = match File::open(source_file) {
        Ok(file) => file,
        Err(_) => return Err(Error::msg(format!("Failed to open file {}", source_file))),
    };

    ash::util::read_spv(&mut file)
        .map_err(|_| Error::msg(format!("Failed to read spirv from file {}", source_file)))
}

/// Performs all required setup to initialize a compute queue. Returns the associated Vulkan handles.
pub fn setup_basic_compute(
    app_name: &CStr,
    api_version: u32,
    instance_extensions: Option<&[*const c_char]>,
    device_extensions: Option<&[*const c_char]>,
) -> Result<VulkanObjects> {
    // Entrypoint
    let entry = Entry::linked();

    // Create a unique instance. For that, prepare an app info with the given name and API version.
    // It should enable all extensions provided in extensions_instance.
    let instance = {
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name)
            .api_version(api_version);

        let create_info = match instance_extensions {
            Some(extensions) => vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(extensions),
            None => vk::InstanceCreateInfo::default().application_info(&app_info),
        };

        unsafe { entry.create_instance(&create_info, None)? }
    };

    // Select a physical device. It should be a device that supports AT LEAST the given API version.
    // Furthermore, it should have at least one queue family that supports both COMPUTE and TRANSFER.
    let devices = unsafe { instance.enumerate_physical_devices()? };

    let find_suitable_queue_family =
        |device: &vk::PhysicalDevice| -> Option<(vk::PhysicalDevice, usize)> {
            let num_families = unsafe {
                instance.get_physical_device_queue_family_properties2_len(device.clone())
            };
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

    let (physical_device, qfam_idx) =
        match devices.iter().filter_map(find_suitable_queue_family).next() {
            Some((physical_device, qfam_idx)) => (physical_device, qfam_idx),
            None => return Err(Error::msg("No suitable device found")),
        };

    // Create a unique (logical) device from the physical device.
    // The device should be created with a single queue from the family you identified above. The
    // queue priority should be 1.0.
    // The device should enable all extensions listed in extensions_device.
    let (device, queue) = {
        let queue_priorities = [1.0f32; 1];
        let queue_create_info = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(qfam_idx as u32)
            .queue_priorities(&queue_priorities); 1];

        let device_create_info = match device_extensions {
            Some(extensions) => vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_info)
                .enabled_extension_names(extensions),
            None => vk::DeviceCreateInfo::default().queue_create_infos(&queue_create_info),
        };

        let device = unsafe {
            instance
                .create_device(physical_device.clone(), &device_create_info, None)
                .map_err(|_| Error::msg("Failed to create logical device"))?
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

    Ok(VulkanObjects {
        instance,
        physical_device,
        device,
        queue,
        command_pool,
    })
}

///
pub fn setup_compute_pipeline(
    device: Device,
    source_file: &str,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> Result<Pipeline> {
    // Create a unique shader module. Use the load_shader() function to load compiled SPIR-V code
    // from the shader source file.
    let shader_module = {
        let shader_code = load_shader(source_file)?;
        let create_info = vk::ShaderModuleCreateInfo::default().code(shader_code.as_slice());
        unsafe { device.create_shader_module(&create_info, None)? }
    };

    // Create a unique pipeline layout using the incoming descriptor set layouts to configure it
    // for accessing descriptors (shader input resources).
    let pipeline_layout = {
        let create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(descriptor_set_layouts);
        unsafe { device.create_pipeline_layout(&create_info, None)? }
    };

    // Create a unique pipeline cache.
    let pipeline_cache = {
        let create_info = vk::PipelineCacheCreateInfo::default();
        unsafe { device.create_pipeline_cache(&create_info, None)? }
    };

    // Create a unique pipeline.
    // First prepare a shader stage info. It should be a compute stage that uses the shader module
    // you made, using the function named "main" as an entry point.
    // Then prepare a pipeline create info that uses the stage info and the above pipeline layout.
    // Finally, create a pipeline with the above pipeline cache and this create info.
    let pipeline = {
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(c"main");

        let create_infos = [vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(pipeline_layout); 1];

        match unsafe { device.create_compute_pipelines(pipeline_cache, &create_infos, None) } {
            Ok(pipelines) => pipelines[0],
            Err(_) => return Err(Error::msg("Failed to create compute pipeline")),
        }
    };

    Ok(Pipeline {
        handle: pipeline,
        layout: pipeline_layout,
        cache: pipeline_cache,
        shader_module,
    })
}
