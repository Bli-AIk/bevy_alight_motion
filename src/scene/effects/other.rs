mod echo;
mod image;
mod motion;

use crate::schema::AmKeyframe;

pub(crate) use echo::*;
pub(crate) use image::*;
pub(crate) use motion::*;

fn parse_vec2_value(value: &str) -> Option<[f32; 2]> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    let x = parts[0].trim().parse::<f32>().ok()?;
    let y = parts[1].trim().parse::<f32>().ok()?;
    Some([x, y])
}

fn parse_color_keyframe(kf: &AmKeyframe) -> Option<AmKeyframe> {
    let color = crate::schema::parse_color(&kf.value).ok()?;
    Some(AmKeyframe {
        time: kf.time,
        value: format!("{},{},{},{}", color[0], color[1], color[2], color[3]),
        easing: kf.easing.clone(),
    })
}
