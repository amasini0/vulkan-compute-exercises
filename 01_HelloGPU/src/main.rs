use anyhow::Result;
use ash::vk;
use framework::{self, VulkanObjects};
use std::env;

fn main() -> Result<()> {
    let program = env::args().next().unwrap(); // first argument always available
    println!("\n{} starting...\n", program);

    // Set up Vulkan environment
    let VulkanObjects {
        instance,
        physical_device,
        device,
        queue,
        command_pool,
    } = {
        let app_name = c"Task 1";
        let api_version = vk::make_api_version(0, 1, 4, 0);
        framework::setup_basic_compute(app_name, api_version, &[], &[])?
    };

    // Print device name
    let mut device_props2 = vk::PhysicalDeviceProperties2::default();
    unsafe { instance.get_physical_device_properties2(physical_device, &mut device_props2) };

    let name = device_props2.properties.device_name_as_c_str()?;
    println!("Device name: {:?}", name);

    // Add warning for debug printf
    println!("\n============================= WARNING ==============================");
    println!("If you can't see any message printed below, make sure the Validation");
    println!("layer is enabled in Vulkan configurator, and Debug Printf is active.");
    println!("====================================================================\n");

    // Create compute pipeline
    let source_file = format!("{}/hello.spv", env::var("OUT_DIR")?);
    let descriptor_set_layouts = [{
        let create_info = vk::DescriptorSetLayoutCreateInfo::default();
        unsafe { device.create_descriptor_set_layout(&create_info, None)? }
    }; 1];

    let pipeline =
        framework::setup_compute_pipeline(device.clone(), &source_file, &descriptor_set_layouts)?;

    // Create command buffer and register commands
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        unsafe { device.allocate_command_buffers(&allocate_info)? }
    };

    // Register commands in command buffer and dispatch
    unsafe {
        device.begin_command_buffer(command_buffers[0], &vk::CommandBufferBeginInfo::default())?;
        device.cmd_bind_pipeline(
            command_buffers[0],
            vk::PipelineBindPoint::COMPUTE,
            pipeline.handle,
        );
        device.cmd_dispatch(command_buffers[0], 4, 1, 1);
        device.end_command_buffer(command_buffers[0])?;
    }

    // Submit command buffer to queue
    unsafe {
        let submit_infos = [vk::SubmitInfo::default().command_buffers(&command_buffers); 1];
        device.queue_submit(queue, &submit_infos, vk::Fence::null())?;
        device.device_wait_idle()?;
    }

    Ok(())
}

/*
==================================== Task 1 ====================================
1) Implement the function framework::setup_basic_compute in framework crate.
2) Implement the function Framework::setup_compute_pipeline in framework crate.
3) Complete the shader file hello.slang to print a basic message.
4) Optional: Experiment with different dispatch/work group configurations. There
is a way to actually set the work group size from the host side! You can try to
do this, but you will probably need your own pipeline creation function for this.
*/
