# Quick Start

Loading an Alight Motion project is straightforward.

## 1. Add the Plugin

Register `AlightMotionPlugin` in your Bevy app:

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

## 2. Load and Spawn

Load an `.amproj` file using the asset server and spawn it as an entity:

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
