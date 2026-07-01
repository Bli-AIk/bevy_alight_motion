# FFI 接入其他游戏引擎简要说明

本文说明如何把 `amproj` 作为动态库接入 Unity、Godot、Unreal、Cocos、自研引擎等非 Bevy 运行时。

FFI 层只负责解析 `.amproj`、按时间采样动画、输出扁平元素数据和内嵌资源字节。实际渲染由接入方引擎完成。

## 构建动态库

在仓库根目录运行：

```bash
cargo build -p amproj --features ffi --release
```

产物位置通常在 workspace 的 `target/release/`：

- Linux: `libamproj.so`
- macOS: `libamproj.dylib`
- Windows: `amproj.dll`

如果单独进入 `am_core/` 构建，也可以运行：

```bash
cargo build --features ffi --release
```

## 接入流程

典型游戏引擎接入流程如下：

1. 引擎启动或资源加载阶段加载动态库。
2. 调用 `am_project_load(path, coord)` 加载 `.amproj`，得到 project handle。
3. 调用 `am_project_metadata(handle)` 读取画布尺寸、fps、总时长等元信息。
4. 每帧根据引擎时间调用 `am_query_frame(handle, time_secs, &count)`。
5. 把返回的 `FlatElement[count]` 转成引擎自己的节点、Mesh、Sprite、Text、Material。
6. 用 `am_get_image_data` / `am_get_font_data` 读取内嵌图片和字体，交给引擎资源系统。
7. 工程卸载时调用 `am_project_free(handle)`。

## 主要 C ABI

核心函数：

```c
int am_project_load(const char* path, const AmCoordConfig* coord);
void am_project_free(int handle);
void am_project_set_coord_config(int handle, const AmCoordConfig* coord);

const AmMetadata* am_project_metadata(int handle);
char* am_project_validation_report(int handle);
void am_string_free(char* value);

const FlatElement* am_query_frame(int handle, float time_secs, int* out_count);
const FrameDelta* am_query_frame_delta(int handle, float from_secs, float to_secs, int* out_count);

int am_get_image_size(int handle, const char* uri);
int am_get_image_data(int handle, const char* uri, unsigned char* out_buffer, int buffer_size);
int am_get_font_size(int handle, const char* font_name);
int am_get_font_data(int handle, const char* font_name, unsigned char* out_buffer, int buffer_size);
```

坐标预设以全局符号导出：

```c
extern const AmCoordConfig AM_COORD_AM_NATIVE;
extern const AmCoordConfig AM_COORD_BEVY_2D;
extern const AmCoordConfig AM_COORD_UNITY_UI;
extern const AmCoordConfig AM_COORD_UNITY_WORLD;
extern const AmCoordConfig AM_COORD_GODOT_2D;
extern const AmCoordConfig AM_COORD_GODOT_CONTROL;
extern const AmCoordConfig AM_COORD_CSS;
extern const AmCoordConfig AM_COORD_OPENGL_NDC;
```

如果某个引擎的绑定层不方便读取动态库全局变量，可以在宿主侧按 `AmCoordConfig` 结构体手动复制同等配置，或先传 `NULL` 使用 AM 原生坐标，再由引擎侧二次转换。

## 数据生命周期

- `am_project_load` 返回 `-1` 表示加载失败。
- `am_query_frame` 返回的指针由库内部持有，不要释放。
- `FlatElement*` 只保证在下一次同 project 的 `am_query_frame` / `am_query_frame_delta` 或 `am_project_free` 前有效。
- 引擎侧如果要跨帧保存数据，应立即复制数组内容。
- `am_project_validation_report` 返回的字符串需要用 `am_string_free` 释放。
- `am_get_image_data` / `am_get_font_data` 的返回值是实际资源大小；如果传入 buffer 太小，会返回所需大小但不会完整写入。

建议不要在渲染线程每帧加载/释放 project。project handle 应该跟随引擎资源生命周期缓存。

## FlatElement 映射

`FlatElement` 是已经按时间采样后的扁平元素。每个元素都包含：

- `id` / `parent_id` / `layer_index`
- `world_matrix[16]`
- `kind`
- fill、stroke、text、effect、opacity、blend mode 等字段
- AM 原始局部 transform：`am_position`、`am_rotation_deg`、`am_scale`、`am_anchor`
- 元素尺寸和画布尺寸
- 起止时间

