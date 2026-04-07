use anyhow::{anyhow, Result};
use ash::vk;
use image::{self, EncodableLayout};
use std::{env, path, ptr, slice};
use vk_mem::{self, Alloc};

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let program = path::Path::new(&args[0])
        .file_name()
        .map(|s| s.to_str().unwrap())
        .unwrap();

    // Check for required command line args.
    if args.len() < 2 {
        Err(anyhow!(
            "{}: missing required argument -- No image file provided\nUsage: {} <image>",
            program,
            program
        ))?
    }
    println!("\n{} starting...\n", program);

    // Read input image from file.
    let mut image = image::open(&args[1])
        .map_err(|e| anyhow!("{}: failed to open '{}' -- {:#}", program, &args[1], e))?
        .into_rgba8();
    let width = image.width();
    let height = image.height();
    let image_size = image.as_bytes().len();
    println!("Image path: \"{}\"", args[1]);
    println!("Dimensions: {} x {}", width, height);

    // Initialize a GPU context and a GPU allocator.
    let entry = unsafe { ash::Entry::load() }
        .map_err(|e| anyhow!("{}: failed to load Vulkan library -- {:#}", program, e))?;
    let api_version = vk::make_api_version(0, 1, 4, 0);
    let context = {
        let app_name = c"Task 5";
        framework::setup_compute_context(&entry, app_name, api_version, &[], &[])
            .map_err(|e| anyhow!("{}: failed to create GPU context -- {:#}", program, e))?
    };
    let allocator = framework::create_allocator(&context, api_version)
        .map_err(|e| anyhow!("{}: failed to create GPU allocator -- {:#}", program, e))?;

    // Print selected physical device name.
    let instance = &context.instance;
    let device_props = unsafe { instance.get_physical_device_properties(context.physical_device) };
    println!("Device name: {:?}\n", device_props.device_name_as_c_str());

    // Description for the resources that go into our pipeline (and descriptor set):
    // - One uniform buffer for image parameters (width, height)
    // - One storage buffer for the source image (width * height * 4 channels, RGBA)
    // - One storage buffer for the result image (width * height * 4 channels, RGBA)
    let allocation_create_info = vk_mem::AllocationCreateInfo {
        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
        usage: vk_mem::MemoryUsage::Auto,
        required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT
            | vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ..Default::default()
    };

    let mut params_buffer = {
        let create_info = vk::BufferCreateInfo::default()
            .size(2 * size_of::<u32>() as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER);
        unsafe { allocator.create_buffer(&create_info, &allocation_create_info) }
            .map_err(|e| anyhow!("{}: failed to create buffer -- {:#}", program, e))?
    };

    let mut source_buffer = {
        let create_info = vk::BufferCreateInfo::default()
            .size(image_size as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER);
        unsafe { allocator.create_buffer(&create_info, &allocation_create_info) }
            .map_err(|e| anyhow!("{}: failed to create buffer -- {:#}", program, e))?
    };

    let mut result_buffer = {
        let create_info = vk::BufferCreateInfo::default()
            .size(image_size as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER);
        unsafe { allocator.create_buffer(&create_info, &allocation_create_info) }
            .map_err(|e| anyhow!("{}: failed to create buffer -- {:#}", program, e))?
    };

    // Prepare a descriptor pool that can provide a descriptor set, two storage buffer descriptors
    // and one uniform buffer descriptor.
    let descriptor_pool = {
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .descriptor_count(1)
                .ty(vk::DescriptorType::UNIFORM_BUFFER),
            vk::DescriptorPoolSize::default()
                .descriptor_count(2)
                .ty(vk::DescriptorType::STORAGE_BUFFER),
        ];
        let create_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);
        unsafe { context.create_descriptor_pool(&create_info, None) }
            .map_err(|e| anyhow!("{}: failed to create descriptor pool -- {:#}", program, e))?
    };

    // Define bindings for the buffers in your descriptor set layout.
    // Make sure they match the bindings in the shader.
    // Create a descriptor set layout from them.
    let descriptor_set_layouts = [{
        let layout_bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        unsafe { context.create_descriptor_set_layout(&create_info, None) }
            .map_err(|e| anyhow!("{}: failed to create layout -- {:#}", program, e))?
    }];

    let descriptor_sets = {
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        unsafe { context.allocate_descriptor_sets(&allocate_info) }
            .map_err(|e| anyhow!("{}: failed to allocate descriptor sets -- {:#}", program, e))?
    };

    // Update the descriptor set to reference your actual buffers in the corresponding bindings.
    // Use "updateDescriptorSets" with correct WriteDescriptorSet structs filled in.
    {
        let buffer_infos = [params_buffer.0, source_buffer.0, result_buffer.0].map(|buffer| {
            [vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .range(vk::WHOLE_SIZE)
                .offset(0)]
        });
        let descriptor_types = [
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::DescriptorType::STORAGE_BUFFER,
            vk::DescriptorType::STORAGE_BUFFER,
        ];
        let descriptor_writes = buffer_infos
            .iter()
            .zip(descriptor_types)
            .enumerate()
            .map(|(i, (buffer_infos, desc_type))| {
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(i as u32)
                    .descriptor_count(1)
                    .descriptor_type(desc_type)
                    .buffer_info(buffer_infos)
            })
            .collect::<Vec<_>>();
        unsafe { context.update_descriptor_sets(&descriptor_writes, &[]) };
    }

    // Fill the created buffers with information.
    // The info buffer is for meta information that we need, the width and the height of the image.
    // Map its memory and then write the two integers in this order: 1) width 2) height.
    unsafe {
        let params = slice::from_raw_parts_mut(
            allocator
                .map_memory(&mut params_buffer.1)
                .map_err(|e| anyhow!("{}: failed to map memory -- {:#}", program, e))?
                .cast::<u32>(),
            2,
        );
        params[0] = width;
        params[1] = height;
        allocator.unmap_memory(&mut params_buffer.1);
    };

    // The src buffer is for storing the image color data.
    // Map its memory and then copy the contents of the image vector there.
    unsafe {
        let source_data = allocator
            .map_memory(&mut source_buffer.1)
            .map_err(|e| anyhow!("{}: failed to map memory -- {:#}", program, e))?;
        ptr::copy(image.as_ptr(), source_data, image_size);
        allocator.unmap_memory(&mut source_buffer.1);
    }

    // Set up the compute pipeline.
    let source_file = format!(
        "{}/sobel.spv",
        env::var("OUT_DIR").unwrap_or(String::from("."))
    );
    let pipeline =
        framework::create_compute_pipeline(&context, &source_file, &descriptor_set_layouts)
            .map_err(|e| anyhow!("{}: failed to create compute pipeline -- {:#}", program, e))?;

    // Allocate a command buffer from the command pool.
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        unsafe { context.allocate_command_buffers(&allocate_info) }
            .map_err(|e| anyhow!("{}: failed to allocate command buffer -- {:#}", program, e))?
    };

    // Register commands in the command buffer.
    unsafe {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        let bind_point = vk::PipelineBindPoint::COMPUTE;

        context.begin_command_buffer(command_buffers[0], &begin_info)?;
        context.cmd_bind_pipeline(command_buffers[0], bind_point, pipeline.handle);
        context.cmd_bind_descriptor_sets(
            command_buffers[0],
            bind_point,
            pipeline.layout,
            0,
            &descriptor_sets,
            &[],
        );

        // Set the proper grid dimensions for your dispatch that need to run for your image.
        // The 2D group size should be 16 in X and 16 in Y dimension.
        // That leaves you with the task of computing how many groups are needed to cover the
        // image's width and height.
        let grid_size_x = match width % 16 {
            0 => width / 16,
            _ => width / 16 + 1,
        };
        let grid_size_y = match height % 16 {
            0 => height / 16,
            _ => height / 16 + 1,
        };
        context.cmd_dispatch(command_buffers[0], grid_size_x, grid_size_y, 1);

        // We need a barrier to make sure the written data can be safely read by the CPU.
        // By now you know the drill! Note that you can probably use code from the previous task.
        let barrier = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ);
        context.cmd_pipeline_barrier(
            command_buffers[0],
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::HOST,
            vk::DependencyFlags::default(),
            &[barrier],
            &[],
            &[],
        );

        context.end_command_buffer(command_buffers[0])?;
    }

    // Submit the command buffer to the queue
    unsafe {
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
        context.queue_submit(context.queue, &[submit_info], vk::Fence::null())?;
        context.device_wait_idle()?;
    }

    // Write the contents of the dst buffer out into an image.
    // It will be named "output.png".
    // You should find it in the location where the built files for THIS TASK are located.
    unsafe {
        let result_data = allocator
            .map_memory(&mut result_buffer.1)
            .map_err(|e| anyhow!("{}: failed to map memory -- {:#}", program, e))?;
        ptr::copy(result_data, image.as_mut_ptr(), image_size);
        allocator.unmap_memory(&mut result_buffer.1);
    };

    image
        .save("output.png")
        .map_err(|e| anyhow!("{}: failed to write image -- {:#}", program, e))?;
    println!("Program finished. Image written to: 'output.png'");

    // Free resources
    unsafe {
        context.free_command_buffers(context.command_pool, &command_buffers);
        context.destroy_descriptor_set_layout(descriptor_set_layouts[0], None);
        context.destroy_descriptor_pool(descriptor_pool, None);

        allocator.destroy_buffer(result_buffer.0, &mut result_buffer.1);
        allocator.destroy_buffer(source_buffer.0, &mut source_buffer.1);
        allocator.destroy_buffer(params_buffer.0, &mut params_buffer.1);
    }

    Ok(())
}
