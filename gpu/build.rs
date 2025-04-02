use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for kernel in std::fs::read_dir("kernels")? {
        let path = kernel?.path();
        SpirvBuilder::new(path, "spirv-unknown-vulkan1.1")
            .print_metadata(MetadataPrintout::Full)
            .build()?;
    }
    Ok(())
}
