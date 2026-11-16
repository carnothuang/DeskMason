# 状态

> 2026-11-15。M0 / M0.5 / M0.6 已完成；M1 文件瓦片日常可用；深色主题与设置窗口 7 页、内容瓦片、搜索窗已完成。

## 本轮（2026-11-15 晚）：三件交互顽疾修复

| 问题 | 根因 | 修复 | 验证 |
|---|---|---|---|
| 拖动一顿一顿 | 拖动中 geometryTimer 每 500ms 触发 `save_placement()`，主线程同步 JSON 读+写盘把渲染卡出帧 | ① `layout_store.rs` 加进程内文档缓存（mtime 校验，load 命中内存、save 写穿）；② geometryTimer 防抖窗口 500→800ms；③ QML 拖动优先 `startSystemMove()`（KWin 拒绝时退回手动） | 三轮连续拖动位置精确累加（3×40,3×20px），无丢步 |
| 点击桌面瓦片全消失 | DDE 桌面被点击时把 dde-shell/desktop 抬到同层 DESKTOP 瓦片之上（窗口仍 IsViewable，只是被盖住） | 新增 `x11::raise_to_desktop_top()`（stack_mode=Above restack）→ C++ 胶水 `dm_raise_tile(widget_id)`（按 widgetId 找窗口）→ 6 瓦片 QML hostTimer（200ms）周期调用 `app.raise_tile()` | xdotool 点桌面 2.5s 后两瓦片完整可见 |
| fcitx5 冲突（需重启输入法才能打中文） | 瓦片是 DESKTOP 层窗口却接受 X 键盘焦点，每次点击都在"桌面层 ↔ 普通窗口"间切换 fcitx5 输入上下文，高频切换后状态错乱 | `dm_spawn_tile`/`dm_pin_root_window` 给窗口加 `Qt::WindowDoesNotAcceptFocus`（WM_HINTS input=False） | xprop 确认 input=False；瓦片内 TextInput 靠 Qt activeFocus 仍可输入（实测字符入框）；fcitx5 状态稳定 |

另：问题 2 的运行时验证还确认了 `＋`→newTileMenu→「待办/随记」创建链路可用（todo-2 瓦片创建、待办添加 1/1 计数正确）。

**质量门**：`cargo fmt` / `cargo clippy --workspace --all-targets`（0 警告）/ `cargo test --workspace`（137 全绿）。

## 本轮（2026-11-15）：深色主题与界面统一

| 项 | 内容 | 验证 |
|---|---|---|
| 深色主题单例 | `Theme.qml` 全量色板（textPrimary/Secondary/Tertiary、panelSubtle、accentSoft 等），瓦片与弹窗统一引用 | 视觉逐项比对 |
| 设置窗口 | 1000×760 无边框窗、顶栏搜索框、可折叠导航（8 项）、卡片流布局；常规/外观/胶囊模式/文件格子/功能格子/快捷与交互/诊断与维护/关于 7 页全部实现，文案统一 | 逐页验证 |
| UI 状态持久化 | `prefs.rs` 键值存储 + `save_ui_state()`，窗口材质/密度/动画等 20+ 项设置落盘 | 重启保持 |
| 搜索窗 Spotlight 化 | 输入即搜（300ms 防抖）、Enter 打开选中项、Up/Down 导航、系统主题图标（bloom，与瓦片一致） | 截图 + xdotool 模拟按键验证 |
| **窗口材质三档** | KWin `_KDE_NET_WM_BLUR_BEHIND` 模糊 + QML 三档透明度：亚克力 0.30（透亮）/ 云母 0.72（暗调，默认）/ 不透明 0.97，搜索窗与设置窗生效 | 三档截图 + std 量化（29.6/9.3/1.3）|
| 内容瓦片配色 | todo/glance/weather/music/quickcapture/search 硬编码色全部 Theme 化；todo 紫色圆形勾选、music 粉色进度条、glance 节日徽章 | 截图验证 |

**构建注意**：QML 是嵌入资源，改 `.qml` 不会触发 `dm-ui` 的 build.rs 重跑，`cargo build`
会拿旧字节码。验证 UI 前先 `cargo clean -p dm-ui` 再 build（或 touch `src/lib.rs`）。
二进制里嵌的是 qmlcachegen 编译后的 UTF-16LE 字节码，可用 `python3` 搜字符串确认版本。

## 已完成

| 项 | 位置 | 验证 |
|---|---|---|
| 技术栈决策 | `docs/01-stack-decision.md` | Rust + Qt Quick/QML + DTK6，cxx-qt 桥接 |
| `dm-core` 领域层 | 瓦片类型、布局、文件排序、瓦片设置 | 30 个测试 |
| `dm-platform` 平台层 | X11、热键、图标、回收站、监听、调起、布局存储、文件移动 | 27 个测试 |
| `dm-ui` 桥接层 | `FileTile` / `AppState` + QML | 真实 DDE 验证 |
| `dm-app` 入口 | 按布局创建多窗口 | 真实 DDE 验证 |

`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`（57 个测试）全绿。

## M1 文件瓦片：能力清单

| 行为 | 状态 |
|---|---|
| 桌面层窗口内渲染 QML | ✅ |
| 文件夹扫描 + 自然排序 + 4 种排序方式 | ✅ |
| XDG 图标主题（支持 bloom 的 `mimetypes/128/` 布局）+ 回退链 | ✅ |
| inotify 自动刷新 | ✅ |
| 单击选中、双击打开、F2 重命名、Delete 到回收站 | ✅ |
| 拖入（文件管理器 → 瓦片，自动处理同名冲突） | ✅ |
| 拖出（瓦片 → 文件管理器，XDND `text/uri-list`） | ✅ |
| **拖到子文件夹上 → 移入该子文件夹** | ✅（根 DropArea 按坐标命中格子） |
| 位置/尺寸持久化 | ✅ |
| **图标大小 / 文件名行数 / 密度 / 排序**（设置菜单，实时生效并落盘） | ✅ |
| **新建瓦片**（`＋` 按钮，多窗口，错位摆放） | ✅ |
| **删除瓦片**（设置菜单 → 确认 → 移出布局并关窗，不动文件夹内容） | ✅ |
| **新建文件夹**（右键菜单 → 输入名称） | ✅ |
| **复制 / 剪切 / 粘贴**（Ctrl+C/X/V 与右键菜单） | ✅ |
| **多选**：Ctrl 点选切换、Shift 连选、Ctrl+A、Esc 清空 | ✅ |
| **框选**：空白处拖矩形、Ctrl 框选累加 | ✅ |
| **选择文件夹**（重映射瓦片目录并持久化） | ✅ |
| **多显示器**：RandR 1.5 拓扑、EDID 身份、越界校正、拔插重吸附 | ✅ |
| **工作区**：扣掉 DDE 任务栏（strut + `_NET_WORKAREA`）后定位 | ✅ |
| **按显示器组合记忆布局**（换屏组合自动存/取） | ✅ |
| **托盘**（SNI：单击显隐、菜单：显隐/新建/退出） | ✅ |
| **全局热键**（默认 Ctrl+Alt+D，可在设置里改） | ✅ |
| **框选自动滚动**（拖到边缘持续滚动并继续选） | ✅ |
| **设置窗口**（热键 + 开机自启，托盘"设置…"打开） | ✅ |
| **多类型窗口**：按瓦片类型加载不同 QML，未知类型给占位 | ✅ |
| **时光瓦片**：时间/日期/星期/农历/节气/节日 | ✅ |
| **天气瓦片**：Open-Meteo + 本地缓存 + 城市切换 | ✅ |
| **待办瓦片**：SQLite 存储、增删改、完成状态、筛选 | ✅ |
| **随记瓦片**：SQLite 存储、记录/置顶/删除、链接识别 | ✅ |
| **搜索窗**：托盘打开、输入即搜（300ms 防抖）、Up/Down 导航、Enter 打开、系统主题图标 | ✅（本地遍历后端） |
| **音乐瓦片**：MPRIS2 读取当前播放器、播放控制按钮 | ✅（读实测，控制在真实播放器上未点） |
| **设置页**：热键、开机自启、新建瓦片默认外观 | ✅ |
| **统一浅色主题**：Fusion + 浅色调色板，深色系统主题下不再白底白字 | ✅ |
| **i18n**：112 条 QML 文案，中英双语，按系统语言加载 | ✅（其它语言待补） |
| **首次引导**：三步说明，只显示一次 | ✅ |
| **本地备份**：zip 打包布局/设置/数据库，可恢复 | ✅ |
| **deb 打包**：`packaging/build-deb.sh` → `deskmason_0.1.0_amd64.deb` | ✅ |
| 框选自动滚动 | ❌ 拖到边缘不会滚动 |

