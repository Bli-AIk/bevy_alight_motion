# `bevy_alight_motion` 向 `alight_motion` 源码架构靠拢的可行性评估

## 结论

结论先说清楚：

- **可行**
- **值得做**
- **但不建议一次性“大爆炸式 1:1 重写”**

更准确的判断是：

- 适合做 **大幅度、分阶段、以语义对齐为目标** 的重构
- 不适合做 **短期内全面推倒重来** 的重构

如果只问一句“要不要往 `alight_motion` 源码架构方向重构”，我的答案是：

**要，但应该做成“渐进迁移”，不是“整仓重写”。**

## 为什么现在已经碰到架构天花板

这几轮排查里，已经出现了多次“不是单点 bug，而是架构信息丢失”的信号。

### 1. 时间语义不稳定

这次 `private/USER_mortis/revenge/split1` 的第一层问题，不是效果公式错，而是：

- comparison 采样时刻比目标早了 `1ms`
- 某个嵌套层正好卡在 `983ms` 激活边界
- 导致首帧整块内容直接缺失

这说明当前工程里：

- “参考视频帧时间”
- “播放器时间”
- “生命周期激活时间”
- “截图时机”

这四套时间语义没有被一个统一的时序模型收口，而是散落在多个系统里靠等待帧数、round/floor、渲染 settle 来凑。

### 2. RTT/嵌套组合缺少“来源语义”

这次第二层关键问题，是嵌套 RTT 纹理进入父层 unified shader 后：

- alpha 又做了一次 gamma 变换
- rgb 又做了一次 premultiply

结果半透明白条被压暗。

这不是“shader 算错一点点”，本质是：

- 当前材质/渲染链路里，**没有稳定表达“这个输入是普通源图，还是来自上一层 RTT 的 premultiplied 结果”**

也就是：**source provenance 丢了。**

这类问题如果只靠 case-by-case 补丁，会无限复发。

### 3. effect 语义分散在 collect / spawn / shader / runtime 多层

目前很多 effect 的语义不是在一个地方闭环，而是散在：

- scene collect
- nested embed flatten
- runtime spawn
- effect update system
- WGSL shader

比如这次链路里就能看到：

- replace-color 的继承和传播
- intrinsic fill 的特殊处理
- nested embed 的 RTT 建立
- unified material 的最终输出约定

这些信息并没有在一个“中间表示层”收束。

这意味着：

- 修一处时常常不知道应该改哪一层
- 每层都能“修出一个局部看似合理但全局副作用巨大的补丁”

### 4. comparison harness 已经在替 runtime 补洞

当前 comparison 系统里已经存在很多“为了让 runtime 看起来稳定”的防抖逻辑：

- `initial_wait_frames`
- `render_wait_frames`
- `settle_signature`
- `prime_capture_requests`
- frame rounding

这些逻辑不是没价值，但它们已经说明：

**runtime 本身没有把“在某个时刻得到确定画面”做成一等能力。**

这也是为什么我们会陷入：

- 不是不能渲染
- 而是很难稳定地在正确时刻渲染出正确结果

## 是否应该向 `alight_motion` 源码架构靠拢

我的判断是：**应该靠拢“架构原则”，不应该盲目追求源码目录/类名级别的 1:1 复刻。**

建议靠拢的，是下面这些原则。

### 1. 明确的中间表示层

应该有一层明确的、和 Bevy ECS 解耦的运行前 IR，用来表达：

- 图层树
- 本地时间/父子时间关系
- 生命周期
- effect 列表
- mask / blend / fill / embed / RTT 需求
- 输入源类型

当前代码已经有 `PendingLayer` / collect 体系，但还不够像“可执行 IR”，更像“解析后的半成品数据”。

### 2. 显式的 pass / compositing graph

`alight_motion` 源码架构里最值得借的是：

- 哪些层直接绘制
- 哪些层必须进 offscreen / RTT
- effect 的执行顺序
- 哪些结果是 straight alpha
- 哪些结果是 premultiplied

这些都应该变成 **显式图结构或明确的 pass plan**，而不是运行时到处猜。

### 3. 单一的颜色/alpha 契约

现在最痛的点之一就是：

- sRGB / linear
- straight / premultiplied
- 原始纹理 / RTT 纹理

没有一个全项目统一、可审计的契约。

必须收敛成统一规则，例如：

1. CPU 侧 effect 参数一律按什么空间存
2. shader 输入默认按什么契约解释
3. RTT 输出是什么契约
4. 再采样 RTT 时要不要解码/去 premultiply
5. comparison 保存图像时是哪个阶段的结果

如果这个契约不先收紧，重构规模越大，bug 只会越隐蔽。

### 4. 时间推进和生命周期调度分层

建议把下面四件事彻底分开：

- 设置时间
- 推进生命周期
- 等待渲染稳定
- 执行截图/比对

当前这些行为虽然已经有 stage，但还是较强地耦合在 example/comparison 流程里。

