# bevy_alight_motion

[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-APACHE) <img src="https://img.shields.io/github/repo-size/Bli-AIk/bevy_alight_motion.svg"/> <img src="https://img.shields.io/github/last-commit/Bli-AIk/bevy_alight_motion.svg"/> <br>
<img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />

> Current Status: 🚧 Experimental, Evolving

**bevy_alight_motion** — Bevy plugin for loading and playing Alight Motion project files.

| English | Simplified Chinese          |
|---------|-----------------------------|
| English | [简体中文](./readme_zh-hans.md) |

## Introduction

Animation is more than just visuals; it is a form of logic. In many creative circles, complex behaviors and patterns are
crafted entirely through motion design tools, yet they often lack a direct, loss-less path into modern game engines.

`bevy_alight_motion` is an experimental exploration dedicated to bringing these visually-expressed logics onto a "proper
track" within the [Bevy](https://bevyengine.org/) engine. It attempts to natively support Alight Motion project files,
aiming to bridge the gap between the intuition of motion design and the performance of high-level game logic.

Currently, this is a pioneering attempt — a journey to see how we can transform professional motion graphics into
living, interactive game systems without the friction of manual code recreation.

## Features

* 📂 **Experimental Loading** — Load `.amproj` ZIP archives and standalone `.xml` project files.
* 🎬 **Motion-to-Logic** — Automatic keyframe animation with cubic-bezier and step easing support.
* 🗺️ **Coordinate Mapping** — Coordinate system conversion (Alight Motion top-left origin to Bevy center origin).
* 📦 **Scene Hierarchy** — Support for nested scenes (pre-compositions).
* ⏯️ **ECS Control** — Customizable playback control via standard components.
* 🚀 **Future Vision** — Ongoing exploration into supporting more complex shape types and effects.

## Bevy Version Support

| `bevy` | `bevy_alight_motion` |
|--------|----------------------|
| 0.18   | 0.3.0                |
| 0.17   | < 0.3.0              |

## How to Use

1. **Add Dependency** to your `Cargo.toml`:
   ```toml
   [dependencies]
   bevy_alight_motion = { git = "https://github.com/Bli-AIk/souprune", path = "crates/bevy_alight_motion" }
   ```

2. **Register the Plugin** in your Bevy App:
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

3. **Load a Project**:
   ```rust
   fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
       commands.spawn(Camera2d);
       // Load the AM project from your assets folder
       load_am_project(&mut commands, &asset_server, "am/project.amproj");
   }
   ```

4. **Run the Example Player**:
   ```bash
   cargo run --example player
   ```

## How to Build

### Prerequisites

* Rust 1.80 or later (uses 2024 edition)

### Build Steps

1. **Clone the repository**:
   ```bash
   git clone https://github.com/Bli-AIk/souprune.git
   cd souprune/crates/bevy_alight_motion
   ```

2. **Build the project**:
   ```bash
   cargo build --release
   ```

3. **Run tests**:
   ```bash
   cargo test
   ```

## Dependencies

This project uses the following crates:

| Crate                                           | Version | Description                                 |
|-------------------------------------------------|---------|---------------------------------------------|
| [bevy](https://crates.io/crates/bevy)           | 0.18    | Game engine                                 |
| [quick-xml](https://crates.io/crates/quick-xml) | 0.37    | High-performance XML pull-parser/serializer |
| [serde](https://crates.io/crates/serde)         | 1.0     | Serialization/deserialization framework     |
| [zip](https://crates.io/crates/zip)             | 2.2     | ZIP archive reading/writing                 |
| [thiserror](https://crates.io/crates/thiserror) | 2.0     | Error derive macros                         |

## Contributors

### Non-Code Contributors

* **71**: Provided many Alight Motion example projects (including Undertale bullet patterns and visual PVs) during
  testing, provided a great help with this project!
* **陈皮**: Provided many Alight Motion example projects (mostly Undertale bullet patterns), providing great help for AM
  integration.

## Contributing

Contributions are welcome!
Whether you want to fix a bug, add a feature, or improve documentation:

* Submit an **Issue** or **Pull Request**.
* Share ideas and discuss design or architecture.

## License

This project is licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)
  or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
* MIT license ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option.