### 多窗口

每个瓦片一个独立的 QML 窗口：C++ 胶水用 `QQmlComponent::createWithInitialProperties`
注入 `widgetId`，Rust 的 `AppState::create_tile()` 负责写布局并触发开窗。
实测：点 `＋` 后出现第二个窗口（`file-2`，错位 32px），重启后两个窗口都按保存的位置恢复。

## 关键实测结论

1. **桌面层窗口在 DDE 上收得到鼠标和键盘**（`_NET_WM_WINDOW_TYPE_DESKTOP` 不会屏蔽输入）。
   验证输入时注意遮挡：DDE 桌面图标由 `dde-shell/desktop` 全屏窗口绘制，
   用户开着浏览器时点击自然落在浏览器上。
2. **`Qt.Desktop` 窗口标志在 QQuickWindow 上无效**，必须手写 X11 原子属性。
3. **cxx-qt 0.10 的属性 setter 名是 `set_<snake_case>`**，自定义 invokable 不能同名，
   否则报 "defined multiple times"。本项目的选择器统一用 `choose_*`。
4. QML 的 `Drag.Automatic` 足以发起符合规范的 XDND 拖出，无需手写协议。
5. **嵌套 `DropArea` 抢不到事件**：根 DropArea 覆盖整块瓦片时，格子里再放 DropArea 收不到
   拖放。改成只在根 DropArea 里用 `grid.indexAt()` 按坐标命中，确定性更好。
6. **Qt 标准按钮默认是英文**：需要启动时加载 `/usr/share/qt6/translations/qtbase_<locale>.qm`，
   否则对话框显示 OK/Cancel。
7. **窗口级 `Shortcut` 会抢走对话框里的按键**：Return/Delete 必须在任意对话框打开时禁用，
   否则在重命名输入框里按 Delete 会弹出删除确认。

## 目录

```
crates/
  dm-core/      瓦片类型、布局、排序、瓦片设置（纯逻辑）
  dm-platform/  平台能力（唯一允许碰 X11/DBus/文件系统副作用的地方）
  dm-ui/        cxx-qt 桥 + QML 资源
  dm-app/       桌面进程入口
  dm-shell/     无界面探针（桌面层 / 热键）
docs/           决策与状态
```

## 剪贴板实现要点

- 写：`text/uri-list` + `x-special/gnome-copied-files`（首行 `copy`/`cut`）。
  文件管理器靠后者区分"复制"与"剪切"，只写 uri-list 会被当成复制。
- 读：优先 `urls()`；只有 gnome 格式时解析其首行后的 URI；再退回纯文本里的
  `file://` 或绝对路径。三种来源都覆盖，才能跟不同文件管理器互通。
- 剪切粘贴成功后清空剪贴板，避免重复粘贴。
- 实测：Ctrl+C 写出两种格式；`xclip` 与自读都能解析；从外部目录剪切粘贴后
  源文件消失、目标出现、剪贴板清空。

## 多选实现要点

- 选择状态在 QML（下标数组 + anchor），批量操作**按 URL/路径传参**而不是下标——
  删除会让后续下标失效，传路径才安全。
- `tile.itemCount` 变化时用 `Connections` 裁掉越界下标。
- 拖出时把 `text/uri-list` 传成**数组**：拼成带换行的单个字符串会被 Qt 当成一个 URL
  （换行被转义成 `%0A`），实测踩过。
- 批量删除的确认框区分单项/多项文案。

## 打包（M5）

- `packaging/build-deb.sh` 构建 release 二进制并打成 deb：
  `/usr/bin/deskmason`、`/usr/share/applications/deskmason.desktop`、
  `/usr/share/icons/hicolor/scalable/apps/deskmason.svg`、copyright。
- 产物 `deskmason_0.1.0_amd64.deb` 约 3.3MB（二进制 11MB），依赖写成 Qt6 运行时 +
  libsqlite3 + libxcb。
- Release 二进制实测可启动：托盘、热键、瓦片窗口、首次引导都正常。
- 尚未做：玲珑包、仓库签名、自动更新检查。

## 首次引导与备份（M4 后半）

- 引导：`settings.json` 里的 `onboarded` 标记；三步（欢迎 / 瓦片用法 / 托盘与快捷键），
  跳过或完成后写入标记，重启不再出现（实测重启后 0 个引导窗口）。
- 备份：`~/.local/share/deskmason/backups/deskmason-<时间戳>.zip`，含
  `widget-layout.json`、`settings.json`、`todo.db`；设置页有「立即备份 / 恢复最近一次」。
- **SQLite 的坑**：应用运行中 `todo.db` 可能只有 0 字节（数据都在 `-wal` 里）。
  直接拷主文件会得到空库。改用 `VACUUM INTO` 生成一致性快照再打包；
  单测模拟"连接保持打开 + WAL 里有数据"，解压后仍能查到那条记录。
- 恢复后提示"请重启 DeskMason 生效"——运行中的连接仍指向旧文件。

## i18n（M4 第二种）

- 源语言是中文；`lupdate`（`/usr/lib/qt6/bin/lupdate`）抽出 101 条 `qsTr` 文案，
  `lrelease` 生成 `translations/deskmason_en_US.qm`，已入库。
- `.qm` 用 `include_bytes!` 编进二进制，启动时按 `LANG`/`LC_ALL` 落盘到
  `~/.cache/deskmason/i18n/`，再用 `QTranslator` 加载。这样不依赖 qrc 构建配置。
