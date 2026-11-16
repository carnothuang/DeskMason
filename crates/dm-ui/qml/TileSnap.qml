import QtQuick

// 瓦片拖动/缩放共用的磁吸引擎。
//
// 所有瓦片的几何规则只有一个来源，拖动与缩放不得各写一份：
// - 留缝：所有"对齐留缝"都用 release_gap()（唯一来源 layout_store::RELEASE_GAP，
//   与松手结算一致）——吸附位置就是松手后稳定存在的位置，视觉上"边对齐"。
// - 相对边（我的右边 vs 邻居左边）→ 吸到 邻边 - gap 的位置，留出可操作缝；
// - 同侧边（我的右边 vs 邻居右边）→ 精确对齐（列/行边界对齐）；
// - 主屏四边 → 吸到 屏边 + gap 的位置（与松手结算的退缝位置一致）；
// - 迟滞：吸附中超出 unsnapDist 才解除，避免在阈值附近抖动。
//
// 邻居几何在按下时取一次快照（appState.neighbor_rects），拖动路径上不做
// 任何磁盘/跨窗口查询。桌面层窗口 KWin 不接手 move/resize，全部手动改
// window.x/y/width/height。
QtObject {
    id: root

    // AppState 实例（neighbor_rects / release_gap / clamp_rect 所在对象）。
    property var appState
    // 吸附迟滞带：吸附状态下指针偏离吸附位超过它才解除吸附。
    readonly property int unsnapDist: 14

    // 运行时留缝（像素）：唯一来源 layout_store::RELEASE_GAP。
    readonly property int gap: (appState && appState.release_gap)
                               ? appState.release_gap() : 10

    // 邻瓦几何快照：[{x, y, w, h}, ...]
    property var neighbors: []
    // 屏幕归属探测坐标：h 轴吸附用 vProbe（瓦片纵向中心），v 轴吸附用
    // hProbe（瓦片横向中心）。由调用方在按下时更新（beginDrag 的入参）。
    property real hProbe: 0
    property real vProbe: 0
    // 吸附基准值（全局坐标）；-1 表示该轴该边未吸附。
    // 拖动时窗口四条边都可能吸附，四条边各持一个迟滞状态才不会串台。
    property real snapLeft: -1
    property real snapRight: -1
    property real snapTop: -1
    property real snapBottom: -1

    function refreshNeighbors() {
        var list = []
        if (appState && appState.neighbor_rects) {
            var raw = appState.neighbor_rects()
            for (var i = 0; i + 3 < raw.length; i += 4) {
                list.push({
                    x: parseFloat(raw[i]),
                    y: parseFloat(raw[i + 1]),
                    w: parseFloat(raw[i + 2]),
                    h: parseFloat(raw[i + 3])
                })
            }
        }
        root.neighbors = list
    }

    // 未吸附时找最近候选边；吸附中由 applySnap 处理迟滞。
    // axis: "h"（左右边，吸附值是 x）| "v"（上下边，吸附值是 y）。
    // side: 拖动的边。"right" 的候选是邻居左边与右边；其余类推。
    // screens 是全部屏幕可用区 [{x, y, w, h}, ...]；空数组时不算屏幕边。
    // 屏幕边候选按"矩形中心所在的屏"取——瓦片停在哪块屏，就吸哪块屏
    // 的边（多屏布局合法，不允许把拖到副屏的瓦片往主屏拽）。
    function screenForCenter(screens, cx, cy) {
        if (!screens || screens.length === 0)
            return null
        // 先找包含中心的屏。
        for (var i = 0; i < screens.length; ++i) {
            var s = screens[i]
            if (cx >= s.x && cx < s.x + s.w && cy >= s.y && cy < s.y + s.h)
                return s
        }
        // 都不包含时取中心最近的一块。
        var best = null
        var bestDist = Infinity
        for (var j = 0; j < screens.length; ++j) {
            var sc = screens[j]
            var dx = sc.x + sc.w / 2 - cx
            var dy = sc.y + sc.h / 2 - cy
            var d = dx * dx + dy * dy
            if (d < bestDist) {
                bestDist = d
                best = sc
            }
        }
        return best
    }

    function findSnap(raw, axis, side, screens, snapDist) {
        var best = -1
        var bestDist = snapDist
        for (var i = 0; i < root.neighbors.length; ++i) {
            var n = root.neighbors[i]
            var cands
            if (axis === "h") {
                cands = (side === "right")
                        ? [n.x - root.gap, n.x + n.w]
                        : [n.x + n.w + root.gap, n.x]
            } else {
                cands = (side === "bottom")
                        ? [n.y - root.gap, n.y + n.h]
                        : [n.y + n.h + root.gap, n.y]
            }
            for (var j = 0; j < cands.length; ++j) {
                var d = Math.abs(raw - cands[j])
                if (d <= bestDist) {
                    bestDist = d
                    best = cands[j]
                }
            }
        }
        // 所属屏幕四边：每条边只吸自己方向的那一侧（与松手结算落位一致，
        // 结算把贴边瓦片退到屏边内侧 gap 处）。这里吸的是"边位置"：
        // 左边吸到 left + gap，右边吸到 right - gap，上下同理。
        if (screens && screens.length > 0) {
            var screen = null
            if (axis === "h") {
                // 左右边：瓦片纵向中心在哪块屏，就吸那块屏。
                screen = screenForCenter(screens, raw, root.vProbe)
            } else {
                // 上下边：瓦片横向中心在哪块屏，就吸那块屏。
                screen = screenForCenter(screens, root.hProbe, raw)
            }
            if (screen) {
                var screenCands
                if (axis === "h") {
                    screenCands = (side === "right")
                            ? [screen.x + screen.w - root.gap]
                            : [screen.x + root.gap]
                } else {
                    screenCands = (side === "bottom")
                            ? [screen.y + screen.h - root.gap]
                            : [screen.y + root.gap]
                }
                for (var k = 0; k < screenCands.length; ++k) {
                    var ds = Math.abs(raw - screenCands[k])
                    if (ds <= bestDist) {
                        bestDist = ds
                        best = screenCands[k]
                    }
                }
            }
        }
        return best
    }

    // 该边当前的吸附基准值；-1 表示未吸附。
    function heldSlot(axis, side) {
        if (axis === "h")
            return side === "right" ? root.snapRight : root.snapLeft
        return side === "bottom" ? root.snapBottom : root.snapTop
    }

    function setSlot(axis, side, value) {
        if (axis === "h") {
            if (side === "right")
                root.snapRight = value
            else
                root.snapLeft = value
        } else {
            if (side === "bottom")
                root.snapBottom = value
            else
                root.snapTop = value
        }
    }

    // 带迟滞的吸附：吸附中先看是否仍在迟滞带内，出带再重新找。
    function applySnap(raw, axis, side, screen, snapDist) {
        var held = heldSlot(axis, side)
        if (held >= 0) {
            if (Math.abs(raw - held) <= root.unsnapDist)
                return held
            setSlot(axis, side, -1)
        }
        var snapped = findSnap(raw, axis, side, screen, snapDist)
        if (snapped >= 0) {
            setSlot(axis, side, snapped)
            return snapped
        }
        return raw
    }

    function beginDrag(centerX, centerY) {
        root.refreshNeighbors()
        if (!isNaN(centerX))
            root.hProbe = centerX
        if (!isNaN(centerY))
            root.vProbe = centerY
        root.snapLeft = -1
        root.snapRight = -1
        root.snapTop = -1
        root.snapBottom = -1
    }

    function endDrag() {
        root.snapLeft = -1
        root.snapRight = -1
        root.snapTop = -1
        root.snapBottom = -1
    }
}
