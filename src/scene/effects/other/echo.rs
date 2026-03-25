//! Extracts the `echokf` effect from layer effect lists.
//! It turns the authored repeat-like echo properties into a normalized parameter
//! struct that runtime systems use to spawn and animate echo copies.
//!
//! 负责从图层 effect 列表里提取 `echokf` 效果。它会把作者写下的
//! echo 属性转换成规范化参数结构，供运行时系统生成并驱动 echo 副本。

use crate::schema::{AmAnimatedFloat, AmEffect};

#[derive(Debug, Clone)]
pub struct EchokfParams {
    pub seconds: AmAnimatedFloat,
    pub count: AmAnimatedFloat,
    pub alpha: AmAnimatedFloat,
    pub mode: i32,
    pub enabled: bool,
}

impl Default for EchokfParams {
    fn default() -> Self {
        Self {
            seconds: AmAnimatedFloat {
                value: Some(0.5),
                keyframes: Vec::new(),
            },
            count: AmAnimatedFloat {
                value: Some(1.0),
                keyframes: Vec::new(),
            },
            alpha: AmAnimatedFloat::default(),
            mode: 1,
            enabled: false,
        }
    }
}

impl EchokfParams {
    pub fn max_count(&self) -> u32 {
        if self.count.keyframes.is_empty() {
            self.count.value.unwrap_or(1.0) as u32
        } else {
            let kf_max = self
                .count
                .keyframes
                .iter()
                .filter_map(|kf| kf.value.parse::<f32>().ok())
                .fold(f32::NEG_INFINITY, f32::max);
            let max = self.count.value.unwrap_or(0.0).max(kf_max);
            max.ceil() as u32
        }
    }

    pub fn static_seconds(&self) -> f32 {
        self.seconds.value.unwrap_or(0.5)
    }

    pub fn is_dynamic(&self) -> bool {
        !self.count.keyframes.is_empty() || !self.seconds.keyframes.is_empty()
    }
}

pub(crate) fn extract_echokf_effect(effects: &[AmEffect]) -> EchokfParams {
    let mut params = EchokfParams::default();

    let effect = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.repeat.echokf");
    let Some(effect) = effect else {
        return params;
    };

    params.enabled = true;

    for prop in &effect.properties {
        match prop.name.as_str() {
            "seconds" => {
                if !prop.keyframes.is_empty() {
                    params.seconds = AmAnimatedFloat {
                        value: prop.value.parse::<f32>().ok(),
                        keyframes: prop.keyframes.clone(),
                    };
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.seconds = AmAnimatedFloat {
                        value: Some(v),
                        keyframes: Vec::new(),
                    };
                }
            }
            "count" => {
                if !prop.keyframes.is_empty() {
                    params.count = AmAnimatedFloat {
                        value: prop.value.parse::<f32>().ok(),
                        keyframes: prop.keyframes.clone(),
                    };
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.count = AmAnimatedFloat {
                        value: Some(v),
                        keyframes: Vec::new(),
                    };
                }
            }
            "alpha" => {
                if !prop.keyframes.is_empty() {
                    params.alpha = AmAnimatedFloat {
                        value: prop.value.parse::<f32>().ok(),
                        keyframes: prop.keyframes.clone(),
                    };
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.alpha = AmAnimatedFloat {
                        value: Some(v),
                        keyframes: Vec::new(),
                    };
                }
            }
            "mode" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.mode = v;
                }
            }
            _ => {}
        }
    }

    params
}
