# bevy_alight_motion dylib 化重构方案

> 目标：将项目改造为纯粹的 AM（Alight Motion）工程数据提取层，编译为 dylib，
> 供任意语言（C/C++/C#/Python/...）的游戏引擎通过 C-ABI 调用。
> 现有的 Bevy 渲染前端保留，作为 dylib 的一个消费者继续存在。

---

## 一、现状分析

### 1.1 代码规模

| 层级 | 行数 | Bevy 耦合度 | 重构策略 |
|---|---|---|---|
| 核心数据层（schema, loader, validation, error, effects_registry） | ~8,800 | 极低（5 处污染） | 提取为 core crate，去 Bevy 化 |
| 纯数学层（easing, interpolation） | ~1,150 | 极低（3 处污染） | 提取为 core crate |
| 渲染材料（SDF, gaussian_blur, masked_sprite, group_fill） | ~1,500 | 完全耦合 | 留在 Bevy 插件 |
| ECS 运行时（animation systems, scene spawn, effects RTT） | ~27,900 | 完全耦合 | 留在 Bevy 插件 |
| **总计** | **~41,000** | | |

### 1.2 Bevy 污染清单（需要修复的 5 处）

| 文件 | 行号 | 内容 | 修复方案 |
|---|---|---|---|
| `schema/types/animation.rs` | 10 | `use bevy::prelude::Vec4;` | 替换为 `[f32; 4]` 或 `glam::Vec4` |
| `animation/interpolation.rs` | 105 | `bevy::log::trace!` | 替换为 `log` crate 的 `trace!` |
| `animation/interpolation.rs` | 192-194 | `bevy::prelude::Vec4` in `interpolate_color` | 替换为 `[f32; 4]` |
| `animation/interpolation.rs` | 223-224 | `bevy::prelude::Vec4` in `parse_keyframe_color` | 替换为 `[f32; 4]` |
| `loader/project_loading.rs` | 14-16 | `use bevy::asset::...` | 剥离 AssetLoader trait，保留纯解析逻辑 |

### 1.3 代码质量评估

**优势：**
- 模块分层架构清晰：`schema → loader → animation` 三层关注点分离
- serde XML 反序列化设计合理，`AmScene`/`AmLayer` 的类型建模完整覆盖 AM 所有图层类型
- 缓动函数实现正确（cubic-bezier、step、cyclic），已通过单元测试验证
- 关键帧插值算法还原了 AM 的 `reverseInterpolateFirstFrame` 行为
- 错误类型使用 thiserror 设计，规范且可扩展

**待改进：**
- 纯数学模块（interpolation）耦合了 Bevy log 宏，缺乏边界意识
- `reverse_interpolate_float/vec2/vec3_impl` 三份几乎相同的代码，应使用泛型消除重复
- `AmAnimatedColor::value` 类型为 `bevy::prelude::Vec4`，渲染框架类型泄漏到数据层
- `validation.rs` 中 `log_report` 和 `log_report_wasm` 约 400 行重复逻辑
- `plugin/build.rs` 中 30+ 个 system 工编排复杂且脆弱，过度依赖 `.chain()`/`.after()`
- 中英双语注释在部分模块中不一致

---

## 二、目标架构

```
┌─────────────────────────────────────────────────┐
│  Unity / Godot / 自定义 C 引擎 / 其他           │
│  通过 C-ABI / FFI 调用                          │
└────────────────────┬────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────┐
│  libamproj.so / .dylib / .dll                  │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │ C-ABI 导出层（am_ffi.rs）                  │  │
│  │ - am_project_load(path) → handle           │  │
│  │ - am_project_metadata(handle) → Metadata   │  │
│  │ - am_project_query_frame(handle, t) → FlatElement[] │
│  │ - am_project_free(handle)                  │  │
│  ├────────────────────────────────────────────┤  │
│  │ 核心 Rust crate（amproj） │  │
│  │ schema + easing + interpolation            │  │
│  │ + loader + validation + effects_registry   │  │
│  └────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────┐
│  bevy_alight_motion（Bevy 插件）                │
│  依赖 amproj，复用数据层       │
│  负责：ECS 实体生成、材质管理、渲染管线          │
└─────────────────────────────────────────────────┘
```

