use ash::vk;
use crate::context::Context;

/// Handles for pipeline-related objects
pub struct Pipeline<'a> {
    pub(super) context: &'a Context<'a>,
    pub handle: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub cache: vk::PipelineCache,
    pub shader_module: vk::ShaderModule,
}

impl<'a> Drop for Pipeline<'a> {
    fn drop(&mut self) {
        unsafe {
            self.context.device.destroy_pipeline(self.handle, None);
            self.context.device.destroy_pipeline_cache(self.cache, None);
            self.context.device.destroy_pipeline_layout(self.layout, None);
            self.context.device.destroy_shader_module(self.shader_module, None);
        }
    }
}
