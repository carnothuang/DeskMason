# DeskMason 技术栈决策

> 2026-11-14。目标：只支持 Deepin 的本地优先桌面瓦片应用。

## 一、目标平台的实测事实

在本机（Deepin 25 / DDE / KWin / X11，3440×1440）实测：

| 事实 | 证据 |
|---|---|
| 窗管是 KWin，Wayland 会话是 Treeland（wlroots 系） | `wmctrl -m` → KWin；`/usr/share/wayland-sessions` 只有 `treeland.desktop` |
| 两者都支持 layer-shell | G1-1 在 KWin 上跑通桌面层；Treeland 基于 wlroots，原生支持 `zwlr_layer_shell_v1` |
| Deepin 的原生应用栈是 Qt 6 + DTK6 | 已装 `libqt6core6 6.8.0`、`dtk6core/gui/widget`、`dde-qt6integration`（DDE 的 Qt6 平台主题） |
| DDE 的现代界面用 Qt Quick + DTK Declarative | 仓库里有 `dtk6-exhibition`（DTK6 QML 控件示例）与 `libdtk6declarative-dev` |
| Qt Quick 开发包可装但未装 | `qt6-declarative-dev 6.8.0+dfsg-0deepin17`、`libdtk6declarative-dev 6.7.52` 在 deepin 源 |
| GTK4 不存在 | `pkg-config gtk4` 无结果，Deepin 不装 GTK4 |
| 托盘可用 | `org.kde.StatusNotifierWatcher` 有宿主注册（DDE 托盘） |
| X11 全局热键可用 | G1-2 在 Xvfb 上全组合命中；实机受 `dde-lock` 抢焦点待复验 |

## 二、结论

**Rust（业务 + 平台层）+ Qt Quick/QML（界面）+ DTK6 Declarative（Deepin 原生控件），
用 cxx-qt 桥接。**

```
┌─────────────────────────────────────────────────────┐
│ QML：瓦片外壳、9 种内容、设置页、引导、对话框、动画    │  Qt Quick + Dtk QML
├─────────────────────────────────────────────────────┤
│ dm-ui：cxx-qt QObject 桥（属性/信号/模型）            │
├─────────────────────────────────────────────────────┤
│ dm-core：领域模型、布局、设置、迁移、排序、策略         │  Rust
│ dm-shell：宿主生命周期、托盘、热键、多屏、瓦片组        │
│ dm-platform：X11 / Wayland / D-Bus / XDG / inotify    │
└─────────────────────────────────────────────────────┘
```

## 三、为什么不是别的

### 不用纯 Rust UI（iced / egui / Slint）

完整功能集需要覆盖：9 种瓦片、富文本 Markdown 编辑、中文 IME、无障碍、
拖放、缩放动画、可折叠外壳、13 语言。纯 Rust UI 框架在这几项上要么缺失、
要么要自己造：

| 能力 | Qt Quick | iced/egui/Slint |
|---|---|---|
| 中文 IME / 输入法候选窗 | 成熟 | 弱或需自造 |
| 富文本与 Markdown 预览 | 成熟（Text/RichText） | 基本要自造 |
| 无障碍（AT-SPI） | Qt 已接 | 基本没有 |
| 控件与主题生态 | DTK 现成 | 无 Deepin 主题 |
| 文件对话框 / 托盘 / 剪贴板 | Qt 自带 | 需逐项接 |

Slint 的声明式写法最接近 QML，但生态不足以支撑完整功能集。

### 不用 GTK4

Deepin 没装 GTK4；GTK 应用在 DDE 上观感割裂（标题栏、对话框、主题、字体渲染）。
"只支持 Deepin"这一条直接排除 GTK。

### 不用 C++ 作为主实现语言

可以走，而且 DTK6 是 C++ 的。但：

- 后台全是并发与生命周期：`inotify` 递归监听、文件复制进度、搜索索引、
  媒体会话订阅。Rust 的 `Send/Sync` + 无 GC 在这些地方更稳。
- 崩溃面：一个常驻桌面的程序，C++ 的悬垂指针会变成用户数据的风险。

保留例外：如果 cxx-qt 在某处（例如自定义 Wayland 协议、DTK 深度定制）明显碍事，
允许写**最小 C++ 胶水**，但不得承载业务逻辑。

### 不用 Electron / Tauri

与"低内存、原生质感"直接冲突；Web 壳的常驻内存开销过高。

## 四、cxx-qt 的定位与边界

- Rust 侧暴露 `#[cxx_qt::qobject]` 类型：属性、信号、`QAbstractListModel`。
- QML 侧只做**呈现与交互**，不写业务规则。判断逻辑一律回 Rust。
- 桥接层禁止出现 `QString`/`QVariant` 的业务语义，只做类型转换。
- 若某个 QML 组件需要的数据超过 5 个标量字段，改传模型，不堆属性。

## 五、桌面层方案（Deepin 专用）

| 会话 | 方案 | 状态 |
|---|---|---|
| DDE / X11 | `_NET_WM_WINDOW_TYPE_DESKTOP` + `_NET_WM_STATE_BELOW` + skip-taskbar/pager | G1-1 已验证 |
| DDE / Treeland | `zwlr_layer_shell_v1`，layer=background，anchor 四边，exclusive_zone=-1 | 待补探针 |

两条路径都实现 `dm-platform::DesktopLayer`，宿主不感知差异。

## 六、待办的前置条件

```bash
sudo apt install qt6-declarative-dev libdtk6declarative-dev ninja-build cmake
```

装完后 `dm-ui` 加入 workspace，M0 的验收是：一个 QML 窗口 + 一个 Rust QObject
属性双向绑定 + 一个 `ListView` 虚拟化列表，在 DDE X11 下跑起来。

## 七、风险清单

| 风险 | 影响 | 应对 |
|---|---|---|
| cxx-qt 与 DTK QML 模块的构建集成 | M0 卡住 | 先做最小样例；不行就退回"C ABI + 200 行 C++桥" |
| Treeland 下 layer-shell + QtWayland | 桌面层在 Wayland 不可用 | 用 `wayland-client` 自建 surface，Qt 只画内容；或先只支持 X11 |
| DTK6 Declarative 文档稀缺 | 控件用法靠读源码 | 以 `dtk6-exhibition` 为样例源 |
| 完整功能集工作量 | 12-18 人月量级 | 按瓦片类型分批，每批可独立发布 |
| Qt LGPLv3 与 GPLv3 项目的兼容 | 法务 | LGPLv3 允许 GPLv3 项目动态链接，保留替换权说明 |
