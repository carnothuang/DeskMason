pragma Singleton
import QtQuick

// DeskMason 深色主题。
// 所有瓦片与窗口从这里取颜色，避免散落的魔法色值。
QtObject {
    id: theme

    // ---------- 面板 ----------
    // 瓦片面板：深色半透明（亚克力／云母暗色形态）。
    readonly property color panel: "#D9202022"        // 主面板底色
    readonly property color panelSolid: "#202022"     // 不透明场景（菜单、弹层）
    readonly property color panelSubtle: "#262628"    // 输入框、次级面板
    readonly property color panelHover: "#2E2E30"     // 悬停
    readonly property color panelActive: "#3A3A3C"    // 按下

    // ---------- 边框 ----------
    readonly property color border: "#24FFFFFF"       // 14% 白
    readonly property color borderStrong: "#3DFFFFFF"
    readonly property color accentBorder: "#3477E8"

    // ---------- 文字 ----------
    readonly property color textPrimary: "#F2F2F2"
    readonly property color textSecondary: "#B8B8BC"
    readonly property color textTertiary: "#8A8A8E"
    readonly property color textDisabled: "#5C5C60"

    // ---------- 强调 ----------
    readonly property color accent: "#3477E8"         // 蓝色强调（选中/链接）
    readonly property color accentSoft: "#333477E8"   // 选中背景
    readonly property color accentPurple: "#9B6BE8"   // 开关（紫色 toggle）
    readonly property color danger: "#E34D4D"
    readonly property color success: "#4CAF6D"
    readonly property color warning: "#F2C94C"
    readonly property color dueText: "#F0A35C"        // 到期日橙色

    // ---------- 搜索弹窗 ----------
    readonly property color spotlightBg: "#F0262628"  // 搜索弹窗半透明深色面板

    // ---------- 圆角 ----------
    readonly property int radiusTile: 14              // 瓦片外框
    readonly property int radiusCard: 10              // 设置卡
    readonly property int radiusControl: 8            // 按钮/输入框
    readonly property int radiusItem: 6               // 列表项

    // ---------- 尺寸 ----------
    readonly property int headerHeight: 40
    readonly property int statusHeight: 20
}
