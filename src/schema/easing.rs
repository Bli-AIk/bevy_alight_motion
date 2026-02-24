//! # easing.rs
//!
//! # 缓动函数模块
//!
//! Easing functions for animation interpolation.
//! 用于动画插值的缓动函数。

/// Easing function type.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Easing {
    /// Linear interpolation (default).
    #[default]
    Linear,
    /// Step function (instant transition).
    Step { x: f32, y: f32 },
    /// Cubic bezier curve with control points.
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
    /// Bounce easing (standard ease-out-bounce).
    /// Parameters are stored but currently ignored in evaluation (using standard bounce).
    Bounce { p1: f32, p2: f32 },
    /// Reverse bounce easing (ease-in-bounce).
    /// Used when the animation starts slow and ends with a bounce.
    ReverseBounce { p1: f32, p2: f32 },
    /// Cyclic easing (oscillation).
    /// Creates a wave-like motion with multiple oscillations between keyframes.
    /// Parameters: step_length, sharpness, skew, decay, reserved
    Cyclic {
        step_length: f32,
        sharpness: f32,
        skew: f32,
        decay: f32,
        reserved: f32,
    },
    /// Elastic easing with spring-like oscillation.
    /// Parameters: step_length, attack, decay, magnitude
    Elastic {
        step_length: f32,
        attack: f32,
        decay: f32,
        magnitude: f32,
    },
    /// Elastic step easing - stepped elastic effect.
    /// Parameters: step_length, magnitude
    ElasticStep { step_length: f32, magnitude: f32 },
}

impl Easing {
    /// Parse easing string from AM format.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.is_empty() {
            return Easing::Linear;
        }

        let parts: Vec<&str> = s.split_whitespace().collect();

        // Check for "reverse" prefix (e.g., "reverse bounce 2.0 0.0")
        if parts.first().copied() == Some("reverse") {
            match parts.get(1).copied() {
                Some("bounce") => {
                    let p1 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let p2 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    return Easing::ReverseBounce { p1, p2 };
                }
                _ => return Easing::Linear,
            }
        }

        match parts.first().copied() {
            Some("step") => {
                let x = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1.0);
                let y = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                Easing::Step { x, y }
            }
            Some("cubicBezier") => {
                let x1 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let y1 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let x2 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
                let y2 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
                Easing::CubicBezier { x1, y1, x2, y2 }
            }
            Some("bounce") => {
                let p1 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let p2 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                Easing::Bounce { p1, p2 }
            }
            Some("cyclic") => {
                let step_length = parts
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.2857143);
                let sharpness = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let skew = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.5);
                let decay = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let reserved = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                Easing::Cyclic {
                    step_length,
                    sharpness,
                    skew,
                    decay,
                    reserved,
                }
            }
            Some("elastic") => {
                let step_length = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.25);
                let attack = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0);
                let decay = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.5);
                let magnitude = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
                Easing::Elastic {
                    step_length,
                    attack,
                    decay,
                    magnitude,
                }
            }
            Some("elasticStep") => {
                let step_length = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.2);
                let magnitude = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.5);
                Easing::ElasticStep {
                    step_length,
                    magnitude,
                }
            }
            _ => Easing::Linear,
        }
    }

    /// Convert easing back to AM format string.
    /// Returns `None` for `Linear` (AM uses absence of easing attribute for linear).
    pub fn to_am_string(&self) -> Option<String> {
        match self {
            Easing::Linear => None,
            Easing::Step { x, y } => Some(format!("step {} {}", x, y)),
            Easing::CubicBezier { x1, y1, x2, y2 } => {
                Some(format!("cubicBezier {} {} {} {}", x1, y1, x2, y2))
            }
            Easing::Bounce { p1, p2 } => Some(format!("bounce {} {}", p1, p2)),
            Easing::ReverseBounce { p1, p2 } => Some(format!("reverse bounce {} {}", p1, p2)),
            Easing::Cyclic {
                step_length,
                sharpness,
                skew,
                decay,
                reserved,
            } => Some(format!(
                "cyclic {} {} {} {} {}",
                step_length, sharpness, skew, decay, reserved
            )),
            Easing::Elastic {
                step_length,
                attack,
                decay,
                magnitude,
            } => Some(format!(
                "elastic {} {} {} {}",
                step_length, attack, decay, magnitude
            )),
            Easing::ElasticStep {
                step_length,
                magnitude,
            } => Some(format!("elasticStep {} {}", step_length, magnitude)),
        }
    }

    /// Evaluate the easing function at normalized time t (0.0-1.0).
    pub fn evaluate(&self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::Step {
                x: step_length,
                y: smoothing,
            } => am_step(t, *step_length, *smoothing),
            Easing::CubicBezier { x1, y1, x2, y2 } => cubic_bezier_y_for_x(t, *x1, *y1, *x2, *y2),
            Easing::Bounce { p1, p2 } => am_bounce(t, *p1, *p2),
            Easing::ReverseBounce { p1, p2 } => am_reverse_bounce(t, *p1, *p2),
            Easing::Cyclic {
                step_length,
                sharpness,
                skew,
                decay,
                reserved,
            } => am_cyclic(t, *step_length, *sharpness, *skew, *decay, *reserved),
            Easing::Elastic {
                step_length,
                attack: _,
                decay,
                magnitude,
            } => am_elastic(t, *step_length, *decay, *magnitude),
            Easing::ElasticStep {
                step_length,
                magnitude,
            } => am_elastic_step(t, *step_length, *magnitude),
        }
    }
}

