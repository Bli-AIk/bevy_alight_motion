//! Keeps the legacy CPU-side mask-clipping entry point for SDF layers.
//! 保留 SDF 图层旧的 CPU 侧遮罩裁剪入口。
//!
//! Shader-based mask clipping has replaced this path, so the system body is intentionally disabled.
//! The file still exists as the named hook for the SDF animation module, documenting that clipping
//! is now expected to happen in materials instead of by toggling entity visibility here.
//! 现在遮罩裁剪已经迁移到 shader/material 路径，因此这个系统体是刻意停用的。
//! 保留此文件是为了作为 SDF 动画模块里的具名入口，明确说明裁剪职责已经转到材质层，
//! 不再由这里通过可见性切换来完成。

use bevy::prelude::*;

pub fn apply_mask_clipping_system(
    _playback: Res<crate::animation::AmPlayback>,
    _query: Query<(
        &GlobalTransform,
        &ChildOf,
        &crate::scene::AmMaskInfo,
        &mut Visibility,
        &crate::scene::AmLayerMarker,
    )>,
    _parent_query: Query<&GlobalTransform>,
) {
    // Disabled: using shader-based mask clipping instead
}
