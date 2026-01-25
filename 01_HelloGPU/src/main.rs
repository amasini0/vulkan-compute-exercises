use anyhow::Result;
use ash::vk;
use framework;
use std::env;

fn main() -> Result<()> {
    let program = env::args().next().unwrap(); // first argument always available
    println!("\n{} starting...\n", program);

    // Setup a compute context.
    let context = {
        let app_name = c"Task 1";
        let api_version = vk::make_api_version(0, 1, 4, 0);
        framework::setup_compute_context(app_name, api_version, &[], &[])?
    };

    // Print selected physical device name.
    let instance = &context.instance;
    let device_props = unsafe { instance.get_physical_device_properties(context.physical_device) };
    println!("Device name: {:?}\n", device_props.device_name_as_c_str());

    // Print a warning to remind activation of debug printf.
    println!("============================= WARNING ==============================");
    println!("If you can't see any message printed below, make sure the Validation");
    println!("layer is enabled in Vulkan configurator, and Debug Printf is active.");
    println!("====================================================================\n");

    // Set up a simple compute pipeline.
    let device = &context.device;
    let source_file = format!("{}/hello.spv", env::var("OUT_DIR")?);
    let descriptor_set_layouts = [{
        let create_info = vk::DescriptorSetLayoutCreateInfo::default();
        unsafe { device.create_descriptor_set_layout(&create_info, None)? }
    }; 1];

    let pipeline =
        framework::setup_compute_pipeline(device, &source_file, &descriptor_set_layouts)?;

    // Allocate a command buffer from the command pool.
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe { device.allocate_command_buffers(&allocate_info)? }
    };

    // Register commands in the command buffer, submit the command buffer to the compute queue,
    // then wait for completion on device.
    unsafe {
        let begin_info = vk::CommandBufferBeginInfo::default();
        let command_buffer = command_buffers[0];

        device.begin_command_buffer(command_buffer, &begin_info)?;
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            pipeline.handle,
        );
        device.cmd_dispatch(command_buffer, 4, 1, 1);
        device.end_command_buffer(command_buffer)?;

        let submit_infos = [vk::SubmitInfo::default().command_buffers(&command_buffers); 1];
        device.queue_submit(context.queue, &submit_infos, vk::Fence::null())?;
        device.device_wait_idle()?;
    }

    // Destroy manually created objects.
    unsafe {
        device.destroy_descriptor_set_layout(descriptor_set_layouts[0], None);
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
