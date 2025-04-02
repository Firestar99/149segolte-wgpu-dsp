use std::path::Path;

use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("compute_shader");
    SpirvBuilder::new(path, "spirv-unknown-vulkan1.1")
        .capability(spirv_builder::Capability::Int16)
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}