- 实测：`LANG=en_US.UTF-8` 启动后，设置菜单、提示、空状态全部为英文
  （Icon size / File names / Density / Sort / Choose folder… / Delete this tile…）。
- **已知限制**：Rust 侧的状态文案（如"3 项"）没有走 Qt 翻译，仍是中文；
  需要时再抽成资源键。

## 设置页与默认外观（M4 第一种）

- 设置存 `~/.config/deskmason/settings.json`，新增 `tileDefaults`
  （图标大小/文件名行数/密度/排序）；旧文件缺这个字段会自动补默认值。
- 新建文件瓦片继承默认外观：`layout_store::new_file_widget` 从宿主设置读取。
  实测：把默认图标改成大后，新建的 `file-4` 写入 `iconSize: 64`，已有的瓦片不变。
- 主题：QML 里逐控件改 palette 不彻底（Window 背景和控件样式仍跟系统走）。
  改成启动时 `QQuickStyle::setStyle("Fusion")` + `QGuiApplication::setPalette` 浅色，
  一次覆盖所有窗口、对话框与菜单。
- ComboBox 的弹出层在无窗口管理器环境下不好用，设置页改用「按钮 + 菜单」，
  与瓦片设置菜单一致。

## 音乐（M3 第二种）

- 用 MPRIS2（D-Bus）而不是自建播放后端：Linux 上播放器、浏览器都会实现它。
- 选播放器的规则：优先 `PlaybackStatus == Playing`，否则取第一个。
- 元数据是 `a{sv}`：标题/专辑/封面取字符串，艺术家可能是数组，长度是 `i64`。
- **长度可能是垃圾值**：Edge 把 `mpris:length` 报成 `i64::MAX`，进度会显示成
  `153722867280:54`。现在超过 24 小时的长度按"未知"处理，只显示当前位置。
- 实测：读到 Microsoft Edge 正在播放的 B 站视频（标题/暂停按钮/`36:07` 位置）。
  播放/暂停/上下首按钮已接线，但没有在用户的真实播放上点击（会打断播放）。

## 搜索（M3 第一种）

- **后端现状**：本地受限遍历（常见目录、深度 6、350ms 时间预算、跳过隐藏与
  `node_modules/target/build` 等噪音目录），结果按"前缀命中优先 → 名字更短 → 路径"排序。
- **deepin-anything 还没接上**：本机只装了 server/index（7.1.1），查询库
  `deepin-anything-libs`/`-dev` 未安装（源里是 6.2.1，与 server 7.1.1 版本不一致）。
  接入点是 `dm-platform::search`，装好后应在这里优先走索引、遍历只作兜底。
- 入口：托盘「搜索…」打开居中搜索窗（Esc 关闭，双击打开结果）；也可作为搜索格子贴在桌面。
- 实测：`notes` → `/tmp/.../Documents/notes.md`；中文文件名有单测覆盖。

## 随记瓦片（M2 第四种）

- 与待办共用一个 SQLite 文件（`todo.db`），独立 `captures` 表。
- 链接自动识别（`http(s)://` 开头）并显示不同图标；列表摘要压平换行并截断。
- 置顶在前，其余按创建时间倒序。
- 实测：输入 `remember this` 回城后落库（`text|remember this|0`），列表与计数同步。

## 待办瓦片（M2 第三种）

- 存储：SQLite（`~/.local/share/deskmason/todo.db`），`tasks` + `steps` 两张表，
  外键级联删除；`PRAGMA user_version` 记录 schema 版本。
- 领域层：筛选（全部/未完成/已完成）、排序（未完成在前 → 到期升序 → 创建倒序）、
  到期文案（今天/明天/后天/逾期 N 天/N 天后/日期）。
- UI：添加、勾选完成、删除、筛选切换；列表按登录时长自动刷新。
- 颜色：桌面瓦片是浅色面板，必须显式固定控件调色板，否则跟随系统深色主题时
  "白底白字"。已给所有窗口设置浅色 palette，并给输入框显式设置 placeholder 颜色。

## 天气瓦片（M2 第二种）

- 数据源 Open-Meteo（无需 API key）：地理编码 + 当前温度/天气代码 + 今日最高最低。
- 网络请求在**后台线程**，QML 每 200ms 轮询结果——UI 不会被 8 秒超时卡住。
- 失败时回退本地缓存（`~/.cache/deskmason/weather-<城市>.json`），并显示"离线：显示上次数据"。
- 城市可改，写入瓦片 `data.city`；改城市后重新地理编码。
- WMO 天气代码映射成中文 + DejaVu 覆盖的符号（☀ ☁ ☂ ❄ ⚡），不依赖彩色 emoji 字体。
- 实测：北京 `10° 阴 4°/11°`、切到上海 `18° 阴 13°/19°`，两个城市各自生成缓存。

### 踩坑：Rust 字符串续行