---

## 三、坐标映射（可配置）

### 3.1 设计原则

**不做固定坐标系枚举，做参数化的坐标映射配置。** 引擎通过 `CoordMappingConfig` 描述自己的坐标系约定，dylib 据此生成目标坐标系的 world_matrix。

好处：
- 新引擎接入无需改 dylib 代码
- 同一个引擎的不同渲染模式（UI 模式 vs 世界模式）用不同 config 即可
- 未来出现新坐标系约定时零维护成本

### 3.2 配置结构

```rust
struct CoordMappingConfig {
    // ── 原点 ───────────────────────────────────
    /// 原点在画布中的位置，取值 0.0 ~ 1.0。
    /// (0, 0) = 左上，(0.5, 0.5) = 中心，(0, 1) = 左下。
    origin_anchor: [f32; 2],

    // ── 轴向 ───────────────────────────────────
    /// X 轴正方向：1.0 = 右，-1.0 = 左
    x_direction: f32,
    /// Y 轴正方向：1.0 = 下，-1.0 = 上
    y_direction: f32,

    // ── 旋转 ───────────────────────────────────
    /// 旋转正方向：1.0 = 逆时针，-1.0 = 顺时针
    rotation_sign: f32,
    /// 旋转零度指向：(1, 0) = 右，(0, -1) = 上，(0, 1) = 下
    rotation_zero_axis: [f32; 2],

    // ── 锚点约定 ───────────────────────────────
    /// 引擎锚点相对于元素包围盒的位置，取值 0.0 ~ 1.0。
    /// (0.5, 0.5) = 中心（AM 默认），(0, 0) = 左上角（CSS/Godot Control）。
    engine_anchor: [f32; 2],

    // ── 深度 ───────────────────────────────────
    /// 每个层级在 Z 轴上的间距。
    /// 正数：层级越深 Z 越大（标准 3D），负数：层级越深 Z 越小。
    z_spacing: f32,

    // ── 矩阵约定 ───────────────────────────────
    /// 矩阵存储顺序：true = 列主序（OpenGL/Vulkan/Bevy），false = 行主序（DirectX/Unity）
    column_major: bool,
}
```

### 3.3 常见引擎预设

dylib 提供以下预设常量，引擎也可以完全自定义：

