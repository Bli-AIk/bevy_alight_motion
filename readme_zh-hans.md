# bevy_alight_motion

[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-APACHE) <img src="https://img.shields.io/github/repo-size/Bli-AIk/bevy_alight_motion.svg"/> <img src="https://img.shields.io/github/last-commit/Bli-AIk/bevy_alight_motion.svg"/> <br>
<img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />

> 当前状态: 🚧 实验性，持续演进中

**bevy_alight_motion** — 用于加载和播放 Alight Motion 项目文件的 Bevy 插件。

| English                | 简体中文 |
|------------------------|------|
| [English](./readme.md) | 简体中文 |

## 简介

动画不仅仅是视觉效果，它也是逻辑的一种表现形式。在许多创作领域，复杂的行为逻辑完全通过动效设计工具来编排，但这些创意往往缺乏一条进入游戏引擎的“正轨”。

`bevy_alight_motion`
是一次实验性的探索，致力于将这些以动画表现逻辑的可视化创作方式引入正轨。它尝试在 [Bevy](https://bevyengine.org/) 引擎中提供对
Alight Motion 工程文件的原生支持，旨在消除动效设计的直觉与高性能游戏逻辑之间的隔阂。

目前，这是一个先行者的尝试——我们正在探索如何将专业的动效设计转化为鲜活的、可交互的游戏系统，而无需经过繁琐且损耗巨大的手工代码还原。

## 功能

* 📂 **实验性加载** — 加载 `.amproj` ZIP 归档和独立的 `.xml` 项目文件。
* 🎬 **动效即逻辑** — 自动关键帧动画，支持 cubic-bezier (三次贝塞尔) 和 step (步进) 缓动。
* 🗺️ **坐标映射** — 坐标系转换 (Alight Motion 的左上角原点转换为 Bevy 的中心原点)。
* 📦 **场景层级** — 支持嵌套场景 (预合成)。
* ⏯️ **ECS 控制** — 通过标准组件实现可自定义的播放控制。
* 🚀 **未来愿景** — 持续探索对更多形状类型和特效的支持。

## Bevy 版本支持

| `bevy` | `bevy_alight_motion` |
|--------|----------------------|
| 0.18   | 0.3.0                |
| 0.17   | < 0.3.0              |

## 如何使用

1. **添加依赖** 到你的 `Cargo.toml`:
   ```toml
   [dependencies]
   bevy_alight_motion = { git = "https://github.com/Bli-AIk/souprune", path = "crates/bevy_alight_motion" }
   ```

2. **注册插件** 在你的 Bevy App 中:
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

3. **加载项目**:
   ```rust
   fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
       commands.spawn(Camera2d);
       // 从你的 assets 文件夹加载 AM 项目
       load_am_project(&mut commands, &asset_server, "am/project.amproj");
   }
   ```

4. **运行示例播放器**:
   ```bash
   cargo run --example player
   ```

## 如何构建

### 前置条件

* Rust 1.80 或更高版本 (使用 2024 edition)

### 构建步骤

1. **克隆仓库**:
   ```bash
   git clone https://github.com/Bli-AIk/souprune.git
   cd souprune/crates/bevy_alight_motion
   ```

2. **构建项目**:
   ```bash
   cargo build --release
   ```

3. **运行测试**:
   ```bash
   cargo test
   ```

## 依赖项

本项目使用了以下 crate:

| Crate                                           | 版本   | 描述              |
|-------------------------------------------------|------|-----------------|
| [bevy](https://crates.io/crates/bevy)           | 0.18 | 游戏引擎            |
| [quick-xml](https://crates.io/crates/quick-xml) | 0.37 | 高性能 XML 解析/序列化库 |
| [serde](https://crates.io/crates/serde)         | 1.0  | 序列化/反序列化框架      |
| [zip](https://crates.io/crates/zip)             | 2.2  | ZIP 归档读写库       |
| [thiserror](https://crates.io/crates/thiserror) | 2.0  | 错误派生宏           |

## 贡献者

### 非代码贡献者

* **71**: 他在此项目测试过程中提供了许多 Alight Motion 示例工程（包括 Undertale 弹幕，以及一些视觉 PV），为此项目提供了很大帮助！
* **陈皮**: 他提供了许多 Alight Motion 示例工程（主要是 Undertale 弹幕），为 AM 集成提供了很大帮助！

## 贡献

欢迎贡献！
无论你是想修复 Bug、添加新功能还是改进文档：

* 提交 **Issue** 或 **Pull Request**。
* 分享想法并讨论设计或架构。

## 许可

本项目采用以下任一许可协议授权：

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)
  或 [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
* MIT license ([LICENSE-MIT](LICENSE-MIT) 或 [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

可根据你的选择使用。
