// Quick test to understand the bounce behavior at the specific case mentioned
use std::f32::consts::PI;

// Simplified version of the relevant functions to test
fn am_reverse_bounce(t: f32, p1: f32, p2: f32) -> f32 {
    1.0 - am_bounce(1.0 - t, p1, p2)
}

fn am_bounce(t: f32, first_step_length: f32, bounciness: f32) -> f32 {
    if first_step_length == 0.0 {
        return 1.0;
    }

    let adjusted_t = t + (first_step_length / 2.0);
    let mut period_start = 0.0_f32;
    let mut current_period = first_step_length;
    let mut amplitude = 1.0_f32;

    loop {
        let period_end = period_start + current_period;

        if adjusted_t <= period_end {
            let check_point = (current_period / 3.0) + period_start;
            if check_point > (first_step_length / 2.0) + 1.0
                || (current_period < 0.1 && period_end > (first_step_length / 2.0) + 1.0)
            {
                return 1.0;
            }

            let local_progress = (adjusted_t - period_start) / current_period;
            let centered = (local_progress - 0.5) * 2.0;
            let parabola = centered.abs().powi(2);
            return (parabola * amplitude) + (1.0 - amplitude);
        }

        current_period *= bounciness;
        amplitude *= bounciness;

        if amplitude < 0.005 {
            return 1.0;
        }

        period_start = period_end;
    }
}

fn main() {
    // Simulate the case: last keyframe at t=0.228 with "reverse bounce 2.0 0.0", query at t=0.334
    // local_t = (0.334 - 0.228) / (next_kf_time - 0.228)
    // But if 0.334 is PAST the last keyframe, then:
    // - If there's no next keyframe, the interpolation uses (last_kf, last_kf, 0.0)
    // - So easing would be applied to local_t=0.0
    
    println!("=== BOUNCE BEHAVIOR TEST ===\n");
    
    // Case 1: Bounce easing at different local_t values
    println!("Bounce easing (p1=2.0, p2=0.0):");
    for t in [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0].iter() {
        let val = am_bounce(*t, 2.0, 0.0);
        println!("  t={:.1} -> {:.4}", t, val);
    }
    
    println!("\nReverse Bounce easing (p1=2.0, p2=0.0):");
    for t in [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0].iter() {
        let val = am_reverse_bounce(*t, 2.0, 0.0);
        println!("  t={:.1} -> {:.4}", t, val);
    }
    
    // Case 2: When query time is past last keyframe
    println!("\n=== PAST LAST KEYFRAME ===");
    println!("If last keyframe is at t=0.228 and we query at t=0.334:");
    println!("find_keyframes_internal would return (last_kf, last_kf, 0.0)");
    println!("So local_t passed to easing.evaluate is 0.0");
    let val = am_reverse_bounce(0.0, 2.0, 0.0);
    println!("Reverse bounce(0.0, 2.0, 0.0) = {:.4}", val);
    println!("This means interpolation would be: lerp(last_val, last_val, {:.4}) = last_val", val);
    
    // Case 3: Before first keyframe
    println!("\n=== BEFORE FIRST KEYFRAME ===");
    println!("If first keyframe is at t=0.228 and we query at t=0.1:");
    println!("find_keyframes_internal would return (first_kf, first_kf, 0.0)");
    let val = am_reverse_bounce(0.0, 2.0, 0.0);
    println!("Reverse bounce(0.0, 2.0, 0.0) = {:.4}", val);
    println!("This means interpolation would be: lerp(first_val, first_val, {:.4}) = first_val", val);
}
