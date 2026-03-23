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
