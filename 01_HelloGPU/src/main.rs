use anyhow::Result;
use ash::vk;
use framework;
use std::env;

fn main() -> Result<()> {
    let program = env::args().next().unwrap(); // first argument always available
    println!("\n{} starting...\n", program);

    // Set up Vulkan environment
    let vulkan_objects = {
        let app_name = c"Task 1";
        let api_version = vk::make_api_version(0, 1, 4, 0);
        framework::setup_basic_compute(app_name, api_version, None, None)?
    };

    // Print device name
    let instance = vulkan_objects.instance;
    let physical_device = vulkan_objects.physical_device;
    let mut device_props2 = vk::PhysicalDeviceProperties2::default();
    unsafe { instance.get_physical_device_properties2(physical_device, &mut device_props2) };

    let name = device_props2.properties.device_name_as_c_str()?;
    println!("Device name: {:?}", name);

    // Create compute pipeline
    let device = vulkan_objects.device;
    let source_file = format!("{}/hello.spv", env::var("OUT_DIR")?);
    let descriptor_set_layouts = [{
        let create_info = vk::DescriptorSetLayoutCreateInfo::default();
        unsafe { device.create_descriptor_set_layout(&create_info, None)? }
    }; 1];

    let pipeline_objects =
        framework::setup_compute_pipeline(device.clone(), &source_file, &descriptor_set_layouts)?;

    // Create command buffer and register commands
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(vulkan_objects.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        unsafe { device.allocate_command_buffers(&allocate_info)? }
    };

    // Register commands in command buffer and dispatch
    let pipeline = pipeline_objects.pipeline;
    unsafe {
        device.begin_command_buffer(command_buffers[0], &vk::CommandBufferBeginInfo::default())?;
        device.cmd_bind_pipeline(command_buffers[0], vk::PipelineBindPoint::COMPUTE, pipeline);
        device.cmd_dispatch(command_buffers[0], 4, 1, 1);
        device.end_command_buffer(command_buffers[0])?;
    }

    // Submit command buffer to queue
    let submit_info = [vk::SubmitInfo::default(); 1];
    unsafe {
        let create_info = vk::FenceCreateInfo::default();
        let fence = device.create_fence(&create_info, None)?;
        device.queue_submit(vulkan_objects.queue, &submit_info, fence)?;
        device.wait_for_fences(&[fence; 1], true, 10000)?;
    }

    Ok(())
}

/*
==================================== Task 1 ====================================
1) Implement the function Framework::setupBasicCompute in framework.cpp.
2) Implement the function Framework::setupCompuePipeline in framework.cpp.
3) Complete the shader file hello.comp to print a basic message.
4) Optional: Experiment with different dispatch/work group configurations. There
is a way to actually set the work group size from the C++ side! You can try to
do this, but you will probably need your own pipeline creation function for this.
*/