```rust
impl CoordMappingConfig {
    /// AM 原始：左上原点，X 右 +，Y 下 +，旋转顺时针，锚点中心
    const AM_NATIVE: CoordMappingConfig = CoordMappingConfig {
        origin_anchor: [0.0, 0.0],
        x_direction: 1.0,
        y_direction: 1.0,
        rotation_sign: -1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 0.001,
        column_major: true,
    };

    /// Bevy 2D：中心原点，X 右 +，Y 上 +，旋转逆时针，锚点中心
    const BEVY_2D: CoordMappingConfig = CoordMappingConfig {
        origin_anchor: [0.5, 0.5],
        x_direction: 1.0,
        y_direction: -1.0,
        rotation_sign: 1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 0.001,
        column_major: true,
    };

    /// Unity UI（Canvas）：左下原点，X 右 +，Y 上 +，旋转逆时针，锚点中心
    const UNITY_UI: CoordMappingConfig = CoordMappingConfig {
        origin_anchor: [0.0, 1.0],
        x_direction: 1.0,
        y_direction: -1.0,
        rotation_sign: 1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 1.0,
        column_major: false, // Unity 行主序
    };

    /// Unity 世界空间
    const UNITY_WORLD: CoordMappingConfig = CoordMappingConfig {
        origin_anchor: [0.5, 0.5],
        x_direction: 1.0,
        y_direction: -1.0,
        rotation_sign: 1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 1.0,
        column_major: false,
    };

    /// Godot 2D：左上原点，X 右 +，Y 下 +，旋转顺时针，锚点中心
    const GODOT_2D: CoordMappingConfig = CoordMappingConfig {
        origin_anchor: [0.0, 0.0],
        x_direction: 1.0,
        y_direction: 1.0,
        rotation_sign: -1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 0.001,
        column_major: true,
    };

    /// Godot Control：左上原点，X 右 +，Y 下 +，旋转顺时针，锚点左上角
    const GODOT_CONTROL: CoordMappingConfig = CoordMappingConfig {
        origin_anchor: [0.0, 0.0],
        x_direction: 1.0,
        y_direction: 1.0,
        rotation_sign: -1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.0, 0.0],
        z_spacing: 0.001,
        column_major: true,
    };

    /// CSS / Web：左上原点，X 右 +，Y 下 +，旋转顺时针，锚点左上角
    const CSS: CoordMappingConfig = CoordMappingConfig {
        origin_anchor: [0.0, 0.0],
        x_direction: 1.0,
        y_direction: 1.0,
        rotation_sign: -1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.0, 0.0],
        z_spacing: 1.0,
        column_major: true,
    };

    /// OpenGL NDC：中心原点，X 右 +，Y 上 +，旋转逆时针，无锚点
    const OPENGL_NDC: CoordMappingConfig = CoordMappingConfig {
        origin_anchor: [0.5, 0.5],
        x_direction: 1.0,
        y_direction: -1.0,
        rotation_sign: 1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 0.001,
        column_major: true,
    };
}
```

### 3.4 转换逻辑

dylib 内部对每个元素执行以下管道：

```rust
fn apply_coord_mapping(
    am_position: [f32; 2],    // AM 原始坐标，左上原点 + 锚点偏移
    am_rotation_deg: f32,     // AM 旋转角度（AM 使用顺时针正方向）
    am_scale: [f32; 2],       // AM 缩放
    element_size: [f32; 2],   // 元素宽高
    canvas_size: [f32; 2],    // 画布宽高
    config: &CoordMappingConfig,
) -> [f32; 16] // 4x4 矩阵
{
    // Step 1: 原点映射
    // AM 左上 (0,0) → 目标原点
    let origin_x = config.origin_anchor[0] * canvas_size[0];
    let origin_y = config.origin_anchor[1] * canvas_size[1];

    // Step 2: 锚点差异补偿
    // AM 锚点 = (0.5, 0.5)（元素中心）
    // 如果引擎锚点不同，需要偏移
    let anchor_dx = (0.5 - config.engine_anchor[0]) * element_size[0];
    let anchor_dy = (0.5 - config.engine_anchor[1]) * element_size[1];

    // Step 3: 计算目标坐标
    let target_x = ((am_position[0] + anchor_dx) * config.x_direction) + origin_x;
    let target_y = ((am_position[1] + anchor_dy) * config.y_direction) + origin_y;

    // Step 4: 旋转方向转换（AM 顺时针 → 目标约定）
    let target_rotation = am_rotation_deg * config.rotation_sign;

    // Step 5: 构建 4x4 变换矩阵
    let matrix = build_transform_matrix(
        target_x, target_y,
        target_rotation, config.rotation_zero_axis,
        am_scale,
        layer_index * config.z_spacing,
    );

    // Step 6: 矩阵存储格式转换
    if config.column_major { matrix } else { transpose(matrix) }
}
```

### 3.5 每种预设对应的公式

| 预设 | 公式 |
|---|---|
| `AM_NATIVE` | `x = am_x`, `y = am_y`, `rot = -am_rot` |
| `BEVY_2D` | `x = am_x - cw/2`, `y = ch/2 - am_y`, `rot = am_rot` |
| `UNITY_UI` | `x = am_x`, `y = ch - am_y`, `rot = am_rot`，行主序矩阵 |
| `UNITY_WORLD` | `x = am_x - cw/2`, `y = ch/2 - am_y`, `rot = am_rot`，行主序矩阵 |
| `GODOT_2D` | 同 `AM_NATIVE` |
| `GODOT_CONTROL` | `x = am_x - w/2`, `y = am_y - h/2`, `rot = -am_rot` |
| `CSS` | 同 `GODOT_CONTROL` |

