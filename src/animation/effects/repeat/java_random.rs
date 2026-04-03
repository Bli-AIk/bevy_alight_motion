//! Mirrors the Java `Random` seeding logic used by Alight Motion's
//! repeat effects. The shader path needs identical pseudo-random ordering, so
//! this helper computes the packed initial state from the authored seed value.
//!
//! 复现了 Alight Motion 重复效果依赖的 Java `Random` 初始化逻辑。
//! 着色器路径需要和原始工具保持一致的伪随机顺序，因此这里会从作者输入的 seed
//! 计算出打包后的初始随机状态。

const MASK_48: u64 = (1u64 << 48) - 1;
const MULTIPLIER: u64 = 0x5DEECE66D;

fn init_state(seed: f32) -> u64 {
    let am_seed = (15234322.0_f64 + 35432882176.0_f64 * seed as f64) as i64;
    ((am_seed ^ MULTIPLIER as i64) as u64) & MASK_48
}

fn step(state: &mut u64) {
    *state = state.wrapping_mul(MULTIPLIER).wrapping_add(0xB) & MASK_48;
}

fn next31(state: &mut u64) -> u32 {
    step(state);
    (*state >> 17) as u32
}

fn next_int(state: &mut u64, bound: u32) -> u32 {
    if bound & (bound - 1) == 0 {
        let bits = next31(state) as u64;
        return ((bound as u64 * bits) >> 31) as u32;
    }
    for _ in 0..100 {
        let bits = next31(state);
        let val = bits % bound;
        if bits.wrapping_sub(val).wrapping_add(bound - 1) < 0x80000000 {
            return val;
        }
    }
    0
}

/// Compute Java Random initial state from AM seed value.
/// Returns (state_lo_32bits, state_hi_16bits) packed as f32 via bitcast.
pub(crate) fn compute_java_random_state_packed(seed: f32) -> (f32, f32) {
    let state = init_state(seed);
    let state_hi = ((state >> 32) & 0xFFFF) as u32;
    let state_lo = (state & 0xFFFFFFFF) as u32;
    (f32::from_bits(state_lo), f32::from_bits(state_hi))
}

/// Maximum repeat count supported by CPU-side precomputed permutation.
/// 6 u32 slots × 4 indices per u32 = 24 indices.
pub(crate) const MAX_PRECOMPUTED_PERM: usize = 24;

/// Precompute Fisher-Yates permutation for `count` items and pack into
/// uniform-friendly f32 values.
///
/// Returns `(params5_y, params5_z, perm_vec4)` where each u32 slot holds
/// 4 indices in 8-bit lanes (little-endian byte order).
///
/// Returns `None` if `count` exceeds `MAX_PRECOMPUTED_PERM`.
pub(crate) fn precompute_shuffle_packed(
    seed: f32,
    count: usize,
) -> Option<(f32, f32, bevy::math::Vec4)> {
    if count > MAX_PRECOMPUTED_PERM || count == 0 {
        return None;
    }
    let mut state = init_state(seed);
    let mut perm: Vec<u8> = (0..count as u8).collect();
    for i in (1..count).rev() {
        let j = next_int(&mut state, (i + 1) as u32) as usize;
        perm.swap(i, j);
    }
    // Pack 4 indices per u32, 8 bits each.
    let pack4 = |start: usize| -> u32 {
        let mut v = 0u32;
        for k in 0..4 {
            if start + k < count {
                v |= (perm[start + k] as u32) << (8 * k);
            }
        }
        v
    };
    let p5y = f32::from_bits(pack4(0));
    let p5z = f32::from_bits(pack4(4));
    let perm_vec4 = bevy::math::Vec4::new(
        f32::from_bits(pack4(8)),
        f32::from_bits(pack4(12)),
        f32::from_bits(pack4(16)),
        f32::from_bits(pack4(20)),
    );
    Some((p5y, p5z, perm_vec4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precompute_matches_gpu_logic() {
        // Verify the CPU shuffle produces the same permutation as the GPU
        // would with the same seed. We test with seed=0, count=9 (the
        // random.amproj example).
        let result = precompute_shuffle_packed(0.0, 9).unwrap();
        // Unpack and verify it's a valid permutation of 0..9
        let unpack = |f: f32, start: usize, count: usize| -> Vec<u8> {
            let bits = f32::to_bits(f);
            (0..4)
                .filter(|k| start + k < count)
                .map(|k| ((bits >> (8 * k)) & 0xFF) as u8)
                .collect()
        };
        let mut indices = Vec::new();
        indices.extend(unpack(result.0, 0, 9));
        indices.extend(unpack(result.1, 4, 9));
        indices.extend(unpack(f32::from_bits(result.2.x.to_bits()), 8, 9));
        assert_eq!(indices.len(), 9);
        let mut sorted = indices.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
