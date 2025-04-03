use anyhow::{anyhow, Context, Result};
use std::num::NonZeroU64;

const MAX_SEEDS: u64 = 100_000_000;
const RETURN_SIZE: u64 = 2; // in bytes
const MAX_BIND_SIZE: u64 = 127 * 1024 * 1024; // 127 MB
const SHADER_THREADS: u64 = 32;

#[allow(dead_code)]
struct Uniforms {
    max_seeds: u32,
    chunk: u32,
}

impl Uniforms {
    fn new(chunk: u32) -> Self {
        Self {
            max_seeds: MAX_SEEDS as u32,
            chunk,
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let (device, queue) = initialize()?;

    let module =
        unsafe { device.create_shader_module_spirv(&wgpu::include_spirv_raw!(env!("shader.spv"))) };

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let output_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: MAX_BIND_SIZE,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (MAX_SEEDS * RETURN_SIZE).div_ceil(MAX_BIND_SIZE) * MAX_BIND_SIZE,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    min_binding_size: None,
                    has_dynamic_offset: false,
                },
                count: None,
            },
            // Output buffer
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    // This is the size of a single element in the buffer.
                    min_binding_size: Some(NonZeroU64::new(RETURN_SIZE).unwrap()),
                    has_dynamic_offset: false,
                },
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_data_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("compute_shader"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    for chunk in 0..(MAX_SEEDS * RETURN_SIZE).div_ceil(MAX_BIND_SIZE) {
        let uniforms = unsafe {
            let data = Uniforms::new(chunk as u32);
            let data_ptr = &data as *const Uniforms as *const u8;
            std::slice::from_raw_parts(data_ptr, std::mem::size_of::<Uniforms>())
        };
        queue.write_buffer(&uniform_buffer, 0, &uniforms);

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = MAX_BIND_SIZE.div_ceil(SHADER_THREADS).div_ceil(64);
        compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);

        // Now we drop the compute pass, giving us access to the encoder again.
        drop(compute_pass);

        encoder.copy_buffer_to_buffer(
            &output_data_buffer,
            0,
            &download_buffer,
            chunk * MAX_BIND_SIZE,
            output_data_buffer.size(),
        );
    }

    let command_buffer = encoder.finish();
    queue.submit([command_buffer]);

    // Mapping requires that the GPU be finished using the buffer before it resolves, so mapping has a callback to tell you when the mapping is complete.
    let buffer_slice = download_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |e| {
        // In this case we know exactly when the mapping will be finished,
        // so we don't need to do anything in the callback.
        if let Err(e) = e {
            eprintln!("Error mapping buffer: {:?}", e);
        }
    });
    // Wait for the GPU to finish working on the submitted work. This doesn't work on WebGPU, so we would need
    // to rely on the callback to know when the buffer is mapped.
    device.poll(wgpu::Maintain::Wait);

    let data: Vec<u64> = buffer_slice
        .get_mapped_range()
        .chunks_exact(RETURN_SIZE as usize)
        .take(MAX_SEEDS as usize)
        .map(|chunk| match RETURN_SIZE {
            1 => chunk[0] as u64,
            2 => u16::from_ne_bytes([chunk[0], chunk[1]]) as u64,
            4 => u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as u64,
            8 => u64::from_ne_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]),
            _ => panic!("Invalid return size"),
        })
        .collect();
    let count = data.iter().filter(|&&x| x == 1).count();

    println!("Valid: {}, Invalid: {}", count, MAX_SEEDS - count as u64);

    Ok(())
}

fn initialize() -> Result<(wgpu::Device, wgpu::Queue), anyhow::Error> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .context("Failed to create adapter")?;

    println!("Running on Adapter: {:#?}", adapter.get_info());

    let downlevel_capabilities = adapter.get_downlevel_capabilities();
    if !downlevel_capabilities
        .flags
        .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
    {
        return Err(anyhow!("Adapter does not support compute shaders"));
    }

    Ok(pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH
                | wgpu::Features::SHADER_I16
                | wgpu::Features::SHADER_F16,
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
        },
        None,
    ))
    .context("Failed to create device")?)
}
