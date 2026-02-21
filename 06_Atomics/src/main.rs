use anyhow::{anyhow, Result};
use ash::{vk, Entry};
use std::{env, slice};
use vk_mem;
use vk_mem::Alloc;

fn main() -> Result<()> {
    let program = env::args().into_iter().next().unwrap();
    println!("\n{} starting...\n", program);

    // Initialize Vulkan
    let entry = unsafe { Entry::load() }
        .map_err(|e| anyhow!("{}: failed to load Vulkan entrypoint -- {:#}", program, e))?;

    // Create GPU context and allocator
    let (context, allocator) = {
        let app_name = c"Task 06";
        let api_version = vk::make_api_version(0, 1, 4, 0);
        let context = framework::setup_compute_context(&entry, app_name, api_version, &[], &[])
            .map_err(|e| anyhow!("{}: failed to create compute context -- {:#}", program, e))?;
        let allocator = framework::create_allocator(&context, api_version)
            .map_err(|e| anyhow!("{}: failed to create allocator -- {:#}", program, e))?;
        (context, allocator)
    };

    let num_workgroups = 25; // How many workgroups we will start
    let workgroup_size = 128; // How many threads we expect in each workgroup
    let nums_tested = num_workgroups * workgroup_size; // Total number of threads

    // Description for the resources that go into our pipeline (and descriptor set):
    // - One storage buffer for the counter (size of a single int)
    // - One storage buffer for the output array with primes (size of numTested * size of (int)).
    // - Both of them should be host-visible, host-coherent AND device-local.
    let allocation_create_info = vk_mem::AllocationCreateInfo {
        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
        usage: vk_mem::MemoryUsage::Auto,
        required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT
            | vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ..Default::default()
    };

    let mut counter_buffer = {
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(size_of::<u32>() as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER);
        unsafe { allocator.create_buffer(&buffer_create_info, &allocation_create_info) }
            .map_err(|e| anyhow!("{}: failed to create buffer -- {:#}", program, e))?
    };

    let mut output_buffer = {
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(nums_tested * size_of::<u32>() as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER);
        unsafe { allocator.create_buffer(&buffer_create_info, &allocation_create_info) }
            .map_err(|e| anyhow!("{}: failed to create buffer -- {:#}", program, e))?
    };

    // Take all the necessary steps to have a descriptor set that references the above created
    // buffers. Make sure the bindings match the shader "primes.slang".
    let descriptor_set_layouts = [{
        let layout_bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        unsafe { context.create_descriptor_set_layout(&create_info, None) }.map_err(|e| {
            anyhow!(
                "{}: failed to create descriptor set layout -- {:#}",
                program,
                e
            )
        })?
    }];

    let descriptor_pool = {
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .descriptor_count(2)
            .ty(vk::DescriptorType::STORAGE_BUFFER)];
        let create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(2);
        unsafe { context.create_descriptor_pool(&create_info, None) }
            .map_err(|e| anyhow!("{}: failed to create descriptor pool -- {:#}", program, e))?
    };

    let descriptor_sets = {
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        unsafe { context.allocate_descriptor_sets(&allocate_info) }
            .map_err(|e| anyhow!("{}: failed to allocate descriptor sets -- {:#}", program, e))?
    };

    {
        let buffer_infos = [
            vk::DescriptorBufferInfo::default()
                .buffer(counter_buffer.0)
                .offset(0)
                .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfo::default()
                .buffer(output_buffer.0)
                .offset(0)
                .range(vk::WHOLE_SIZE),
        ];
        let descriptor_writes = [
            vk::WriteDescriptorSet::default()
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .dst_set(descriptor_sets[0])
                .dst_binding(0)
                .buffer_info(&buffer_infos[0..=0]),
            vk::WriteDescriptorSet::default()
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .dst_set(descriptor_sets[0])
                .dst_binding(1)
                .buffer_info(&buffer_infos[1..=1]),
        ];
        unsafe { context.update_descriptor_sets(&descriptor_writes, &[]) };
    }

    // Map and initialize buffer memories.
    // Set the counter, a single integer, to 0.
    // Next, map the output array to a variable "outputMapped". Initialize the entries in the
    // array all to UINT_MAX, or 0xFFFFFFFF (equivalent).
    let counter_mapped = unsafe {
        allocator
            .map_memory(&mut counter_buffer.1)
            .map_err(|e| anyhow!("{}: failed to map buffer memory -- {:#}", program, e))?
            .cast::<u32>()
    };
    unsafe { *counter_mapped = 0 };

    let output_mapped = unsafe {
        allocator
            .map_memory(&mut output_buffer.1)
            .map_err(|e| anyhow!("{}: failed to map buffer memory -- {:#}", program, e))?
            .cast::<u32>()
    };
    unsafe { output_mapped.write_bytes(0xFF, nums_tested as usize) };

    // Create compute pipeline.
    let pipeline = {
        let source_file = format!(
            "{}/primes.spv",
            env::var("OUT_DIR").unwrap_or(String::from("."))
        );
        framework::create_compute_pipeline(&context, &source_file, &descriptor_set_layouts)
            .map_err(|e| anyhow!("{}: failed to create compute pipeline -- {:#}", program, e))?
    };

    // Allocate command buffer and register commands.
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe { context.allocate_command_buffers(&allocate_info) }
            .map_err(|e| anyhow!("{}: failed to allocate command buffers -- {:#}", program, e))?
    };

    unsafe {
        let cmd_buffer = command_buffers[0];
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let bind_point = vk::PipelineBindPoint::COMPUTE;
        let barrier_src = vk::PipelineStageFlags::COMPUTE_SHADER;
        let barrier_dst = vk::PipelineStageFlags::HOST;
        let barrier = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ);

        context.begin_command_buffer(cmd_buffer, &begin_info)?;
        context.cmd_bind_pipeline(cmd_buffer, bind_point, pipeline.handle);
        context.cmd_bind_descriptor_sets(
            cmd_buffer,
            bind_point,
            pipeline.layout,
            0,
            &descriptor_sets,
            &[],
        );
        context.cmd_dispatch(cmd_buffer, num_workgroups as u32, 1, 1);
        context.cmd_pipeline_barrier(
            cmd_buffer,
            barrier_src,
            barrier_dst,
            Default::default(),
            &[barrier],
            &[],
            &[],
        );
        context.end_command_buffer(cmd_buffer)?;
    }

    // Submit command buffer to compute queue.
    unsafe {
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
        context.queue_submit(context.queue, &[submit_info], vk::Fence::null())?;
        context.device_wait_idle()?;
    }

    // Print sorted outputs.
    let num_primes = unsafe { *counter_mapped };
    let primes = unsafe { slice::from_raw_parts_mut(output_mapped, num_primes as usize) };
    primes.sort();

    println!("Tested up to : {:>4}", nums_tested);
    println!("Primes found : {:>4}", num_primes);
    println!("{:=^50}", "");
    primes.iter().enumerate().for_each(|(i, prime)| {
        print!(" {:4}", prime);
        if i % 10 == 9 {
            println!();
        }
    });
    println!("\n{:=^50}", "");

    // Free resources
    unsafe {
        allocator.unmap_memory(&mut output_buffer.1);
        allocator.unmap_memory(&mut counter_buffer.1);

        allocator.destroy_buffer(output_buffer.0, &mut output_buffer.1);
        allocator.destroy_buffer(counter_buffer.0, &mut counter_buffer.1);

        context.free_command_buffers(context.command_pool, &command_buffers);
        context.destroy_descriptor_pool(descriptor_pool, None);
        context.destroy_descriptor_set_layout(descriptor_set_layouts[0], None);
    }
    Ok(())
}

/*
==================================== Task 6 ====================================
1) Create all the resources you need to fill a descriptor set for the shader.
2) Map and initialize the buffer memories in main.rs.
3) Complete the shader file primes.slang to compute and store prime numbers.
4) Optional: Why are the numbers not in order? Sort them on the CPU and print.
*/