### 3.6 数据输出

每个 `FlatElement` 携带两套数据，引擎按需取用：

```
FlatElement {
    // 目标坐标系下的世界变换矩阵（已应用所有父子累积 + CoordMappingConfig）
    world_matrix: [f32; 16],

    // 原始 AM 数据（任何时候都可以自己做自定义转换）
    am_position: [f32; 2],       // AM 原始坐标（左上原点，Y 向下，锚点 = 元素中心）
    am_rotation_deg: f32,        // AM 原始旋转（顺时针为正）
    am_scale: [f32; 2],          // AM 原始缩放（1.0 = 100%）
    am_anchor: [f32; 2],         // 锚点相对于元素左上角的像素偏移
    element_width: f32,
    element_height: f32,

    // 画布元信息
    canvas_width: f32,
    canvas_height: f32,
    ...
}
```

引擎如果对 dylib 生成的 `world_matrix` 不满意（例如想做额外的坐标空间变换），可以基于原始 AM 数据自行计算——所有原始信息都在。

---

## 四、dylib 导出接口设计

### 4.1 C-ABI 函数签名（草案）

```c
// ── 生命周期 ──────────────────────────────────────

/** 打开 AM 项目文件（.amproj 或 .xml）。
    @param coord 坐标映射配置。传 NULL 使用默认（AM_NATIVE）。
    返回不透明句柄，失败返回 -1。 */
int32_t am_project_load(const char* path, const AmCoordConfig* coord);

/** 释放项目。句柄失效后不可再使用。 */
void am_project_free(int32_t handle);

// ── 元数据 ────────────────────────────────────────

/** 运行时更新坐标映射配置（不需要重新加载项目）。 */
void am_project_set_coord_config(int32_t handle, const AmCoordConfig* coord);

/** 获取项目元数据。返回的指针在 am_project_free 前有效。 */
const AmMetadata* am_project_metadata(int32_t handle);

/** 获取验证报告 JSON 字符串。调用方负责释放（调用 am_string_free）。 */
const char* am_project_validation_report(int32_t handle);

// ── 逐帧查询 ──────────────────────────────────────

/** 查询指定时间点的所有可见元素（已插值）。
    @param time_secs 时间，单位秒
    @param out_count 输出元素数量
    @return FlatElement 数组，调用方不需要释放（下次调用覆盖） */
const FlatElement* am_query_frame(int32_t handle, float time_secs, int32_t* out_count);

/** 查询指定时间范围内变化（新增/移除/修改）的元素。
    用于增量更新，减少引擎侧重建开销。 */
const FrameDelta* am_query_frame_delta(int32_t handle, float from_secs, float to_secs, int32_t* out_count);

// ── 资源访问 ──────────────────────────────────────

/** 获取嵌入图片的 raw bytes。返回长度，buffer 由调用方分配。
    先调用 am_get_image_size 获取所需 buffer 大小。 */
int32_t am_get_image_size(int32_t handle, const char* uri);
int32_t am_get_image_data(int32_t handle, const char* uri, uint8_t* out_buffer, int32_t buffer_size);

/** 获取嵌入字体的 raw bytes。同上模式。 */
int32_t am_get_font_size(int32_t handle, const char* font_name);
int32_t am_get_font_data(int32_t handle, const char* font_name, uint8_t* out_buffer, int32_t buffer_size);
```

### 4.2 核心数据结构

