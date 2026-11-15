# 材质：KWin 模糊与三档背景（已实现）

## 设计

- 亚克力 = 模糊 + 高透明度，壁纸细节透出
- 云母 = 模糊 + 低透明度，只透色不透光（底色更不透明）
- 不透明 = 纯色面板

## 实现（2026-11-15 完成）

1. **KWin 模糊**：`dm-platform/x11.rs::apply_blur_behind()` 给窗口挂
   `_KDE_NET_WM_BLUR_BEHIND=CARDINAL 1`。本机 KWin `activeEffects` 确认 blur
   特效在跑，atom 设置后壁纸即被高斯模糊。
2. **胶水**：`dm-ui/src/lib.rs::dm_apply_window_blur()`（读 prefs，opaque 档跳过）；
   `window_glue.cpp` 在搜索窗 / 设置窗 `setVisible` 前调用。
3. **QML 三档背景**（search.qml，SettingsPanel 实例只读 prefs 无副作用）：
   - acrylic: alpha 0.30（壁纸清晰透出）
   - mica:    alpha 0.72（默认档，透色不透光）
   - opaque:  alpha 0.97（纯色面板）

## 实测（同一壁纸，弹窗中部纯壁纸透出区，亮度 mean/std）

| 档位      | mean | std  | 观感          |
| ------- | ---- | ---- | ----------- |
| acrylic | 51   | 29.6 | 壁纸细节透出、模糊纹理 |
| mica    | 42   | 9.3  | 暗调、只透色      |
| opaque  | 39   | 1.3  | 纯色          |

三档的亮度与离散度梯度符合预期；绝对值会随所用壁纸变化。

## 注意

- 改材质后需重启应用（blur atom 在映射前设置，QML 背景色在窗口创建时读取）。
- xdotool 测试前先 pkill fcitx5（preedit 拦截按键）。
- 改 QML 后必须 `cargo clean -p dm-ui` 再 build。
