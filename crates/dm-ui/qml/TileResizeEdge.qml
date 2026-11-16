import QtQuick

// 瓦片四边缩放边条 + 邻瓦/屏幕边界磁吸。
//
// 交互规则（与拖动共用 TileSnap 引擎，规则完全一致）：
// - 拉动瓦片的某条边，靠近其他瓦片的同轴边界（snapDist px 内）时，
//   尺寸"卡"一下吸附到对齐位置——明显的阻尼感；
// - 相对边（我的右边 vs 邻居左边）→ 吸附位置留 gap 缝（release_gap，
//   与松手结算一致），明确瓦片边界；
// - 同侧边（我的右边 vs 邻居右边）→ 精确对齐（列/行边界对齐）；
// - 瓦片所在屏四边 → 吸到屏边内侧 gap 处（与松手结算一致）；
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

    // 全部屏幕可用区快照：按下时取一次 [{x, y, w, h}, ...]；取不到则不算
    // 屏幕边。钳制线取瓦片中心所在的屏（多屏布局合法）。
    property var screenArea: null

    property var startGeom: null

    // 全局指针坐标（屏幕坐标）。与 WindowDragEdge 同一参考系铁律：
    // 场景坐标会随窗口移动而变化，不能用于几何增量计算。
    function cursor() {
        if (appState && appState.cursor_pos) {
            var p = appState.cursor_pos()
            return { x: Number(p[0]), y: Number(p[1]) }
        }
        // 兜底：理论上不会走到（拖右/底边窗口不动时等价）。
        return { x: 0, y: 0 }
    }

    function refreshScreen() {
        if (appState && appState.screen_rects) {
            var raw = appState.screen_rects()
            var list = []
            for (var i = 0; i + 3 < raw.length; i += 4) {
                list.push({
                    x: Number(raw[i]),
                    y: Number(raw[i + 1]),
                    w: Number(raw[i + 2]),
                    h: Number(raw[i + 3])
                })
            }
            root.screenArea = list
        } else {
            root.screenArea = null
        }
    }

    // 瓦片中心所在的屏（钳制线与屏幕边吸附都针对它）。
    // 找不到（screenArea 为空）返回 null。
    function homeScreen() {
        if (!root.screenArea || root.screenArea.length === 0)
            return null
        var s = root.startGeom
        var cx = s.x + s.w / 2
        var cy = s.y + s.h / 2
        for (var i = 0; i < root.screenArea.length; ++i) {
            var area = root.screenArea[i]
            if (cx >= area.x && cx < area.x + area.w
                    && cy >= area.y && cy < area.y + area.h)
                return area
        }
        // 都不包含时取中心最近的一块。
        var best = null
        var bestDist = Infinity
        for (var j = 0; j < root.screenArea.length; ++j) {
            var sc = root.screenArea[j]
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

    // 拉动某条边时，该边允许到达的硬上限（缝隙线）：越过即钉死。
    //
    // 规则：我的被拖边 + gap 不得越过任何"同轴相对"的邻居边。
    // 以拉底边为例：下方邻居的顶边是硬上限 → 被拖边 ≤ 邻居顶边 - gap。
    // 只有横向（拉底/顶边时）或纵向（拉左/右边时）投影与邻居有交集的
    // 才算同轴邻居——斜对角的瓦片不限制。
    // 钳制线过近导致尺寸会低于 minSize 时放弃该条钳制（宁可越线，
    // 不把瓦片压瘪；邻居极近时磁吸早把边吸走了，这里是最后防线）。
    // screenGap 是屏幕边的钳制线（settle 结算的落位，恒 ≥ minSize 可达）。
    function clampFor(edge, screenGap) {
        var s = root.startGeom
        var result = screenGap
        for (var i = 0; i < snap.neighbors.length; ++i) {
            var n = snap.neighbors[i]
            if (edge === "bottom") {
                // 纵向投影有交集的邻居才限制我的底边。
                if (s.x < n.x + n.w && n.x < s.x + s.w
                        && n.y >= s.y + s.h - 1 && n.y < result)
                    result = n.y - snap.gap
            } else if (edge === "top") {
                if (s.x < n.x + n.w && n.x < s.x + s.w
                        && n.y + n.h <= s.y + 1 && n.y + n.h > result)
                    result = n.y + n.h + snap.gap
            } else if (edge === "right") {
                if (s.y < n.y + n.h && n.y < s.y + s.h
                        && n.x >= s.x + s.w - 1 && n.x < result)
                    result = n.x - snap.gap
            } else if (edge === "left") {
                if (s.y < n.y + n.h && n.y < s.y + s.h
                        && n.x + n.w <= s.x + 1 && n.x + n.w > result)
                    result = n.x + n.w + snap.gap
            }
        }
        return result
    }

    // 该方向的钳制线是否会与 minSize 冲突（钳制后尺寸不足）。
    function clampReachable(edge, line) {
        var s = root.startGeom
        if (edge === "bottom")
            return line - s.y >= root.minSize
        if (edge === "top")
            return s.y + s.h - line >= root.minSize
        if (edge === "right")
            return line - s.x >= root.minSize
        return s.x + s.w - line >= root.minSize
    }

    function beginDrag() {
        snap.beginDrag(targetWindow.x + targetWindow.width / 2,
                       targetWindow.y + targetWindow.height / 2)
        root.refreshScreen()
        var g = root.cursor()
        root.startGeom = {
            x: targetWindow.x,
            y: targetWindow.y,
            w: targetWindow.width,
            h: targetWindow.height,
            gx: g.x,
            gy: g.y
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

    // 统一落位：收进屏幕集合（任一块屏内都合法）后写回窗口。
    function applyRect(x, y, w, h) {
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

    // edge: "left" / "right" / "top" / "bottom"。
    // 被拖边 = min(指针目标, 钳制线)——强制缝隙规则：越过邻居缝隙线
    // 即钉死在缝隙位置（尺寸被强制缩短），而不是冻结在最后一帧。
    function drag(edge, gx, gy) {
        if (!root.startGeom || !root.targetWindow)
            return
        var s = root.startGeom
        var screen = root.homeScreen()

        if (edge === "right") {
            var rawRight = s.x + s.w + (gx - s.gx)
            var right = snap.applySnap(rawRight, "h", "right", root.screenArea, root.snapDist)
            var limit = root.clampFor("right", screen ? screen.x + screen.w - snap.gap : right)
            if (right > limit && root.clampReachable("right", limit))
                right = limit
            var rw = Math.max(root.minSize, Math.round(right - s.x))
            root.applyRect(s.x, s.y, rw, s.h)
        } else if (edge === "left") {
            var fixedRight = s.x + s.w
            var rawLeft = s.x + (gx - s.gx)
            var left = snap.applySnap(rawLeft, "h", "left", root.screenArea, root.snapDist)
            var limitL = root.clampFor("left", screen ? screen.x + snap.gap : left)
            if (left < limitL && root.clampReachable("left", limitL))
                left = limitL
            left = Math.min(left, fixedRight - root.minSize)
            var x = Math.round(left)
            var lw = Math.max(root.minSize, Math.round(fixedRight - left))
            root.applyRect(x, s.y, lw, s.h)
        } else if (edge === "bottom") {
            var rawBottom = s.y + s.h + (gy - s.gy)
            var bottom = snap.applySnap(rawBottom, "v", "bottom", root.screenArea, root.snapDist)
            var limitB = root.clampFor("bottom", screen ? screen.y + screen.h - snap.gap : bottom)
            if (bottom > limitB && root.clampReachable("bottom", limitB))
                bottom = limitB
            var bh = Math.max(root.minSize, Math.round(bottom - s.y))
            root.applyRect(s.x, s.y, s.w, bh)
        } else if (edge === "top") {
            var fixedBottom = s.y + s.h
            var rawTop = s.y + (gy - s.gy)
            var top = snap.applySnap(rawTop, "v", "top", root.screenArea, root.snapDist)
            var limitT = root.clampFor("top", screen ? screen.y + snap.gap : top)
            if (top < limitT && root.clampReachable("top", limitT))
                top = limitT
            top = Math.min(top, fixedBottom - root.minSize)
            var y = Math.round(top)
            var th = Math.max(root.minSize, Math.round(fixedBottom - top))
            root.applyRect(s.x, y, s.w, th)
        }
    }

    // ------------------------------------------------------------ 四条边
    // 指针坐标一律用 root.cursor()（全局屏幕坐标，与窗口位置无关）；
    // 场景坐标在窗口自身移动时会污染增量计算。

    MouseArea {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: root.edgeSize
        cursorShape: Qt.SizeHorCursor
        onPressed: function (mouse) {
            root.beginDrag()
        }
        onPositionChanged: function (mouse) {
            if (!pressed)
                return
            var g = root.cursor()
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
            root.beginDrag()
        }
        onPositionChanged: function (mouse) {
            if (!pressed)
                return
            var g = root.cursor()
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
            root.beginDrag()
        }
        onPositionChanged: function (mouse) {
            if (!pressed)
                return
            var g = root.cursor()
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
            root.beginDrag()
        }
        onPositionChanged: function (mouse) {
            if (!pressed)
                return
            var g = root.cursor()
            root.drag("top", g.x, g.y)
        }
        onReleased: root.endDrag()
    }
}
