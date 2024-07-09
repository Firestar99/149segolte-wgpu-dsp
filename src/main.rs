use std::num::NonZeroU64;
use std::time::Instant;

use wgpu::util::DeviceExt;
use wgpu::{BufferAsyncError, Device, Queue, RequestDeviceError, ShaderModule};

#[allow(dead_code)]
struct Uniforms {
    star_count: u32,
    resource_multiplier: f32,
}

async fn init_device() -> Result<(Device, Queue), RequestDeviceError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
}

fn load_compute_shader_module(device: &Device) -> ShaderModule {
    let shader_bytes: &[u8] = include_bytes!(env!("compute.spv"));
    let spirv = std::borrow::Cow::Owned(wgpu::util::make_spirv_raw(shader_bytes).into_owned());
    let shader_binary = wgpu::ShaderModuleDescriptorSpirV {
        label: None,
        source: spirv,
    };

    // Load the shaders from disk
    unsafe { device.create_shader_module_spirv(&shader_binary) }
}

async fn run_compute_shader(
    workgroups: (u32, u32, u32),
    star_count: u32,
    resource_multiplier: f32,
) -> Result<Vec<bool>, BufferAsyncError> {
    let (device, queue) = init_device().await.expect("Failed to create device");
    let module = load_compute_shader_module(&device);

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            // XXX - some graphics cards do not support empty bind layout groups, so
            // create a dummy entry.
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(1).unwrap()),
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                },
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: None,
                    ty: wgpu::BufferBindingType::Uniform,
                },
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: "main_cs",
    });

    let mut buffer = vec![0u8; (workgroups.0 * workgroups.1 * workgroups.2 * 4) as usize];
    let uniform_buffer = unsafe {
        let data = Uniforms {
            star_count,
            resource_multiplier,
        };
        let data_ptr = &data as *const Uniforms as *const u8;
        std::slice::from_raw_parts(data_ptr, std::mem::size_of::<Uniforms>())
    };

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: buffer.len() as wgpu::BufferAddress,
        // Can be read to the CPU, and can be copied from the shader's storage buffer
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Compute Output"),
        contents: &buffer,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });

    let uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniforms"),
        contents: uniform_buffer,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: uniforms.as_entire_binding(),
            },
        ],
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.set_pipeline(&compute_pipeline);
        cpass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
    }

    encoder.copy_buffer_to_buffer(
        &storage_buffer,
        0,
        &readback_buffer,
        0,
        buffer.len() as wgpu::BufferAddress,
    );

    queue.submit(Some(encoder.finish()));

    let (tx, rx) = futures::channel::oneshot::channel();
    let buffer_slice = readback_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |res| {
        let _ = tx.send(res);
    });
    device.poll(wgpu::Maintain::Wait);

    let _ = rx.await.expect("Failed to map buffer");

    buffer = buffer_slice.get_mapped_range().to_vec();

    // for x in buffer.chunks(8) {
    //     for y in x {
    //         print!("{:08b} :", y);
    //     }
    //     println!();
    // }
    //
    Ok(buffer
        .iter()
        .map(|&x| {
            vec![
                x & 1 != 0,
                x & 2 != 0,
                x & 4 != 0,
                x & 8 != 0,
                x & 16 != 0,
                x & 32 != 0,
                x & 64 != 0,
                x & 128 != 0,
            ]
        })
        .flatten()
        .collect())
}

async fn compute() {
    let start_time = Instant::now();
    let workgroups = (125, 125, 200);

    if let Ok(result) = run_compute_shader(workgroups, 64, 100.0).await {
        let time = start_time.elapsed();
        println!("Time took by shader: {:?}", time);
        let start_time = Instant::now();
        println!("Result length: {}", result.len());

        let result: Vec<u32> = result
            .iter()
            .enumerate()
            .filter(|(_, &x)| x)
            .map(|(i, _)| i as u32)
            .collect();
        println!("Result length: {}", result.len());

        let time = start_time.elapsed();
        println!("Time took processing: {:?}", time);
    }
}

fn main() {
    futures::executor::block_on(compute());
}
