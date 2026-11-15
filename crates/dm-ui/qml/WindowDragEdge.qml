import QtQuick

// 桌面层瓦片窗口的拖动边条：窗口顶部 8px 高的隐形把手。
//
// 拖动一律手动改 window.x/y（不走 startSystemMove）：磁吸要求逐帧
// 拦截坐标，系统拖动由窗管直接改窗口位置，QML 拦不住。
// 参考系铁律：指针坐标必须用 appState.cursor_pos()（QCursor::pos，
// 屏幕全局坐标）。QML 的 mapToItem(null) 给的是窗口内场景坐标，
// 窗口一动场景坐标跟着变——拿它当全局坐标会形成"窗口动 → 坐标变
// → 目标位置变"的反馈回路：拖动以一半速度跟手并振荡（鬼畜），
// 撞上 clamp 后误差被整段甩出，瓦片会被钉到屏幕角落。
// 磁吸规则统一来自 TileSnap：
// - 窗口任意一条边靠近邻瓦同轴边界（snapDist px 内）→ 吸附：
//   相对边留 gap 缝，同侧边精确对齐，屏幕四边吸到屏边内侧 gap 处；
// - 吸附中继续拉动超过迟滞带才解除（阻尼感 + 防抖动）。
// 松手立即 save_geometry：Rust 侧 settle_rect 再做一次全局留缝结算
// （与磁吸落位一致，正常情况不会改动），geometryTimer 兜底防抖落盘。
MouseArea {
    id: root

    // 拖动目标窗口（ApplicationWindow 的 id）。
    property var targetWindow
    // AppState 实例（neighbor_rects / release_gap / clamp_rect invokable 所在对象）。
    property var appState
    // 拖动进行中（外部 geometryTimer 落盘位置）。
    property bool dragging: false
    // 磁吸距离：窗口边进入候选边 snapDist px 内触发吸附。
    readonly property int snapDist: 10
    // z 值：边条默认盖在最上。
    z: 100
    cursorShape: Qt.SizeAllCursor
    height: 8
    anchors.top: parent.top
    anchors.left: parent.left
    anchors.right: parent.right

    // 统一磁吸引擎：拖动与缩放共用同一套规则与常量。
    TileSnap {
        id: snap
        appState: root.appState
    }

    // 主屏可用区快照：按下时取一次 {x, y, w, h}；取不到则不算屏幕边。
    property var screenArea: null

    // 按下时刻的全局指针坐标与窗口位置（参考系原点，此后只算增量）。
    property real pressX: 0
    property real pressY: 0
    property real winX: 0
    property real winY: 0

    function refreshScreen() {
        if (appState && appState.clamp_rect) {
            // 用一个超大矩形问主屏可用区：clamp_rect 会把它收进区内，
            // 返回的 x/y/w/h 就是可用区本身。
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

    // 全局指针坐标；cursor_pos 不可用时退回事件坐标（仅单屏且窗口
    // 不动时等价，实际不会走到）。
    function globalCursor(mouse) {
        if (appState && appState.cursor_pos) {
            var p = appState.cursor_pos()
            return { x: Number(p[0]), y: Number(p[1]) }
        }
        var g = mapToItem(null, mouse.x, mouse.y)
        return { x: g.x, y: g.y }
    }

    // 拖动时窗口四条边都可能对齐。每条边各自吸附（迟滞状态互相独立）；
    // 同轴两条边同时命中时取位移更近的一条，另一条跟着平移。
    function applyMoveSnap(rawLeft, rawTop, w, h) {
        var xLeft = snap.applySnap(rawLeft, "h", "left", root.screenArea, root.snapDist)
        var xRight = snap.applySnap(rawLeft + w, "h", "right", root.screenArea, root.snapDist)
        var nx = rawLeft
        if (xLeft !== rawLeft && xRight === rawLeft + w)
            nx = xLeft
        else if (xRight !== rawLeft + w && xLeft === rawLeft)
            nx = xRight - w
        else if (xLeft !== rawLeft && xRight !== rawLeft + w)
            nx = Math.abs(xLeft - rawLeft) <= Math.abs(xRight - rawLeft - w) ? xLeft : xRight - w

        var yTop = snap.applySnap(rawTop, "v", "top", root.screenArea, root.snapDist)
        var yBottom = snap.applySnap(rawTop + h, "v", "bottom", root.screenArea, root.snapDist)
        var ny = rawTop
        if (yTop !== rawTop && yBottom === rawTop + h)
            ny = yTop
        else if (yBottom !== rawTop + h && yTop === rawTop)
            ny = yBottom - h
        else if (yTop !== rawTop && yBottom !== rawTop + h)
            ny = Math.abs(yTop - rawTop) <= Math.abs(yBottom - rawTop - h) ? yTop : yBottom - h

        return { x: nx, y: ny }
    }

    // ------------------------------------------------------------ 公共拖动 API
    // 把手自身的 MouseArea 与标题行代理（main.qml）都走这三个函数。

    function dragBegin() {
        dragging = true
        snap.beginDrag()
        root.refreshScreen()
        var cur = root.globalCursor(null)
        pressX = cur.x
        pressY = cur.y
        winX = targetWindow.x
        winY = targetWindow.y
    }

    function dragMove() {
        if (!dragging)
            return
        var cur = root.globalCursor(null)
        var rawX = winX + cur.x - pressX
        var rawY = winY + cur.y - pressY
        var snapped = applyMoveSnap(rawX, rawY, targetWindow.width, targetWindow.height)
        var nx = Math.round(snapped.x)
        var ny = Math.round(snapped.y)
        // 双屏场景：手动拖动也只允许停在主屏范围内。
        if (appState && appState.clamp_rect) {
            var fixed = appState.clamp_rect(nx, ny, targetWindow.width, targetWindow.height)
            nx = Number(fixed[0])
            ny = Number(fixed[1])
        }
        targetWindow.x = nx
        targetWindow.y = ny
    }

    function dragEnd() {
        if (!dragging)
            return
        dragging = false
        snap.endDrag()
        // 松手立刻做一次全局「松手留缝」结算；磁吸落位与结算一致，
        // 正常不会再挪。系统层兜底见窗口 geometryTimer。
        if (appState && appState.save_geometry) {
            appState.save_geometry(targetWindow.x, targetWindow.y,
                                   targetWindow.width, targetWindow.height)
        }
    }

    // ------------------------------------------------------------ 把手自身事件

    onPressed: function (mouse) {
        root.dragBegin()
    }
    onReleased: function (mouse) {
        root.dragEnd()
    }
    onPositionChanged: function (mouse) {
        if (!pressed)
            return
        root.dragMove()
    }
}
