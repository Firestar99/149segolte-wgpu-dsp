use std::path::Path;

use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("shader");
    SpirvBuilder::new(path, "spirv-unknown-vulkan1.2")
        .capability(spirv_builder::Capability::VulkanMemoryModel)
        .capability(spirv_builder::Capability::Int16)
        .capability(spirv_builder::Capability::StorageBuffer16BitAccess)
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}