```c
typedef struct {
    float width, height;
    int32_t fps;
    float total_time_secs;
    const char* bgcolor;       // ARGB hex string, e.g. "#ff000000"
    int32_t am_version;
} AmMetadata;

// ── 坐标映射配置 ────────────────────────────

typedef struct {
    float origin_anchor[2];    // 原点在画布中的位置，(0,0)=左上，(0.5,0.5)=中心，(0,1)=左下
    float x_direction;         // X 轴正方向，1.0=右，-1.0=左
    float y_direction;         // Y 轴正方向，1.0=下，-1.0=上
    float rotation_sign;       // 旋转正方向，1.0=逆时针，-1.0=顺时针
    float rotation_zero_axis[2]; // 旋转零度指向，(1,0)=右，(0,-1)=上
    float engine_anchor[2];    // 引擎锚点规范，(0.5,0.5)=中心，(0,0)=左上角
    float z_spacing;           // Z 轴层级间距
    int32_t column_major;      // 1=列主序（OpenGL/Vulkan/Bevy），0=行主序（DX/Unity）
} AmCoordConfig;

// 预设常量（在头文件中声明，dylib 中定义）
extern const AmCoordConfig AM_COORD_AM_NATIVE;
extern const AmCoordConfig AM_COORD_BEVY_2D;
extern const AmCoordConfig AM_COORD_UNITY_UI;
extern const AmCoordConfig AM_COORD_UNITY_WORLD;
extern const AmCoordConfig AM_COORD_GODOT_2D;
extern const AmCoordConfig AM_COORD_GODOT_CONTROL;
extern const AmCoordConfig AM_COORD_CSS;
extern const AmCoordConfig AM_COORD_OPENGL_NDC;

typedef struct {
    int32_t id;
    int32_t parent_id;
    int32_t layer_index;       // z-order 排序依据
    float world_matrix[16];    // 4x4 列主序（与 OpenGL/Vulkan 一致）

    // ── 形状（当 kind == SHAPE 时有效）──────
    int32_t kind;              // RECT, ELLIPSE, POLYGON, PATH, ARROW, TEXT, IMAGE, NULLOBJ
    float shape_params[16];    // 形状参数（具体含义见 kind 对应文档）
    const char* path_data;     // SVG path d 字符串（仅 kind == PATH）

    // ── 填充 ────────────────────────────────
    int32_t fill_type;         // NONE, SOLID, LINEAR_GRADIENT, RADIAL_GRADIENT, IMAGE
    float fill_color[4];       // RGBA
    float fill_gradient_start[2], fill_gradient_end[2];
    float fill_gradient_start_color[4], fill_gradient_end_color[4];
    const char* fill_image_uri;

    // ── 描边 ────────────────────────────────
    float stroke_width;
    float stroke_color[4];
    int32_t stroke_cap;       // BUTT, ROUND, SQUARE
    int32_t stroke_join;      // MITER, ROUND, BEVEL
    float stroke_miter_limit;

    // ── 文本（仅 kind == TEXT）──────────────
    const char* text_content;
    const char* text_font;
    float text_size;
    float text_wrap_width;
    int32_t text_align;       // LEFT, CENTER, RIGHT

    // ── 视觉效果 ────────────────────────────
    float opacity;
    int32_t blend_mode;       // NORMAL, MULTIPLY, SCREEN, ADD, OVERLAY
    int32_t effects_count;
    EffectInstance effects[MAX_EFFECTS_PER_ELEMENT];

    // ── AM 原始坐标信息（调试和自定义转换用）─
    float am_position[2];     // AM 原始坐标，左上原点，Y 向下
    float am_anchor[2];       // 锚点相对于元素左上角的偏移
    float element_width, element_height;

    // ── 生命周期 ────────────────────────────
    float start_time_secs;
    float end_time_secs;
} FlatElement;

typedef struct {
    int32_t effect_type;      // GAUSSIAN_BLUR, CHROMA_KEY, SHADOW, PIXELATE, RGB_SPLIT, ...
    float params[16];         // 效果参数（每种效果有对应的参数布局文档）
} EffectInstance;

typedef struct {
    int32_t added_count;
    int32_t removed_ids[MAX_DELTA_CHANGES];
    int32_t modified_count;
    FlatElement* added;       // 指向内部缓冲区
} FrameDelta;
```