用 `\` 续行的多行 URL 把缩进空格留在了 URL 里，导致 `http: invalid uri character`。
URL 拼接一律写成单行，不依赖续行规则。

## 时光瓦片（M2 第一种）

- 农历换算用 `chinese-lunisolar-calendar`（chrono 支持），不自己维护年份表；
  闰月、大小月都由库负责。
- 节气用通行近似公式（1900 基准 + 24 个分钟偏移），误差最多一天，测试钉住
  2024/2025 的立春与冬至。
- 节日：公历优先（元旦/劳动节/国庆…），其次农历（春节/元宵/端午/七夕/中秋/重阳/腊八/小年）。
- 实测：真实环境显示 `10:59 / 11月15日 星期日 / 农历十月初七`。

## 多类型窗口

`dm_spawn_tile(engine, widget_id, qml_path)` 由 Rust 决定用哪个 QML：
文件瓦片 → `main.qml`，其余类型 → `placeholder.qml`。新增一种瓦片只需要
「一个 QML + `qml_path_for` 里一行」，宿主不再按类型分支。

`spawn_all_tiles` 现在遍历**所有**瓦片（之前只遍历文件瓦片，其它类型永远不出窗口）。

## 设置与自启

- `~/.config/deskmason/settings.json` 存宿主设置（热键、自启开关）。
- 开机自启写 `~/.config/autostart/deskmason.desktop`，`Exec` 指向当前可执行文件。
- 热键在启动时读取并注册，改完需重启生效；格式非法时在设置窗里明确提示。
- 设置窗口是普通窗口（不贴桌面层），重复打开会前置已有实例。

## 托盘与全局热键

- 托盘用 **ksni**（SNI + dbusmenu 现成实现），手写 D-Bus 签名很容易在不同宿主上
  出现"图标在但菜单点不开"。
- 图标不用主题名，直接生成 22×22 的 ARGB32 位图（深蓝底 + 砖块网格），
  避免依赖某个主题里恰好有合适的图标。
- 全局热键默认 **Ctrl+Alt+D**：X11 `XGrabKey`，在独立线程轮询，只改原子变量。
- 跨线程到 Qt 的桥：托盘/热键只改 `AtomicBool` 与命令队列；每个窗口 200ms 轮询
  一次版本号，自己应用显隐、执行 `new-tile` / `quit`。这样不跨线程碰 QObject。
- **`Qt.quit()` 在本架构下无效**：它发 `QQmlEngine::quit`，而我们是
  `QQmlComponent` 逐窗口创建、没有 `engine.load()` 建立的连接。退出要走
  `QCoreApplication::quit()`（C++ 胶水 `dm_app_quit`）。
- 实测：托盘注册进 `org.kde.StatusNotifierWatcher`；菜单四项（显隐/新建/分隔/退出）
  layout 正确；新建瓦片、隐藏全部、退出都通过 D-Bus 触发验证；热键两次
  按下对应 `IsUnMapped` ↔ `IsViewable`。

## 按显示器组合记忆布局

- 组合键 = 所有显示器 id 排序去重后用 `|` 拼接。同一组屏 → 同一个键，
  顺序无关。
- 磁盘格式升到 **schema v2**：新增 `active_topology` 与 `placements`
  （组合键 → 瓦片 id → 摆放）。v1 布局自动迁移，字段全有默认值。
- 切换组合时先 `remember_placements(旧键)` 存档，再 `apply_topology(新键)`：
  新组合有档就恢复，没有就保持当前位置，随后由 `reconcile_document` 收进屏幕。
- 归属跟着矩形走：用"重叠面积最大的显示器"判定，拖到另一块屏后重启不会再被拽回。
- 实测：真实双屏下 `active_topology` 是两块屏 EDID 的拼接，摆放档正确写入；
  v1 布局迁移后位置/标题/尺寸全部保留。

## 工作区实现要点

- DDE 的 dock 是 `_NET_WM_WINDOW_TYPE_DOCK`，并且**设置了** `_NET_WM_STRUT_PARTIAL`
  （本机是底部 37px、覆盖 x 0..2560，即第二块屏的底部）。
- `_NET_WORKAREA` 只有一个全局矩形（`0,0,3440,3003`），多屏时信息不足；
  所以两者都用：先从 strut 切边，再与全局工作区求交。
- 每条 strut 只切它覆盖的那条边，且只在覆盖到该显示器时才生效。
- 实测：第二块屏（1600 高）工作区变成 1563 高，放在 `y=2900` 的瓦片被收进
  `y=2483`；第一块屏不受影响。

## 多显示器实现要点

- 身份用 **EDID 指纹**（前 16 字节哈希），不用 RandR 输出编号——编号会随重插、
  改主屏、睡眠唤醒变化。
- 用 RandR 1.5 的 `get_monitors`（而不是逐个 output）：它已经是"显示器"粒度，
  多输出合一的屏不会算成两块。
- 启动时和每 5 秒各校正一次：同步 `monitors` 表、把越界矩形收进所属屏幕、
  把"显示器已拔掉"的瓦片迁到重叠面积最大的那块（没有就主屏）。
- 校正逻辑抽成了纯函数 `reconcile_document`，用合成拓扑做测试（Xvfb 不支持
  虚拟多显示器）。
- 实测：本机真实环境有两块屏（3440×1440 + 2560×1600），EDID 身份正确；
  放在第二块屏上的瓦片被识别并停在该屏。

### 框选实现要点

- `GridView` 设 `interactive: false`：桌面上拖拽用于框选，滚动交给滚轮/滚动条。
  否则 Flickable 会把拖拽当成滚动。
- 框选区是 GridView **下方**的 MouseArea（`z: -1`），起始点必须落在空白处；
  从格子按下仍然是选中/拖出，与文件管理器一致。
- 命中计算要把 grid 内坐标加上 `contentX/contentY` 转成内容坐标，再用
  `floor(x / cellWidth)` 求行列；只遍历矩形覆盖的行列，不扫全部条目。

## 下一步

M1（宿主与文件瓦片）到此收尾。剩下的是内容瓦片与外围能力：

1. **M2 内容瓦片**：四种全部完成（时光/天气/待办/随记）。
2. **M3 搜索与媒体**：搜索窗 ✅（本地后端）、音乐 ✅；deepin-anything FFI 待库。
3. **M2 补强**：待办的步骤/备注/到期编辑、随记的图片与 Markdown。
3. **M4 外围**：全局外观设置页、引导、i18n（`qsTr` + `.ts/.qm`）、备份。
4. **M5 打包**：deb / 玲珑、更新检查。

按依赖顺序，下一步应做 **M2 的第一种内容瓦片**：先做「时光」（纯计算、无外部依赖），
再按 待办 → 随记 → 天气 的顺序推进。
3. **全局快捷键唤起与托盘**（M1 宿主能力）。
4. 应用内 i18n：目前 QML 文案硬编码中文，需要走 `qsTr` + `.ts/.qm`。

## 测试环境注意

本机 Xvfb 会话里 **fcitx5 输入法在运行**，`xdotool type` 输入的英文字母会被组词
（例如 `return4` 变成 `然而turn`，`report.md` 变成 `报告-改。m d`）。
自动化输入测试要先确认输入法状态，或只用 ASCII 且接受输入法可能改写。
第一次按 Return 会被输入法用于上屏，第二次才落到对话框——这不是应用缺陷。

## 已知限制

- 图标查找有进程内缓存，主题切换后需重启进程才生效。
- 文件夹选择已从 `QtQuick.Dialogs.FolderDialog` 换成自绘 `FolderPicker.qml`
  （本机 Qt 缺 `Qt.labs.folderlistmodel`，原生实现又依赖 portal / dde-file-dialog）。
  目录浏览由 Rust 侧 `list_directories()` 提供，只列目录、无符号链接循环保护。
- Treeland（Wayland）路径尚未实现，当前只有 X11。
- 调试构建 87MB（含 Qt 动态链接与调试符号），发布态需另测体积与 RSS。

## 晚间第二轮：文件瓦片布局修复 + 瓦片缩放磁吸（任务 #13/#14/#15）

### 文件瓦片布局塌陷（#13，已修复）

症状：420x520 窗口里标题 "Desktop" 垂直居中、图标网格被挤到底部只露一排。
根因：`main.qml` 标题行的 `windowDrag` MouseArea 设了 `Layout.fillHeight: true`，
ColumnLayout 把多余高度全分给标题行 → 标题垂直居中、网格塌底。
修复：删掉 `Layout.fillHeight`（一行）。与 QML 嵌入资源缓存无关（cargo clean
不解决），排查期间曾误判为图标/插件/缓存问题，走了弯路。

### 瓦片缩放 + 边界磁吸 + 对齐间隔（#14/#15，新增能力）

此前瓦片只能移动不能缩放。本轮一并补齐：

- **`TileResizeEdge.qml`**（新）：四边 8px 隐形缩放边条（顶部边条避开
  WindowDragEdge 拖动把手区）。拖动时手动改 window.x/y/width/height
  （桌面层窗口 KWin 不接手 resize）。
- **磁吸**：按下时经新 invokable `neighbor_rects()` 取其他瓦片几何快照；
  拖动路径只做纯计算。同轴邻瓦边界距离 ≤10px 时吸附（"卡一下"的提示），
  迟滞 14px——吸附中继续拉动超出才释放，避免抖动。
- **吸附语义**（对应 #15 对齐间隔）：相对边（我的右边 vs 邻居左边）
  吸附位置留 8px 间隔；同侧边（右 vs 右）精确对齐。
- 6 个瓦片 QML（main/todo/weather/glance/music/quickcapture）均已挂上。
- 邻居快照含一个坑：布局存档里可能有"离屏旧 rect"（上次会话残留），
  会让磁吸目标偏移——实测中出现过 snapped=751 而非 745 的现象，属预期。

验证（xdotool XTEST 模拟）：右边缘拖到 912 → 吸附 908（邻瓦左边界-8），
宽度 636 精确命中；拖出吸附带后宽度跟手（迟滞生效）；垂直方向吸附同样
工作（snapped = 邻瓦顶边-8）。测试时注意被邻瓦覆盖的边条区域
（如 music 瓦片盖住文件瓦片底部边条左段），需在未被覆盖处按下。
cargo clean -p dm-ui（QML 缓存）后重建才生效。clippy 0 警告、138 测试全绿。

## 晚间第三轮：内容瓦片单例 / 历史回看 / 路径选择 / 输入修复（任务 #16～#19）

### #16 内容瓦片单例，且只能从「设置」开启

用户约定：待办、随记、音乐、天气、时光各只允许一个，且只能通过设置页开关；
文件格子的「+」只允许再建一个文件格子。

- **布局文档是唯一真相**：`layout_store::add_widget()` 对非 `File` 类型硬约束
  （已有同种类型直接返回 `None`）；新增 `has_kind()` / `widget_of_kind()`。
- **旧配置收敛**：启动时 `spawn_all_tiles()` 调 `enforce_singletons()`，内部纯逻辑
  `dedupe_singletons()` 每种类型只留第一个、删除重复项并写回；实测启动日志
  `已合并重复的功能瓦片: todo-6`，窗口数 7 → 6。
- **设置页即时联动**：`SettingsPanel::set_tool_enabled(kind_key, enabled)` 开启时
  `add_widget()` + `spawn_tile()`，关闭时 `remove_widget()` + 新增的 C++
  `dm_close_tile()`（close + deleteLater）。prefs 不再存这 5 个开关，改为每次
  从布局读 `has_kind()`。关闭「时光」→ 布局 `glance=[]` 且窗口 1→0，
  再开 → 新建 `glance-7` 窗口 0→1。
- **入口收敛**：托盘去掉按类型子菜单、`host::kind_new_command()` 一律返回
  `"settings"`、所有瓦片的 `new-tile:<kind>` 宿主命令改跳设置页；「+」菜单
  只剩「新建文件格子…」与「更多功能格子…」。
- `save_placement()` 加孤儿瓦片保护，避免设置页关窗后 `onClosing` 把旧几何写回。

### #17 已完成待办的历史回看

- `TodoContent` 新增 `doneCount`（统计全部已完成任务，不受当前筛选影响）。
- 待办瓦片标题行加「已完成 N / 返回未完成」按钮，切到 `choose_filter("done")`
  后列表显示带删除线的已完成任务，标题变为「待办 · 已完成」。
- 文件格子标题行加 `◷` 按钮 → `historyDialog`：独立的 `TodoContent { id: history }`
  在 `Component.onCompleted` 固定 `choose_filter("done")`，列出已完成任务、每项
  提供「恢复」（`toggle_task`），底部显示「共 N 项已完成」。
- 实测：对话框显示「测试待办项 / 恢复 / 共 1 项已完成」，待办瓦片显示「已完成 1」。

### #18 新建文件格子必须显式选择文件夹路径

- 删除 `AppState::create_tile()` / `create_tile_of_kind()`，唯一入口改为
  `create_file_tile(folder_path)`：`url_to_path()` 兼容普通路径与 `file://`，
  `is_dir()` 校验失败则不创建任何瓦片；成功才 `add_file_widget()` + `spawn_tile()`。
