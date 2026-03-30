use anyhow::{anyhow, Result};
use ash::{vk, Entry};
use framework::pointcloud::{self, Point};
use framework::{self, Buffer};
use glam::{Mat4, Vec3};
use image::RgbaImage;
use std::path::Path;
use std::env;

// Hardcoded image dimensions.
const IMAGE_WIDTH: u32 = 800;
const IMAGE_HEIGHT: u32 = 800;

// Structure to hold parameters that need to be sent to GPU.
// Padding between fields is required to make sure the struct adheres to the std140 layout used for
// uniform buffers on GPU.
#[repr(C)]
#[repr(align(16))]
struct Parameters {
    width: u32,
    _padding0: [u8; 12],
    height: u32,
    _padding1: [u8; 12],
    num_points: u32,
    _padding2: [u8; 12],
    mvp: Mat4,
}

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let program = Path::new(&args[0]).file_name().unwrap().to_str().unwrap();

    // Check for required command line args.
    if args.len() < 2 {
        Err(anyhow!(
            "{}: missing required argument -- No image point cloud file provided\nUsage: {} <point cloud file>",
            program,
            program
        ))?;
    }
    println!("\n{} starting...\n", args[0]);

    // Initialize GPU compute context and allocator.
    let entry = unsafe { Entry::load() }
        .map_err(|e| anyhow!("{}: failed to load Vulkan entrypoint -- {:#}", program, e))?;
    let app_name = c"Task 7";
    let api_version = vk::make_api_version(0, 1, 4, 0);
    let context = framework::setup_compute_context(&entry, app_name, api_version, &[], &[])
        .map_err(|e| anyhow!("{}: failed to create compute context -- {:#}", program, e))?;
    let allocator = framework::create_allocator(&context, api_version)
        .map_err(|e| anyhow!("{}: failed to create allocator -- {:#}", program, e))?;

    // Read point cloud from file.
    let point_cloud = pointcloud::load_cloud(&args[1]).map_err(|e| {
        anyhow!(
            "{}: Failed to load point cloud from file {} -- {:#}",
            program,
            args[1],
            e
        )
    })?;

    // Description for the resources that go into our pipeline (and descriptor set):
    // - One uniform buffer for the rendering parameters (size of Parameters struct)
    // - One storage buffer for the point cloud (pointcloud.size() * size of Point struct)
    // - One storage buffer for the output image (width * height * size of uint32).
    // - All of them should be host-visible, host-coherent and device-local.
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
        unsafe { context.create_descriptor_set_layout(&create_info, None) }.map_err(|e| {
            anyhow!(
                "{}: failed to create descriptor set layout -- {:#}",
                program,
                e
            )
        })?
    }];

    let params_buffer =
        Buffer::<Parameters>::new_shared(&allocator, 1, vk::BufferUsageFlags::UNIFORM_BUFFER)
            .map_err(|e| anyhow!("{}: failed to create buffer -- {:#}", program, e))?;

    let points_buffer = Buffer::<Point>::new_shared(
        &allocator,
        point_cloud.points.len(),
        vk::BufferUsageFlags::STORAGE_BUFFER,
    )
    .map_err(|e| anyhow!("{}: failed to create buffer -- {:#}", program, e))?;

    let output_buffer = Buffer::<u32>::new_shared(
        &allocator,
        (IMAGE_WIDTH * IMAGE_HEIGHT) as usize,
        vk::BufferUsageFlags::STORAGE_BUFFER,
    )
    .map_err(|e| anyhow!("{}: failed to create buffer -- {:#}", program, e))?;

    // Take all the necessary steps to have a descriptor set that references the above created
    // buffers. Make sure the bindings match the shader.
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

    let descriptor_sets = {
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        unsafe { context.allocate_descriptor_sets(&allocate_info) }
            .map_err(|e| anyhow!("{}: failed to allocate descriptor sets -- {:#}", program, e))?
    };

    {
        let descriptor_info = [
            vk::DescriptorBufferInfo::default()
                .buffer(params_buffer.handle())
                .range(vk::WHOLE_SIZE)
                .offset(0),
            vk::DescriptorBufferInfo::default()
                .buffer(points_buffer.handle())
                .range(vk::WHOLE_SIZE)
                .offset(0),
            vk::DescriptorBufferInfo::default()
                .buffer(output_buffer.handle())
                .range(vk::WHOLE_SIZE)
                .offset(0),
        ];

        let descriptor_set = descriptor_sets[0];
        let descriptor_writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&descriptor_info[0..1]),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&descriptor_info[1..2]),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&descriptor_info[2..3]),
        ];

        unsafe { context.update_descriptor_sets(&descriptor_writes, &[]) };
    }

    // Set rendering params.
    let center = Vec3::from((point_cloud.minimum + point_cloud.maximum) * 0.5);
    let span = (point_cloud.maximum - point_cloud.minimum).length();
    let view = Mat4::look_at_rh(
        center + Vec3::new(-0.9 * span, 0.0, 0.15 * span),
        center.into(),
        Vec3::new(0.0, 0.0, -1.0),
    );
    let projection = Mat4::perspective_rh(
        45.0,
        IMAGE_WIDTH as f32 / IMAGE_HEIGHT as f32,
        0.1,
        2.0 * span,
    );

    let params_slice = params_buffer
        .map_slice_mut()
        .map_err(|e| anyhow!("{}: failed to map buffer memory -- {:#}", program, e))?;
    params_slice[0].width = IMAGE_WIDTH;
    params_slice[0].height = IMAGE_HEIGHT;
    params_slice[0].num_points = point_cloud.points.len() as u32;
    params_slice[0].mvp = projection * view;

    let points_slice = points_buffer
        .map_slice_mut()
        .map_err(|e| anyhow!("{}: failed to map buffer memory -- {:#}", program, e))?;
    points_slice.copy_from_slice(&point_cloud.points);

    let output_slice = output_buffer
        .map_slice_mut()
        .map_err(|e| anyhow!("{}: failed to map buffer memory -- {:#}", program, e))?;
    output_slice.fill(0xFFFFFFFF);

    // Create pipeline.
    let source_file = format!(
        "{}/pointcloud.spv",
        env::var("OUT_DIR").unwrap_or(String::from("."))
    );
    let pipeline =
        framework::create_compute_pipeline(&context, &source_file, &descriptor_set_layouts)
            .map_err(|e| anyhow!("{}: failed to create pipeline -- {:#}", program, e))?;

    // Create command buffers.
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe { context.allocate_command_buffers(&allocate_info) }
            .map_err(|e| anyhow!("{}: failed to allocate command buffers -- {:#}", program, e))?
    };

    // Dispatch commands to the GPU.
    unsafe {
        // Fill command buffer.
        let command_buffer = command_buffers[0];
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        let bind_point = vk::PipelineBindPoint::COMPUTE;

        let work_groups_number = if point_cloud.points.len() % 128 == 0 {
            point_cloud.points.len() / 128
        } else {
            point_cloud.points.len() / 128 + 1
        } as u32;

        let barrier = vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ);
        let dependencies = vk::DependencyFlags::empty();

        context.begin_command_buffer(command_buffer, &begin_info)?;
        context.cmd_bind_pipeline(command_buffer, bind_point, pipeline.handle);
        context.cmd_bind_descriptor_sets(
            command_buffer,
            bind_point,
            pipeline.layout,
            0,
            &descriptor_sets,
            &[],
        );
        context.cmd_dispatch(command_buffer, work_groups_number, 1, 1);
        context.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::HOST,
            dependencies,
            &[barrier],
            &[],
            &[],
        );
        context.end_command_buffer(command_buffer)?;

        // Submit command buffer.
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
        context.queue_submit(context.queue, &[submit_info], vk::Fence::null())?;
        context.device_wait_idle()?;
    }

    // Debug prints
    // println!("\nCPU DATA\n=======================");
    // println!("width: {}, height: {}", IMAGE_WIDTH, IMAGE_HEIGHT,);
    // println!("num_points: {}", point_cloud.points.len());
    // println!("mvp {{ x_axis: {}, ... }}", params_slice[0].mvp.x_axis);
    // println!(
    //     "points[20] {{ position: {}, color: {} }}",
    //     point_cloud.points[20].position, point_cloud.points[20].color
    // );

    // Save output image.
    let mut image = RgbaImage::new(IMAGE_WIDTH, IMAGE_HEIGHT);
    unsafe {
        let base_ptr = image.as_mut_ptr().cast::<u32>();
        for (id, p) in output_slice.iter().enumerate() {
            *base_ptr.add(id) =
                0xFF000000 | ((p & 0xF) << 4) | ((p & 0xF0) << 8) | ((p & 0xF00) << 12);
        }
    }
    image
        .save("output.png")
        .map_err(|e| anyhow!("{}: failed to write image -- {:#}", e, program))?;
    println!("Program finished. Image written to 'output.png'");

    // Cleanup.
    params_buffer.unmap();
    points_buffer.unmap();
    output_buffer.unmap();

    unsafe {
        context.free_command_buffers(context.command_pool, &command_buffers);
        context.destroy_descriptor_pool(descriptor_pool, None);
        context.destroy_descriptor_set_layout(descriptor_set_layouts[0], None);
    }

    Ok(())
}

/*
==================================== Task 7 ====================================
0) Pass the path of an .obj file in the assets directory as a program argument.
1) Create your resources and descriptor sets.
2) Modify the structs so that their contents can be read by the GPU.
3) Compute the number of required work groups to process one point per thread.
4) Complete the shader file pointcloud.slang to perform point cloud rendering.
5) Optional: What happens if you don't use atomics or add the depth to the color?
*/