### 4.3 字符串生命周期

FFI 中 `const char*` 类型的字段（如 `text_content`、`fill_image_uri`）指向 dylib 内部缓冲区，在下次 `am_query_frame` 调用或 `am_project_free` 之前有效。调用方如需持久化，应立即拷贝。

---

## 五、引擎侧桥接指南

### 5.1 桥接分层

```
L1 骨架层（1-2 天）
  └─ FFI 调用 → 创建节点 → 设置父级 → 设置变换矩阵 → 控制可见性

L2 基础渲染层（1-2 周）
  └─ 形状绘制 → 纯色/渐变/图片填充 → 描边 → 文本渲染 → 混合模式

L3 效果层（按需，每个效果 0.5-1 天）
  └─ 高斯模糊 → 色度键 → 阴影 → 像素化 → RGB 分离 → ...（逐个实现 shader）

L4 RTT 层（深度集成）
  └─ Mask 离屏渲染 → 嵌套场景 RenderTexture → Ping-Pong Buffer 管理
```

### 5.2 各引擎接入要点

#### Unity

```
- FFI: [DllImport("libamproj")] static extern int am_project_load(string path, ref AmCoordConfig coord);
- 坐标: 传入 AM_COORD_UNITY_UI 或 AM_COORD_UNITY_WORLD 预设
- 矩阵: Unity 行主序（column_major=false 已处理），直接赋值给 Transform.localToWorldMatrix
- 形状: 使用 SpriteShape 包或自建 Mesh（GenerateMeshFromShapeData）
- 效果: 使用 Shader Graph 或手写 HLSL
- 文本: 使用 TextMeshPro
- 字体: 从 dylib 获取 ttf bytes → FontAsset
- 图片: 从 dylib 获取 raw bytes → Texture2D.LoadImage
```

#### Godot

```
- FFI: GDExtension（C++）或 C# 的 [DllImport]
- 坐标: 传入 AM_COORD_GODOT_2D 预设（同 AM 原生：左上原点，Y 向下）
  如果使用 Control 节点，传入 AM_COORD_GODOT_CONTROL（锚点左上角）
- 形状: CanvasItem.draw_rect/draw_circle/draw_polygon 或 Polygon2D 节点
- 效果: CanvasItem shader（gdshader）
- 文本: Label 节点
- 字体: 从 dylib 获取 ttf bytes → FontFile
- 图片: 从 dylib 获取 raw bytes → Image.load_png_from_buffer → ImageTexture
```

#### 自定义 C/C++ 引擎

```
- FFI: 直接调用
- 坐标: 按需选择
- 形状: 自行生成三角形 mesh 或使用 nanoVG/Skia 等 2D 库
- 效果: 手写 GLSL/SPIR-V shader
- 文本: 使用 freetype + harfbuzz 或 stb_truetype
- 字体/图片: 从 dylib 获取 raw bytes → 引擎自己的资源管线
```

---

## 六、重构实施计划

### 6.1 阶段划分

| 阶段 | 工作内容 | 预估工时 | 产出物 |
|---|---|---|---|
| **P0：核心 crate 提取** | 创建 `amproj`，迁移 schema + easing + interpolation + error + validation + effects_registry，去 Bevy 化 | 2-3 天 | `crates/am_core/` |
| **P1：Loader 剥离** | 抽取 ZIP/XML 解析核心，去掉 Bevy AssetLoader trait 依赖 | 1-2 天 | `am_core/loader/`（纯 IO 版本） |
| **P2：FFI 导出层** | 实现 C-ABI 接口，FlatElement 数据结构，坐标转换 | 2-3 天 | `am_core/ffi.rs` |
| **P3：Bevy 插件适配** | 让原 `bevy_alight_motion` 依赖 `am_core`，复用数据层 | 1-2 天 | 更新 Cargo.toml 依赖 |
| **P4：测试验证** | 确保 AM 项目解析结果一致，跨坐标系一致性测试 | 2-3 天 | 测试用例 |
| **P5：文档** | C-ABI 接口文档、引擎接入指南、坐标系说明 | 1 天 | `doc/ffi_api.md` |
| **合计** | | **8-13 天** | |

