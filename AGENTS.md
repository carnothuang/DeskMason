# DeskMason 开发约定

## 硬约束

- 只支持 Deepin（DDE / KWin / Treeland）。不为 GNOME、KDE、XFCE 做适配，
  不为它们的差异增加开关。遇到桌面环境差异时，以 Deepin 行为为准。
- 业务与平台层用 Rust，界面用 Qt Quick / QML。
- 功能按 Linux 桌面重新设计，不做裁减；数据格式变化必须提供迁移路径。
- 本地优先：布局、设置、待办与随记全部留在本机，不引入账号与云同步。

## 工程规则

- Rust 2021，`cargo fmt` / `cargo clippy -D warnings` / `cargo test` 必须通过。
- 平台相关代码只能出现在 `dm-platform`。其他 crate 不得直接 `use x11rb`、
  调 D-Bus、读 `/proc`。这条是硬边界，靠 `dm-platform` 的 trait 隔离。
- 一个瓦片类型 = 一个 `dm-core` 模块 + 一个 QML 组件 + 一条注册项。
  新增瓦片类型不得修改宿主的分支逻辑。
- 数据格式双写不兼容时，必须带 `schemaVersion` 与迁移函数。
- 文案走统一资源表，不散落在 QML 里。

## 构建与运行

```bash
cargo build
cargo test
cargo run -p dm-shell -- --probe-desktop-layer   # X11 桌面层探针
```

QML 部分需要 `qt6-declarative-dev` 与 `libdtk6declarative-dev`。QML 是嵌入资源，
改 `.qml` 后必须 `cargo clean -p dm-ui` 再 build，否则会加载旧字节码。

## 提交

只使用用户身份，不加任何助手/工具署名。
