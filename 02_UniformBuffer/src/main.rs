use anyhow::{Result, anyhow};
use ash::{Entry, vk};
use framework;
use std::env;
use std::slice;

fn main() -> Result<()> {
    let program = env::args().into_iter().next().unwrap();
    println!("\n{} starting...\n", program);

    // Setup a compute context.
    let entry = unsafe { Entry::load()? };
    let context = {
        let app_name = c"Task 2";
        let api_version = vk::make_api_version(0, 1, 4, 0);
        framework::setup_compute_context(&entry, app_name, api_version, &[], &[])?
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

    // Create a buffer binding for the buffer you want to read from in the shader.
    // You can use any binding index you want, just make sure that it matches your
    // definitions in the shader (you can just use 0 in both places).
    // The buffer should be a uniform buffer. We just need one, not an array of buffers.
    // Also, we want to use it in the compute stage.
    let layout_bindings = [vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .stage_flags(vk::ShaderStageFlags::COMPUTE)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1); 1];

    // Create a unique descriptor set layout to organize your bindings (you only have one).
    // The layouts are sent to the pipeline so it knows how the descriptor sets bound to it look.
    // Set up a compute pipeline using the previously created descriptor set layout.
    let device = &context.device;
    let source_file = format!("{}/print.spv", env::var("OUT_DIR")?);
    let descriptor_set_layouts = [{
        let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        unsafe { device.create_descriptor_set_layout(&create_info, None)? }
    }; 1];

    let pipeline =
        framework::setup_compute_pipeline(device, &source_file, &descriptor_set_layouts)?;

    // We will want a resource to put in our descriptor set.
    // A single uniform buffer is needed The buffer should have enough room to store 3 integers.
    // The memory we use for the buffer should be device-local, but also be visible to the host and
    // host-coherent so we can write to it from the CPU.
    // Once the buffer is created, you will have to (in addition):
    // 1) Get the buffer's memory requirements (a struct)
    // 2) Find a memory type that fulfills the requirements (you can use the code below)
    // 3) Allocate a large enough chunk of it to store the data
    // 4) Bind the memory to the buffer
    let buffer_length = 3;
    let (buffer_handle, buffer_memory) = {
        let buffer_size = (buffer_length * size_of::<i32>()) as vk::DeviceSize;
        let create_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer_handle = unsafe { device.create_buffer(&create_info, None)? };

        // Find a suitable memory type for the buffer.
        let buffer_mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer_handle) };
        let buffer_mem_flags = vk::MemoryPropertyFlags::DEVICE_LOCAL
            | vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT;
        let device_mem_props =
            unsafe { instance.get_physical_device_memory_properties(context.physical_device) };
        let mem_type_idx = framework::find_memory_type_idx(
            &device_mem_props.memory_types,
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
        let buffer_memory = unsafe { device.allocate_memory(&allocate_info, None)? };
        unsafe { device.bind_buffer_memory(buffer_handle, buffer_memory, 0)? };

        (buffer_handle, buffer_memory)
    };

    // Fill buffer memory with some numbers: 7458410, 3542145 and 1647875.
    // Map it with no offset and for its entire size (VK_WHOLE_SIZE works) and cast it
    // to an integer slice (necessary, because you cannot write to a void*)
    let buffer_slice = {
        let map_flags = vk::MemoryMapFlags::default();
        unsafe {
            slice::from_raw_parts_mut(
                device
                    .map_memory(buffer_memory, 0, vk::WHOLE_SIZE, map_flags)?
                    .cast::<i32>(),
                buffer_length,
            )
        }
    };

    buffer_slice[0] = 7458410;
    buffer_slice[1] = 3542145;
    buffer_slice[2] = 1647875;

    // We will want a descriptor pool that can provide 1 descriptor set
    // and has room for 1 uniform buffer descriptor, nothing else.
    let descriptor_pool = {
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1); 1];
        let create_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);
        unsafe { device.create_descriptor_pool(&create_info, None)? }
    };

    // Allocate a single unique descriptor set from your descriptor pool, with the
    // descriptor set layout you defined above.
    let descriptor_sets = {
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        unsafe { device.allocate_descriptor_sets(&allocate_info)? }
    };

    // Enter the buffer into your descriptor set.
    //
    // First, prepare a descriptor buffer info that describes which buffer will be used, and how.
    // Select your buffer from above, with no offset, and using its full size (VK_WHOLE_SIZE).
    //
    // Then Prepare a WriteDescriptorSet struct that specifies what entries should be overwritten
    // in which descriptor set.
    // You want to update your descriptor set from above.
    // You want to update the binding index you chose above.
    // We don't have an array of buffers, so just set array element to 0.
    // The descriptor type is uniform buffer. We are not updating image infos, so leave those empty.
    //
    // Finally, execute the updates by calling updateDescriptorSets on the device.
    // We are only doing writes, so no (0) copies should be passed.
    {
        let descriptor_buffer_infos = [vk::DescriptorBufferInfo::default()
            .buffer(buffer_handle)
            .offset(0)
            .range(vk::WHOLE_SIZE); 1];

        let descriptor_writes = [vk::WriteDescriptorSet::default()
            .dst_set(descriptor_sets[0])
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .buffer_info(&descriptor_buffer_infos); 1];

        unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) }
    }

    // Allocate a command buffer from the command pool.
    //
    // Bind your descriptor set before dispatching a compute job that needs it.
    // It should be bound to the "compute" pipeline bind point. Use your pipeline's layout.
    // You can define an offset for the set IDs, but best leave it at 0. Bind the first
    // (and only) entry from your vector of descriptor sets. We don't use dynamic offsets.
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
        let bind_point = vk::PipelineBindPoint::COMPUTE;
        let handle = pipeline.handle;
        let layout = pipeline.layout;

        device.begin_command_buffer(command_buffer, &begin_info)?;
        device.cmd_bind_pipeline(command_buffer, bind_point, handle);
        device.cmd_bind_descriptor_sets(
            command_buffer,
            bind_point,
            layout,
            0,
            &descriptor_sets,
            &[],
        );
        device.cmd_dispatch(command_buffer, 8, 1, 1);
        device.end_command_buffer(command_buffer)?;

        let submit_infos = [vk::SubmitInfo::default().command_buffers(&command_buffers); 1];
        device.queue_submit(context.queue, &submit_infos, vk::Fence::null())?;
        device.device_wait_idle()?;
    }

    // Destroy manually created objects.
    unsafe {
        device.destroy_descriptor_pool(descriptor_pool, None);
        device.free_memory(buffer_memory, None);
        device.destroy_buffer(buffer_handle, None);
        device.destroy_descriptor_set_layout(descriptor_set_layouts[0], None);
    }

    Ok(())
}

/*
==================================== task 2 ====================================
1) create a descriptor set layout with a single uniform buffer in main.rs.
2) fill the uniform buffer's memory with the magic numbers in main.rs.
3) Create a descriptor pool that is big enough for our use.
4) Create and fill a descriptor set for your pipeline in main.rs.
5) Bind your descriptor set before dispatching the compute job in main.rs.
6) Complete the shader file print.slang to print the completed message!

// TODO: find a way to use RenderDoc with Rust
7) Optional: Try the debugging functionality of RenderDoc:
- Enable DEBUG_SHADERS in CMake
- Start RenderDoc
- Launch your Vulkan executable from within RenderDoc (make sure the working
 directory is pointing to where your compiled shaders are)
- Explore and see how you can step through compute shader code
*/
