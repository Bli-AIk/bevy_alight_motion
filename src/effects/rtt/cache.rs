//! Caches RTT textures so they can be reused across loop boundaries.
//! This avoids the GPU upload spikes that occur when many render-to-texture
//! images are re-created at loop boundaries.
//!
//! RTT 纹理缓存，使纹理可以在循环边界复用。
//! 避免了循环边界批量重建 RTT 纹理导致的 GPU 上传尖峰。

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

/// Pool of recycled RTT texture handles keyed by pixel dimensions
/// `(width, height)`.
///
/// 按像素尺寸 `(width, height)` 分组的 RTT 纹理回收池。
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
}