- **环境坑**：本机 Qt 缺 `Qt.labs.folderlistmodel`，`QtQuick.Dialogs.FolderDialog`
  加载直接失败；改走原生实现又依赖 portal / dde-file-dialog，实测只出现 1x1
  helper 窗口。最终自绘 `FolderPicker.qml` + Rust 目录浏览接口
  `home_folder()` / `list_directories()` / `is_directory()`，外面包一层
  `NewFileTileDialog.qml` 供所有瓦片共用。
- 实测：弹出后默认 `/home/can`、子目录列表正确，进入 `/home/can/Documents` 后
  选择，布局新增 `file-7 → /home/can/Documents/Qoder`，而不是当前格子的
  `/home/can/Desktop`。测试产生的 `file-7` 已清理。

### #19 待办 / 随记无法输入中英文

- 根因：上一轮为规避 fcitx5 冲突给**所有**瓦片设了
  `Qt::WindowDoesNotAcceptFocus`（`WM_HINTS input=False`）。输入框虽有 Qt
  `activeFocus`，X 输入焦点仍在别的应用，键盘事件进不来。
- 修复：`dm_spawn_tile(..., bool accepts_focus)` 只给 `Todo` / `QuickCapture`
  放行焦点，其余瓦片仍 `input=False`；QML 输入框 `onActiveFocusChanged` 调新
  invokable `app.focus_tile()` → C++ `dm_focus_tile()`（`requestActivate()` +
  `dm_focus_window_x11(winId)`）→ `x11::focus_window()`
  （`set_input_focus(POINTER_ROOT, window_id, CURRENT_TIME)`）。
- `quickcapture.qml` 的 `commit()` 顺手把 `input.forceActiveKeyFocus ? ...`
  改回 `input.forceActiveFocus()`。
- 实测：`xprop` 待办 `input=True`、文件瓦片仍 `input=False`（fcitx5 冲突不回归）；
  点击待办/随记输入框后 X focus 分别为 `DeskMason 待办` / `DeskMason 随记`，
  可输入 `todo` / `note`；`xdotool type "hello123"` 被 fcitx 组词成「合理咯23」，
  反证中文输入路径已打通。

### 验证与质量门

- 运行时：6 个窗口整齐布局（文件 420x520+272+390、待办 +716+390、
  随记 428x480+1160+390、天气 420x181+272+100、时光 420x260+716+100、
  音乐 420x186+1160+100）；测试污染的 `placements`、离屏 rect 与临时待办项已清理，
  只保留用户已有的「测试待办项」。
- `cargo fmt --all` 通过；`cargo clippy --all-targets` 0 警告；`cargo test`
  共 **139 passed / 0 failed**（52 + 85 + 2）。
- 18 个文件（16 改 + 2 新 QML）本地提交，**未 push**、无远端。

## 晚间第四轮：八项交互打磨 + 瓦片外观统一（任务 #20～#28）

