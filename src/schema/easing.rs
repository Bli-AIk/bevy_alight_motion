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
    /// Cyclic easing (sinusoidal oscillation).
    /// Creates a wave-like motion with multiple oscillations between keyframes.
    /// Parameters: period (cycle length), phase, amplitude, p4, p5
    Cyclic {
        period: f32,
        phase: f32,
        amplitude: f32,
        p4: f32,
        p5: f32,
    },
}

impl Easing {
    /// Parse easing string from AM format.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.is_empty() {
            return Easing::Linear;
        }

        let parts: Vec<&str> = s.split_whitespace().collect();
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
                let period = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.1);
                let phase = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let amplitude = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.5);
                let p4 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let p5 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                Easing::Cyclic {
                    period,
                    phase,
                    amplitude,
                    p4,
                    p5,
                }
            }
            _ => Easing::Linear,
        }
    }

    /// Evaluate the easing function at normalized time t (0.0-1.0).
    pub fn evaluate(&self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::Step { .. } => {
                // Step function: hold previous value until t reaches 1.0
                if t < 1.0 { 0.0 } else { 1.0 }
            }
            Easing::CubicBezier { x1, y1, x2, y2 } => cubic_bezier_y_for_x(t, *x1, *y1, *x2, *y2),
            Easing::Bounce { p1, p2 } => am_bounce(t, *p1, *p2),
            Easing::Cyclic {
                period,
                phase,
                amplitude,
                ..
            } => am_cyclic(t, *period, *phase, *amplitude),
        }
    }
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
/// The bounce curve shows multiple bounces with slow amplitude decay.
/// p1 controls the first touch timing, p2 controls amplitude retention.
fn am_bounce(t: f32, p1: f32, p2: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    // From analysis: first_touch ≈ p1/2, period ≈ p1
    // p2 is amplitude retention per bounce (~0.96 = slow decay)
    let first_touch = p1 * 0.47; // Calibrated from video data
    let period = p1 * 0.93; // Calibrated from video data
    let n_bounces = 5;
    let amplitude_retention = p2;

    // First descent
    if t < first_touch {
        let progress = t / first_touch;
        return progress * progress;
    }

    // After first touch: bouncing
    let time_after = t - first_touch;

    // Which bounce cycle?
    let cycle = (time_after / period) as i32;
    if cycle >= n_bounces {
        return 1.0;
    }

    let local_t = (time_after - cycle as f32 * period) / period;

    // Amplitude with slow decay
    let amplitude = amplitude_retention.powi(cycle);

    // Parabola: at local_t=0 and 1, we're at bottom; at 0.5, we're at peak
    let bounce_height = 4.0 * local_t * (1.0 - local_t) * amplitude;

    1.0 - bounce_height
}

/// AM-style cyclic easing with sinusoidal oscillation.
///
/// Creates a wave-like motion that oscillates around a center point.
/// Based on reference video analysis: the cyclic easing produces pure
/// sinusoidal oscillation independent of linear interpolation progress.
///
/// period: length of one cycle in t-space (0.0856 = ~11.7 cycles)
/// phase: center offset factor (affects where oscillation is centered)
/// amplitude: oscillation amplitude (0.5 = ±0.5 around center)
///
/// Derived formula from video frame analysis:
/// eased_t = center + amplitude * sin(2π * cycles * t + φ)
///
/// Where:
/// - center = 0.5 + phase / 3.0 (empirically derived)
/// - cycles = 1.0 / period
/// - φ = -π/2 radians (-90°) initial phase offset
///   This starts the wave at its minimum (valley)
///
/// This produces:
/// - Pure sinusoidal oscillation around the center
/// - At t=0: eased_t = center - amplitude (wave valley)
/// - At t=0.25/cycles: eased_t = center (midpoint, rising)
/// - At t=0.5/cycles: eased_t = center + amplitude (wave peak)
fn am_cyclic(t: f32, period: f32, phase: f32, amplitude: f32) -> f32 {
    // Prevent division by zero
    let safe_period = period.max(0.001);

    // Number of cycles over the keyframe span
    let cycles = 1.0 / safe_period;

    // Center of oscillation, offset by phase parameter
    // Empirically derived: phase/3 gives the correct center offset
    let center = 0.5 + phase / 3.0;

    // Initial phase offset: -π/2 starts at wave valley
    let phi = -std::f32::consts::FRAC_PI_2;

    // Angle for sine oscillation
    let angle = 2.0 * std::f32::consts::PI * cycles * t + phi;

    // Cyclic easing: pure sinusoidal oscillation around center
    center + amplitude * angle.sin()
}

/// Solve cubic bezier curve: find Y for given X.
/// Control points are (0,0), (x1,y1), (x2,y2), (1,1).
fn cubic_bezier_y_for_x(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    // Apply AM-specific x-coordinate correction ONLY when:
    // - x1 is close to 1.0 (within 0.1)
    // - x2 is close to 0.0 (within 0.1)
    //
    // For these "extreme" curves, AM uses slightly different x-coordinates:
    //   x1_corrected = x1 * 0.95
    //   x2_corrected = x2 * 0.95 + 0.05
    //
    // This was derived from video frame analysis comparing AM's actual output
    // with standard CSS cubic-bezier behavior.
    let (x1_corr, x2_corr) = if (x1 - 1.0).abs() < 0.1 && x2.abs() < 0.1 {
        (x1 * 0.95, x2 * 0.95 + 0.05)
    } else {
        (x1, x2)
    };

    // Find t for given x using Newton's method
    let mut t = x;
    for _ in 0..8 {
        let x_t = bezier_component(t, x1_corr, x2_corr);
        let dx = x - x_t;
        if dx.abs() < 1e-6 {
            break;
        }
        let dx_dt = bezier_derivative(t, x1_corr, x2_corr);
        if dx_dt.abs() < 1e-6 {
            break;
        }
        t += dx / dx_dt;
        t = t.clamp(0.0, 1.0);
    }

    bezier_component(t, y1, y2)
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
        let easing = Easing::Step { x: 1.0, y: 0.0 };
        assert!((easing.evaluate(0.0) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(0.5) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(0.99) - 0.0).abs() < 0.001);
        assert!((easing.evaluate(1.0) - 1.0).abs() < 0.001);
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
