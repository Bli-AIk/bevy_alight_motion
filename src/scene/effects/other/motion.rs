use crate::schema::{AmAnimatedFloat, AmEffect};

#[derive(Debug, Clone, Default)]
pub struct SwingParams {
    pub freq: AmAnimatedFloat,
    pub a1: AmAnimatedFloat,
    pub a2: AmAnimatedFloat,
    pub phase: AmAnimatedFloat,
    pub swing_type: i32,
}

impl SwingParams {
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.freq.value.is_some()
            || !self.freq.keyframes.is_empty()
            || self.a1.value.is_some()
            || !self.a1.keyframes.is_empty()
            || self.a2.value.is_some()
            || !self.a2.keyframes.is_empty()
    }
}

pub(crate) fn extract_swing_effect(effects: &[AmEffect]) -> SwingParams {
    let mut params = SwingParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.swing2")
    else {
        return params;
    };

    params.a1.value = Some(-30.0);
    params.a2.value = Some(30.0);
    params.freq.value = Some(1.0);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "freq" => {
                if !prop.keyframes.is_empty() {
                    params.freq.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.freq.value = Some(v);
                }
            }
            "a1" => {
                if !prop.keyframes.is_empty() {
                    params.a1.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.a1.value = Some(v);
                }
            }
            "a2" => {
                if !prop.keyframes.is_empty() {
                    params.a2.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.a2.value = Some(v);
                }
            }
            "phase" => {
                if !prop.keyframes.is_empty() {
                    params.phase.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.phase.value = Some(v);
                }
            }
            "type" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.swing_type = v;
                }
            }
            _ => {}
        }
    }

    params
}

pub(crate) fn extract_spin_rpm(effects: &[AmEffect]) -> AmAnimatedFloat {
    let mut rpm = AmAnimatedFloat::default();
    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.spin")
    else {
        return rpm;
    };

    rpm.value = Some(60.0);
    for prop in &effect.properties {
        if prop.name == "rpm" {
            if !prop.keyframes.is_empty() {
                rpm.keyframes = prop.keyframes.clone();
            } else if let Ok(v) = prop.value.parse::<f32>() {
                rpm.value = Some(v);
            }
        }
    }
    rpm
}

#[derive(Debug, Clone, Default)]
pub struct OscillateParams {
    pub direction: i32,
    pub angle: AmAnimatedFloat,
    pub freq: AmAnimatedFloat,
    pub mag: AmAnimatedFloat,
    pub wave_type: i32,
    pub phase: AmAnimatedFloat,
}

pub(crate) fn extract_oscillate_effect(effects: &[AmEffect]) -> OscillateParams {
    let mut params = OscillateParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.oscillate3")
    else {
        return params;
    };

    params.angle.value = Some(45.0);
    params.freq.value = Some(2.0);
    params.mag.value = Some(25.0);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "direction" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.direction = v;
                }
            }
            "angle" => {
                if !prop.keyframes.is_empty() {
                    params.angle.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.angle.value = Some(v);
                }
            }
            "freq" => {
                if !prop.keyframes.is_empty() {
                    params.freq.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.freq.value = Some(v);
                }
            }
            "mag" => {
                if !prop.keyframes.is_empty() {
                    params.mag.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.mag.value = Some(v);
                }
            }
            "type" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.wave_type = v;
                }
            }
            "phase" => {
                if !prop.keyframes.is_empty() {
                    params.phase.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.phase.value = Some(v);
                }
            }
            _ => {}
        }
    }

    params
}

#[derive(Debug, Clone)]
pub struct JitterParams {
    pub angle: AmAnimatedFloat,
    pub freq: AmAnimatedFloat,
    pub mag: AmAnimatedFloat,
    pub seed: AmAnimatedFloat,
    pub slack: AmAnimatedFloat,
    pub zjitter: AmAnimatedFloat,
    pub enabled: bool,
}

impl Default for JitterParams {
    fn default() -> Self {
        Self {
            angle: AmAnimatedFloat {
                value: Some(45.0),
                keyframes: Vec::new(),
            },
            freq: AmAnimatedFloat {
                value: Some(30.0),
                keyframes: Vec::new(),
            },
            mag: AmAnimatedFloat {
                value: Some(25.0),
                keyframes: Vec::new(),
            },
            seed: AmAnimatedFloat {
                value: Some(0.0),
                keyframes: Vec::new(),
            },
            slack: AmAnimatedFloat {
                value: Some(0.0),
                keyframes: Vec::new(),
            },
            zjitter: AmAnimatedFloat {
                value: Some(0.0),
                keyframes: Vec::new(),
            },
            enabled: false,
        }
    }
}

pub(crate) fn extract_jitter_effect(effects: &[AmEffect]) -> JitterParams {
    let mut params = JitterParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.jitter")
    else {
        return params;
    };

    params.enabled = true;

    fn parse_animated_float(prop: &crate::schema::AmProperty, default: f32) -> AmAnimatedFloat {
        if !prop.keyframes.is_empty() {
            AmAnimatedFloat {
                value: prop.value.parse::<f32>().ok().or(Some(default)),
                keyframes: prop.keyframes.clone(),
            }
        } else if let Ok(v) = prop.value.parse::<f32>() {
            AmAnimatedFloat {
                value: Some(v),
                keyframes: Vec::new(),
            }
        } else {
            AmAnimatedFloat {
                value: Some(default),
                keyframes: Vec::new(),
            }
        }
    }

    for prop in &effect.properties {
        match prop.name.as_str() {
            "angle" => params.angle = parse_animated_float(prop, 45.0),
            "freq" => params.freq = parse_animated_float(prop, 30.0),
            "mag" => params.mag = parse_animated_float(prop, 25.0),
            "seed" => params.seed = parse_animated_float(prop, 0.0),
            "slack" => params.slack = parse_animated_float(prop, 0.0),
            "zjitter" => params.zjitter = parse_animated_float(prop, 0.0),
            _ => {}
        }
    }

    params
}