| # | 用户反馈 | 处理 |
|---|---|---|
| 20 | 待办「O」不明显、「已完成 N」与筛选菜单重复 | 删掉「已完成 N / 返回未完成」；`filterButton.glyph` 改 `⚙`，tip 写明三档筛选 |
| 21 | 文件格子出现「时间」按钮，里面却是已完成的待办 | 删除 `main.qml` 的 `TodoContent { id: history }`、`◷` 按钮与 `historyDialog`；已完成待办统一由待办瓦片筛选菜单回看 |
| 22 | 横纵间隔太小、要 5 倍，叠一起还闪烁 | `MIN_TILE_GAP` / `snapGap` / `minGap` 统一 40px；默认落位按 40px 网格；`enforce_min_gaps()` 启动时把窄间隔推开；`separate_overlapping()` 拆重叠；C++ `dm_raise_all_tiles()` 400ms 节流、按 widgetId 排序统一 restack |
| 23 | 双屏时瓦片只能在主屏 | `primary_work_area()` / `clamp_rect_to()` / `clamp_rect()` / `default_tile_rect_in()` / `enforce_primary_screen()`；QML 每 200ms 与拖动/缩放路径都 clamp；C++ `setScreen(primaryScreen())` |
| 24 | 时光可加背景，默认当前桌面壁纸，可自定义且不影响文字 | 新增 `wallpaper.rs` 解析 DDE `wallpaperInfo`（带 `/host` 兜底）；`glance.qml` 背景图 + `#99000000` 遮罩 + 白字/描边，小标签改近乎不透明深色底片；设置 → 外观可开/关、自定义或恢复桌面壁纸 |
| 25 | 天气设置城市无法输入中英文 | 输入瓦片恢复原生激活焦点；`cityField.onActiveFocusChanged` 兜底 `app.focus_tile()` |
| 26 | 随记要能点击看全文 | `CaptureContent.contents` 暴露完整正文；行点击打开详情 `Dialog`（`TextArea` + 复制全文） |
| 27 | 待办要颜色设定 + 单条全文 | `TodoStore::set_color()` + `normalize_color()`（只收 `#RRGGBB`）；`TodoContent.notes/colors/stepTexts` + `set_task_color()`；详情对话框显示标题/到期/备注/步骤 + 6 色色板（×清除） |
| 28 | 待办、随记四角黑边、不透明、与其它瓦片不一致 | 见下方「外观统一」 |

### #28 外观统一：根因与最终方案（本轮最关键）

- **根因**：上一轮只给待办/随记/天气放行 `accepts_focus`，其余三块设
  `Qt::WindowDoesNotAcceptFocus`。Qt 在 `winId()` 之后调用 `setFlag` 会重写
  `_NET_WM_WINDOW_TYPE`：放行焦点的窗口是 `_NET_WM_WINDOW_TYPE_DESKTOP`，
  不抢焦点的是 `_KDE_NET_WM_WINDOW_TYPE_OVERRIDE + _NET_WM_WINDOW_TYPE_NORMAL`。
  KWin 对两者合成本不同，前者表现为四角黑色直角、整块面板不透明。
- **关键实验**：对已映射窗口用 `xprop` 把类型从 `OVERRIDE+NORMAL` 改成 `DESKTOP`，
  改前改后截图 **md5 完全相同** —— 窗管不会因为运行时改类型就重做透明合成，
  所以「映射后改写窗口类型」不能当修复手段，外观由 **MapRequest 时**的类型决定。
- **最终方案**：`dm_spawn_tile(..., bool accepts_focus)` 在**映射前**给所有瓦片统一
  `WindowDoesNotAcceptFocus`（保证 KWin 记住正确类型）→ `setVisible(true)` →
  仅对 `Todo / QuickCapture / Weather` 调用 `setFlag(..., false)` 放开原生激活焦点
  → `dm_apply_tile_window_type()` 把 EWMH 类型写回统一值。纯展示瓦片始终
  `input=False`，fcitx5 上下文不会被无谓切换。
- **兜底**：C++ `TileFocusFilter` 按窗口安装（不是装到 `QCoreApplication`），
  输入瓦片 `QEvent::MouseButtonPress` 时显式 `dm_focus_window_x11(winId())`。
  早期尝试把过滤器装在 app 上是无效的（Qt 只把事件过滤器作用于 receiver 本身）。
- 实测 `xprop`：6 块瓦片类型全部为
  `_KDE_NET_WM_WINDOW_TYPE_OVERRIDE, _NET_WM_WINDOW_TYPE_NORMAL`；
  `WM_HINTS input=True` 仅待办/随记/天气，其余 `False`。

### 验证与质量门

- 截图 `/tmp/shots/final-gaps.png`：6 块瓦片两行整齐、半透明圆角一致、四角无黑边；
  时光瓦片壁纸背景与大号时间/日期/农历均清晰（画面里两个橙色小点是壁纸本身的眼睛）。
- 强制间隔生效：启动日志 `已把瓦片收回主屏: weather-3, glance-7, music-5`；
  第二行统一 y=2020，横纵相邻间隔 40/40/40/44/40/80px，全部 ≥40。
- 输入：点击随记/待办/天气后 X 焦点分别为 `DeskMason 随记 / 待办 / 天气`；
  `xdotool key --window` 输入 `hello` / `world` / `shanghai` 均入框，fcitx5
  候选栏分别出现「合理咯」「我热烈的」「上海」，中英文路径都通。
- 详情：待办详情显示完整标题/无到期时间/备注/「× + 6 色」色板；
  随记全文显示完整正文与「复制全文」。QML 无加载错误。
- `cargo fmt --all` 通过；`cargo clippy --all-targets` **0 警告**；
  `cargo test --workspace` **149 passed / 0 failed**（52 + 95 + 2）。
- 已知非阻断：热键 `Ctrl+Alt+D` 被其它程序占用，日志提示后继续运行；
  `FolderPicker` 无符号链接循环保护；Wayland/Treeland 路径未实现。
- 本地提交 `676d9e3`（17 文件 +1362/−185），**未 push**、无远端。

## 晚间第五轮：瓦片层级归位 + 全局「松手留缝」（任务 #29 / #30）

### 需求

1. 待办、随记、天气、时光等瓦片会压住桌面上的其它窗口，不应处于最顶层；
2. 设定一条全局规则：下方瓦片顶部贴住上方瓦片底部时，松手后下移 10px，
   左右同理；贴住主屏上/下/左/右边缘同理，避免贴死后抓不住边条再次拖动。

### 层级：从全局置顶改为「桌面之上、普通窗口之下」

- **根因**：`hostTimer` 每 200ms 调 `dm_raise_all_tiles()`，X11
  `StackMode::ABOVE` 把 6 块瓦片抬到全局栈顶。`_NET_CLIENT_LIST_STACKING`
  实测底→顶为「桌面 → 文件/时光 → 文本编辑器 → WorkBuddy →
  待办/天气/随记/音乐」，后 4 块确实压住普通窗口。
- **三个不能用的做法**：
  - 全局 `StackMode::BELOW`：瓦片会掉到 KWin 桌面 frame 之下而消失；
  - 直接写 `_NET_WM_STATE`：EWMH 规定已映射窗口必须发 client message，
    窗管不会因属性变化重新分层；
  - 拿 Qt 的 `winId()`（client）做 `sibling` restack：KWin 已把瓦片重父级到
    frame，sibling 不同父会 BadMatch。
