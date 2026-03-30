use crate::context::Context;
use anyhow::Result;
use vk_mem;

///
pub fn create_allocator(context: &Context, api_version: u32) -> Result<vk_mem::Allocator> {
    let mut create_info = vk_mem::AllocatorCreateInfo::new(
        &context.instance,
        &context.device,
        context.physical_device,
    );
    create_info.vulkan_api_version = api_version;
    unsafe { Ok(vk_mem::Allocator::new(create_info)?) }
}
