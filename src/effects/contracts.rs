//! Defines explicit runtime contracts for composite textures and embed-scene
//! render planning.
//!
//! AM keeps pass sources and render-time wrappers as first-class data. This
//! module starts moving the Bevy runtime in the same direction by making
//! texture provenance and embed-scene composite intent explicit instead of
//! scattering them across anonymous flags and booleans.

use bevy::prelude::*;

/// Source category for the main texture sampled by the unified effect shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureSourceKind {
    /// Authored texture or solid-color fill directly owned by the layer.
    #[default]
    LayerTexture,
    /// Output from an offscreen embed-scene render target.
    RenderTarget,
    /// Lift/copy-background composite texture.
    LiftComposite,
    /// Render target produced by an embed-scene mask subtree.
    EmbedMask,
}

impl TextureSourceKind {
    pub const fn sampled_from_offscreen(self) -> bool {
        !matches!(self, Self::LayerTexture)
    }

    pub const fn as_uniform_id(self) -> f32 {
        match self {
            Self::LayerTexture => 0.0,
            Self::RenderTarget => 1.0,
            Self::LiftComposite => 2.0,
            Self::EmbedMask => 3.0,
        }
    }
}

/// Alpha contract carried by a sampled texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureAlphaContract {
    /// RGB is straight (not multiplied by alpha).
    #[default]
    Straight,
    /// RGB is already premultiplied by alpha.
    Premultiplied,
}

impl TextureAlphaContract {
    pub const fn as_uniform_flag(self) -> f32 {
        match self {
            Self::Straight => 0.0,
            Self::Premultiplied => 1.0,
        }
    }
}

/// Explicit contract for a texture that later shader stages will sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureSourceContract {
    pub source_kind: TextureSourceKind,
    pub alpha_contract: TextureAlphaContract,
}

impl TextureSourceContract {
    pub const fn new(source_kind: TextureSourceKind, alpha_contract: TextureAlphaContract) -> Self {
        Self {
            source_kind,
            alpha_contract,
        }
    }

    pub const fn layer_texture() -> Self {
        Self::new(
            TextureSourceKind::LayerTexture,
            TextureAlphaContract::Straight,
        )
    }

    pub const fn render_target() -> Self {
        Self::new(
            TextureSourceKind::RenderTarget,
            TextureAlphaContract::Premultiplied,
        )
    }

    pub const fn lift_composite() -> Self {
        Self::new(
            TextureSourceKind::LiftComposite,
            TextureAlphaContract::Premultiplied,
        )
    }

    pub const fn embed_mask() -> Self {
        Self::new(
            TextureSourceKind::EmbedMask,
            TextureAlphaContract::Premultiplied,
        )
    }

    pub const fn uses_premultiplied_alpha(self) -> bool {
        matches!(self.alpha_contract, TextureAlphaContract::Premultiplied)
    }

    pub fn to_uniform_flags(self) -> Vec4 {
        Vec4::new(
            self.source_kind.sampled_from_offscreen() as u32 as f32,
            self.alpha_contract.as_uniform_flag(),
            self.source_kind.as_uniform_id(),
            0.0,
        )
    }
}

impl Default for TextureSourceContract {
    fn default() -> Self {
        Self::layer_texture()
    }
}

/// First-stage runtime plan for an embed scene before the render strategy is
/// finalized. It keeps composite requirements and the RTT source contract
/// together so later systems do not have to reconstruct that intent from
/// unrelated booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmbedSceneRenderPlan {
    pub requires_composite: bool,
    pub dynamic_resolution: bool,
    pub composite_source_contract: TextureSourceContract,
}

impl EmbedSceneRenderPlan {
    pub const fn new(requires_composite: bool, dynamic_resolution: bool) -> Self {
        Self {
            requires_composite,
            dynamic_resolution,
            composite_source_contract: TextureSourceContract::render_target(),
        }
    }
}

impl Default for EmbedSceneRenderPlan {
    fn default() -> Self {
        Self::new(false, false)
    }
}

#[cfg(test)]
mod tests {
    use super::{TextureAlphaContract, TextureSourceContract, TextureSourceKind};
    use bevy::prelude::Vec4;

    #[test]
    fn test_layer_texture_contract_flags() {
        let flags = TextureSourceContract::layer_texture().to_uniform_flags();
        assert_eq!(flags, Vec4::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_render_target_contract_flags() {
        let flags = TextureSourceContract::render_target().to_uniform_flags();
        assert_eq!(flags, Vec4::new(1.0, 1.0, 1.0, 0.0));
    }

    #[test]
    fn test_custom_contract_helpers() {
        let contract = TextureSourceContract::new(
            TextureSourceKind::LiftComposite,
            TextureAlphaContract::Premultiplied,
        );
        assert!(contract.source_kind.sampled_from_offscreen());
        assert!(contract.uses_premultiplied_alpha());
        assert_eq!(contract.to_uniform_flags(), Vec4::new(1.0, 1.0, 2.0, 0.0));
    }
}
