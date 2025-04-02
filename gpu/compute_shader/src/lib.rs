#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
// #![deny(warnings)]

use glam::UVec3;
use spirv_std::{glam, spirv};

#[allow(dead_code)]
pub struct Uniforms {
    max_seeds: u32,
    chunk: u32,
}

pub fn compute(_seed: u32) -> bool {
    return true;
}

// LocalSize/numthreads of (x = 4, y = 4, z = 2)
#[spirv(compute(threads(4, 4, 2)))]
pub fn compute_shader(
    #[spirv(num_workgroups)] num_workgroups: UVec3,
    #[spirv(workgroup_id)] workgroup_id: UVec3,
    #[spirv(local_invocation_index)] local_invocation_index: u32,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &Uniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] output: &mut [u16],
) {
    let work_group_index = workgroup_id.x * num_workgroups.y * num_workgroups.z
        + workgroup_id.z * num_workgroups.y
        + workgroup_id.y;
    let local_index = local_invocation_index;
    let global_index = work_group_index * 32 + local_index;
    let seed = global_index * (uniforms.chunk + 1);

    if global_index >= uniforms.max_seeds {
        return;
    }

    if compute(seed) {
        output[global_index as usize] = 1;
    } else {
        output[global_index as usize] = 0;
    }
}
