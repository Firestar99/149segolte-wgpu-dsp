use std::path::Path;

use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("compute_shader");
    SpirvBuilder::new(path, "spirv-unknown-vulkan1.2")
        .capability(spirv_builder::Capability::VulkanMemoryModel)
        .capability(spirv_builder::Capability::Int8)
        .capability(spirv_builder::Capability::StorageBuffer8BitAccess)
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}
