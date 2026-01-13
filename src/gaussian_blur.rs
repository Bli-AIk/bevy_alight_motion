//! Gaussian Blur Effect Implementation
//!
//! This module provides a high-performance separable Gaussian blur using RTT (Render-to-Texture).
//! The blur is implemented as two 1D passes (horizontal + vertical) instead of a single 2D pass,
//! reducing complexity from O(radius²) to O(2 * radius).
//!
//! ## Architecture
//!
//! For each layer with blur:
//! 1. Original sprite renders to the main scene
//! 2. Horizontal blur pass: Original -> RTT_H (rendered by blur camera H)
//! 3. Vertical blur pass: RTT_H -> RTT_V (rendered by blur camera V)
//! 4. Display RTT_V as the final sprite (replaces original)
//!
//! ## Implementation Strategy
//!
//! Since Bevy's Material2d requires render passes, we use a multi-camera approach:
//! - Camera 0: Main scene (default)
//! - Camera H: Renders horizontal blur pass to RTT_H
//! - Camera V: Renders vertical blur pass to RTT_V (final output)
//!
//! The final output is displayed as a sprite with RTT_V texture.

use bevy::{
    camera::RenderTarget,
    camera::visibility::RenderLayers,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

// ============================================================================
// Materials for blur passes
// ============================================================================

/// Material for horizontal Gaussian blur pass.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GaussianBlurHMaterial {
    /// Blur parameters: (radius, tex_width, tex_height, unused)
    #[uniform(0)]
    pub blur_params: Vec4,

    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

impl Default for GaussianBlurHMaterial {
    fn default() -> Self {
        Self {
            blur_params: Vec4::ZERO,
            texture: None,
        }
    }
}

impl GaussianBlurHMaterial {
    pub fn new(texture: Handle<Image>, radius: f32, width: f32, height: f32) -> Self {
        Self {
            blur_params: Vec4::new(radius, width, height, 0.0),
            texture: Some(texture),
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.blur_params.x = radius;
    }
}

impl Material2d for GaussianBlurHMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/gaussian_blur_h.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Material for vertical Gaussian blur pass.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GaussianBlurVMaterial {
    /// Blur parameters: (radius, tex_width, tex_height, unused)
    #[uniform(0)]
    pub blur_params: Vec4,

    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

impl Default for GaussianBlurVMaterial {
    fn default() -> Self {
        Self {
            blur_params: Vec4::ZERO,
            texture: None,
        }
    }
}

impl GaussianBlurVMaterial {
    pub fn new(texture: Handle<Image>, radius: f32, width: f32, height: f32) -> Self {
        Self {
            blur_params: Vec4::new(radius, width, height, 0.0),
            texture: Some(texture),
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.blur_params.x = radius;
    }
}

impl Material2d for GaussianBlurVMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/gaussian_blur_v.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

// ============================================================================
// Components
// ============================================================================

/// Component marking an entity that needs Gaussian blur effect.
/// Attach this to the original layer entity.
#[derive(Component, Debug, Clone)]
pub struct GaussianBlurEffect {
    /// Blur radius in pixels (AM strength * multiplier)
    pub radius: f32,
    /// Source texture width
    pub width: f32,
    /// Source texture height
    pub height: f32,
    /// Whether RTT infrastructure has been set up
    pub rtt_ready: bool,
}

impl Default for GaussianBlurEffect {
    fn default() -> Self {
        Self {
            radius: 0.0,
            width: 100.0,
            height: 100.0,
            rtt_ready: false,
        }
    }
}

/// Component storing the RTT resources for a blurred layer.
#[derive(Component, Debug)]
pub struct GaussianBlurRtt {
    /// Texture after horizontal blur pass
    pub rtt_h: Handle<Image>,
    /// Texture after vertical blur pass (final output)
    pub rtt_v: Handle<Image>,
    /// Camera for horizontal blur pass
    pub camera_h: Entity,
    /// Camera for vertical blur pass
    pub camera_v: Entity,
    /// Mesh entity for horizontal blur pass (source -> rtt_h)
    pub mesh_h: Entity,
    /// Mesh entity for vertical blur pass (rtt_h -> rtt_v)
    pub mesh_v: Entity,
    /// RenderLayer for horizontal pass
    pub layer_h: u8,
    /// RenderLayer for vertical pass
    pub layer_v: u8,
    /// Original texture (source)
    pub original_texture: Handle<Image>,
}

/// Marker for blur pass mesh entities
#[derive(Component, Debug)]
pub struct BlurPassMesh {
    /// Parent blur entity
    pub parent_entity: Entity,
    /// Which pass (0 = horizontal, 1 = vertical)
    pub pass: u8,
}

/// Marker for blur pass cameras
#[derive(Component, Debug)]
pub struct BlurPassCamera {
    /// Parent blur entity
    pub parent_entity: Entity,
    /// Which pass (0 = horizontal, 1 = vertical)
    pub pass: u8,
    /// RenderLayer for cleanup
    pub render_layer: u8,
}

// ============================================================================
// Resource for render layer allocation
// ============================================================================

/// Resource managing render layers for blur passes.
/// Each blur layer needs 2 layers (H and V passes).
#[derive(Resource, Default)]
pub struct BlurRenderLayerPool {
    /// Bitset tracking which layers are in use
    /// Blur uses layers 20-31 to avoid conflict with embed RTT (1-19)
    used_layers: u32,
}

impl BlurRenderLayerPool {
    const BLUR_LAYER_START: u8 = 20;
    const BLUR_LAYER_END: u8 = 31;

    /// Allocate a pair of render layers for H and V passes.
    pub fn allocate_pair(&mut self) -> Option<(u8, u8)> {
        for i in 0..((Self::BLUR_LAYER_END - Self::BLUR_LAYER_START) / 2) {
            let bit_h = i * 2;
            let bit_v = i * 2 + 1;
            if (self.used_layers & (1 << bit_h)) == 0 && (self.used_layers & (1 << bit_v)) == 0 {
                self.used_layers |= (1 << bit_h) | (1 << bit_v);
                return Some((
                    Self::BLUR_LAYER_START + bit_h,
                    Self::BLUR_LAYER_START + bit_v,
                ));
            }
        }
        None
    }

    /// Release a pair of render layers.
    pub fn release_pair(&mut self, layer_h: u8, layer_v: u8) {
        if layer_h >= Self::BLUR_LAYER_START && layer_h <= Self::BLUR_LAYER_END {
            let bit = layer_h - Self::BLUR_LAYER_START;
            self.used_layers &= !(1 << bit);
        }
        if layer_v >= Self::BLUR_LAYER_START && layer_v <= Self::BLUR_LAYER_END {
            let bit = layer_v - Self::BLUR_LAYER_START;
            self.used_layers &= !(1 << bit);
        }
    }
}

// ============================================================================
// Systems
// ============================================================================

/// System to set up RTT infrastructure for blur effects.
pub fn setup_blur_rtt_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut h_materials: ResMut<Assets<GaussianBlurHMaterial>>,
    mut v_materials: ResMut<Assets<GaussianBlurVMaterial>>,
    mut layer_pool: ResMut<BlurRenderLayerPool>,
    query: Query<(Entity, &GaussianBlurEffect, &Sprite), Without<GaussianBlurRtt>>,
) {
    for (entity, blur_effect, sprite) in query.iter() {
        // Only set up if blur is actually needed
        if blur_effect.radius < 0.5 {
            continue;
        }

        // Allocate render layers
        let Some((layer_h, layer_v)) = layer_pool.allocate_pair() else {
            bevy::log::warn!("No available render layers for blur on {:?}", entity);
            continue;
        };

        let orig_width = blur_effect.width;
        let orig_height = blur_effect.height;
        
        // Calculate blur expansion - the blur glow extends beyond original bounds
        // Use 2x radius for full coverage of the Gaussian distribution
        let blur_expansion = blur_effect.radius * 2.0;
        
        // Expanded dimensions for RTT textures (original + expansion on all sides)
        let expanded_width = orig_width + blur_expansion * 2.0;
        let expanded_height = orig_height + blur_expansion * 2.0;

        // Create RTT textures with expanded dimensions
        let rtt_h = create_rtt_texture(&mut images, expanded_width, expanded_height, "blur_rtt_h");
        let rtt_v = create_rtt_texture(&mut images, expanded_width, expanded_height, "blur_rtt_v");

        // Create mesh for blur passes with expanded dimensions
        // The mesh is centered, so it covers [-expanded_width/2, expanded_width/2]
        let blur_mesh = create_blur_mesh_with_uv_expansion(
            &mut meshes,
            orig_width,
            orig_height,
            blur_expansion,
        );

        // Get original texture from sprite
        let original_texture = sprite.image.clone();

        // Create horizontal blur material
        // Pass original dimensions for correct UV calculations in shader
        let h_material = h_materials.add(GaussianBlurHMaterial::new(
            original_texture.clone(),
            blur_effect.radius,
            orig_width,
            orig_height,
        ));

        // Create vertical blur material (input is rtt_h output)
        // Pass expanded dimensions since rtt_h is expanded
        let v_material = v_materials.add(GaussianBlurVMaterial::new(
            rtt_h.clone(),
            blur_effect.radius,
            expanded_width,
            expanded_height,
        ));

        // Create horizontal blur pass mesh entity
        let mesh_h = commands
            .spawn((
                Name::new("BlurPassH"),
                BlurPassMesh {
                    parent_entity: entity,
                    pass: 0,
                },
                bevy::mesh::Mesh2d(blur_mesh.clone()),
                MeshMaterial2d(h_material),
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLayers::layer(layer_h as usize),
                Visibility::Visible,
            ))
            .id();

        // Create vertical blur pass mesh entity
        let mesh_v = commands
            .spawn((
                Name::new("BlurPassV"),
                BlurPassMesh {
                    parent_entity: entity,
                    pass: 1,
                },
                bevy::mesh::Mesh2d(blur_mesh),
                MeshMaterial2d(v_material),
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLayers::layer(layer_v as usize),
                Visibility::Visible,
            ))
            .id();

        // Create camera for horizontal pass
        let camera_h = commands
            .spawn((
                Name::new("BlurCameraH"),
                BlurPassCamera {
                    parent_entity: entity,
                    pass: 0,
                    render_layer: layer_h,
                },
                Camera2d,
                Camera {
                    target: RenderTarget::Image(rtt_h.clone().into()),
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    order: -100 - (layer_h as isize),
                    ..default()
                },
                RenderLayers::layer(layer_h as usize),
                Transform::from_xyz(0.0, 0.0, 1000.0),
            ))
            .id();

        // Set orthographic projection to match RTT dimensions
        commands.entity(camera_h).insert(Projection::Orthographic(OrthographicProjection {
            near: -1000.0,
            far: 1000.0,
            scale: 1.0,
            area: Rect::new(
                -expanded_width / 2.0,
                -expanded_height / 2.0,
                expanded_width / 2.0,
                expanded_height / 2.0,
            ),
            ..OrthographicProjection::default_2d()
        }));

        // Create camera for vertical pass
        let camera_v = commands
            .spawn((
                Name::new("BlurCameraV"),
                BlurPassCamera {
                    parent_entity: entity,
                    pass: 1,
                    render_layer: layer_v,
                },
                Camera2d,
                Camera {
                    target: RenderTarget::Image(rtt_v.clone().into()),
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    order: -100 - (layer_v as isize),
                    ..default()
                },
                RenderLayers::layer(layer_v as usize),
                Transform::from_xyz(0.0, 0.0, 1000.0),
            ))
            .id();

        // Set orthographic projection to match RTT dimensions
        commands.entity(camera_v).insert(Projection::Orthographic(OrthographicProjection {
            near: -1000.0,
            far: 1000.0,
            scale: 1.0,
            area: Rect::new(
                -expanded_width / 2.0,
                -expanded_height / 2.0,
                expanded_width / 2.0,
                expanded_height / 2.0,
            ),
            ..OrthographicProjection::default_2d()
        }));

        // Update the original entity's sprite to display final RTT output
        // Use expanded dimensions so the blur glow is visible
        commands.entity(entity).insert((
            GaussianBlurRtt {
                rtt_h,
                rtt_v: rtt_v.clone(),
                camera_h,
                camera_v,
                mesh_h,
                mesh_v,
                layer_h,
                layer_v,
                original_texture,
            },
            Sprite {
                image: rtt_v,
                custom_size: Some(Vec2::new(expanded_width, expanded_height)),
                ..default()
            },
        ));

        bevy::log::info!(
            "[BlurRTT] Set up blur RTT for {:?}: radius={:.1}, orig={}x{}, expanded={}x{}, layers=({}, {})",
            entity,
            blur_effect.radius,
            orig_width,
            orig_height,
            expanded_width,
            expanded_height,
            layer_h,
            layer_v
        );
    }
}

/// System to update blur parameters when radius changes.
pub fn update_blur_params_system(
    query: Query<(Entity, &GaussianBlurEffect, &GaussianBlurRtt), Changed<GaussianBlurEffect>>,
    mut h_materials: ResMut<Assets<GaussianBlurHMaterial>>,
    mut v_materials: ResMut<Assets<GaussianBlurVMaterial>>,
    mesh_h_query: Query<(&BlurPassMesh, &MeshMaterial2d<GaussianBlurHMaterial>)>,
    mesh_v_query: Query<(&BlurPassMesh, &MeshMaterial2d<GaussianBlurVMaterial>)>,
) {
    for (entity, blur_effect, _rtt) in query.iter() {
        // Update horizontal material for this entity
        for (mesh_marker, material_handle) in mesh_h_query.iter() {
            if mesh_marker.parent_entity == entity && mesh_marker.pass == 0 {
                if let Some(material) = h_materials.get_mut(&material_handle.0) {
                    material.set_radius(blur_effect.radius);
                    bevy::log::debug!(
                        "[BlurRTT] Updated H material radius to {:.1} for {:?}",
                        blur_effect.radius,
                        entity
                    );
                }
            }
        }

        // Update vertical material for this entity
        for (mesh_marker, material_handle) in mesh_v_query.iter() {
            if mesh_marker.parent_entity == entity && mesh_marker.pass == 1 {
                if let Some(material) = v_materials.get_mut(&material_handle.0) {
                    material.set_radius(blur_effect.radius);
                    bevy::log::debug!(
                        "[BlurRTT] Updated V material radius to {:.1} for {:?}",
                        blur_effect.radius,
                        entity
                    );
                }
            }
        }
    }
}

/// System to clean up blur RTT resources when entities are despawned.
pub fn cleanup_blur_rtt_system(
    mut commands: Commands,
    mut layer_pool: ResMut<BlurRenderLayerPool>,
    mut removed: RemovedComponents<GaussianBlurRtt>,
    rtt_query: Query<&GaussianBlurRtt>,
    mesh_query: Query<(Entity, &BlurPassMesh)>,
    camera_query: Query<(Entity, &BlurPassCamera)>,
) {
    // Handle removed blur components
    for entity in removed.read() {
        bevy::log::debug!("Blur RTT removed from {:?}", entity);
    }

    // Clean up orphaned blur meshes
    for (mesh_entity, mesh_marker) in mesh_query.iter() {
        if rtt_query.get(mesh_marker.parent_entity).is_err() {
            bevy::log::debug!("Despawning orphaned blur mesh {:?}", mesh_entity);
            commands.entity(mesh_entity).despawn();
        }
    }

    // Clean up orphaned blur cameras
    for (camera_entity, camera_marker) in camera_query.iter() {
        if rtt_query.get(camera_marker.parent_entity).is_err() {
            layer_pool.release_pair(camera_marker.render_layer, camera_marker.render_layer + 1);
            bevy::log::debug!(
                "Despawning orphaned blur camera {:?}, releasing layer {}",
                camera_entity,
                camera_marker.render_layer
            );
            commands.entity(camera_entity).despawn();
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn create_rtt_texture(images: &mut Assets<Image>, width: f32, height: f32, _label: &str) -> Handle<Image> {
    let extent = Extent3d {
        width: width.max(1.0) as u32,
        height: height.max(1.0) as u32,
        depth_or_array_layers: 1,
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(extent);
    images.add(image)
}

fn create_blur_mesh(meshes: &mut Assets<Mesh>, width: f32, height: f32) -> Handle<Mesh> {
    create_blur_mesh_with_uv_expansion(meshes, width, height, 0.0)
}

/// Create a mesh for blur passes with UV expansion for glow overflow.
/// The mesh is physically expanded by blur_expansion on each side,
/// and UVs extend beyond [0,1] range to sample the transparent boundary.
fn create_blur_mesh_with_uv_expansion(
    meshes: &mut Assets<Mesh>,
    orig_width: f32,
    orig_height: f32,
    blur_expansion: f32,
) -> Handle<Mesh> {
    // Expanded half-dimensions
    let hw = orig_width / 2.0 + blur_expansion;
    let hh = orig_height / 2.0 + blur_expansion;

    // Vertices cover expanded area
    let vertices = vec![
        [-hw, -hh, 0.0],
        [hw, -hh, 0.0],
        [hw, hh, 0.0],
        [-hw, hh, 0.0],
    ];

    let normals = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ];

    // UV expansion - extend beyond [0,1] to sample transparent boundary
    // This allows the shader to blend edge pixels with transparent (fade out)
    let uv_expand_x = if orig_width > 0.0 {
        blur_expansion / orig_width
    } else {
        0.0
    };
    let uv_expand_y = if orig_height > 0.0 {
        blur_expansion / orig_height
    } else {
        0.0
    };

    // UVs with expansion: negative values and >1 values sample outside original texture
    let uvs = vec![
        [-uv_expand_x, 1.0 + uv_expand_y],           // bottom-left
        [1.0 + uv_expand_x, 1.0 + uv_expand_y],     // bottom-right
        [1.0 + uv_expand_x, -uv_expand_y],          // top-right
        [-uv_expand_x, -uv_expand_y],               // top-left
    ];

    let indices = vec![0u32, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::all(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    meshes.add(mesh)
}

// ============================================================================
// Plugin
// ============================================================================

/// Plugin for Gaussian blur effect.
pub struct GaussianBlurPlugin;

impl Plugin for GaussianBlurPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlurRenderLayerPool>()
            .add_systems(
                Update,
                (
                    setup_blur_rtt_system,
                    update_blur_params_system,
                    cleanup_blur_rtt_system,
                ),
            );
    }
}
