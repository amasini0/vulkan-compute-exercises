use crate::context::Context;
use crate::utils;
use anyhow::{anyhow, Result};
use ash::vk;

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
            self.context
                .device
                .destroy_pipeline_layout(self.layout, None);
            self.context
                .device
                .destroy_shader_module(self.shader_module, None);
        }
    }
}

/// Sets up a compute pipeline on the given device using the provided shaders and layout bindings.
/// If successful, returns a Pipeline struct containing all the pipeline-related objects.
pub fn create_compute_pipeline<'a>(
    context: &'a Context,
    source_file: &str,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> Result<Pipeline<'a>> {
    // Create a unique shader module. Use the load_shader() function to load compiled SPIR-V code
    // from the shader source file.
    let shader_module = {
        let shader_code = utils::load_shader(source_file)?;
        let create_info = vk::ShaderModuleCreateInfo::default().code(shader_code.as_slice());
        unsafe { context.device.create_shader_module(&create_info, None)? }
    };

    // Create a unique pipeline layout using the incoming descriptor set layouts to configure it
    // for accessing descriptors (shader input resources).
    let pipeline_layout = {
        let create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(descriptor_set_layouts);
        unsafe { context.device.create_pipeline_layout(&create_info, None)? }
    };

    // Create a unique pipeline cache.
    let pipeline_cache = {
        let create_info = vk::PipelineCacheCreateInfo::default();
        unsafe { context.device.create_pipeline_cache(&create_info, None)? }
    };

    // Create a unique pipeline.
    // First prepare a shader stage info. It should be a compute stage that uses the shader module
    // you made, using the function named "main" as an entry point.
    // Then prepare a pipeline create info that uses the stage info and the above pipeline layout.
    // Finally, create a pipeline with the above pipeline cache and this create info.
    let pipeline = {
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(c"main");

        let create_infos = [vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(pipeline_layout); 1];

        match unsafe {
            context
                .device
                .create_compute_pipelines(pipeline_cache, &create_infos, None)
        } {
            Ok(pipelines) => pipelines[0],
            Err(_) => Err(anyhow!("Failed to create compute pipeline"))?,
        }
    };

    Ok(Pipeline {
        context: &context,
        handle: pipeline,
        layout: pipeline_layout,
        cache: pipeline_cache,
        shader_module,
    })
}
