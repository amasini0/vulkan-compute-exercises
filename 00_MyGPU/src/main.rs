use anyhow::Result;
use ash::{Entry, vk};
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    println!("\n{} starting...\n", args[0]);

    // Get Vulkan entrypoint
    let entry = Entry::linked();

    // Create instance
    let instance = {
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Task 0")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"None")
            .api_version(vk::make_api_version(0, 1, 4, 0));

        let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

        unsafe { entry.create_instance(&create_info, None)? }
    };

    // Query physical devices
    let devices = unsafe { instance.enumerate_physical_devices()? };
    println!("Devices: count = {:>2}", devices.len());
    println!("===================");

    for (i, device) in devices.iter().enumerate() {
        // Device properties
        let mut sg_props = vk::PhysicalDeviceSubgroupSizeControlProperties::default();
        let mut props2 = vk::PhysicalDeviceProperties2::default().push_next(&mut sg_props);
        unsafe { instance.get_physical_device_properties2(*device, &mut props2) };

        let device_name = props2.properties.device_name_as_c_str()?.to_str()?;
        let device_type = props2.properties.device_type;
        let api_major = vk::api_version_major(props2.properties.api_version);
        let api_minor = vk::api_version_minor(props2.properties.api_version);

        let limits = props2.properties.limits;
        let max_compute_shared_mem_size = limits.max_compute_shared_memory_size;
        let max_compute_wg_count = limits.max_compute_work_group_count;
        let max_compute_wg_size = limits.max_compute_work_group_size;
        let max_compute_wg_invocations = limits.max_compute_work_group_invocations;

        let max_compute_wg_sg = sg_props.max_compute_workgroup_subgroups;
        let max_sg_size = sg_props.max_subgroup_size;
        let min_sg_size = sg_props.min_subgroup_size;

        // Device memory properties
        let mut mem_props2 = vk::PhysicalDeviceMemoryProperties2::default();
        unsafe { instance.get_physical_device_memory_properties2(*device, &mut mem_props2) };

        let memory_heaps = mem_props2.memory_properties.memory_heaps_as_slice();
        let memory_types = mem_props2.memory_properties.memory_types_as_slice();

        // Device queue families
        let queue_families = unsafe {
            let num_families =
                instance.get_physical_device_queue_family_properties2_len(*device);
            let mut queue_families = vec![vk::QueueFamilyProperties2::default(); num_families];
            instance.get_physical_device_queue_family_properties2(*device, &mut queue_families);
            queue_families
        };

        println!("GPU{}:", i);
        println!("\t{:<24} = {}", "deviceName", device_name);
        println!("\t{:<24} = {:?}", "deviceType", device_type);
        println!("\t{:<24} = {}.{}", "apiVersion", api_major, api_minor);
        println!(
            "\t{:<24} = {}",
            "maxSharedMemorySize", max_compute_shared_mem_size
        );
        println!("\t{:<24} = {:?}", "maxWorkgroupCount", max_compute_wg_count);
        println!("\t{:<24} = {:?}", "maxWorkgroupSize", max_compute_wg_size);
        println!(
            "\t{:<24} = {}",
            "maxWorkgroupInvocations", max_compute_wg_invocations
        );
        println!("\t{:<24} = {}", "maxWorkgroupSubgroups", max_compute_wg_sg);
        println!(
            "\t{:<24} = {}/{}",
            "min/maxSubgroupSize", min_sg_size, max_sg_size
        );

        println!("\tmemoryHeaps: count = {:>2}", memory_heaps.len());
        println!("\t{:-<25}", "");
        for vk::MemoryHeap { size, flags } in memory_heaps.iter() {
            println!("\t\t{{ flags: {:?}, size: {:?} }},", flags, size);
        }

        println!("\tmemoryTypes: count = {:>2}", memory_types.len());
        println!("\t{:-<25}", "");
        for vk::MemoryType {
            property_flags,
            heap_index,
        } in memory_types.iter()
        {
            println!(
                "\t\t{{ property_flags: {:?}, heap_index: {:?} }},",
                property_flags, heap_index
            );
        }

        println!("\tqueueFamilies: count = {:>2}", queue_families.len());
        println!("\t{:-<25}", "");
        for qfam_props2 in queue_families.iter()
        {
            let flags = qfam_props2.queue_family_properties.queue_flags;
            let count = qfam_props2.queue_family_properties.queue_count;
            println!("\t\t{{ flags: {:?}, count: {} }}", flags, count);
        }

        println!();
    }

    Ok(())
}

/*
==================================== Task 0 ====================================
1) Create a Vulkan instance. Give it an application name "Task 0", a version
number 1, and an engine name of "None". The API version should be 1.3.
2) Query your physical devices. Retrieve their properties and print out
- the total number of devices
- their names
- the latest API they support (human-readable! Might be a bit tricky!)
3) Physical devices have several types of property collections available. Out of
all of them, find three properties that should be relevant to compute jobs and
print them out.
4) Make sure to clean up everything! Either by explicitly destroying your objects
(in the right order) or by using the smart pointers of vulkan.hpp.
5) Optional: you can go and explore a bit, checking out the individual features
of your devices and queue families it provides. What kinds does it support?
Check out an advanced feature using the pNext pointer of VkPhysicalDeviceFeatures2.
*/
