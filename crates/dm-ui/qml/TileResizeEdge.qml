import QtQuick

// 瓦片四边缩放边条 + 邻瓦/屏幕边界磁吸。
//
// 交互规则（与拖动共用 TileSnap 引擎，规则完全一致）：
// - 拉动瓦片的某条边，靠近其他瓦片的同轴边界（snapDist px 内）时，
//   尺寸"卡"一下吸附到对齐位置——明显的阻尼感；
// - 相对边（我的右边 vs 邻居左边）→ 吸附位置留 gap 缝（release_gap，
//   与松手结算一致），明确瓦片边界；
// - 同侧边（我的右边 vs 邻居右边）→ 精确对齐（列/行边界对齐）；
// - 主屏四边 → 吸到屏边内侧 gap 处（与松手结算一致）；
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
    readonly property int minSize: 160

    // 统一磁吸引擎：拖动与缩放共用同一套规则与常量。
    TileSnap {
        id: snap
        appState: root.appState
    }

    // 主屏可用区快照：按下时取一次 {x, y, w, h}；取不到则不算屏幕边。
    property var screenArea: null

    property var startGeom: null

    function refreshScreen() {
        if (appState && appState.clamp_rect) {
            var probe = appState.clamp_rect(0, 0, 100000, 100000)
            root.screenArea = {
                x: Number(probe[0]),
                y: Number(probe[1]),
                w: Number(probe[2]),
                h: Number(probe[3])
            }
        } else {
            root.screenArea = null
        }
    }

    // 候选矩形侵入邻瓦 gap 安全区的最大深度；0 表示间隔达标。
    // 安全区以运行时 gap 为准，与磁吸/松手结算一致。
    function invade(x, y, w, h) {
        var worst = 0
        for (var i = 0; i < snap.neighbors.length; ++i) {
            var n = snap.neighbors[i]
            var dx = Math.min(x + w + snap.gap - n.x, n.x + n.w + snap.gap - x)
            var dy = Math.min(y + h + snap.gap - n.y, n.y + n.h + snap.gap - y)
            if (dx > 0 && dy > 0)
                worst = Math.max(worst, Math.min(dx, dy))
        }
        return worst
    }

    function beginDrag(gx, gy) {
        snap.beginDrag()
        root.refreshScreen()
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
        snap.endDrag()
        // 缩放结束同样走一次全局松手结算，规则与拖动完全一致。
        if (appState && appState.save_geometry && targetWindow) {
            appState.save_geometry(targetWindow.x, targetWindow.y,
                                   targetWindow.width, targetWindow.height)
        }
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

        if (edge === "right") {
            var rawRight = s.x + s.w + (gx - s.gx)
            var right = snap.applySnap(rawRight, "h", "right", root.screenArea, root.snapDist)
            var rw = Math.max(root.minSize, Math.round(right - s.x))
            root.applyRect(s.x, s.y, rw, s.h)
        } else if (edge === "left") {
            var fixedRight = s.x + s.w
            var rawLeft = s.x + (gx - s.gx)
            var left = Math.min(snap.applySnap(rawLeft, "h", "left", root.screenArea, root.snapDist),
                                fixedRight - root.minSize)
            var x = Math.round(left)
            var lw = Math.max(root.minSize, Math.round(fixedRight - left))
            root.applyRect(x, s.y, lw, s.h)
        } else if (edge === "bottom") {
            var rawBottom = s.y + s.h + (gy - s.gy)
            var bottom = snap.applySnap(rawBottom, "v", "bottom", root.screenArea, root.snapDist)
            var bh = Math.max(root.minSize, Math.round(bottom - s.y))
            root.applyRect(s.x, s.y, s.w, bh)
        } else if (edge === "top") {
            var fixedBottom = s.y + s.h
            var rawTop = s.y + (gy - s.gy)
            var top = Math.min(snap.applySnap(rawTop, "v", "top", root.screenArea, root.snapDist),
                               fixedBottom - root.minSize)
            var y = Math.round(top)
            var th = Math.max(root.minSize, Math.round(fixedBottom - top))
            root.applyRect(s.x, y, s.w, th)
        }
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
