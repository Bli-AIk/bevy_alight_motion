//! This file groups miscellaneous effect extractors that do not fit the larger
//! common/transform/compositing buckets. It re-exports echo, image, and motion
//! effect parsers and provides a couple of tiny parsing helpers they share.
//!
//! 这个文件把那些不适合放进 common/transform/compositing 分类的杂项 effect
//! extractor 收拢到一起。它重导出 echo、image 和 motion 相关解析器，并提供几个
//! 这些模块共用的小型解析辅助函数。

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