- **最终方案** `x11::keep_tile_below()`：
  1) 写 `_NET_WM_STATE`（SKIP_TASKBAR / SKIP_PAGER / BELOW）；
  2) 发 `_NET_WM_STATE` client message（`_NET_WM_STATE_ADD` + BELOW）给 root；
  3) 找桌面 frame：从 root children 向下两层找带
     `_NET_WM_WINDOW_TYPE_DESKTOP` 的 client，取 stacking 最高的那个并缓存；
  4) 把瓦片 **frame**（从 client 向上爬到 root 的直接子窗口）用
     `sibling(桌面 frame) + StackMode::ABOVE` 定向 restack。
  C++ 侧 `dm_keep_tiles_below()` 节流 1000ms 按 widgetId 排序执行，
  QML 方法由 `raise_all()` 改名 `keep_below()`，语义与实现一致。
- 点击桌面空白处后 stacking 顺序不变，瓦片既不消失也不压窗口。

### 全局留缝：RELEASE_GAP = 10

- 唯一真相 `layout_store::RELEASE_GAP = 10`，通过 `AppState::release_gap()`
  暴露给 QML；`MIN_TILE_GAP = 40` 仍是新建默认落位 / resize 磁吸 / 重叠重排
  的目标间隔。
- `settle_rect_in()` 统一处理八种接触：我的顶/底边贴邻瓦底/顶边 → 下移/上移；
  我的左/右边贴邻瓦右/左边 → 右移/左移；贴主屏上/下/左/右边 → 向内退 10px。
  已拉开距离（含 40px 对齐间隔）的位置完全不动。
- 系统拖动由 KWin 直接改窗口位置，QML 的磁吸 / `minGap` 都拦不住贴死，
  所以统一在 `AppState::save_geometry()` 落盘时结算，再由 C++
  `dm_set_tile_geometry()` 回写 Qt 窗口（只改布局档不回写是无效的）。
  触发点：`WindowDragEdge.onReleased` / `TileResizeEdge.endDrag()` 即时结算，
  800ms `geometryTimer` 兜底系统拖动（此时 QML 收不到 `onReleased`）。
- `enforce_min_gaps()` → `enforce_release_gaps()`：启动整理只保证 ≥10px，
  不再把用户主动贴住后留下的 10px 缝抹回 40px；40px 规则仍在默认/磁吸生效。
- 单测新增 4 个：下/右接触、上/左反向接触、屏幕四边、已分离位置不动；
  `tight_gaps_are_pushed_to_release_gap` 覆盖 2px/5px → 10px 与 40px 不被改。

### 验证

- `xprop -root _NET_CLIENT_LIST_STACKING`（底→顶）：
  `桌面×2 → 文件/待办/天气/时光/音乐/随记 → 文本编辑器/Cindy/rime/WorkBuddy`；
  激活文本编辑器后截图显示它**盖住**随记瓦片（修复前相反）。
- 留缝实测（窗口几何与布局档同步）：
  时光贴文件底 `y=1980 → 1990`；待办贴文件右 `x=495 → 505`；
  天气贴屏幕右上 `(2195,1440) → (2185,1450)`；
  音乐贴屏幕左下 `(55,2854) → (65,2807)`（KWin 工作区已把底边推开 47px，
  左边缘仍按规则 +10）。
- 输入无回归：点击后 X 焦点分别为 `DeskMason 随记 / 待办 / 天气`；
  `hello` / `world` 入框、fcitx 候选「合理咯」「我热烈的」；
  天气「设置城市」对话框正常打开且「福州」被选中。
- 质量门：`cargo fmt --all -- --check` 通过、`cargo clippy --all-targets`
  **0 警告**、`cargo test --workspace` **153 passed / 0 failed**（52 + 99 + 2）、
  `cargo build -p dm-app` 成功；应用日志除热键占用外无 QML / X11 错误。
- 测试后已恢复布局并重启，6 块瓦片回到 40px 网格原位。
- 本地提交 `76c5535`（12 文件 +440/−81，代码）+ 一次独立的状态文档补记，
  身份均为 `can <can@local>`，**未 push**、无远端。

## 晚间第六轮：外部引用清理与文档重塑（任务 #31～#33）

### 背景

最初需求是给 DeskMason 编写正式说明文档、配置 GitHub 地址并推送（且不得推送
与外部参考项目相关的代码）。随后明确改为**暂停推送**，并要求移除与参考项目
有关的全部说明——本项目参考过它的交互思路，但没有引用它的任何代码或资产，
未来发展方向也完全不同；同时删除文件夹中与 DeskMason 无关的文件和说明。

### 清理内容（提交 `b4dca51`，21 文件 +156/−259）

- **文案替换 18 处**：QML 注释、UI 提示、文档中的外部项目参照措辞
  （「1:1 复刻」「源仓库」等），全部改为 DeskMason 自身描述；同步清理 7 个
  `target/cxxqt/**` 构建缓存里的 QML 副本。
- **文档重塑**：`README.md` 重写为正式说明文档（功能表、交互约定、技术栈、
  构建运行、数据路径、已知限制）；`AGENTS.md` 重写为不含外部参照的开发约定；
  `docs/01-stack-decision.md`、`docs/03-material-plan.md` 重写为 DeskMason
  自身的技术与材料说明；删除 `docs/02-parity-matrix.md`（对照矩阵已无意义）。
- **无关文件移出版本控制**：`.cargo/config.toml` 与 `dev-env.sh` 是当前开发机
  的沙盒桥接配置（含本机绝对路径），`git rm --cached` 移出跟踪，磁盘保留
  以免破坏本机构建；`.gitignore` 增加对应条目并在 README 注明。

### 验证

- 外部项目名称全文扫描零残留（唯一命中是 `http.rs` 测试 URL
  `127.0.0.1:1`，属误报）；`docs/`、`translations/`、`packaging/` 全文扫描
  外部参照措辞均无命中。
- 提交历史署名核对：全部为本机用户身份（`can` / `carnothuang`），无陌生署名。
- 质量门：`cargo fmt --all`、`cargo clippy --all-targets` **0 警告**、
  `cargo test --workspace` **153 passed / 0 failed**（52 + 99 + 2）、
  `cargo clean -p dm-ui && cargo build -p dm-app` 成功。
- 应用未启动验证（遵守用户取消后台启动任务的指令，不重启）。

### 状态

- 清理完成前**无 remote、未 push**（此前遵守暂停推送指令）。已向用户说明：
  历史提交信息中含外部项目名称引用；随后用户决定以当前代码状态重建历史
  （单一初始提交）再推送，见下一节。

## 晚间第七轮：历史重建与首次推送 GitHub

### 需求

移除历史提交信息（旧历史含外部项目名称引用），以当前代码状态作为第一个
提交，推送到 GitHub `carnothuang/DeskMason`。

### 做法

- 旧 11 个提交先备份到本地 ref `refs/backup/pre-squash-main`
  （`f852d95`，仅本地存在，推送验证后删除）。
- `git checkout --orphan new-main` + `git add -A` 生成单一初始提交
  「DeskMason：Deepin 桌面瓦片应用」，身份 `can <can@local>`。
- 状态文档中第六轮记录里引用的外部项目名称一并改写（本节也是重建前的
  最后一次修订）。
- `docs/00-status.md` 记录的是开发过程史，含旧提交号属预期；对外推送的
  说明入口 `README.md` 自始不含任何外部参照。

