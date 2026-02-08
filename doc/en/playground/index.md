---
title: Playground
---

# 🎮 Playground

<script setup>
import { onMounted } from 'vue'

onMounted(() => {
  // Ensure client-side rendering
})
</script>

Here you can upload Alight Motion project files (`.amproj`) and preview the rendering directly in your browser.

<ClientOnly>
  <AmPlayground />
</ClientOnly>

## How to Use

1. **Upload a file**: Click the upload area or drag & drop an `.amproj` file
2. **View the report**: After loading, a Validation Report will show which features are supported
3. **Control playback**: Use the playback controls and timeline

## Notes

- This Playground runs in WebGL2 single-threaded mode, performance may be lower than native
- Some features (like Audio, Video layers) are not currently supported
- Large projects may take longer to load

## Supported Features

See [Implemented Features](/en/implemented-features) for details
