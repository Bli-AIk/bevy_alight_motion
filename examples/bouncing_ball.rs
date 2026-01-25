//! Bouncing Ball Example - A physics-based demo using Alight Motion animation
//! 弹球示例 - 使用 Alight Motion 动画的物理演示
//!
//! # Usage / 用法
//! ```bash
//! cargo run -p bevy_alight_motion --example bouncing_ball
//! ```
//!
//! This example demonstrates:
//! - Loading AM projects with animated elements (弹板 flipper)
//! - Simple physics simulation (gravity, collision)
//! - Dynamic entity spawning and despawning
//!
//! # Controls / 控制
//! - **Space**: Play/Pause toggle (播放/暂停切换)
//! - **R**: Reset to beginning (keeps current play state) (重置到开头，保持当前播放状态)
//! - **P**: Replay from beginning (resets and plays) (重新播放)
//! - **Left/Right**: Step backward/forward by one frame (pauses playback) (单帧步进)
//! - **Up/Down**: Speed up/slow down playback (加速/减速)
//! - **L**: Toggle loop mode (循环模式切换)
//! - **F5**: Toggle force stop (animation frozen for debugging) (强制停止)
//! - **Mouse Left Click**: Spawn a ball at cursor position (在鼠标位置生成小球)

use bevy::prelude::*;
use bevy_alight_motion::prelude::*;

// ============================================================================
// Simple Physics Constants
// ============================================================================

/// Gravity acceleration (pixels per second squared)
const GRAVITY: f32 = 600.0;

/// Ball radius in pixels
const BALL_RADIUS: f32 = 20.0;

/// Bounce coefficient (energy retained after collision)
const BOUNCE_COEFFICIENT: f32 = 0.7;

/// Canvas dimensions (from showcase.amproj)
const CANVAS_WIDTH: f32 = 1440.0;
const CANVAS_HEIGHT: f32 = 1080.0;

// ============================================================================
// Components
// ============================================================================

/// Marks an entity as a physics ball
#[derive(Component)]
struct Ball {
    velocity: Vec2,
}

/// Stores the flipper's collision data (updated every frame from AM animation)
#[derive(Resource, Default)]
struct FlipperCollider {
    /// Pivot point in Bevy world coordinates
    pivot_world: Vec2,
    /// Flipper length from pivot to end
    length: f32,
    /// Current rotation angle in radians
    rotation: f32,
    /// Previous frame rotation (for angular velocity calculation)
    prev_rotation: f32,
    /// Flipper thickness for collision
    thickness: f32,
    /// Is the flipper entity active
    active: bool,
}

/// UI text component for status display
#[derive(Component)]
struct StatusText;

/// Resource to hold the circle mesh handle
#[derive(Resource)]
struct BallMeshHandle(Handle<Mesh>);

// ============================================================================
// Plugin
// ============================================================================

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bouncing Ball Example - bevy_alight_motion".to_string(),
                resolution: bevy::window::WindowResolution::new(
                    CANVAS_WIDTH as u32,
                    CANVAS_HEIGHT as u32,
                ),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmProjectResolution::FitWindow)
        .init_resource::<FlipperCollider>()
        .add_plugins(AlightMotionPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                handle_mouse_click,
                update_ui,
                update_flipper_collider,
                physics_update,
                despawn_out_of_bounds,
            )
                .chain(),
        )
        .run();
}

