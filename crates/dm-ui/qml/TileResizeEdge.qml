import QtQuick

// 瓦片四边缩放边条 + 邻瓦边界磁吸。
//
// 交互规则：
// - 拉动瓦片的某条边，靠近其他瓦片的同轴边界（阈值 snapDist px）时，
//   尺寸"卡"一下吸附到对齐位置——这就是用户要的"卡顿提示"；
// - 吸附位置与邻瓦之间保留 snapGap px 的间隔，明确瓦片边界；
// - 吸附状态下继续拉动超过 unsnapDist px 才解除吸附（迟滞，避免抖动）。
//
// 邻居几何在按下时取一次快照（neighbor_rects invokable），拖动路径上
// 不做任何磁盘/跨窗口查询。桌面层窗口 KWin 不接手 resize，所以全部
// 手动改 window.x/y/width/height，geometryTimer 防抖落盘。
Item {
    id: root

    anchors.fill: parent

    property var targetWindow
    property var appState
    z: 100

    readonly property int edgeSize: 8
    readonly property int snapDist: 10
    // 相对边吸附留 40px（原 8px 的 5 倍），明确瓦片边界。
    readonly property int snapGap: 40
    // 任何时候都不允许两块瓦片靠得比 40px 更近，更不允许重叠。
    readonly property int minGap: 40
    readonly property int unsnapDist: 14
    readonly property int minSize: 160

    // 邻瓦几何快照：[{x, y, w, h}, ...]
    property var neighbors: []
    // 吸附基准值（窗口坐标系）；-1 表示未吸附。H 是左右边（x 轴），V 是上下边。
    property real snapH: -1
    property real snapV: -1
    property var startGeom: null

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

    // 未吸附时找最近的邻瓦边界；超过 snapDist 返回 -1。
    // side 是本瓦片正在拖动的边。规则：
    // - 相对边（我的右边 vs 邻居左边）→ 吸附位置留 snapGap 间隔；
    // - 同侧边（我的右边 vs 邻居右边）→ 精确对齐（列边界对齐）。
    function findSnap(raw, axis, side) {
        var best = -1
        var bestDist = root.snapDist
        for (var i = 0; i < root.neighbors.length; ++i) {
            var n = root.neighbors[i]
            var cands
            if (axis === "h")
                cands = (side === "right")
                        ? [n.x - root.snapGap, n.x + n.w]
                        : [n.x + n.w + root.snapGap, n.x]
            else
                cands = (side === "bottom")
                        ? [n.y - root.snapGap, n.y + n.h]
                        : [n.y + n.h + root.snapGap, n.y]
            for (var j = 0; j < cands.length; ++j) {
                var d = Math.abs(raw - cands[j])
                if (d <= bestDist) {
                    bestDist = d
                    best = cands[j]
                }
            }
        }
        return best
    }

    // 带迟滞的吸附：吸附中先看是否仍在迟滞带内，出带再重新找。
    function applySnap(raw, axis, side) {
        var held = axis === "h" ? root.snapH : root.snapV
        if (held >= 0) {
            if (Math.abs(raw - held) <= root.unsnapDist)
                return held
            if (axis === "h")
                root.snapH = -1
            else
                root.snapV = -1
        }
        var snapped = findSnap(raw, axis, side)
        if (snapped >= 0) {
            if (axis === "h")
                root.snapH = snapped
            else
                root.snapV = snapped
            return snapped
        }
        return raw
    }

    function beginDrag(gx, gy) {
        root.refreshNeighbors()
        root.snapH = -1
        root.snapV = -1
        root.startGeom = {
            x: targetWindow.x,
            y: targetWindow.y,
            w: targetWindow.width,
            h: targetWindow.height,
            gx: gx,
            gy: gy,
            // 按下瞬间的"侵入深度"：允许把已有的过近间隔拉宽，
            // 但不允许比按下时更挤，也不允许叠到一起。
            invade: root.invade(targetWindow.x, targetWindow.y,
                                targetWindow.width, targetWindow.height)
        }
    }

    function endDrag() {
        root.startGeom = null
        root.snapH = -1
        root.snapV = -1
        // 缩放结束同样走一次全局松手结算，规则与拖动完全一致。
        if (appState && appState.save_geometry && targetWindow) {
            appState.save_geometry(targetWindow.x, targetWindow.y,
                                   targetWindow.width, targetWindow.height)
        }
    }

    // 候选矩形侵入邻瓦 minGap 安全区的最大深度；0 表示间隔达标。
    function invade(x, y, w, h) {
        var worst = 0
        for (var i = 0; i < root.neighbors.length; ++i) {
            var n = root.neighbors[i]
            var dx = Math.min(x + w + root.minGap - n.x, n.x + n.w + root.minGap - x)
            var dy = Math.min(y + h + root.minGap - n.y, n.y + n.h + root.minGap - y)
            if (dx > 0 && dy > 0)
                worst = Math.max(worst, Math.min(dx, dy))
        }
        return worst
    }

    // 统一落位：先拒绝会让间隔变小的结果，再收进主屏可用区。
    function applyRect(x, y, w, h) {
        if (!root.startGeom || root.invade(x, y, w, h) > root.startGeom.invade)
            return
        var fx = x
        var fy = y
        var fw = w
        var fh = h
        if (appState && appState.clamp_rect) {
            var fixed = appState.clamp_rect(x, y, w, h)
            fx = Number(fixed[0])
            fy = Number(fixed[1])
            fw = Number(fixed[2])
            fh = Number(fixed[3])
        }
        root.targetWindow.x = fx
        root.targetWindow.y = fy
        root.targetWindow.width = fw
        root.targetWindow.height = fh
    }

    // edge: "left" / "right" / "top" / "bottom"；gx/gy 是全局指针坐标。
    function drag(edge, gx, gy) {
        if (!root.startGeom || !root.targetWindow)
            return
        var s = root.startGeom
        var x = s.x
        var y = s.y
        var w = s.w
        var h = s.h

        if (edge === "right") {
            var rawEdge = s.x + s.w + (gx - s.gx)
            w = Math.max(root.minSize, Math.round(applySnap(rawEdge, "h", "right") - s.x))
        } else if (edge === "left") {
            var right = s.x + s.w
            var rawLeft = s.x + (gx - s.gx)
            var left = Math.min(applySnap(rawLeft, "h", "left"), right - root.minSize)
            x = Math.round(left)
            w = Math.max(root.minSize, Math.round(right - left))
        } else if (edge === "bottom") {
            var rawBottom = s.y + s.h + (gy - s.gy)
            h = Math.max(root.minSize, Math.round(applySnap(rawBottom, "v", "bottom") - s.y))
        } else if (edge === "top") {
            var bottom = s.y + s.h
            var rawTop = s.y + (gy - s.gy)
            var top = Math.min(applySnap(rawTop, "v", "top"), bottom - root.minSize)
            y = Math.round(top)
            h = Math.max(root.minSize, Math.round(bottom - top))
        }
        root.applyRect(x, y, w, h)
    }

    // ------------------------------------------------------------ 四条边

    MouseArea {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: root.edgeSize
        cursorShape: Qt.SizeHorCursor
        onPressed: function (mouse) {
            var g = mapToItem(null, mouse.x, mouse.y)
            root.beginDrag(g.x, g.y)
        }
        onPositionChanged: function (mouse) {
            if (!pressed)
                return
            var g = mapToItem(null, mouse.x, mouse.y)
            root.drag("right", g.x, g.y)
        }
        onReleased: root.endDrag()
    }

    MouseArea {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: root.edgeSize
        cursorShape: Qt.SizeHorCursor
        onPressed: function (mouse) {
            var g = mapToItem(null, mouse.x, mouse.y)
            root.beginDrag(g.x, g.y)
        }
        onPositionChanged: function (mouse) {
            if (!pressed)
                return
            var g = mapToItem(null, mouse.x, mouse.y)
            root.drag("left", g.x, g.y)
        }
        onReleased: root.endDrag()
    }

    MouseArea {
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.edgeSize
        cursorShape: Qt.SizeVerCursor
        onPressed: function (mouse) {
            var g = mapToItem(null, mouse.x, mouse.y)
            root.beginDrag(g.x, g.y)
        }
        onPositionChanged: function (mouse) {
            if (!pressed)
                return
            var g = mapToItem(null, mouse.x, mouse.y)
            root.drag("bottom", g.x, g.y)
        }
        onReleased: root.endDrag()
    }

    MouseArea {
        anchors.top: parent.top
        anchors.topMargin: root.edgeSize
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.edgeSize
        cursorShape: Qt.SizeVerCursor
        onPressed: function (mouse) {
            var g = mapToItem(null, mouse.x, mouse.y)
            root.beginDrag(g.x, g.y)
        }
        onPositionChanged: function (mouse) {
            if (!pressed)
                return
            var g = mapToItem(null, mouse.x, mouse.y)
            root.drag("top", g.x, g.y)
        }
        onReleased: root.endDrag()
    }
}
