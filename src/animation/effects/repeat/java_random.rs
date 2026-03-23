/// Compute Java Random initial state from AM seed value.
/// Returns (state_lo_32bits, state_hi_16bits) packed as f32 via bitcast.
pub(crate) fn compute_java_random_state_packed(seed: f32) -> (f32, f32) {
    let am_seed = (15234322.0_f32 + 35432882176.0_f32 * seed) as i64;
    let multiplier: i64 = 0x5DEECE66D;
    let init_state = ((am_seed ^ multiplier) as u64) & ((1u64 << 48) - 1);
    let state_hi = ((init_state >> 32) & 0xFFFF) as u32;
    let state_lo = (init_state & 0xFFFFFFFF) as u32;
    (f32::from_bits(state_lo), f32::from_bits(state_hi))
}
