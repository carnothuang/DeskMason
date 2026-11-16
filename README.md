# DeskMason · 桌面瓦匠

把常用文件夹和随手工具做成贴在桌面上的「瓦片」。瓦片不是普通窗口，而是挂在桌面层的常驻面板：始终位于桌面图标之上、日常应用窗口之下，点桌面空白处不会消失，也不会挡住正在用的程序。

**本地优先**：布局、设置、待办与随记全部存在本机，不需要账号、不连云端、不上传任何数据。文件瓦片只是引用文件夹，文件本身仍在原来的位置。

---

## 功能

| 瓦片 | 能力 |
| --- | --- |
| 文件格子 | 绑定任意文件夹，展示并打开文件、拖放、排序、视图密度；顶部 `＋` 只能再建一个文件格子，且必须选择**新的**文件夹路径 |
| 待办 | 新增与勾选、未完成／全部／已完成筛选、单条详情（标题、备注、到期、步骤）、颜色标记 |
| 随记 | 快速记录，点击单条查看全文并支持复制 |
| 天气 | 城市设置（中文／英文输入均可）、天气与预报展示 |
| 音乐 | 通过 MPRIS 控制播放器，封面、进度、播放状态 |
| 时光 | 时钟、日期、星期、农历、节气与节日；可用当前桌面壁纸作背景，并在设置中自定义或关闭 |
| 设置 | 瓦片开关、外观与材质、胶囊模式、文件格子、功能格子、快捷与交互、诊断与维护、关于 |

待办、随记、音乐、天气、时光为**全局单例**，只能在「设置」里开启或关闭，不会重复创建。

## 交互约定

- **拖动**：抓住瓦片顶部把手拖动，优先交给窗管做系统拖动，不支持时退回手动路径。
- **缩放**：拖四条边缘或四角改变尺寸；靠近相邻瓦片的边界会产生吸附对齐。
- **间隔**：新建与自动排布默认保留 40px 间隔；松手时如果瓦片边缘正好贴住相邻瓦片或屏幕四边，会自动退开 10px 留出可抓取的缝隙，方便下次再拖。
- **多屏**：所有瓦片强制约束在**主屏**工作区，启动时校正历史布局、拆开重叠并补齐最小间隔。
- **层级**：瓦片保持在桌面之上、普通窗口之下；点击桌面空白处不会消失，也不会盖住文本编辑器等应用窗口。
- **输入法**：只有含输入框的瓦片接受 X 键盘焦点，纯展示瓦片不抢焦点，避免与 fcitx5 的输入上下文反复切换。

## 技术栈

| 层 | 选择 |
| --- | --- |
| 业务与平台 | Rust（edition 2021，rust-toolchain 1.96.0） |
| 界面 | Qt Quick / QML，Qt 6 |
| Rust ↔ Qt 桥接 | `cxx-qt`，把 Rust 的 QObject 与 invokable 暴露给 QML |
| 桌面层 | X11 / EWMH（`x11rb` + RandR），窗口类型与层级直接管理 |
| 本地存储 | JSON 布局档 + SQLite（待办、随记）+ 键值偏好 |
| 系统集成 | D-Bus / MPRIS、XDG autostart、StatusNotifierItem 托盘、X11 全局热键 |
| 打包 | deb（`packaging/build-deb.sh`） |

## 运行环境

- **Deepin 25 / DDE，X11 会话**（KWin）。其它桌面环境与 Wayland / Treeland 组合目前不支持。
- Rust 1.85 以上（`rust-toolchain.toml` 固定 1.96.0）。
- Qt 6 开发包、CMake / Ninja、C++ 编译器、SQLite3、X11 开发库。

Deepin 25 上可先安装：

```bash
sudo apt install qt6-declarative-dev qml6-module-qtquick-controls \
                 qml6-module-qtquick-layouts cmake ninja-build clang \
                 libsqlite3-dev libxcb1-dev pkg-config
```

> 常规 Deepin 环境用系统默认工具链构建即可。`.cargo/config.toml` 与 `dev-env.sh`
> 只是当前开发机的沙盒桥接配置（含本机绝对路径），已从版本控制中排除；
> 在常规环境里请忽略或移除它们，避免覆盖本机工具链设置。

## 构建与运行

```bash
cargo build --workspace          # 构建全部 crate
cargo test --workspace           # 领域层、平台层、UI 层单元测试
cargo clippy --all-targets       # 静态检查
cargo fmt --all -- --check       # 格式检查

cargo run -p dm-app              # 启动桌面瓦片进程（二进制名 deskmason-ui）
cargo run -p dm-shell            # 无界面宿主进程
```

开发时可用 `DESKMASON_OPEN=settings` 或 `DESKMASON_OPEN=search` 直接拉起对应窗口。

QML 以资源形式嵌入二进制：**修改 `.qml` 后必须清理缓存再构建**，否则会加载到旧字节码：

```bash
cargo clean -p dm-ui && cargo build -p dm-app
```

打 deb 包：

```bash
packaging/build-deb.sh           # 产物在 target/deb/
```

## 目录结构

```
crates/
  dm-core/      领域模型：瓦片类型、几何、布局、设置与持久化格式
  dm-platform/  平台能力：X11 桌面层、显示拓扑、热键、托盘、文件操作、
                图标、壁纸、SQLite 存储、布局档读写（唯一平台边界）
  dm-shell/     无界面宿主进程
  dm-ui/        cxx-qt 桥、QML 界面、C++ 窗口胶水
  dm-app/       Qt 应用入口，按布局创建各瓦片窗口
packaging/      deb 构建脚本、desktop 文件、应用图标
translations/   翻译源文件与编译产物
```

平台相关代码只允许出现在 `dm-platform`；其它 crate 不直接使用 `x11rb`、D-Bus 或 `/proc`。一个瓦片类型 = 一个领域模块 + 一个 QML 组件 + 一条注册项，新增类型不需要改宿主的分支逻辑。

## 数据与配置

| 路径 | 内容 |
| --- | --- |
| `~/.config/deskmason/widget-layout.json` | 瓦片布局（唯一真相），带 `schemaVersion` 与迁移 |
| `~/.config/deskmason/` | 偏好设置、UI 状态 |
| `~/.local/share/deskmason/` | 待办／随记 SQLite 数据库 |
| `~/.local/share/deskmason/backups/` | 本地备份（布局、设置、数据库） |

布局档在进程内有缓存，拖动与缩放按防抖节奏写盘，避免拖动时同步 I/O 掉帧。设置页提供本地备份与恢复。

## 已知限制

- 仅支持 Deepin 的 X11 会话；Wayland / Treeland 的 layer-shell 路径尚未实现。
- 全局热键 `Ctrl+Alt+D` 可能被其它程序占用，日志提示后程序继续运行。
- 天气与部分图标依赖本机网络／主题，离线时降级展示。
- QML 资源有编译缓存，改界面后需按上文清理 `dm-ui` 再构建。

## 许可证

`GPL-3.0-only`（见 `Cargo.toml` 的 `workspace.package.license`）。
