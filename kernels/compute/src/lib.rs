#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
// #![deny(warnings)]

use glam::UVec3;
use spirv_std::arch::atomic_or;
use spirv_std::memory::{Scope, Semantics};
use spirv_std::{glam, spirv};

#[allow(dead_code)]
pub struct Uniforms {
    star_count: u32,
    resource_multiplier: f32,
}

#[allow(dead_code)]
pub struct GameDesc {
    seed: i32,
    star_count: usize,
    resource_multiplier: f32,
    habitable_count: usize,
}

pub fn compute(_game: GameDesc) -> bool {
    return false;
}

pub fn set_bit(array: &mut [u32], index: usize) {
    let word = index / 32;
    let bit = index % 32;

    unsafe {
        atomic_or::<_, { Scope::Workgroup as u32 }, { Semantics::UNIFORM_MEMORY.bits() }>(
            &mut array[word],
            1 << bit,
        );
    }
}

// LocalSize/numthreads of (x = 4, y = 4, z = 2)
#[spirv(compute(threads(4, 4, 2)))]
pub fn main_cs(
    #[spirv(num_workgroups)] num_workgroups: UVec3,
    #[spirv(workgroup_id)] workgroup_id: UVec3,
    #[spirv(local_invocation_index)] local_invocation_index: u32,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] output: &mut [u32],
    #[spirv(uniform, descriptor_set = 0, binding = 1)] uniforms: &Uniforms,
) {
    let work_group_index = workgroup_id.x * num_workgroups.y * num_workgroups.z
        + workgroup_id.z * num_workgroups.y
        + workgroup_id.y;
    let local_index = local_invocation_index;
    let global_index = work_group_index * 32 + local_index;

    let game = GameDesc {
        seed: global_index as i32,
        star_count: uniforms.star_count as usize,
        resource_multiplier: uniforms.resource_multiplier,
        habitable_count: 0,
    };
    if compute(game) {
        set_bit(output, global_index as usize);
    }
}
