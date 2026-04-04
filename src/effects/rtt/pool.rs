//! Implements the small allocator for embed-scene render layers.
//! Composite embeds need unique off-screen layers, and this pool tracks which
//! slots are in use so RTT setup and cleanup can borrow and release them safely.
//!
//! 实现了嵌套场景 render layer 的小型分配器。走 composite 路径的 embed
//! 需要唯一的离屏 layer，这个池子会记录哪些槽位正在使用，让 RTT 的设置与清理阶段
//! 能够安全地借出和归还它们。

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct EmbedSceneRenderLayerPool {
    used_layers: Vec<u64>,
}

impl EmbedSceneRenderLayerPool {
    pub fn acquire(&mut self) -> Option<usize> {
        for (block_index, block) in self.used_layers.iter_mut().enumerate() {
            if *block != u64::MAX {
                let bit_index = block.trailing_ones() as usize;
                *block |= 1u64 << bit_index;
                return Some(block_index * 64 + bit_index + 1);
            }
        }

        self.used_layers.push(1u64);
        Some(self.used_layers.len().saturating_sub(1) * 64 + 1)
    }

    pub fn allocate(&mut self) -> Option<usize> {
        self.acquire()
    }

    pub fn release(&mut self, layer: usize) {
        if layer == 0 {
            return;
        }

        let layer_index = layer - 1;
        let block_index = layer_index / 64;
        let bit_index = layer_index % 64;
        let Some(block) = self.used_layers.get_mut(block_index) else {
            return;
        };
        *block &= !(1u64 << bit_index);

        while matches!(self.used_layers.last(), Some(&0)) {
            self.used_layers.pop();
        }
    }

    #[allow(dead_code)]
    pub fn used_count(&self) -> u32 {
        self.used_layers
            .iter()
            .map(|block| block.count_ones())
            .sum()
    }

    #[allow(dead_code)]
    pub fn available_count(&self) -> u32 {
        u32::MAX.saturating_sub(self.used_count())
    }

    pub fn is_exhausted(&self) -> bool {
        false
    }
}
