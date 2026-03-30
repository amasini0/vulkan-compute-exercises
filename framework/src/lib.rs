// Public modules
/// Types and functions to work with point clouds
pub mod pointcloud;

// Public exports from private modules
pub use allocator::*;
pub use buffer::*;
pub use context::*;
pub use pipeline::*;
pub use utils::find_memory_type_idx;

// Private modules
mod allocator;
mod buffer;
mod context;
mod pipeline;

mod utils {
    use anyhow::{anyhow, Result};
    use ash::vk;
    use std::fs::File;

    ///
    pub(crate) fn load_shader(source_file: &str) -> Result<Vec<u32>> {
        let mut file =
            File::open(source_file).map_err(|_| anyhow!("Failed to open file {}", source_file))?;

        // Read spir-v binary from open file.
        ash::util::read_spv(&mut file)
            .map_err(|_| anyhow!("Failed to read spir-v from file {}", source_file))
    }

    /// Returns the index for the first memory type that satisfies the provided memory requirements
    /// and property flags.
    pub fn find_memory_type_idx(
        memory_types: &[vk::MemoryType],
        requirements: vk::MemoryRequirements,
        flags: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        memory_types
            .iter()
            .enumerate()
            .find(|(i, mem_type)| {
                let is_supported_type = requirements.memory_type_bits & (1 << i) != 0;
                let has_requested_flags = mem_type.property_flags.contains(flags);
                is_supported_type && has_requested_flags
            })
            .map(|(i, _)| i as _)
    }
}