当前 `kind` 约定：

| kind | 含义 |
| --- | --- |
| 1 | 矩形 / 圆角矩形 |
| 2 | 圆 / 椭圆 |
| 3 | 多边形、星形、扇形等 |
| 4 | Path |
| 5 | 线段 / 箭头 |
| 6 | Text |
| 7 | Image |
| 8 | Null |
| 9 | EmbedScene |
| 10 | Camera |

接入时可以先支持最常用路径：

1. `kind == 7`：创建引擎 Sprite/Quad，使用 `fill_image_uri` 对应的图片资源。
2. `kind == 6`：创建 Text 节点，使用 `text_content`、`text_font`、`text_size`。
3. `kind == 1/2/3/4/5`：创建几何 Mesh 或引擎 UI Shape。
4. 对 `world_matrix` 应用到节点 transform，或拆解矩阵后映射到引擎 Transform。
5. 使用 `opacity`、`fill_color`、`blend_mode` 驱动材质。

## 资源读取

图片读取伪代码：

```c
int size = am_get_image_size(handle, uri);
if (size > 0) {
    uint8_t* bytes = malloc(size);
    int written = am_get_image_data(handle, uri, bytes, size);
    if (written == size) {
        // 交给引擎图片解码/纹理上传系统
    }
    free(bytes);
}
```

字体读取同理使用 `am_get_font_size` / `am_get_font_data`。

`uri` 一般来自 `FlatElement.fill_image_uri`，字体名来自 `FlatElement.text_font`。

## 坐标系建议

优先选择最接近目标引擎的坐标预设：

- Unity UI: `AM_COORD_UNITY_UI`
- Unity World 2D/3D 平面: `AM_COORD_UNITY_WORLD`
- Godot Node2D: `AM_COORD_GODOT_2D`
- Godot Control: `AM_COORD_GODOT_CONTROL`
- Web Canvas/CSS: `AM_COORD_CSS`
- OpenGL NDC 管线: `AM_COORD_OPENGL_NDC`

如果引擎项目有自己的画布缩放、UI anchor、DPI 策略，建议先用接近的预设跑通，再在宿主侧统一做最终缩放和 viewport 适配。

## 引擎侧实现建议

Unity:

- C# 用 `DllImport` 声明 C ABI。
- `FlatElement` 用 `[StructLayout(LayoutKind.Sequential)]` 对齐。
- 每帧把返回数组复制到托管数组或 `NativeArray`，不要持有裸指针跨帧。
- 图片字节可转成 `Texture2D.LoadImage`，字体接入需要按项目字体系统处理。

Godot:

- GDExtension/C++ 或 C# 均可调用动态库。
- 推荐把 project handle 封装成 Godot Resource。
- 每帧生成或更新 `Node2D` / `Control` / `MeshInstance2D`。

Unreal:

- 用 `FPlatformProcess::GetDllHandle` / `GetDllExport` 加载符号。
- 把 project handle 封装成 `UObject` 或 subsystem 管理。
- `FlatElement` 转成 `UWidget`、`UTexture2D`、`ProceduralMeshComponent` 或自定义 Slate 绘制数据。

自研引擎:

- 直接按 C ABI 加载函数指针。
- 在资源线程加载 `.amproj` 和 embedded media。
- 在主线程/动画线程调用 `am_query_frame`，复制结果后交给渲染线程。

## 最小调用顺序

```c
int handle = am_project_load("example.amproj", &AM_COORD_UNITY_UI);
if (handle < 0) {
    return;
}

const AmMetadata* meta = am_project_metadata(handle);

int count = 0;
const FlatElement* elements = am_query_frame(handle, 1.25f, &count);
for (int i = 0; i < count; i++) {
    const FlatElement* e = &elements[i];
    // 转换为引擎节点或渲染命令
}

am_project_free(handle);
```

## 当前限制

- FFI 层不直接创建引擎材质、节点或纹理。
- 复杂效果会以 `effects` 参数形式暴露，目标引擎需要自己实现对应 shader/material。
- `am_query_frame_delta` 当前主要返回目标时间点的新增/修改数据，接入方仍应准备好全量刷新路径。
- 跨线程使用时要立即复制返回数据，不要在另一个线程长期持有内部指针。

