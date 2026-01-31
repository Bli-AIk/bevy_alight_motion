# GitHub Workflows for bevy_alight_motion

此项目包含以下GitHub Actions工作流，用于自动化CI/CD流程。

## 工作流说明

### 1. **CI工作流** (`.github/workflows/ci.yml`)

- **触发条件**: 推送到 `main`/`develop` 分支和创建PR时
- **功能**:
  - 代码格式检查 (`cargo fmt`)
  - Clippy代码静态分析（warnings视为错误）
  - 构建和测试crate
  - 安全审计检查

### 2. **Coverage工作流** (`.github/workflows/coverage.yml`)

- **触发条件**: 推送到 `main` 分支和创建PR时
- **功能**:
  - 生成代码覆盖率报告
  - 上传到Codecov

### 3. **依赖更新工作流** (`.github/workflows/update-deps.yml`)

- **触发条件**: 每周一自动运行 或 手动触发
- **功能**:
  - 更新依赖版本
  - 自动创建PR如果有更新

## 配置说明

### 📦 **系统依赖 (Ubuntu兼容)**

所有工作流会自动安装以下Bevy依赖：
- `libasound2-dev` - 音频支持
- `libudev-dev` - 设备管理  
- `libwayland-dev` - Wayland显示服务器
- `libxkbcommon-dev` - 键盘处理
- `libxrandr-dev` - 显示管理
- `libx11-dev` - X11支持
- `libxi-dev` - 输入扩展
- `libxinerama-dev` - 多头显示
- `libxcursor-dev` - 光标支持
- `libgl1-mesa-dev` - OpenGL支持
- `pkg-config` - 包配置

### Actions版本

- `actions/checkout@v6`: 仓库检出
- `dtolnay/rust-toolchain@stable`: Rust安装
- `actions/cache@v5`: 依赖缓存
- `rustsec/audit-check@v2`: 安全审计
- `codecov/codecov-action@v5`: 覆盖率报告
- `peter-evans/create-pull-request@v8`: 自动PR

所有Actions都锁定到特定版本以确保安全性和可重现性。

## 使用act验证工作流

可以使用 [act](https://github.com/nektos/act) 本地验证工作流：

```bash
# 验证所有工作流
act -l

# 验证CI测试任务
act push -j test -n

# 验证安全审计
act push -j security -n

# 验证Coverage工作流
act push -W .github/workflows/coverage.yml -n
```

## Issue和PR模板

### Issue模板

- **Bug report**: 报告bug或意外行为
- **Feature request**: 建议新功能或改进（包括AM效果支持）
- **Refactor request**: 建议代码结构改进

### PR模板

PR模板包含以下部分：
- PR类型选择（包括Shader变更选项）
- 关联Issue
- 变更描述
- 测试方法
- 截图/视频对比
- 视频对比测试结果

## 特别说明

bevy_alight_motion是一个渲染密集型库，视频对比测试是验证渲染正确性的主要方式。在提交涉及渲染变更的PR时，请确保：

1. 运行相关的视频对比测试
2. 提供AM参考截图和bevy_alight_motion输出截图
3. 说明任何已知的差异及其原因