### 6.2 Cargo.toml 拆分

重构后的 crate 结构：

```
crates/
├── am_core/                    # 纯数据 crate，零 Bevy 依赖
│   ├── Cargo.toml              # deps: serde, serde_json, quick-xml, zip, thiserror, log
│   └── src/
│       ├── lib.rs
│       ├── schema/             # AM XML 数据结构
│       ├── easing.rs           # 缓动函数
│       ├── interpolation.rs    # 关键帧插值
│       ├── loader.rs           # ZIP/XML 解析（纯 IO）
│       ├── validation.rs       # 场景验证
│       ├── error.rs            # 错误类型
│       ├── effects_registry/   # 效果注册表
│       └── ffi.rs              # C-ABI 导出（feature = "ffi"）
│
├── bevy_alight_motion/         # Bevy 插件（现有代码）
│   ├── Cargo.toml              # deps: am_core + bevy + ...
│   └── src/
│       ├── plugin/             # Bevy 插件入口（依赖 am_core）
│       ├── animation/          # ECS 动画系统
│       ├── scene/              # 实体生成
│       ├── effects/            # RTT 混合渲染管线
│       ├── sdf*.rs             # SDF 材质
│       ├── gaussian_blur.rs    # 高斯模糊材质
│       └── ...
```

### 6.3 am_core 的依赖（重构后）

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
quick-xml = { version = "0.39", features = ["serialize"] }
zip = { version = "8.1", default-features = false, features = ["deflate"] }
thiserror = "2.0"
log = "0.4"                    # 替代 bevy::log
glam = { version = "0.29", features = ["libm"] }  # 替代 bevy::prelude::Vec4 和相关数学