/// AM-style step easing (staircase with optional smoothstep transition).
///
/// Implementation based on AM source code (StepEasing.java).
/// Parameters:
/// - step_length: duration of each step (in t-space, 0-1)
/// - smoothing: smoothstep transition zone at the end of each step (0 = instant, 1 = full smooth)
fn am_step(t: f32, step_length: f32, smoothing: f32) -> f32 {
    let safe_step = step_length.max(0.001);
    let step_base = t - (t % safe_step);
    let within_step = t % safe_step;
    let smooth_edge0 = safe_step * (1.0 - smoothing);
    let denominator = safe_step - smooth_edge0;
    let smooth_t = if denominator.abs() < f32::EPSILON {
        0.0
    } else {
        ((within_step - smooth_edge0) / denominator).clamp(0.0, 1.0)
    };
    let smooth_val = smooth_t * smooth_t * (3.0 - 2.0 * smooth_t);
    step_base + smooth_val * safe_step
}

/// Standard Ease-Out-Bounce function.
fn ease_out_bounce(x: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;

    if x < 1.0 / d1 {
        n1 * x * x
    } else if x < 2.0 / d1 {
        let x = x - 1.5 / d1;
        n1 * x * x + 0.75
    } else if x < 2.5 / d1 {
        let x = x - 2.25 / d1;
        n1 * x * x + 0.9375
    } else {
        let x = x - 2.625 / d1;
        n1 * x * x + 0.984375
    }
}

/// Standard Ease-In-Bounce function.
#[allow(dead_code)]
fn ease_in_bounce(x: f32) -> f32 {
    1.0 - ease_out_bounce(1.0 - x)
}

/// AM-style bounce with configurable parameters.
///
/// Implementation based on AM source code (BounceEasing.java).
/// p1 = firstStepLength: controls the duration of the first bounce cycle
/// p2 = bounciness: controls amplitude decay and period shrinking per bounce
fn am_bounce(t: f32, first_step_length: f32, bounciness: f32) -> f32 {
    // Edge cases
    if first_step_length == 0.0 {
        return 1.0;
    }

    // AM shifts t by half the first step length
    let adjusted_t = t + (first_step_length / 2.0);

    let mut period_start = 0.0_f32;
    let mut current_period = first_step_length;
    let mut amplitude = 1.0_f32;

    loop {
        let period_end = period_start + current_period;

        if adjusted_t <= period_end {
            // We're in this bounce cycle
            // Check if we've gone past the animation range
            let check_point = (current_period / 3.0) + period_start;
            if check_point > (first_step_length / 2.0) + 1.0
                || (current_period < 0.1 && period_end > (first_step_length / 2.0) + 1.0)
            {
                return 1.0;
            }

            // Calculate parabola within this cycle
            // local_progress goes from 0 to 1 within the cycle
            let local_progress = (adjusted_t - period_start) / current_period;
            // Transform to [-1, 1] range centered at 0.5
            let centered = (local_progress - 0.5) * 2.0;
            // Squared parabola: 0 at edges, 1 at center
            let parabola = centered.abs().powi(2);
            // Apply amplitude and return
            // When parabola=0 (center), return 1-amplitude (lowest point of bounce)
            // When parabola=1 (edges), return 1.0 (target value)
            return (parabola * amplitude) + (1.0 - amplitude);
        }

        // Move to next bounce cycle
        current_period *= bounciness;
        amplitude *= bounciness;

        if amplitude < 0.005 {
            return 1.0;
        }

        period_start = period_end;
    }
}

/// AM-style reverse bounce (ease-in-bounce) with configurable parameters.
///
/// In AM, "reverse bounce" uses ReversedEasing which applies:
/// interpolate(t) = 1 - base.interpolate(1 - t)
fn am_reverse_bounce(t: f32, p1: f32, p2: f32) -> f32 {
    1.0 - am_bounce(1.0 - t, p1, p2)
}

