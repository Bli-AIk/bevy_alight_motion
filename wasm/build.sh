#!/bin/bash
set -e

# ==============================================================================
# bevy_alight_motion WASM 构建脚本
#
# 注意事项（GitHub Pages 部署坑点）：
# 1. 单线程模式：使用 webgl2 feature 避免 COOP/COEP 头问题
# 2. 文件优化：使用 wasm-opt -Oz 压缩体积
# 3. Jekyll 干扰：确保 doc/public/.nojekyll 文件存在
# ==============================================================================

echo "🔧 Building bevy_alight_motion WASM (单线程/WebGL2 模式)..."

# Ensure wasm target is installed
rustup target add wasm32-unknown-unknown

# 创建临时目录，仅包含 shaders（WASM playground 不需要 projects 等测试资源）
# Create temp directory with only shaders (WASM playground doesn't need test assets)
WASM_ASSETS_DIR=$(mktemp -d)
trap "rm -rf $WASM_ASSETS_DIR" EXIT
cp -r ../assets/shaders "$WASM_ASSETS_DIR/"

# 设置 assets 路径供 bevy_embedded_assets 使用（仅 shaders）
export BEVY_ASSET_PATH="$WASM_ASSETS_DIR"
echo "📁 Asset path: $BEVY_ASSET_PATH (shaders only, ~80KB)"
echo "   Original assets: $(du -sh ../assets | cut -f1) (excluded to reduce WASM size)"

# Build in release mode for smaller size
# 注意：Cargo.toml 已配置 webgl2 feature，避免多线程依赖
cargo build --target wasm32-unknown-unknown --release

# Create output directory
mkdir -p ../doc/public/wasm

# Generate JS bindings with wasm-bindgen
echo "📦 Generating JS bindings..."
wasm-bindgen \
    --target web \
    --out-dir ../doc/public/wasm \
    --out-name bevy_alight_motion \
    ./target/wasm32-unknown-unknown/release/bevy_alight_motion_wasm.wasm

# Optimize WASM size with wasm-opt (strongly recommended for production)
if command -v wasm-opt &> /dev/null; then
    echo "🗜️ Optimizing WASM size with wasm-opt -Oz..."
    wasm-opt -Oz \
        ../doc/public/wasm/bevy_alight_motion_bg.wasm \
        -o ../doc/public/wasm/bevy_alight_motion_bg.wasm
    echo "   Optimization complete!"
else
    echo "⚠️ wasm-opt not found, skipping optimization (30-50% size reduction missed)"
    echo "   Install with: cargo install wasm-opt"
fi

# Ensure .nojekyll exists (避免 Jekyll 忽略 _assets 等目录)
touch ../doc/public/.nojekyll
echo "📄 Ensured .nojekyll exists"

# Write build metadata (供 Playground 页面显示版本信息)
BUILD_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
WASM_SIZE=$(stat -c%s ../doc/public/wasm/bevy_alight_motion_bg.wasm 2>/dev/null || echo "0")
cat > ../doc/public/wasm/build_info.json <<EOF
{
  "build_time": "$BUILD_TIME",
  "git_hash": "$GIT_HASH",
  "git_branch": "$GIT_BRANCH",
  "wasm_size_bytes": $WASM_SIZE
}
EOF
echo "📝 Build metadata written to build_info.json"

echo ""
echo "✅ Build complete!"
echo "   Output: ../doc/public/wasm/"
ls -lh ../doc/public/wasm/

echo ""
echo "📋 下一步：将 wasm 目录部署到 gh-pages 分支（不要提交到源码分支）"