更理想的架构应该让“给定时间点，生成稳定画面”成为 runtime 的标准能力，而不是 comparison 专属脚本能力。

## 可行性评估

### 技术可行性

**高。**

原因：

- 当前仓库已经有相当多的功能覆盖
- 解析层、动画层、visual 层、effect 层已经存在
- 不是“从 0 到 1”，而是“从半成品到结构收敛”

真正缺的不是能力，而是：

- 层级边界
- 数据契约
- 渲染图模型

### 工程可行性

**中高。**

原因：

- 代码已经足够大，继续局部补丁的边际收益在下降
- 但仓库又还没大到完全无法迁移

难点在于：

- 当前有很多正在工作的 case
- 一次性重构极容易把已有通过项打回去
- visual regression 成本高，VPS 又慢

所以它适合做“带测试护栏的分段迁移”。

### 风险

**高，但可控。**

主要风险不是“写不出来”，而是：

- 重构周期长
- 中途通过率可能明显波动
- 很容易出现“新架构更优雅，但短期通过率掉更多”

因此必须把重构目标写得非常具体。

## 不建议做的事情

### 1. 不建议目录级 1:1 模仿

不要为了“像源码”而去复制目录、类名、文件切分方式。

原因：

- `bevy_alight_motion` 的执行模型是 ECS + shader + Bevy render graph
- `alight_motion` 原码不是这个运行时

直接照搬结构，只会得到“形式像，语义还是错”的中间态。

### 2. 不建议一次性替换全部 effect 路径

最危险的做法是：

- 一口气重写 collect
- 一口气重写 spawn
- 一口气重写 shader
- 一口气重写 comparison

这会让定位回归变得几乎不可能。

### 3. 不建议先追求“代码漂亮”

当前阶段最重要的是：

- 语义正确
- pass 正确
- 时间正确
- alpha/color 契约正确

不是文件先好看，不是模块先“优雅”。

## 建议的重构路线

我建议按 4 个阶段走。

### Phase 0: 固化语义基线

目标：

- 不继续无节制加补丁
- 先把关键语义写成文档和 trace 约定

要产出：

- 时间语义文档
- color/alpha 契约文档
- RTT 输入输出契约文档
- 关键样例分组

建议选的样例集：

- `basic/fill/*`
- `hong_jiao` 中最短且代表性的样例
- `private/USER_mortis/revenge/split1`

### Phase 1: 引入明确的 Runtime IR

目标：

- collect 结果不再只是“待 spawn 数据”
- 而是“可执行的 layer/runtime plan”

这层至少要表达：

- layer id / parent id
- local time transform
- activation range
- visual source type
- effect stack
- compositing requirement
- output contract

### Phase 2: 重做 compositing / RTT plan

目标：

- 让“谁进 RTT、谁直接画、谁吃哪张纹理、谁输出什么契约”变成显式规则

这里是最该向 `alight_motion` 架构原则靠拢的部分。

建议结果是：

- 普通视觉层
- offscreen pass 节点
- composite 节点
- effect pass 节点

这些节点在 CPU 侧就先定下来，而不是 shader/系统运行时碰到了再猜。

### Phase 3: 收敛时间推进与 comparison

目标：

- runtime 支持“跳到任意时刻并稳定渲染”
- comparison 只是调用 runtime 能力，而不是自己持有一套补偿逻辑

理想状态：

- comparison 只负责：
  - 选择时间点
  - 请求稳定渲染
  - 抓图
  - 比对

- runtime 负责：
  - 生命周期
  - 依赖 spawn
  - offscreen settle
  - 材质状态一致性

### Phase 4: 再考虑大拆文件和风格统一

这个阶段才处理：

- 文件体积
- API 边界
- 模块命名
- 公共 helper 清理

顺序不能反。

## 我给这件事的最终建议

### 建议做

因为现在遇到的问题已经不是“继续修几个 effect 就好”，而是：

- 时间、RTT、effect、capture 四套语义正在互相泄漏
- 每修一个 case，都会暴露“信息没被架构保住”

### 但必须按“渐进重构”做

推荐策略：

- 先继续以最短样例维持通过率提升
- 同时开一条“runtime IR + compositing plan”支线
- 先把最痛的链路迁进去：
  - nested embed
  - RTT source contract
  - replace / pixelate / mask / fill 这些最常组合的 effect

### 现阶段最值得先动的三个点

1. `Runtime IR`
   让 collect/spawn 中间层不再丢语义。

2. `RTT source contract`
   把 normal texture / RTT texture / premultiplied output 的来源显式编码。

3. `time-to-frame contract`
   让 comparison 采样、生命周期激活、render settle 统一到同一时序模型。

## 一句话决策

如果目标是：

- 短期继续靠补丁把所有 case 拉过线

那还能继续做，但效率会越来越差。

如果目标是：

- 让这个仓库后面还能稳定扩 effect、扩样例、扩通过率

那就应该启动一次 **以语义收敛为核心的大幅度重构**。

**可行，且值得做；但必须分阶段推进，不能一次性推翻重来。**
