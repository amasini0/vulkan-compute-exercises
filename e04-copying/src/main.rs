use anyhow::Result;
use ash::{Entry, vk};
use framework;
use std::env;
use std::slice;
use vk_mem::Alloc;

fn main() -> Result<()> {
    let program_name = env::args().into_iter().next().unwrap();
    println!("\n{} starting...\n", program_name);

    // Set up a compute context.
    let entry = unsafe { Entry::load()? };
    let api_version = vk::make_api_version(0, 1, 4, 0);
    let context = {
        let app_name = c"Task 4";
        framework::setup_compute_context(&entry, &app_name, api_version, &[], &[])?
    };

    // Print selected physical device name.
    let instance = &context.instance;
    let device_props = unsafe { instance.get_physical_device_properties(context.physical_device) };
    println!("Device name: {:?}\n", device_props.device_name_as_c_str()?);

    // Create memory allocator.
    let allocator = framework::create_allocator(&context, api_version)?;

    // Create four buffers using the Vulkan memory allocator.
    // Buffer A should be of size sizeA all others should be sizeX.
    // All of them should be able to be copied from and copied to.
    // Each buffer also needs a separate allocation (read: memory). Buffer A's allocation should
    // have required flags "propsA" (reachable from host, device-local), all others should have the
    // required flags "propsX" (device-local).
    let size_a = 60 * size_of::<i32>();
    let size_x = 20 * size_of::<i32>();
    let usage_flags = vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC;
    let props_a = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
    let props_x = vk::MemoryPropertyFlags::DEVICE_LOCAL;

    let buffer_create_info_a = vk::BufferCreateInfo::default()
        .size(size_a as vk::DeviceSize)
        .usage(usage_flags)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let allocation_create_info_a = vk_mem::AllocationCreateInfo {
        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
        usage: vk_mem::MemoryUsage::Auto,
        required_flags: props_a,
        ..Default::default()
    };
    let mut buffer_a =
        unsafe { allocator.create_buffer(&buffer_create_info_a, &allocation_create_info_a)? };

    let buffer_create_info_x = vk::BufferCreateInfo::default()
        .size(size_x as vk::DeviceSize)
        .usage(usage_flags)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let allocation_create_info_x = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::Auto,
        required_flags: props_x,
        ..Default::default()
    };
    let mut buffer_b =
        unsafe { allocator.create_buffer(&buffer_create_info_x, &allocation_create_info_x)? };
    let mut buffer_c =
        unsafe { allocator.create_buffer(&buffer_create_info_x, &allocation_create_info_x)? };
    let mut buffer_d =
        unsafe { allocator.create_buffer(&buffer_create_info_x, &allocation_create_info_x)? };

    // The content of this array will be copied across our different buffers
    let magic = [
        1, 0, 11, 13101, 103, 22, 44, 2828, 7, 7, 0, 13, 21, 34, 55, 0, 77, 99, 12, 21, 0, 1, 2, 3,
        5, 6, 7, 8, 9, 4, 10, 11, 12, 13, 610, 987, 1597, 2584, 4181, 6765, 89, 144, 233, 377, 22,
        44, 10101, 30, 0, 0, 7, 7, 12, 144, 2568, 1, 303, 0, 2, 377,
    ];

    // Map the entire host-visible memory of buffer A and write all 60 magic numbers there
    let slice_a: &mut [i32] =
        unsafe { slice::from_raw_parts_mut(allocator.map_memory(&mut buffer_a.1)?.cast(), 60) };
    slice_a.clone_from_slice(&magic);

    // Create our basic command buffer
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe { context.allocate_command_buffers(&allocate_info)? }
    };

    // Register commands in the command buffer, submit the command buffer to the compute queue,
    // then wait for completion on device.
    {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { context.begin_command_buffer(command_buffers[0], &begin_info)? };

        // Distribute the contents of buffer A to the different device-local buffers
        let copy_a2b_0_0_20 = vk::BufferCopy::default()
            .src_offset(0)
            .dst_offset(0)
            .size(20 * size_of::<i32>() as vk::DeviceSize);
        let copy_a2c_20_0_20 = vk::BufferCopy::default()
            .src_offset(20 * size_of::<i32>() as vk::DeviceSize)
            .dst_offset(0)
            .size(20 * size_of::<i32>() as vk::DeviceSize);
        let copy_a2d_40_0_20 = vk::BufferCopy::default()
            .src_offset(40 * size_of::<i32>() as vk::DeviceSize)
            .dst_offset(0)
            .size(20 * size_of::<i32>() as vk::DeviceSize);

        unsafe {
            context.cmd_copy_buffer(
                command_buffers[0],
                buffer_a.0,
                buffer_b.0,
                &[copy_a2b_0_0_20],
            );
            context.cmd_copy_buffer(
                command_buffers[0],
                buffer_a.0,
                buffer_c.0,
                &[copy_a2c_20_0_20],
            );
            context.cmd_copy_buffer(
                command_buffers[0],
                buffer_a.0,
                buffer_d.0,
                &[copy_a2d_40_0_20],
            );
        }

        // We will need barriers to ensure that a copy becomes visible before the next copy starts.
        // Let's create a general memory barrier, which can enforce visibility for certain accesses
        // across ALL device memory.
        // We will use this to enforce that the copies can be seen before we initiate any further
        // copying: transfer write accesses (source) should become visible to all later transfer
        // writes and transfer reads (destination).
        // This basically says that written data must be visible when we read again, but also that
        // writes must be published before we write again!
        // We need the latter because the order of the copies matters to obtain the correct result.
        let transfer_barrier = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ);

        // Prepare another general memory barrier.
        // We will use this one to ensure that all copies are available before we read buffer A
        // from the host.
        // The source should thus be transfer writes, and the destination host reads.
        let host_barrier = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ);

        // Record the following commands to the command buffer in this exact order:
        // 1.   Apply the first memory barrier. This ensures that the above copies can be seen by
        //      later copies.
        //      To apply it, you must invoke pipelineBarrier on the command buffer, describe the
        //      relevant stages and which memory barrier to enforce.
        //      Pipeline barriers make sure that execution of one stage finishes before another
        //      stage begins (one stage SYNCHRONIZES WITH another).
        //      Since all we do is copy, the only stages we care about here are the transfer stages.
        //      With a pipeline barrier from transfer to transfer, a copy will definitely finish
        //      executing before the next one starts.
        //      But we know that execution alone is not everything, memory accesses must also
        //      become AVAILABLE, so later operations can see them.
        //      Since making memory available is expensive, Vulkan asks you to provide explicit
        //      memory barriers to enforce availability/visibility as part of a pipeline barrier.
        // 2.   Copy 19 integers from buffer C to buffer A. Use offsets of 1R / 1W integers.
        // 3.   Apply the first memory barrier again. This ensures that the copy above has finished.
        // 4.   Copy  4 integers from buffer D to buffer A. Use offsets of  0R / 10W integers.
        // 5.   Copy  5 integers from buffer C to buffer A. Use offsets of 15R / 15W integers.
        // 6.   Copy the integer at location 7 in buffer A to location 5 in the same buffer.
        // 7.   Apply the first memory barrier again. This ensures that all copies are done.
        // 8.   Copy  4 integers from buffer B to buffer A. Use offsets of 11R /  6W integers.
        // 9.   Apply the second memory barrier for reading buffer A from the host safely. use the
        //      host stage as destination stage.
        let transfer_stage = vk::PipelineStageFlags::TRANSFER;
        let host_stage = vk::PipelineStageFlags::HOST;
        let dep_flags = vk::DependencyFlags::default();

        let copy_c2a_1_1_19 = vk::BufferCopy::default()
            .src_offset(1 * size_of::<i32>() as vk::DeviceSize)
            .dst_offset(1 * size_of::<i32>() as vk::DeviceSize)
            .size(19 * size_of::<i32>() as vk::DeviceSize);
        let copy_d2a_0_10_4 = vk::BufferCopy::default()
            .src_offset(0)
            .dst_offset(10 * size_of::<i32>() as vk::DeviceSize)
            .size(4 * size_of::<i32>() as vk::DeviceSize);
        let copy_c2a_15_15_5 = vk::BufferCopy::default()
            .src_offset(15 * size_of::<i32>() as vk::DeviceSize)
            .dst_offset(15 * size_of::<i32>() as vk::DeviceSize)
            .size(5 * size_of::<i32>() as vk::DeviceSize);
        let copy_a2a_6_4_1 = vk::BufferCopy::default()
            .src_offset(7 * size_of::<i32>() as vk::DeviceSize)
            .dst_offset(5 * size_of::<i32>() as vk::DeviceSize)
            .size(1 * size_of::<i32>() as vk::DeviceSize);
        let copy_b2a_11_6_4 = vk::BufferCopy::default()
            .src_offset(11 * size_of::<i32>() as vk::DeviceSize)
            .dst_offset(6 * size_of::<i32>() as vk::DeviceSize)
            .size(4 * size_of::<i32>() as vk::DeviceSize);

        unsafe {
            context.cmd_pipeline_barrier(
                command_buffers[0],
                transfer_stage,
                transfer_stage,
                dep_flags,
                &[transfer_barrier],
                &[],
                &[],
            );
            // ===================================
            context.cmd_copy_buffer(
                command_buffers[0],
                buffer_c.0,
                buffer_a.0,
                &[copy_c2a_1_1_19],
            );
            context.cmd_pipeline_barrier(
                command_buffers[0],
                transfer_stage,
                transfer_stage,
                dep_flags,
                &[transfer_barrier],
                &[],
                &[],
            );
            // ===================================
            context.cmd_copy_buffer(
                command_buffers[0],
                buffer_d.0,
                buffer_a.0,
                &[copy_d2a_0_10_4],
            );
            context.cmd_copy_buffer(
                command_buffers[0],
                buffer_c.0,
                buffer_a.0,
                &[copy_c2a_15_15_5],
            );
            context.cmd_copy_buffer(
                command_buffers[0],
                buffer_a.0,
                buffer_a.0,
                &[copy_a2a_6_4_1],
            );
            context.cmd_pipeline_barrier(
                command_buffers[0],
                transfer_stage,
                transfer_stage,
                dep_flags,
                &[transfer_barrier],
                &[],
                &[],
            );
            // ===================================
            context.cmd_copy_buffer(
                command_buffers[0],
                buffer_b.0,
                buffer_a.0,
                &[copy_b2a_11_6_4],
            );
            context.cmd_pipeline_barrier(
                command_buffers[0],
                transfer_stage,
                host_stage,
                dep_flags,
                &[host_barrier],
                &[],
                &[],
            );
            // ===================================
            context.end_command_buffer(command_buffers[0])?
        };
    }

    // Submit the command buffer to the compute queue and wait for completion.
    unsafe {
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
        context.queue_submit(context.queue, &[submit_info], vk::Fence::null())?;
        context.device_wait_idle()?;
    }

    // Print buffer data on host.
    for i in 0..20 {
        print!("{} ", slice_a[i]);
    }
    println!();

    // Destroy manually created objects.
    unsafe {
        allocator.unmap_memory(&mut buffer_a.1);
        allocator.destroy_buffer(buffer_d.0, &mut buffer_d.1);
        allocator.destroy_buffer(buffer_c.0, &mut buffer_c.1);
        allocator.destroy_buffer(buffer_b.0, &mut buffer_b.1);
        allocator.destroy_buffer(buffer_a.0, &mut buffer_a.1);
    }

    Ok(())
}
