use anyhow::{Result, anyhow};
use ash::vk;
use framework::{self, VulkanObjects};
use std::env;
use std::slice;

fn main() -> Result<()> {
    let program = env::args().into_iter().next().unwrap();
    println!("\n{} starting...\n", program);

    let VulkanObjects {
        instance,
        physical_device,
        device,
        queue,
        command_pool,
    } = {
        let app_name = c"Task 3";
        let api_version = vk::make_api_version(0, 1, 4, 0);
        framework::setup_basic_compute(&app_name, api_version, &[], &[])?
    };

    // Create a unique descriptor set layout.
    // It should have a single storage buffer at some binding index.
    let descriptor_set_layouts = [{
        let layout_bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE); 1];
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        unsafe { device.create_descriptor_set_layout(&create_info, None)? }
    }; 1];

    // Create pipeline from source file and descriptor set layouts
    let pipeline_objects = {
        let source_file = format!("{}/fibonacci.spv", env::var("OUT_DIR")?);
        framework::setup_compute_pipeline(device.clone(), &source_file, &descriptor_set_layouts)?
    };

    // TODO: We will want a resource to put in our descriptor set. A single storage buffer is needed.
    // The buffer should have enough room to store 32 integers. The memory we use for the buffer
    // should device-local, but also be visible to the host and host-coherent so we can write to it
    // from the CPU. Once the buffer is created, you will have to (in addition):
    // 1) Get the buffer's memory requirements (a struct)
    // 2) Find a memory type that fulfills the requirements (you can use the code below)
    // 3) Allocate a large enough chunk of it to store the data
    // 4) Bind the memory to the buffer
    let buffer_length = 32;
    let buffer_size = buffer_length * size_of::<i32>();
    let (buffer_handle, buffer_memory) = {
        // Create buffer object
        let create_info = vk::BufferCreateInfo::default()
            .size(buffer_size as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer_handle = unsafe { device.create_buffer(&create_info, None)? };

        // Find suitable memory type for buffer
        let buffer_mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer_handle) };
        let buffer_mem_flags = vk::MemoryPropertyFlags::DEVICE_LOCAL
            | vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT;
        let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let mem_type_idx = framework::find_memory_type_idx(
            &mem_props.memory_types,
            buffer_mem_reqs,
            buffer_mem_flags,
        )
        .ok_or(anyhow!(
            "No suitable memory type for buffer allocation found."
        ))?;

        // Allocate memory and bind to buffer handle
        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(buffer_size as vk::DeviceSize)
            .memory_type_index(mem_type_idx);
        let buffer_memory = unsafe { device.allocate_memory(&allocate_info, None)? };
        unsafe { device.bind_buffer_memory(buffer_handle, buffer_memory, 0)? };

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
        unsafe { device.create_descriptor_pool(&create_info, None)? }
    };

    let descriptor_sets = {
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        unsafe { device.allocate_descriptor_sets(&allocate_info)? }
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
        unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };
    }

    // Create command buffer and dispatch compute shader
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe { device.allocate_command_buffers(&allocate_info)? }
    };

    unsafe {
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

        let begin_info = vk::CommandBufferBeginInfo::default();
        device.begin_command_buffer(command_buffer, &begin_info)?;
        device.cmd_bind_pipeline(command_buffer, bind_point, pipeline_handle);
        device.cmd_bind_descriptor_sets(
            command_buffer,
            bind_point,
            pipeline_layout,
            0,
            &descriptor_sets,
            &[],
        );
        device.cmd_dispatch(command_buffer, 1, 1, 1);
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::HOST,
            vk::DependencyFlags::default(),
            &[],
            &buffer_memory_barriers,
            &[],
        );
        device.end_command_buffer(command_buffer)?;

        let submit_infos = [vk::SubmitInfo::default().command_buffers(&command_buffers); 1];
        device.queue_submit(queue, &submit_infos, vk::Fence::null())?;
        device.device_wait_idle()?;
    }

    // Access data on host
    unsafe {
        let map_flags = vk::MemoryMapFlags::default();
        slice::from_raw_parts(
            device
                .map_memory(buffer_memory, 0, vk::WHOLE_SIZE, map_flags)?
                .cast::<i32>(),
            buffer_length,
        )
    }
    .iter()
    .enumerate()
    .for_each(|(n, fib)| println!("{:<2} : {:<}", n, fib));

    Ok(())
}
