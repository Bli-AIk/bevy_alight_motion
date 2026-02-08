#!/bin/bash

# 自动测试 basic/* 和 effects/* 示例
# 每个示例运行10秒，间隔2秒

cargo build -p bevy_alight_motion
cd "$(dirname "$0")"

# 获取所有 basic 和 effects 下的 .amproj 文件
examples=$(find assets/projects/basic assets/projects/effects -name "*.amproj" 2>/dev/null | \
    sed 's|assets/projects/||;s|\.amproj||' | sort)

for example in $examples; do
    echo "========================================"
    echo "运行示例: $example"
    echo "========================================"
    
    # 在后台运行示例
    cargo run -p bevy_alight_motion --example player --features "debug,video-debug" -- "$example" &
    PID=$!
    
    # 等待15秒
    sleep 15
    
    # 终止进程
    kill $PID 2>/dev/null
    wait $PID 2>/dev/null
    
    echo "示例 $example 完成"
    echo ""
    
    # 间隔2秒
    sleep 2
done

echo "所有示例测试完成！"
