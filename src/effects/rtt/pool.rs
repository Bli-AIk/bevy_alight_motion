//! This file implements the small allocator for embed-scene render layers.
//! Composite embeds need unique off-screen layers, and this pool tracks which
//! slots are in use so RTT setup and cleanup can borrow and release them safely.
//!
//! 这个文件实现了嵌套场景 render layer 的小型分配器。走 composite 路径的 embed
//! 需要唯一的离屏 layer，这个池子会记录哪些槽位正在使用，让 RTT 的设置与清理阶段
//! 能够安全地借出和归还它们。

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct EmbedSceneRenderLayerPool {
    used_layers: u32,
    waiting_count: u32,
}

impl EmbedSceneRenderLayerPool {
    pub fn acquire(&mut self) -> Option<u8> {
        for i in 0..31 {
            if (self.used_layers & (1 << i)) == 0 {
                self.used_layers |= 1 << i;
                return Some(i + 1);
            }
        }
        self.waiting_count += 1;
        None
    }

    pub fn allocate(&mut self) -> Option<u8> {
        self.acquire()
    }

    pub fn release(&mut self, layer: u8) {
        if (1..=31).contains(&layer) {
            self.used_layers &= !(1 << (layer - 1));
            if self.waiting_count > 0 {
                self.waiting_count -= 1;
            }
        }
    }

    #[allow(dead_code)]
    pub fn used_count(&self) -> u32 {
        self.used_layers.count_ones()
    }

    #[allow(dead_code)]
    pub fn available_count(&self) -> u32 {
        31 - self.used_layers.count_ones()
    }

    pub fn is_exhausted(&self) -> bool {
        self.used_layers == 0x7FFF_FFFF
    }
}