// ============================================================================
// Setup
// ============================================================================

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut meshes: ResMut<Assets<Mesh>>) {
    // Spawn camera
    commands.spawn(Camera2d);

    // Load the showcase AM project (contains the animated flipper)
    load_am_project(&mut commands, &asset_server, "am/showcase.amproj");

    // Create a circle mesh for balls and store its handle
    let circle_mesh = meshes.add(Circle::new(BALL_RADIUS));
    commands.insert_resource(BallMeshHandle(circle_mesh));

    // Spawn UI for status display (like player.rs)
    commands.spawn((
        Text::new("Loading..."),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        StatusText,
    ));

    // Instructions (like player.rs but with click to spawn)
    commands.spawn((
        Text::new("[Space] Play/Pause | [R] Reset | [P] Replay | [F5] Force Stop | [LEFT/RIGHT] Frame Step | [UP/DOWN] Speed | [L] Loop | [Click] Spawn Ball"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

// ============================================================================
// Input Handling
// ============================================================================

fn handle_input(keyboard: Res<ButtonInput<KeyCode>>, mut playback: ResMut<AmPlayback>) {
    // Play/Pause toggle
    if keyboard.just_pressed(KeyCode::Space) {
        playback.toggle();
    }

    // Reset (keeps current play/pause state)
    if keyboard.just_pressed(KeyCode::KeyR) {
        playback.reset();
    }

    // Replay (reset and start playing)
    if keyboard.just_pressed(KeyCode::KeyP) {
        playback.reset();
        playback.playing = true;
    }

    // Force stop toggle (F5) - freezes all animation updates for inspector editing
    if keyboard.just_pressed(KeyCode::F5) {
        playback.toggle_force_stop();
        let status = if playback.force_stopped { "ON" } else { "OFF" };
        bevy::log::info!(
            "Force stop: {} (animation updates frozen for inspector editing)",
            status
        );
    }

    // Frame-by-frame stepping (Left/Right arrows)
    // Pauses playback and moves one frame at a time
    let frame_duration_ms = 1000.0 / 30.0; // 30 fps
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        playback.playing = false;
        playback.current_time_ms = (playback.current_time_ms - frame_duration_ms).max(0.0);
        bevy::log::info!("[FrameStep] time={:.1}ms", playback.current_time_ms);
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        playback.playing = false;
        playback.current_time_ms =
            (playback.current_time_ms + frame_duration_ms).min(playback.total_time_ms);
        bevy::log::info!("[FrameStep] time={:.1}ms", playback.current_time_ms);
    }

    // Speed control (up = faster, down = slower)
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        playback.speed = (playback.speed + 0.1).min(4.0);
        bevy::log::info!("[Speed] {:.1}x", playback.speed);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        playback.speed = (playback.speed - 0.1).max(0.1);
        bevy::log::info!("[Speed] {:.1}x", playback.speed);
    }

    // Loop mode toggle
    if keyboard.just_pressed(KeyCode::KeyL) {
        playback.looping = !playback.looping;
        bevy::log::info!("Loop mode: {}", playback.looping);
    }
}

fn handle_mouse_click(
    commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    materials: ResMut<Assets<ColorMaterial>>,
    ball_mesh: Option<Res<BallMeshHandle>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(mesh) = ball_mesh else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Convert cursor position to world coordinates
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    spawn_ball_at(commands, materials, &mesh.0, world_pos.x, world_pos.y);
}

fn update_ui(playback: Res<AmPlayback>, mut query: Query<&mut Text, With<StatusText>>) {
    for mut text in query.iter_mut() {
        let status = if playback.force_stopped {
            "FORCE STOPPED"
        } else if playback.playing {
            "Playing"
        } else {
            "Paused"
        };
        let loop_status = if playback.looping { "Loop" } else { "Once" };

        **text = format!(
            "{} | Time: {:.0}/{:.0}ms | Speed: {:.2}x | {}",
            status, playback.current_time_ms, playback.total_time_ms, playback.speed, loop_status
        );
    }
}

// ============================================================================
// Ball Spawning
// ============================================================================

fn spawn_ball_at(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mesh_handle: &Handle<Mesh>,
    x: f32,
    y: f32,
) {
    // Random initial horizontal velocity
    let vx = (rand_simple() - 0.5) * 50.0;

    // Create a colored material for this ball
    let color = random_color();
    let material = materials.add(ColorMaterial::from(color));

    commands.spawn((
        Ball {
            velocity: Vec2::new(vx, 0.0),
        },
        bevy::mesh::Mesh2d(mesh_handle.clone()),
        MeshMaterial2d(material),
        Transform::from_translation(Vec3::new(x, y, 100.0)),
    ));

    bevy::log::info!("Spawned ball at ({:.1}, {:.1})", x, y);
}

/// Simple pseudo-random number generator (0.0 to 1.0)
fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 10000) as f32 / 10000.0
}

fn random_color() -> Color {
    let colors = [
        Color::srgb(1.0, 0.4, 0.4), // Red
        Color::srgb(0.4, 1.0, 0.4), // Green
        Color::srgb(0.4, 0.4, 1.0), // Blue
        Color::srgb(1.0, 1.0, 0.4), // Yellow
        Color::srgb(1.0, 0.4, 1.0), // Magenta
        Color::srgb(0.4, 1.0, 1.0), // Cyan
    ];
    colors[(rand_simple() * colors.len() as f32) as usize % colors.len()]
}

// ============================================================================
// Flipper Collider Update
// ============================================================================

/// Update the flipper collider based on the AM layer's transform
fn update_flipper_collider(
    mut flipper: ResMut<FlipperCollider>,
    query: Query<&GlobalTransform, With<AmLayerMarker>>,
) {
    // Find the flipper layer (only use the first shape found)
    if let Some(global_transform) = query.iter().next() {
        // Get the flipper's current world transform
        let (scale, rotation, translation) = global_transform.to_scale_rotation_translation();

        // Convert rotation quaternion to angle (Z rotation in 2D)
        let angle = rotation.to_euler(EulerRot::ZYX).0;

        // Store previous rotation for angular velocity calculation
        flipper.prev_rotation = flipper.rotation;

        // Update collider data
        // From showcase.amproj:
        // - Base size: 248.92 x 83.15 (this is HALF-extents in AM)
        // - Pivot offset: (-248.92, 0) - means rotation around left edge
        // - Scale: (1.15, 0.24)
        // - Position: (300.97, 880.70) in AM coords
        //
        // AM's size property represents HALF-extents, so we multiply by 2 for full dimensions
        // Full size = half_size * 2 * scale
        let half_width = 248.92;
        let half_height = 83.15;
        let full_width = half_width * 2.0 * scale.x.abs();
        let full_height = half_height * 2.0 * scale.y.abs();

        // The flipper's pivot is at its left edge
        // In Bevy coords: translation is the pivot position
        flipper.pivot_world = Vec2::new(translation.x, translation.y);
        flipper.length = full_width;
        flipper.rotation = angle;
        flipper.thickness = full_height;
        flipper.active = true;
    }
}

