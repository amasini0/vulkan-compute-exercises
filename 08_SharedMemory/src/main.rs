use anyhow::Result;
use ash::{vk, Entry};
use std::env;

fn main() -> Result<()> {
    let program = env::args().into_iter().next().unwrap();
    println!("{} starting...\n", program);

    // Create compute context.
    let entry = unsafe { Entry::load() }?;
    let app_name = c"Task 08";
    let api_version = vk::make_api_version(0, 1, 4, 0);
    let context = framework::setup_compute_context(&entry, app_name, api_version, &[], &[])?;

    // Print selected physical device name.
    let instance = &context.instance;
    let device_props = unsafe { instance.get_physical_device_properties(context.physical_device) };
    println!("Device name: {:?}\n", device_props.device_name_as_c_str()?);

    // Print a warning to remind activation of debug printf.
    println!("============================= WARNING ==============================");
    println!("If you can't see any message printed below, make sure the Validation");
    println!("layer is enabled in Vulkan configurator, and Debug Printf is active.");
    println!("====================================================================\n");

    // Create compute pipeline.
    let source_file = format!(
        "{}/communicate.spv",
        env::var("OUT_DIR").unwrap_or(String::from("."))
    );
    let pipeline = framework::create_compute_pipeline(&context, &source_file, &[])?;

    // Create command buffer.
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe { context.allocate_command_buffers(&allocate_info) }?
    };

    // Register commands and submit for execution.
    unsafe {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let command_buffer = command_buffers[0];
        let bind_point = vk::PipelineBindPoint::COMPUTE;

        context.begin_command_buffer(command_buffer, &begin_info)?;
        context.cmd_bind_pipeline(command_buffer, bind_point, pipeline.handle);
        context.cmd_dispatch(command_buffer, 1, 1, 1);
        context.end_command_buffer(command_buffer)?;

        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
        context.queue_submit(context.queue, &[submit_info], vk::Fence::null())?;
        context.device_wait_idle()?;
    }

    Ok(())
}

/*
==================================== Task 8 ====================================
1) Complete the shader file communicate.comp to use shared memory.
2) Optional: Experiment! What happens if you skip synchronization barriers? Why?
Can you find a reproducible setup where you get wrong results if you skip them?
*/
