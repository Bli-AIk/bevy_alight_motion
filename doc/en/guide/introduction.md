# Introduction

`bevy_alight_motion` is a specialized Bevy plugin designed to load and play Alight Motion project files (`.amproj`). It parses the XML structure of AM projects and reproduces animations faithfully within the Bevy engine.

## What is Alight Motion?

Alight Motion is a professional motion graphics application for mobile devices. It allows users to create complex animations using layers, shapes, effects, and keyframe interpolation.

## Core Goals

- **Faithful Reproduction**: Aiming for pixel-perfect matches with the original AM rendering (with some exceptions noted in the documentation).
- **High Performance**: Utilizing Bevy's ECS and modern rendering techniques like SDF (Signed Distance Fields) and RTT (Render-to-Texture).
- **Ease of Use**: Simple API to load a project and add it as a component to your Bevy scene.
- **Dynamic Integration**: Support for interacting with AM projects from Bevy systems.

## Key Features

- **Layer Support**: Shapes, Null Objects, Embedded Scenes (Groups), Images, and Text.
- **Animation System**: Position, Rotation, Scale, and Pivot animations with full support for linear, step, and Cubic Bezier easing.
- **Rendering Tech**:
  - **SDF Shapes**: High-quality, resolution-independent rendering for rectangles and circles with strokes.
  - **RTT Architecture**: Efficient handling of nested groups and complex clipping.
  - **Unified Shader**: A powerful shader that combines multiple effects (Wipe, Stretch, Masking) in a single pass.
- **Advanced Effects**: Gaussian Blur, Palette Map, Replace Color, Scale Assist, etc.

## Documentation Structure

This documentation is divided into several parts:
- **Guide**: Learn how to install and use the plugin, and understand its internal architecture.
- **Effects**: Detailed articles on every implemented AM feature and effect, including technical implementation details and associated test files.
- **Examples**: A gallery of `.amproj` files used for testing and their current status.
