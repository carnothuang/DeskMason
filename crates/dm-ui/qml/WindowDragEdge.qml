import QtQuick

// 桌面层瓦片窗口的拖动边条：窗口顶部 8px 高的隐形把手。
// 优先交给窗口管理器原生拖动（startSystemMove）：KWin 按指针同步移动，
// 帧率与桌面一致，不会一顿一顿。窗管不接手（返回 false）时退回
// 手动改 window.x/y——桌面层窗口 KWin 也可能不响应 moveresize。
MouseArea {
    id: root

    // 拖动目标窗口（ApplicationWindow 的 id）。
    property var targetWindow
    // AppState 实例（start_move invokable 所在对象），由实例化处传入。
    property var appState
    // 拖动进行中（两种方式共用；外部 geometryTimer 落盘位置）。
    property bool dragging: false
    // startSystemMove 已被窗管接手；此后 QML 不再手动改坐标。
    property bool systemMove: false
    // z 值：放在内容之下时用低值；边条默认盖在最上。
    z: 100
    cursorShape: Qt.SizeAllCursor
    height: 8
    anchors.top: parent.top
    anchors.left: parent.left
    anchors.right: parent.right

    property real pressX: 0
    property real pressY: 0
    property real winX: 0
    property real winY: 0

    onPressed: function (mouse) {
        dragging = true
        systemMove = false
        // 先问宿主能否走 KWin 原生拖动；失败再手动来。
        if (appState && appState.start_move && appState.start_move()) {
            systemMove = true
            return
        }
        var g = mapToItem(null, mouse.x, mouse.y)
        pressX = g.x
        pressY = g.y
        winX = targetWindow.x
        winY = targetWindow.y
    }
    onReleased: function (mouse) {
        dragging = false
        systemMove = false
        // 松手立刻做一次全局「松手留缝」结算：贴住邻瓦或屏幕边缘时
        // 由 Rust 侧统一退开 RELEASE_GAP。系统拖动被 KWin 接管时这里
        // 可能收不到释放事件，由窗口的 geometryTimer 兜底再结算一次。
        if (appState && appState.save_geometry) {
            appState.save_geometry(targetWindow.x, targetWindow.y,
                                   targetWindow.width, targetWindow.height)
        }
    }
    onPositionChanged: function (mouse) {
        if (!pressed || systemMove)
            return
        var g = mapToItem(null, mouse.x, mouse.y)
        var nx = Math.round(winX + g.x - pressX)
        var ny = Math.round(winY + g.y - pressY)
        // 双屏场景：手动拖动也只允许停在主屏范围内。
        if (appState && appState.clamp_rect) {
            var fixed = appState.clamp_rect(nx, ny, targetWindow.width, targetWindow.height)
            nx = Number(fixed[0])
            ny = Number(fixed[1])
        }
        targetWindow.x = nx
        targetWindow.y = ny
    }
}