/// AM-style cyclic easing.
///
/// Implementation based on AM source code (CyclicEasing.java).
/// Parameters:
/// - step_length: period of one oscillation cycle (in t-space, 0-1)
/// - sharpness: blend between cosine (0) and saw/triangle (1) wave
/// - skew: shifts the peak position within each cycle (0-1, 0.5 = centered)
/// - decay: how much the oscillation trends toward linear (0 = pure oscillation, 1 = pure linear)
/// - reserved: unused
fn am_cyclic(
    t: f32,
    step_length: f32,
    sharpness: f32,
    skew: f32,
    decay: f32,
    _reserved: f32,
) -> f32 {
    let safe_step = step_length.max(0.001);

    // Helper: percentage within current step
    let pct_in_step = (t % safe_step) / safe_step;

    // Helper: cosine interpolation within step
    let _cos_interp = 1.0 - ((((t / safe_step) * std::f32::consts::PI * 2.0).cos() + 1.0) / 2.0);

    // Helper: saw/triangle interpolation within step
    let _saw_interp = 1.0 - ((pct_in_step - 0.5).abs() * 2.0);

    // Helper: skew interpolation - adjusts t based on skew parameter
    let skew_factor = if pct_in_step < skew {
        (0.5 * pct_in_step) / skew
    } else if pct_in_step > skew {
        0.5 + ((pct_in_step - skew) / (1.0 - skew)) / 2.0
    } else {
        0.5
    };
    let skew_t = (t - (pct_in_step * safe_step)) + (safe_step * skew_factor);

    // Apply skew to both cos and saw interpolations
    let skew_pct = (skew_t % safe_step) / safe_step;
    let skew_cos = 1.0 - ((((skew_t / safe_step) * std::f32::consts::PI * 2.0).cos() + 1.0) / 2.0);
    let skew_saw = 1.0 - ((skew_pct - 0.5).abs() * 2.0);

    // Mix between cosine and saw based on sharpness
    let mut mix = skew_cos * (1.0 - sharpness) + skew_saw * sharpness;

    // Handle edge cases near t=1
    let step_start = t - (t % safe_step);
    if (safe_step / 4.0) + step_start > 1.0 {
        mix = 0.0;
    } else if (safe_step / 2.0) + step_start < 1.0
        && step_start + ((safe_step * 3.0) / 4.0) > 1.0
        && pct_in_step > skew
    {
        mix = 1.0;
    }

    // Apply decay: blend between oscillation and linear progress
    (mix * (1.0 - (t * decay))) + (t * decay)
}

/// Solve cubic bezier curve: find Y for given X.
/// Control points are (0,0), (x1,y1), (x2,y2), (1,1).
///
/// Implementation based on AM source code (CubicBezierEasing.java).
/// AM clamps the X control points: p1x <= 0.95, p2x >= 0.05
fn cubic_bezier_y_for_x(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    // AM's approach: clamp x control points to avoid numerical instability
    let x1_corr = x1.min(0.95);
    let x2_corr = x2.max(0.05);

    // Handle negative x values (extrapolation)
    if x < 0.0 {
        // Use linear extrapolation based on initial slope
        let slope = (bezier_y_at_t(0.01, y1, y2) - bezier_y_at_t(0.0, y1, y2)) / 0.01;
        return x * slope;
    }

    // Find t for given x using Newton's method
    // AM uses more iterations for values near edges
    let iterations = if !(0.05..=0.95).contains(&x) { 24 } else { 8 };

    let mut t = x;
    let mut last_slope = 1000.0_f32;

    for i in 0..iterations {
        let slope = bezier_derivative(t, x1_corr, x2_corr);
        if slope == 0.0 {
            break;
        }

        // AM's early termination: if slope change is very small after initial iterations
        if i > 2
            && (slope - last_slope).abs()
                < 0.01
                    / (if !(0.05..=0.95).contains(&x) {
                        3.0
                    } else {
                        1.0
                    })
        {
            break;
        }

        let x_t = bezier_component(t, x1_corr, x2_corr);
        t -= (x_t - x) / slope;

        last_slope = slope;
    }

    bezier_y_at_t(t, y1, y2)
}

/// Evaluate Y component of bezier at parameter t (same as bezier_component but for clarity)
fn bezier_y_at_t(t: f32, p1: f32, p2: f32) -> f32 {
    bezier_component(t, p1, p2)
}

/// Evaluate one component of a cubic bezier at parameter t.
/// B(t) = 3(1-t)²t*p1 + 3(1-t)t²*p2 + t³
fn bezier_component(t: f32, p1: f32, p2: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;

    3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3
}

