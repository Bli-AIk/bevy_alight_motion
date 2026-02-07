# 快速开始

加载 Alight Motion 项目非常简单。

## 1. 添加插件

在您的 Bevy 应用中注册 `AlightMotionPlugin`：

```rust
use bevy::prelude::*;
use bevy_alight_motion::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AlightMotionPlugin)
        .add_systems(Startup, setup)
        .run();
}
```

## 2. 加载与生成

使用 asset server 加载 `.amproj` 文件并将其作为实体生成：

```rust
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let scene_handle = asset_server.load("am/showcase.amproj");
    
    commands.spawn(AlightMotionBundle {
        scene: scene_handle,
        ..default()
    });
}
```
