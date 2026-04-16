//! Caches RTT textures so they can be reused across loop boundaries and
//! pre-warmed during project loading.  This avoids the GPU upload spikes that
//! occur when many render-to-texture images are created in a single frame.
//!
//! RTT 纹理缓存，使纹理可以在循环边界复用，也可以在项目加载期间预热。
//! 避免了单帧内批量创建 RTT 纹理导致的 GPU 上传尖峰。

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::scene::{AmLayerSpec, PendingLayer};

/// Pool of pre-created or recycled RTT texture handles keyed by pixel
/// dimensions `(width, height)`.
///
/// 按像素尺寸 `(width, height)` 分组的 RTT 纹理池。
#[derive(Resource, Default)]
pub struct RttTextureCache {
    pools: HashMap<(u32, u32), VecDeque<Handle<Image>>>,
}

impl RttTextureCache {
    /// Return a cached texture handle with the given pixel dimensions, if any.
    pub fn pop(&mut self, width: u32, height: u32) -> Option<Handle<Image>> {
        self.pools.get_mut(&(width, height))?.pop_front()
    }

    /// Store a texture handle for later reuse.
    pub fn push(&mut self, width: u32, height: u32, handle: Handle<Image>) {
        self.pools
            .entry((width, height))
            .or_default()
            .push_back(handle);
    }

    /// Number of cached textures across all dimension buckets.
    pub fn total_cached(&self) -> usize {
        self.pools.values().map(|v| v.len()).sum()
    }

    /// Pre-warm the cache by scanning pending layers for embed scenes that
    /// will require composite RTT textures, and creating those textures
    /// ahead of time so the GPU upload happens before the first frame.
    ///
    /// 扫描待生成图层中的嵌套场景，为需要 composite RTT 的图层预创建纹理，
    /// 让 GPU 上传在首帧之前就完成。
    pub fn pre_warm(&mut self, pending_layers: &[PendingLayer], images: &mut Assets<Image>) {
        let format = selected_rtt_format();
        let mut count = 0u32;
        Self::walk_for_prewarm(pending_layers, images, &format, &mut count, self);
        if count > 0 {
            bevy::log::info!("Pre-warmed {count} RTT textures into cache");
        }
    }

    fn walk_for_prewarm(
        layers: &[PendingLayer],
        images: &mut Assets<Image>,
        format: &TextureFormat,
        count: &mut u32,
        cache: &mut Self,
    ) {
        for layer in layers {
            if matches!(layer.spec, AmLayerSpec::EmbedScene)
                && let Some(plan) = &layer.embed_render_plan
                && plan.requires_composite
                && let Some((w, h)) = layer.embed_scene_size
            {
                let tw = w.max(1.0).ceil() as u32;
                let th = h.max(1.0).ceil() as u32;
                let tex = Image::new_target_texture(tw, th, *format, None);
                cache.push(tw, th, images.add(tex));
                *count += 1;
            }
            Self::walk_for_prewarm(&layer.children, images, format, count, cache);
        }
    }
}

fn selected_rtt_format() -> TextureFormat {
    match std::env::var("AM_EMBED_RTT_FORMAT").ok().as_deref() {
        Some("rgba8unorm") => TextureFormat::Rgba8Unorm,
        Some("bgra8unormsrgb") => TextureFormat::Bgra8UnormSrgb,
        Some("bgra8unorm") => TextureFormat::Bgra8Unorm,
        _ => TextureFormat::Rgba8UnormSrgb,
    }
}
