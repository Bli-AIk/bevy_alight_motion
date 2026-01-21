//! # Interactive Background Example
//! # 交互式背景变色示例
//!
//! This example demonstrates the new architecture features:
//! 本示例演示了新架构的功能：
//!
//! 1. **AmEntitySpawned Hook**: Listen to entity spawn events and inject custom components
//!    **AmEntitySpawned 钩子**: 监听实体生成事件并注入自定义组件
//!
//! 2. **AmLayerName & AmElement**: Query entities by layer name
//!    **AmLayerName & AmElement**: 通过图层名称查询实体
//!
//! 3. **AmSpawnSettings (Optional)**: Filter which layers to spawn
//!    **AmSpawnSettings (可选)**: 过滤要生成的图层
//!
//! ## Usage / 用法
//!
//! ```bash
//! cargo run -p bevy_alight_motion --example interactive_bg -- basic_shape
//! ```
//!
//! ## Controls / 控制
//! - **Hover over shapes**: Change background color
//!   **鼠标悬停在形状上**: 改变背景颜色
//! - **No hover**: Black background
//!   **没有悬停**: 黑色背景
//! - **Space**: Play/Pause animation
//!   **空格**: 播放/暂停动画

use bevy::prelude::*;
use bevy_alight_motion::prelude::*;

/// Marker component for interactive elements.
/// 交互元素的标记组件。
#[derive(Component, Debug, Clone, Default)]
struct InteractiveElement {
    /// Whether this element triggers warm colors (true) or cool colors (false)
    /// 此元素是否触发暖色 (true) 或冷色 (false)
    warm_colors: bool,
}

/// Get the project file based on CLI argument.
fn get_project_file() -> String {
    let args: Vec<String> = std::env::args().collect();
    let project_name = args.get(1).map(|s| s.as_str()).unwrap_or("basic_shape");
    format!("am/{}.amproj", project_name)
}

fn main() {
    let project_file = get_project_file();
    println!("Loading project: {}", project_file);
    println!("\n=== Interactive Background Demo ===");
    println!("Hover over shapes to change background color!");
    println!("- Rectangles (.rect) → Warm colors (orange, red, yellow)");
    println!("- Circles (.circle) → Cool colors (blue, cyan, purple)");
    println!("- No hover → Black background");
    println!("==========================================\n");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Interactive Background - {}", project_file),
                resolution: bevy::window::WindowResolution::new(1280, 960),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK)) // Default black
        .insert_resource(ProjectFile(project_file))
        .insert_resource(AmProjectResolution::FitWindow)
        .add_plugins(AlightMotionPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (
            handle_input,
            handle_hover,
        ))
        .run();
}

#[derive(Resource)]
struct ProjectFile(String);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, project_file: Res<ProjectFile>) {
    // Spawn camera
    commands.spawn(Camera2d);

    // Load the AM project
    let entity = load_am_project(&mut commands, &asset_server, &project_file.0);

    // Optionally: Add spawn settings to filter layers (commented out for demo)
    // 可选: 添加生成设置以过滤图层 (为演示目的而注释掉)
    // commands.entity(entity).insert(AmSpawnSettings {
    //     filter: LayerFilter::AllowList(vec!["MyButton".to_string()]),
    // });
    let _ = entity;

    // === 2.2 扩展钩子系统 (The Hook System) ===
    // Register an observer to listen for AmEntitySpawned events
    // 注册观察者以监听 AmEntitySpawned 事件
    commands.add_observer(on_am_entity_spawned);

    // Spawn UI instructions
    commands.spawn((
        Text::new("Hover over shapes to change background!\n[Space] Play/Pause | [R] Reset"),
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
    ));
}

/// Observer function that handles AmEntitySpawned events.
/// 处理 AmEntitySpawned 事件的观察者函数。
///
/// This is triggered for every entity spawned by the AM library.
/// 这会为 AM 库生成的每个实体触发。
fn on_am_entity_spawned(
    trigger: Trigger<AmEntitySpawned>,
    mut commands: Commands,
    spec_query: Query<&AmLayerSpec>,
) {
    let event = trigger.event();
    
    // Log all spawned entities
    info!(
        "[Hook] Entity spawned: '{}' (id={}, type={:?})",
        event.layer_name, event.layer_id, event.element_type
    );

    // Only add interactive components to Shape elements
    // 只为形状元素添加交互组件
    if event.element_type != AmElementType::Shape {
        return;
    }

    // Check the shape type from AmLayerSpec
    // 从 AmLayerSpec 检查形状类型
    let is_circle = if let Ok(spec) = spec_query.get(event.entity) {
        matches!(spec, AmLayerSpec::SdfShape { shape_type, .. } if shape_type == ".circle")
    } else {
        false
    };

    // Inject interactive components
    // 注入交互组件
    commands.entity(event.entity).insert(InteractiveElement {
        warm_colors: !is_circle, // Rectangles = warm, Circles = cool
    });

    info!(
        "  → Added InteractiveElement to '{}' (warm_colors={})",
        event.layer_name, !is_circle
    );
}

/// Handle keyboard input for playback control.
/// 处理播放控制的键盘输入。
fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut playback: ResMut<AmPlayback>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        playback.playing = !playback.playing;
        println!("Playback: {}", if playback.playing { "Playing" } else { "Paused" });
    }

    if keyboard.just_pressed(KeyCode::KeyR) {
        playback.current_time_ms = 0.0;
        println!("Reset to beginning");
    }
}

/// Handle hover detection and background color change.
/// 处理悬停检测和背景颜色变化。
fn handle_hover(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    interactive_query: Query<(&GlobalTransform, &AmLayerName, &InteractiveElement), With<AmElement>>,
    mut clear_color: ResMut<ClearColor>,
) {
    // Get cursor position
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        // No cursor in window - set to black
        clear_color.0 = Color::BLACK;
        return;
    };

    // Convert to world coordinates
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Simple AABB collision check (simplified - assumes centered sprites)
    // 简单的 AABB 碰撞检测 (简化版 - 假设精灵居中)
    let hover_radius = 50.0; // Approximate hover radius

    let mut hovered = false;
    for (transform, _layer_name, interactive) in interactive_query.iter() {
        let pos = transform.translation();
        let dist_sq = (pos.x - world_pos.x).powi(2) + (pos.y - world_pos.y).powi(2);

        if dist_sq < hover_radius * hover_radius {
            hovered = true;
            
            // Change background color based on element type
            // 根据元素类型改变背景颜色
            let new_color = if interactive.warm_colors {
                // Warm colors for rectangles
                Color::hsl(30.0, 0.8, 0.5) // Orange
            } else {
                // Cool colors for circles
                Color::hsl(210.0, 0.8, 0.5) // Blue
            };

            clear_color.0 = new_color;
            break;
        }
    }

    // If no hover, set background to black
    // 如果没有悬停，设置背景为黑色
    if !hovered {
        clear_color.0 = Color::BLACK;
    }
}