### 验证

- 新历史外部项目名称扫描零命中；orphan 树 77 文件与旧 HEAD 完全一致。
- 推送前远端确认：空仓库、无默认分支、SSH 认证正常。

## 深夜第八轮：任务栏彻底消失、输入框恢复可输入、退出与字体两处回归（v0.1.5 / v0.1.6）

### 背景

用户两次反馈同一件事并附截图：瓦片会出现在 dde-dock 任务栏里，且能从任务栏关掉
（"这是绝对禁止的"），随后又追加"更多瓦片被放到任务栏了……全局只允许托盘图标
存在"。修完任务栏后，又报了 `设置页面成了乱码`、`退出 DeskMason 无法正确退出`。

### 任务栏图标：`shouldSkip()` 的真规则

反汇编 `dde-shell/org.deepin.ds.dock.taskmanager.so` 的 `dock::X11Window::shouldSkip()`
（`0x1027c0`）得到两条判定，且**结果在窗口首次出现时就被 `pthread_once` 缓存**：

1. 有 `_NET_WM_STATE_SKIP_TASKBAR` → 跳过；
2. 否则拿 `_NET_WM_WINDOW_TYPE` 与白名单比对：
   `DIALOG / UTILITY / COMBO / DESKTOP / DND / DOCK / DROPDOWN_MENU / MENU /
   NOTIFICATION / POPUP_MENU / SPLASH / TOOLBAR / TOOLTIP`。

我们此前写的是 `_KDE_NET_WM_WINDOW_TYPE_OVERRIDE + _NET_WM_WINDOW_TYPE_NORMAL`，
两个**都不在白名单里**，只靠 `SKIP_TASKBAR` 一条腿走路；而 `keep_tile_below()`
每 1s 又把 TYPE 写回 `OVERRIDE+NORMAL`，等于周期性把瓦片送出白名单，Qt 在
`show()` 时的一次异步重写就足以让 dock 在采样瞬间看到"普通窗口"。

- 瓦片窗口标志改为 `Qt::Tool | Qt::FramelessWindowHint`：Qt 自己就写
  `_NET_WM_WINDOW_TYPE_UTILITY`（白名单类型），dock 的判定不再依赖运气。
- 新增 `x11::apply_tile_layer()` 作为**映射前**的唯一属性写入点：
  TYPE=UTILITY、STATE=`SKIP_TASKBAR/SKIP_PAGER/BELOW`、MOTIF=`[3,33]`（禁关闭）、
  `_NET_WM_DESKTOP=0xFFFFFFFF`，一次写全。
- 删掉映射后所有 `setFlag()`（原先映射后切 `WindowDoesNotAcceptFocus` 会触发
  Qt 异步重写 `_NET_WM_STATE`，正好抹掉 `SKIP_TASKBAR`）。

### 输入框点了不能打字

`WindowDoesNotAcceptFocus` 让 Qt 写出 `WM_HINTS.input = False`，KWin 因此拒绝在
点击时把键盘焦点交给瓦片（`xdotool windowfocus` 能强制聚焦，证明通道没坏、卡在
WM_HINTS）。改为：纯展示瓦片照旧 `input=False`，输入型瓦片（待办/随记/天气）
写 `WM_HINTS.input=True`，由 `apply_tile_layer(…, accepts_input)` 在映射前写死。

### 退出没反应：命令队列被"抢吃"

托盘动作原先统一入队，8 个瓦片窗口每 2s 各弹一条（`VecDeque::pop_front`），
谁先轮询到就从队列里吃掉；窗口只处理自己认识的分支，于是
`quit / settings / search` 有机会被一个不认识它的窗口吞掉——退出十次里九次无效。

- 退出 / 打开设置 / 打开搜索改为 **Rust 直接调用**，C++ 侧用
  `QMetaObject::invokeMethod(qApp, …, Qt::QueuedConnection)` 排到 GUI 线程
  （`QQmlComponent` 只能在 GUI 线程用，托盘线程直接建窗会崩）。
- 队列只留 `new-tile` 这类"所有窗口分支一致"的请求，抢吃不再有后果。
- 实测发现 `QCoreApplication::quit()` 在本进程里**并不能让 `exec()` 返回**
  （事件循环照常派发事件），因此退出实现为：先礼貌 `quit()`，600ms 未退则
  `std::_Exit(0)`。布局/待办/随记/偏好都是变更即时落盘，无丢数据风险。

### 设置页"乱码"：其实是方框字体

截图放大后确认是**空心方框**（缺字形），不是编码错乱。成因链：

- Qt Quick Controls 的文字默认跟 **DDE 主题字体族**（本机为 `Source Han Sans SC`，
  即 `SourceHanSansSC-Regular.otf`，CFF/OTF 轮廓）走；
- 该字体在这台机器上会**偶发**加载失败，Qt 退化成 `QFontEngineBox`——每个字符
  都画同样宽度的方框，且 `QFontMetrics::inFont()` 仍返回 true（探测不到）；
- 纯 `Text`（瓦片）走的是另一条字体解析，所以同一进程里瓦片正常、设置页全框。

处理：

- 启动时 `dm_apply_ui_font()` 逐个候选族实测：既要 `inFont('中')`，又要
  `horizontalAdvance('i') != horizontalAdvance('W')`（真实字体必然不同宽，方框
  引擎恒等宽）——两条都过才采用。
- 候选顺序里 TrueType 轮廓（`Noto Sans CJK SC` / `Noto Sans SC`）优先，CFF/OTF
  的思源黑体靠后。
- 设置 / 搜索窗口在 QML 里显式 `font.family: app.ui_font_family()`，不让 Controls
  再去继承主题字体。

### 验证

- 6 块瓦片属性稳态：`TYPE=UTILITY`、`STATE=SKIP_TASKBAR,SKIP_PAGER,BELOW`、
  `MOTIF=0x3,0x21`、`depth=32`（透明与圆角正常）；`WM_HINTS.input` 仅
  待办/随记/天气为 True。
- 点击待办瓦片后 X 焦点转移到 `DeskMason 待办`，`xdotool type` 文字确实入框。
- 任务栏 A/B 严格比对（有/无 DeskMason）：图标簇像素级零差异，13 个图标一致，
  无 DeskMason 图标；托盘 SNI 有 `org.kde.StatusNotifierItem-<pid>-1`。
- `wmctrl -i -c` 礼貌关闭被拒（窗口数 8 → 8）。
- 设置页截图：中文、英文、图标全部正常；日志无 QML 报错。
- 托盘"退出 DeskMason"：进程退出、残留窗口 0。
- 质量门：`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets
  -- -D warnings` 零告警、`cargo test --workspace` **159 passed / 0 failed**。

### 环境残留清理

- `~/.config/autostart/deskmason.desktop` 指向已卸载的 `/usr/bin/deskmason`，
  已移入回收站（备份 `/tmp/dm-cleanup-backup/deskmason.desktop.bak`）。
- 无 systemd 用户单元；`~/.config/deskmason`、`~/.local/share/deskmason` 是用户
  数据（布局/待办/偏好），保留。
