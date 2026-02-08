# bevy_alight_motion WASM

This directory contains the WASM build setup for `bevy_alight_motion`.

## Prerequisites

1. Rust with WASM target:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. wasm-bindgen-cli:
   ```bash
   cargo install wasm-bindgen-cli
   ```

3. (Optional) wasm-opt for size optimization:
   ```bash
   cargo install wasm-opt
   ```

## Building

Run the build script:
```bash
./build.sh
```

This will:
1. Build the WASM binary
2. Generate JavaScript bindings
3. Output to `../doc/public/wasm/`

## Output Files

- `bevy_alight_motion.js` - Main JavaScript module
- `bevy_alight_motion_bg.wasm` - WASM binary

## Usage in JavaScript

```javascript
import init, { load_project_from_bytes, play, pause, seek, get_state } from './bevy_alight_motion.js';

// Initialize WASM module
await init();

// Load a project from user-uploaded file
const fileInput = document.getElementById('file-input');
fileInput.addEventListener('change', async (e) => {
    const file = e.target.files[0];
    const bytes = new Uint8Array(await file.arrayBuffer());
    load_project_from_bytes(bytes);
});

// Control playback
play();
pause();
seek(100); // Seek to frame 100

// Get current state
const state = get_state();
console.log(state); // { is_playing: true, current_frame: 50, total_frames: 300, fps: 60 }
```

## Integration with VitePress

The build output is placed in `doc/public/wasm/` which is automatically served by VitePress. The Playground components in `doc/.vitepress/theme/components/` use these files.
