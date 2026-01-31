use anyhow::{Result, anyhow};
use ash::{Entry, vk};
use framework;
use std::env;
use std::slice;

fn main() -> Result<()> {
    let program = env::args().into_iter().next().unwrap();
    println!("\n{} starting...\n", program);

    // Set up a compute context.
    let entry = unsafe { Entry::load()? };
    let context = {
        let app_name = c"Task 3";
        let api_version = vk::make_api_version(0, 1, 4, 0);
        framework::setup_compute_context(&entry, app_name, api_version, &[], &[])?
    };

    // Print selected physical device name.
    let instance = &context.instance;
    let device_props = unsafe { instance.get_physical_device_properties(context.physical_device) };
    println!("Device name: {:?}\n", device_props.device_name_as_c_str()?);

    // Create a unique descriptor set layout.
    // It should have a single storage buffer at some binding index.
    let descriptor_set_layouts = [{
        let layout_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE); 1];
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        unsafe { context.create_descriptor_set_layout(&create_info, None)? }
    }; 1];

    // Create pipeline from source file and descriptor set layouts
    let pipeline_objects = {
        let source_file = format!("{}/fibonacci.spv", env::var("OUT_DIR")?);
        framework::create_compute_pipeline(&context, &source_file, &descriptor_set_layouts)?
    };

    // We will want a resource to put in our descriptor set. A single storage buffer is needed.
    // The buffer should have enough room to store 32 integers. The memory we use for the buffer
    // should device-local, but also be visible to the host and host-coherent so we can write to it
    // from the CPU. Once the buffer is created, you will have to (in addition):
    // 1) Get the buffer's memory requirements (a struct)
    // 2) Find a memory type that fulfills the requirements (you can use the code below)
    // 3) Allocate a large enough chunk of it to store the data
    // 4) Bind the memory to the buffer
    let buffer_length = 32;
    let (buffer_handle, buffer_memory) = {
        let buffer_size = (buffer_length * size_of::<i32>()) as vk::DeviceSize;
        let create_info = vk::BufferCreateInfo::default()
            .size(buffer_size as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer_handle = unsafe { context.create_buffer(&create_info, None)? };

        // Find a suitable memory type for the buffer.
        let buffer_mem_reqs = unsafe { context.get_buffer_memory_requirements(buffer_handle) };
        let buffer_mem_flags = vk::MemoryPropertyFlags::DEVICE_LOCAL
            | vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT;
        let mem_props =
            unsafe { instance.get_physical_device_memory_properties(context.physical_device) };
        let mem_type_idx = framework::find_memory_type_idx(
            &mem_props.memory_types,
            buffer_mem_reqs,
            buffer_mem_flags,
        )
        .ok_or(anyhow!(
            "No suitable memory type for buffer allocation found."
        ))?;

        // Allocate memory and bind it to the buffer handle.
        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(buffer_mem_reqs.size)
            .memory_type_index(mem_type_idx);
        let buffer_memory = unsafe { context.allocate_memory(&allocate_info, None)? };
        unsafe { context.bind_buffer_memory(buffer_handle, buffer_memory, 0)? };

        (buffer_handle, buffer_memory)
    };

    // We will want a pool that can provide 1 descriptor set and 1 storage buffer, nothing else.
    //
    // Allocate a single unique descriptor set from your descriptor pool, with the descriptor set
    // layout you defined above.
    // Connect your buffer to your descriptor set by preparing the necessary resource info, write
    // struct and executing the update.
    let descriptor_pool = {
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .descriptor_count(1)
            .ty(vk::DescriptorType::STORAGE_BUFFER); 1];
        let create_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);
        unsafe { context.create_descriptor_pool(&create_info, None)? }
    };

    let descriptor_sets = {
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        unsafe { context.allocate_descriptor_sets(&allocate_info)? }
    };

    {
        let buffer_infos = [vk::DescriptorBufferInfo::default()
            .buffer(buffer_handle)
            .offset(0)
            .range(vk::WHOLE_SIZE); 1];
        let descriptor_writes = [vk::WriteDescriptorSet::default()
            .dst_set(descriptor_sets[0])
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_infos); 1];
        unsafe { context.update_descriptor_sets(&descriptor_writes, &[]) };
    }

    // Allocate a command buffer from the command pool.
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe { context.allocate_command_buffers(&allocate_info)? }
    };

    // Register commands in the command buffer, submit the command buffer to the compute queue,
    // then wait for completion on device.
    unsafe {
        let begin_info = vk::CommandBufferBeginInfo::default();
        let command_buffer = command_buffers[0];
        let bind_point = vk::PipelineBindPoint::COMPUTE;
        let pipeline_handle = pipeline_objects.handle;
        let pipeline_layout = pipeline_objects.layout;
        let buffer_memory_barriers = [vk::BufferMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ)
            .buffer(buffer_handle)
            .offset(0)
            .size(vk::WHOLE_SIZE); 1];

        context.begin_command_buffer(command_buffer, &begin_info)?;
        context.cmd_bind_pipeline(command_buffer, bind_point, pipeline_handle);
        context.cmd_bind_descriptor_sets(
            command_buffer,
            bind_point,
            pipeline_layout,
            0,
            &descriptor_sets,
            &[],
        );
        context.cmd_dispatch(command_buffer, 1, 1, 1);
        context.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::HOST,
            vk::DependencyFlags::default(),
            &[],
            &buffer_memory_barriers,
            &[],
        );
        context.end_command_buffer(command_buffer)?;

        let submit_infos = [vk::SubmitInfo::default().command_buffers(&command_buffers); 1];
        context.queue_submit(context.queue, &submit_infos, vk::Fence::null())?;
        context.device_wait_idle()?;
    }

    // Access buffer data on host.
    unsafe {
        let map_flags = vk::MemoryMapFlags::default();
        slice::from_raw_parts(
            context
                .map_memory(buffer_memory, 0, vk::WHOLE_SIZE, map_flags)?
                .cast::<i32>(),
            buffer_length,
        )
    }
    .iter()
    .enumerate()
    .for_each(|(n, fib)| println!("{:>4} : {:<}", n, fib));
    println!();

    // Destroy manually created objects.
    unsafe {
        context.destroy_descriptor_pool(descriptor_pool, None);
        context.destroy_buffer(buffer_handle, None);
        context.free_memory(buffer_memory, None);
        context.destroy_descriptor_set_layout(descriptor_set_layouts[0], None);
    }

    Ok(())
}