[features]
ffi = []  # 启用 C-ABI 导出层
```

---

## 七、风险与注意事项

### 7.1 已知风险

| 风险 | 影响 | 缓解措施 |
|---|---|---|
| AM XML schema 版本变化 | 解析失败 | effects_registry 按 amver 标记，schema 使用 serde default 容错 |
| 嵌入资源路径不一致 | 引擎找不到图片/字体 | dylib 提供 raw bytes 访问，引擎自行处理资源创建 |
| 浮点精度在跨引擎间差异 | 像素级不对齐 | world_matrix 使用 f32，不承诺亚像素精度 |
| 锚点语义在复杂嵌套中漂移 | 嵌入场景位置偏差 | 坐标转换在 dylib 侧一次算好，引擎只设矩阵 |
| 线程安全 | 多线程查询 crash | dylib 内部使用 RwLock，每个 handle 独立状态 |

### 7.2 不做什么

- ❌ 不在 dylib 中实现渲染——只输出数据
- ❌ 不提供每帧的内存分配——内部使用固定大小 buffer，下次查询覆盖
- ❌ 不处理音频层（AM 的 Audio 图层）——静默跳过
- ❌ 不处理视频层（AM 的 Video 图层）——静默跳过
- ❌ 不模拟 Bevy 的 ECS 调度——只提供同步查询接口

---

## 八、坐标映射速查表

### 8.1 预设 vs 引擎对照

| 预设常量 | origin_anchor | x_dir | y_dir | rot_sign | rot_zero | eng_anchor | col_major |
|---|---|---|---|---|---|---|---|
| `AM_COORD_AM_NATIVE` | `[0, 0]` | `+1` | `+1` | `-1` | `[1,0]` 右 | `[.5,.5]` | ✅ |
| `AM_COORD_BEVY_2D` | `[.5,.5]` | `+1` | `-1` | `+1` | `[1,0]` 右 | `[.5,.5]` | ✅ |
| `AM_COORD_UNITY_UI` | `[0, 1]` | `+1` | `-1` | `+1` | `[1,0]` 右 | `[.5,.5]` | ❌ |
| `AM_COORD_UNITY_WORLD` | `[.5,.5]` | `+1` | `-1` | `+1` | `[1,0]` 右 | `[.5,.5]` | ❌ |
| `AM_COORD_GODOT_2D` | `[0, 0]` | `+1` | `+1` | `-1` | `[1,0]` 右 | `[.5,.5]` | ✅ |
| `AM_COORD_GODOT_CONTROL` | `[0, 0]` | `+1` | `+1` | `-1` | `[1,0]` 右 | `[0, 0]` | ✅ |
| `AM_COORD_CSS` | `[0, 0]` | `+1` | `+1` | `-1` | `[1,0]` 右 | `[0, 0]` | ✅ |
| `AM_COORD_OPENGL_NDC` | `[.5,.5]` | `+1` | `-1` | `+1` | `[1,0]` 右 | `[.5,.5]` | ✅ |

### 8.2 各字段的语义

| 字段 | 含义 | AM 中的对应值 |
|---|---|---|
| `origin_anchor` | 画布原点在画布中的归一化位置 | `[0, 0]`（左上角） |
| `x_direction` | X 轴正方向符号 | `+1`（向右） |
| `y_direction` | Y 轴正方向符号 | `+1`（向下） |
| `rotation_sign` | 旋转正方向符号 (`+1`=逆时针, `-1`=顺时针) | `-1`（AM 旋转顺时针为正） |
| `rotation_zero_axis` | 旋转 0° 时的朝向 | `[1, 0]`（指向右） |
| `engine_anchor` | 引擎侧锚点在元素包围盒中的归一化位置 | `[0.5, 0.5]`（AM 锚点默认在元素中心） |
| `z_spacing` | 每个层级的 Z 轴增量 | `0.001`（Bevy 2D 微量偏移）、`1.0`（Unity UI 整数偏移） |
| `column_major` | 矩阵存储顺序 | `true`（OpenGL/Vulkan/Bevy）、`false`（DirectX/Unity） |

### 8.3 自定义配置示例

#### Vulkan NDC（viewport 翻 Y）

```c
AmCoordConfig vulkan_ndc = {
    .origin_anchor = {0.5f, 0.5f},
    .x_direction = 1.0f,
    .y_direction = 1.0f,      // Vulkan NDC Y 向下（不翻 viewport 时）
    .rotation_sign = -1.0f,
    .rotation_zero_axis = {1.0f, 0.0f},
    .engine_anchor = {0.5f, 0.5f},
    .z_spacing = 0.001f,
    .column_major = 1,
};
```

#### 自研引擎（左下原点、Y 向上、旋转逆时针、锚点左下角）

```c
AmCoordConfig custom_engine = {
    .origin_anchor = {0.0f, 1.0f},   // 左下角
    .x_direction = 1.0f,             // 右 +
    .y_direction = -1.0f,            // 上 +
    .rotation_sign = 1.0f,           // 逆时针 +
    .rotation_zero_axis = {1.0f, 0.0f},
    .engine_anchor = {0.0f, 0.0f},   // 锚点 = 元素左下角
    .z_spacing = 1.0f,
    .column_major = 1,
};
```

### 8.4 调试检查清单

引擎接入时，如果位置/旋转出现偏差，逐项排查：

1. **原点不对** → 检查 `origin_anchor`。常见错误：应该中心却用了左上
2. **Y 轴方向反了** → 检查 `y_direction`。AM 是 `+1`（下），Bevy 是 `-1`（上）
3. **旋转方向反了** → 检查 `rotation_sign`。AM 顺时针为正(`-1`)，Bevy 逆时针为正(`+1`)
4. **元素偏移了一个半宽/半高** → 检查 `engine_anchor`。常见错误：引擎锚点是左上角但配置为中心
5. **矩阵读出来是转置的** → 检查 `column_major`。Unity/DirectX 行主序 = `false`
6. **深度顺序不对** → 交换 `z_spacing` 符号或检查引擎的深度测试方向