// ============================================================================
// Physics Update
// ============================================================================

fn physics_update(
    time: Res<Time>,
    flipper: Res<FlipperCollider>,
    mut query: Query<(&mut Ball, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (mut ball, mut transform) in query.iter_mut() {
        // Apply gravity
        ball.velocity.y -= GRAVITY * dt;

        // Update position
        transform.translation.x += ball.velocity.x * dt;
        transform.translation.y += ball.velocity.y * dt;

        // Check collision with flipper
        if flipper.active
            && let Some((new_pos, new_vel)) = check_flipper_collision(
                transform.translation.truncate(),
                BALL_RADIUS,
                ball.velocity,
                &flipper,
                dt,
            )
        {
            transform.translation.x = new_pos.x;
            transform.translation.y = new_pos.y;
            ball.velocity = new_vel;
        }

        // Simple wall bouncing (left/right edges)
        let half_width = CANVAS_WIDTH / 2.0;
        if transform.translation.x < -half_width + BALL_RADIUS {
            transform.translation.x = -half_width + BALL_RADIUS;
            ball.velocity.x = ball.velocity.x.abs() * BOUNCE_COEFFICIENT;
        } else if transform.translation.x > half_width - BALL_RADIUS {
            transform.translation.x = half_width - BALL_RADIUS;
            ball.velocity.x = -ball.velocity.x.abs() * BOUNCE_COEFFICIENT;
        }
    }
}

/// Check if a ball collides with the flipper and calculate response
/// Returns Some((new_position, new_velocity)) if collision occurred
fn check_flipper_collision(
    ball_pos: Vec2,
    ball_radius: f32,
    ball_vel: Vec2,
    flipper: &FlipperCollider,
    dt: f32,
) -> Option<(Vec2, Vec2)> {
    let pivot = flipper.pivot_world;
    let angle = flipper.rotation;
    let length = flipper.length;
    let thickness = flipper.thickness;

    // Flipper direction vector (from pivot toward end)
    let direction = Vec2::new(angle.cos(), angle.sin());

    // Find closest point on the flipper line segment to the ball
    let to_ball = ball_pos - pivot;
    let projection_len = to_ball.dot(direction).clamp(0.0, length);
    let closest_point = pivot + direction * projection_len;

    // Distance from ball center to closest point on flipper
    let distance = ball_pos.distance(closest_point);
    let collision_threshold = ball_radius + thickness * 0.5;

    if distance < collision_threshold && distance > 0.001 {
        // Collision detected!

        // Normal vector pointing from flipper toward ball
        let normal = (ball_pos - closest_point).normalize();

        // Push ball out of collision
        let penetration = collision_threshold - distance;
        let new_pos = ball_pos + normal * (penetration + 1.0);

        // Calculate flipper angular velocity (radians per second)
        let angular_velocity = (flipper.rotation - flipper.prev_rotation) / dt.max(0.0001);

        // Calculate the velocity of the flipper at the contact point
        // v = ω × r (for 2D: v_tangent = ω * distance_from_pivot)
        let distance_from_pivot = projection_len;
        let tangent = direction.perp(); // Perpendicular to flipper direction
        let flipper_velocity_at_point = tangent * angular_velocity * distance_from_pivot;

        // Relative velocity of ball with respect to flipper surface
        let relative_velocity = ball_vel - flipper_velocity_at_point;

        // Velocity component along normal
        let vel_normal = relative_velocity.dot(normal);

        // Only respond if ball is moving toward the flipper
        if vel_normal < 0.0 {
            // Reflect the relative velocity
            let reflected_relative = relative_velocity - normal * (2.0 * vel_normal);

            // New velocity = reflected relative velocity + flipper velocity, with energy loss
            let new_vel = (reflected_relative * BOUNCE_COEFFICIENT) + flipper_velocity_at_point;

            return Some((new_pos, new_vel));
        }
    }

    None
}

// ============================================================================
// Cleanup
// ============================================================================

fn despawn_out_of_bounds(mut commands: Commands, query: Query<(Entity, &Transform), With<Ball>>) {
    let bottom = -CANVAS_HEIGHT / 2.0 - 100.0;

    for (entity, transform) in query.iter() {
        if transform.translation.y < bottom {
            commands.entity(entity).despawn();
            bevy::log::trace!("Despawned ball that fell below screen");
        }
    }
}