/// Derivative of bezier component with respect to t.
fn bezier_derivative(t: f32, p1: f32, p2: f32) -> f32 {
    let t2 = t * t;
    let mt = 1.0 - t;

    3.0 * mt * mt * p1 + 6.0 * mt * t * (p2 - p1) + 3.0 * t2 * (1.0 - p2)
}

/// AM-style elastic easing.
///
/// Implementation based on AM source code (ElasticEasing.java).
/// Parameters:
/// - step_length: period of oscillation
/// - decay: controls amplitude decay
/// - magnitude: oscillation amplitude
fn am_elastic(t: f32, step_length: f32, decay: f32, magnitude: f32) -> f32 {
    // basicElasticEase: cos wave with decay
    let basic_elastic_ease = |t: f32| -> f32 {
        let safe_step = step_length.max(0.01);
        let cos_val = (std::f32::consts::PI * t / safe_step).cos();
        let decay_base = 1.0 - t.max(0.005);
        let decay_power = (decay * decay * 15.0) + 1.0;
        cos_val * decay_base.powf(decay_power).abs()
    };

    // interpolateWithoutAttack
    let safe_step = step_length.max(0.01);
    if t >= step_length {
        1.0 - (basic_elastic_ease(t) * magnitude)
    } else {
        // Mix between cosine ramp and elastic
        let cos_ramp = (1.0 - (std::f32::consts::PI * t / safe_step).cos()) / 2.0;
        let elastic_at_step = basic_elastic_ease(step_length) * magnitude;
        let target = 1.0 - elastic_at_step;
        let elastic_now = 1.0 - (basic_elastic_ease(t) * magnitude);
        let blend_factor = (t / safe_step).powf(3.0);
        // mix(a, b, t) = a * (1 - t) + b * t
        (cos_ramp * target) * (1.0 - blend_factor) + elastic_now * blend_factor
    }
}

/// AM-style elastic step easing.
///
/// Implementation based on AM source code (ElasticStepEasing.java).
fn am_elastic_step(t: f32, step_length: f32, magnitude: f32) -> f32 {
    if t < step_length {
        return 0.0;
    }

    // Create an elastic easing for each step
    // ElasticEasing params derived from magnitude:
    // step_length = 0.5 - 0.45 * magnitude
    // attack = 1.0
    // decay = 1.0 - magnitude * 0.5
    // magnitude = (1.0 - magnitude) * 0.5 + 0.5
    let elastic_step_length = 0.5 - (0.45 * magnitude);
    let elastic_decay = 1.0 - (magnitude * 0.5);
    let elastic_magnitude = ((1.0 - magnitude) * 0.5) + 0.5;

    // Compute step-relative position
    let step_progress = (t % step_length) / step_length;
    let step_base = t - (t % step_length);

    // Interpolate elastic within step and offset by step position
    let elastic_val = am_elastic(
        step_progress,
        elastic_step_length,
        elastic_decay,
        elastic_magnitude,
    );
    step_base + (step_length * (elastic_val - 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_parse() {
        assert_eq!(Easing::parse(""), Easing::Linear);
        assert_eq!(
            Easing::parse("step 1.0 0.0"),
            Easing::Step { x: 1.0, y: 0.0 }
        );
        assert_eq!(
            Easing::parse("cubicBezier 0.0 0.0 0.58 1.0"),
            Easing::CubicBezier {
                x1: 0.0,
                y1: 0.0,
                x2: 0.58,
                y2: 1.0
            }
        );
    }

    #[test]
    fn test_easing_linear() {
        let easing = Easing::Linear;
        assert!((easing.evaluate(0.0) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(0.5) - 0.5).abs() < 0.001);
        assert!((easing.evaluate(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_easing_step() {
        // step 1.0 0.0: single step, no smoothing → hold 0 until t=1.0
        let easing = Easing::Step { x: 1.0, y: 0.0 };
        assert!((easing.evaluate(0.0) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(0.5) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(0.99) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(1.0) - 1.0).abs() < 0.001);

        // step 0.25 0.0: 4 steps, no smoothing → staircase
        let easing = Easing::Step { x: 0.25, y: 0.0 };
        assert!((easing.evaluate(0.0) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(0.1) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(0.25) - 0.25).abs() < 0.001);
        assert!((easing.evaluate(0.5) - 0.5).abs() < 0.001);
        assert!((easing.evaluate(0.75) - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_easing_cubic_bezier() {
        // ease-out curve
        let easing = Easing::CubicBezier {
            x1: 0.0,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0,
        };

        assert!((easing.evaluate(0.0) - 0.0).abs() < 0.01);
        assert!((easing.evaluate(1.0) - 1.0).abs() < 0.01);

        // ease-out should be faster at start
        let mid = easing.evaluate(0.5);
        assert!(mid > 0.5, "ease-out at 0.5 should be > 0.5, got {}", mid);
    }
}
