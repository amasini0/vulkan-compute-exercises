use anyhow::{anyhow, Result};
use ash::{vk, Entry};
use framework::{self, Buffer};
use rand::RngExt;
use std::env;

const N: usize = 1024;

struct ShaderParams {
    _rows: usize,
    _cols: usize,
}

fn main() -> Result<()> {
    let program = env::args().into_iter().next().unwrap();
    println!("{} starting...\n", program);

    // Setup compute context and GPU allocator.
    let entry = unsafe { Entry::load() }?;
    let app_name = c"Task 09";
    let api_version = vk::make_api_version(0, 1, 4, 0);
    let context = framework::setup_compute_context(&entry, app_name, api_version, &[], &[])?;
    let allocator = framework::create_allocator(&context, api_version)?;

    // Print selected physical device name.
    let instance = &context.instance;
    let device_props = unsafe { instance.get_physical_device_properties(context.physical_device) };
    println!("Device name: {:?}\n", device_props.device_name_as_c_str()?);

    // TODO: Set up all necessary resources to do an NxN matrix multiplication.
    // Compare timing to CPU baseline. Copy your result from the GPU into a CPU
    // buffer. The error wrt the CPU result should be low (< 0.001).

    // Allocate buffers and initialize matrices with random numbers.
    let buffer_params =
        Buffer::<ShaderParams>::new_shared(&allocator, 1, vk::BufferUsageFlags::UNIFORM_BUFFER)?;
    let buffer_a =
        Buffer::<f32>::new_shared(&allocator, N * N, vk::BufferUsageFlags::STORAGE_BUFFER)?;
    let buffer_b =
        Buffer::<f32>::new_shared(&allocator, N * N, vk::BufferUsageFlags::STORAGE_BUFFER)?;
    let buffer_c =
        Buffer::<f32>::new_shared(&allocator, N * N, vk::BufferUsageFlags::STORAGE_BUFFER)?;

    let params = buffer_params.map_slice_mut()?;
    params[0] = ShaderParams { _rows: N, _cols: N };

    let matrix_a = buffer_a.map_slice_mut()?;
    let matrix_b = buffer_b.map_slice_mut()?;
    let mut rng = rand::rng();
    let uniform_dist = rand::distr::Uniform::<f32>::new_inclusive(0.0, 1.0)?;
    for i in 0..N * N {
        matrix_a[i] = rng.sample(uniform_dist);
        matrix_b[i] = rng.sample(uniform_dist);
    }

    // Create compute pipeline.
    let shader_source = format!(
        "{}/matmul.spv",
        env::var("OUT_DIR").unwrap_or(String::from("."))
    );

    let descriptor_set_layouts = [{
        let bindings = [
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
            vk::DescriptorSetLayoutBinding::default()
                .binding(3)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        unsafe { context.create_descriptor_set_layout(&create_info, None)? }
    }];

    let pipeline =
        framework::create_compute_pipeline(&context, &shader_source, &descriptor_set_layouts)?;

    // Allocate descriptor set and write descriptors to it.
    let descriptor_pool = {
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(3),
        ];

        let create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(1);
        unsafe { context.create_descriptor_pool(&create_info, None)? }
    };

    let descriptor_sets = {
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        unsafe { context.allocate_descriptor_sets(&allocate_info)? }
    };

    {
        let buffer_infos = [
            vk::DescriptorBufferInfo::default()
                .buffer(buffer_params.handle())
                .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfo::default()
                .buffer(buffer_a.handle())
                .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfo::default()
                .buffer(buffer_b.handle())
                .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfo::default()
                .buffer(buffer_c.handle())
                .range(vk::WHOLE_SIZE),
        ];

        let descriptor_writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[0])
                .dst_binding(0)
                .descriptor_count(1)
                .buffer_info(&buffer_infos[0..1])
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[0])
                .dst_binding(1)
                .descriptor_count(1)
                .buffer_info(&buffer_infos[1..2])
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[0])
                .dst_binding(1)
                .descriptor_count(1)
                .buffer_info(&buffer_infos[2..3])
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[0])
                .dst_binding(1)
                .descriptor_count(3)
                .buffer_info(&buffer_infos[3..4])
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER),
        ];

        unsafe {
            context.update_descriptor_sets(&descriptor_writes, &[]);
        }
    }

    // Create command buffer and write commands.
    let command_buffers = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe { context.allocate_command_buffers(&allocate_info)? }
    };

    unsafe {
        let command_buffer = command_buffers[0];
        let bind_point = vk::PipelineBindPoint::COMPUTE;

        let depends = vk::DependencyFlags::empty();
        let barriers = [
            vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ)
        ];

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
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
        context.cmd_dispatch(command_buffer, 1, 1, 1);
        context.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::HOST,
            depends,
            &barriers,
            &[],
            &[],
        );
        context.end_command_buffer(command_buffer)?;
    }

    unsafe {
        let submit_info = [vk::SubmitInfo::default().command_buffers(&command_buffers)];
        context.queue_submit(context.queue, &submit_info, vk::Fence::null())?;
        context.device_wait_idle()?;
    }

    // Check results.
    let matrix_c = buffer_c.map_slice()?;
    for i in 0..N {
        for j in 0..N {
            let element = matrix_c[i*N + j];
            let expected = {
                let mut sum = 0.0;
                for k in 0..N {
                    sum += matrix_a[i*N+k] * matrix_b[k*N+j];
                }
                sum
            };
            if element != expected {
                Err(anyhow!(format!("Error at element {i}{j}: expected {expected}, found {element}")))?;
            }
        }
    }

    Ok(())
}
/*
==================================== Task 9 ====================================
0) Read up on "efficient" matrix multiplication on the GPU in the CUDA guide:
https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html#shared-memory
1) Prepare the environment and necessary GPU resources for matrix multiplication.
2) Implement the corresponding GLSL shader code in matrixmult.comp.
3) Optional: Compare against a version that does not exploit shared memory. Can
you see a clear difference? HINT: You might want to start with this simpler
version anyway before trying matrix multiplication with shared memory. Also:
if you want fast performance, matrix memory should probably be device-local!
*/
