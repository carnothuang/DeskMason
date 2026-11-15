import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

ApplicationWindow {
    id: window
    // 由 C++ 胶水在创建窗口时注入。
    property string widgetId: ""

    visible: false
    flags: Qt.FramelessWindowHint
    color: "transparent"

    // 桌面瓦片是深色面板，把控件调色板固定成深色，
    // 否则跟随系统浅色主题时会出现"黑底黑字"。
    palette.window: "#202022"
    palette.windowText: "#F2F2F2"
    palette.base: "#262628"
    palette.alternateBase: "#2E2E30"
    palette.text: "#F2F2F2"
    palette.button: "#2E2E30"
    palette.buttonText: "#F2F2F2"
    palette.highlight: "#3477E8"
    palette.highlightedText: "#FFFFFF"
    palette.placeholderText: "#8A8A8E"
    palette.toolTipBase: "#2B2B2B"
    palette.toolTipText: "#FFFFFF"

    title: qsTr("DeskMason 文件瓦片")

    AppState { id: app }
    NewFileTileDialog { id: newTileDialog; appState: app }
    FileTile { id: tile }

    property int actionIndex: -1
    property string actionName: ""
    property bool dropHighlight: false
    property int dropTargetIndex: -1

    // 多选：存下标，anchor 用于 Shift 连选。
    property var selection: []
    property int anchorIndex: -1
    property var pendingDeletePaths: []
    // 宿主（托盘/热键）显隐版本；变化时应用可见性。
    property int hostEpoch: -1
    // 框选：坐标是 grid 内坐标，像素；applyMarquee 里再加 contentX/Y 转成内容坐标。
    property bool marqueeActive: false
    property real marqueeX1: 0
    property real marqueeY1: 0
    property real marqueeX2: 0
    property real marqueeY2: 0
    property bool marqueeAdditive: false
    property var marqueeBase: []

    // 对话框打开时屏蔽全局快捷键，否则 Return/Delete 会被窗口抢走。
    readonly property bool dialogOpen: renameDialog.visible || deleteDialog.visible
                                        || newFolderDialog.visible || deleteTileDialog.visible
                                        || folderDialog.visible

    Component.onCompleted: {
        app.attach(window.widgetId)
        tile.attach(window.widgetId)
        window.x = app.windowX
        window.y = app.windowY
        window.width = app.windowW
        window.height = app.windowH
        grid.forceActiveFocus()
    }

    // 目录刷新后，裁掉越界的选择，避免用旧下标去操作新列表。
    Connections {
        target: tile
        function onItemCountChanged() {
            if (window.selection.length === 0)
                return
            var kept = []
            for (var i = 0; i < window.selection.length; ++i) {
                if (window.selection[i] < tile.itemCount)
                    kept.push(window.selection[i])
            }
            window.selection = kept
            if (window.anchorIndex >= tile.itemCount)
                window.anchorIndex = -1
        }
    }

    onXChanged: geometryTimer.restart()
    onYChanged: geometryTimer.restart()
    onWidthChanged: geometryTimer.restart()
    onHeightChanged: geometryTimer.restart()
    onClosing: app.save_geometry(window.x, window.y, window.width, window.height)

    Timer {
        id: geometryTimer
        // 防抖窗口：拖动中 x/y 持续变化会不断 restart，只有停稳
        // 超过 800ms 才真正触发一次磁盘写入；写盘在主线程同步做，
        // 拖动中途触发会把渲染卡出一帧（"一顿一顿"）。
        interval: 800
        onTriggered: app.save_geometry(window.x, window.y, window.width, window.height)
    }

    // 显示器热插拔：5 秒轮询一次拓扑，位置被校正就跟着挪窗口。
    // 托盘与全局热键的落点：200ms 轮询一次版本号与命令队列。
    // 框选时把指针停在边缘会持续滚动；MouseArea 在指针不动时不会发 positionChanged，
    // 所以用定时器接管。
    Timer {
        id: marqueeScrollTimer
        interval: 16
        repeat: true
        running: window.marqueeActive
        onTriggered: window.autoScrollMarquee()
    }

    Timer {
        id: hostTimer
        interval: 200
        running: true
        repeat: true
        onTriggered: {
            // 瓦片始终保持在桌面之上、普通窗口之下，绝不压住别的窗口。
            // 统一按固定顺序固定回 below 层，避免各窗口互抢层级闪烁。
            app.keep_below()
            // 双屏场景：每轮把窗口收回主屏（原生拖动可能把窗口带到副屏）。
            var fixed = app.clamp_rect(window.x, window.y, window.width, window.height)
            window.x = Number(fixed[0])
            window.y = Number(fixed[1])
            window.width = Number(fixed[2])
            window.height = Number(fixed[3])
            var current = app.host_epoch()
            if (current !== window.hostEpoch) {
                window.hostEpoch = current
                window.visible = app.host_visible()
            }

            var command = app.take_host_command()
            if (command === "new-tile")
                newTileDialog.open()
            else if (command.indexOf("new-tile:") === 0)
                app.open_settings()
            else if (command === "settings")
                app.open_settings()
            else if (command === "search")
                app.open_search()
            else if (command === "quit")
                app.quit_app()
        }
    }

    Timer {
        id: monitorTimer
        interval: 5000
        running: true
        repeat: true
        onTriggered: {
            if (app.reconcile()) {
                window.x = app.windowX
                window.y = app.windowY
                window.width = app.windowW
                window.height = app.windowH
            }
        }
    }

    Timer {
        id: pollTimer
        interval: 2000
        running: true
        repeat: true
        onTriggered: tile.tick()
    }

    // ------------------------------------------------------------ 选择模型

    function isSelected(index) {
        return selection.indexOf(index) >= 0
    }

    function selectOnly(index) {
        selection = [index]
        anchorIndex = index
    }

    function toggleSelect(index) {
        var next = selection.slice()
        var pos = next.indexOf(index)
        if (pos >= 0)
            next.splice(pos, 1)
        else
            next.push(index)
        selection = next
        anchorIndex = index
    }

    function selectRange(index) {
        if (anchorIndex < 0) {
            selectOnly(index)
            return
        }
        var lo = Math.min(anchorIndex, index)
        var hi = Math.max(anchorIndex, index)
        var next = []
        for (var i = lo; i <= hi; ++i)
            next.push(i)
        selection = next
    }

    function clearSelection() {
        selection = []
        anchorIndex = -1
    }

    function selectAll() {
        var next = []
        for (var i = 0; i < tile.itemCount; ++i)
            next.push(i)
        selection = next
        if (next.length > 0)
            anchorIndex = next[0]
    }

    // 把 grid 内坐标的矩形换算成内容坐标，命中的格子加进选择。
    function applyMarquee(x1, y1, x2, y2) {
        var cw = grid.cellWidth
        var ch = grid.cellHeight
        if (cw <= 0 || ch <= 0)
            return

        var columns = Math.max(1, Math.floor(grid.width / cw))
        var left = Math.min(x1, x2) + grid.contentX
        var top = Math.min(y1, y2) + grid.contentY
        var right = Math.max(x1, x2) + grid.contentX
        var bottom = Math.max(y1, y2) + grid.contentY

        var firstCol = Math.max(0, Math.floor(left / cw))
        var lastCol = Math.floor(right / cw)
        var firstRow = Math.max(0, Math.floor(top / ch))
        var lastRow = Math.floor(bottom / ch)

        var hits = []
        for (var row = firstRow; row <= lastRow; ++row) {
            for (var col = firstCol; col <= lastCol; ++col) {
                var index = row * columns + col
                if (index < 0 || index >= tile.itemCount)
                    continue
                var cellX = col * cw
                var cellY = row * ch
                if (cellX < right && cellX + cw > left && cellY < bottom && cellY + ch > top)
                    hits.push(index)
            }
        }

        var next = window.marqueeAdditive ? window.marqueeBase.slice() : []
        for (var i = 0; i < hits.length; ++i) {
            if (next.indexOf(hits[i]) < 0)
                next.push(hits[i])
        }
        window.selection = next
        if (next.length > 0)
            window.anchorIndex = next[0]
    }

    // 拖到网格上下边缘时滚动，并在滚动后重新计算命中。
    function autoScrollMarquee() {
        if (!window.marqueeActive)
            return
        var edge = 24
        var step = 14
        var maxY = Math.max(0, grid.contentHeight - grid.height)
        var nextY = grid.contentY

        if (window.marqueeY2 < edge)
            nextY -= step
        else if (window.marqueeY2 > grid.height - edge)
            nextY += step
        else
            return

        nextY = Math.max(0, Math.min(maxY, nextY))
        if (nextY === grid.contentY)
            return

        grid.contentY = nextY
        window.applyMarquee(window.marqueeX1, window.marqueeY1, window.marqueeX2, window.marqueeY2)
    }

    function selectedUrls() {
        var out = []
        for (var i = 0; i < selection.length; ++i) {
            var url = tile.file_url(selection[i])
            if (url !== "")
                out.push(url)
        }
        return out
    }

    function selectedPaths() {
        var out = []
        for (var i = 0; i < selection.length; ++i)
            out.push(tile.paths[selection[i]])
        return out
    }

    // 拖出：拖的是选中项之一就带上整批，否则只拖这一项。
    // text/uri-list 必须是 URL 数组；拼成带换行的单个字符串会被 Qt 当成一个 URL
    // （换行被转义成 %0A）。
    function dragUriList(index) {
        if (isSelected(index) && selection.length > 1)
            return selectedUrls()
        return [tile.file_url(index)]
    }

    function dragPathList(index) {
        if (isSelected(index) && selection.length > 1)
            return selectedPaths().join("\n")
        return tile.paths[index]
    }

    // ------------------------------------------------------------ 界面

    Rectangle {
        anchors.fill: parent
        radius: 14
        color: Theme.panel
        border.width: window.dropHighlight ? 2 : 1
        border.color: window.dropHighlight ? Theme.accentBorder : Theme.border

        // 顶部拖动把手：磁吸拖动的唯一实现，标题行按住也路由到这里。
        WindowDragEdge {
            id: titleDragHandle
            targetWindow: window
            appState: app
        }

        // 四边缩放边条：靠近其他瓦片边界时磁吸并留出间隔。
        TileResizeEdge {
            targetWindow: window
            appState: app
        }

        DropArea {
            id: dropArea
            anchors.fill: parent
            onEntered: function (drag) {
                drag.accepted = drag.hasUrls
                if (drag.hasUrls)
                    window.dropHighlight = true
            }
            onExited: {
                window.dropHighlight = false
                window.dropTargetIndex = -1
            }
            onPositionChanged: function (drag) {
                var index = window.cellIndexAt(drag.x, drag.y)
                window.dropTargetIndex = (index >= 0 && tile.kinds[index] === "dir") ? index : -1
            }
            onDropped: function (drop) {
                window.dropHighlight = false
                window.dropTargetIndex = -1
                if (!drop.hasUrls)
                    return
                var list = []
                for (var i = 0; i < drop.urls.length; ++i)
                    list.push(drop.urls[i])
                var index = window.cellIndexAt(drop.x, drop.y)
                if (index >= 0 && tile.kinds[index] === "dir")
                    tile.import_urls_into(list, index)
                else
                    tile.import_urls(list)
                drop.acceptProposedAction()
            }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 10
            spacing: 6

            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                // 按住标题/空白拖动整个瓦片：与顶部把手共用同一套
                // 磁吸拖动（WindowDragEdge 的公共 API），行为与所有瓦片一致。
                MouseArea {
                    id: windowDrag
                    Layout.fillWidth: true
                    cursorShape: Qt.SizeAllCursor

                    onPressed: function (mouse) {
                        var g = mapToItem(null, mouse.x, mouse.y)
                        titleDragHandle.dragBegin(g.x, g.y)
                    }
                    onReleased: function (mouse) {
                        titleDragHandle.dragEnd()
                    }
                    onPositionChanged: function (mouse) {
                        var g = mapToItem(null, mouse.x, mouse.y)
                        titleDragHandle.dragMove(g.x, g.y)
                    }
                }

                Text {
                    text: tile.title
                    font.pixelSize: 15
                    font.bold: true
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    color: Theme.textPrimary
                }

                Text {
                    text: window.selection.length > 0
                          ? tile.itemCount + " · " + qsTr("选中 %1").arg(window.selection.length)
                          : tile.itemCount
                    font.pixelSize: 12
                    color: window.selection.length > 0 ? Theme.accent : Theme.textTertiary
                }

                HeaderButton {
                    id: newTileButton
                    glyph: "＋"
                    tip: qsTr("新建瓦片")
                    onActivated: newTileMenu.popup(newTileButton, 0, newTileButton.height + 4)
                }

                HeaderButton {
                    id: settingsButton
                    glyph: "⚙"
                    tip: qsTr("瓦片设置")
                    onActivated: settingsMenu.popup(settingsButton, 0, settingsButton.height + 4)
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: "#00000000"

                GridView {
                    id: grid
                    anchors.fill: parent
                    clip: true
                    cellWidth: tile.cellWidth
                    cellHeight: tile.cellHeight
                    model: tile.names
                    focus: true
                    // 桌面上拖拽用于框选，滚动交给滚轮与滚动条。
                    interactive: false

                    // 自己处理滚轮：步长固定，行为可预测（Flickable 在
                    // interactive=false 时的滚轮行为不够稳定）。
                    WheelHandler {
                        onWheel: function (wheel) {
                            var steps = wheel.angleDelta.y / 120
                            if (steps === 0)
                                return
                            var maxY = Math.max(0, grid.contentHeight - grid.height)
                            grid.contentY = Math.max(0, Math.min(maxY, grid.contentY - steps * 80))
                            wheel.accepted = true
                        }
                    }
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.vertical: ScrollBar {
                        contentItem: Rectangle {
                            implicitWidth: 6
                            radius: 3
                            color: parent.pressed ? Theme.borderStrong
                                 : (parent.hovered ? Theme.borderStrong : Theme.border)
                            visible: parent.size < 1.0
                        }
                        background: Rectangle { color: "transparent" }
                    }

                    Keys.onReturnPressed: function (event) {
                        if (grid.currentIndex >= 0)
                            tile.open_item(grid.currentIndex)
                        event.accepted = true
                    }

                    delegate: Item {
                        id: cellRoot
                        width: grid.cellWidth
                        height: grid.cellHeight

                        // 拖出：Qt 的自动拖拽会在 X11 上发起 XDND。
                        Item {
                            id: dragProxy
                            width: 1
                            height: 1
                            Drag.active: dragArea.drag.active
                            Drag.dragType: Drag.Automatic
                            Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                            Drag.hotSpot: Qt.point(0, 0)
                            Drag.mimeData: ({
                                "text/uri-list": window.dragUriList(index),
                                "text/plain": window.dragPathList(index)
                            })
                        }

                        Rectangle {
                            anchors.fill: parent
                            anchors.margins: 3
                            radius: 8
                            color: {
                                if (window.dropTargetIndex === index)
                                    return Theme.accentSoft
                                if (window.isSelected(index))
                                    return Theme.accentSoft
                                return "transparent"
                            }
                        }

                        Column {
                            anchors.centerIn: parent
                            spacing: 4
                            width: parent.width - 8

                            Image {
                                anchors.horizontalCenter: parent.horizontalCenter
                                width: tile.iconSize
                                height: tile.iconSize
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                                source: (tile.iconPaths[index] && tile.iconPaths[index] !== "")
                                        ? "file://" + tile.iconPaths[index]
                                        : ""
                                sourceSize.width: tile.iconSize * 2
                                sourceSize.height: tile.iconSize * 2
                            }

                            Text {
                                width: parent.width
                                visible: tile.nameLines > 0
                                text: tile.names[index]
                                horizontalAlignment: Text.AlignHCenter
                                elide: tile.nameLines > 1 ? Text.ElideNone : Text.ElideMiddle
                                wrapMode: tile.nameLines > 1 ? Text.Wrap : Text.NoWrap
                                maximumLineCount: tile.nameLines > 1 ? 2 : 1
                                font.pixelSize: 11
                                color: Theme.textPrimary
                            }
                        }

                        MouseArea {
                            id: dragArea
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            drag.target: dragProxy
                            drag.threshold: 10
                            onClicked: function (mouse) {
                                grid.currentIndex = index
                                grid.forceActiveFocus()
                                if (mouse.button === Qt.RightButton) {
                                    if (!window.isSelected(index))
                                        window.selectOnly(index)
                                    window.actionIndex = index
                                    window.actionName = tile.names[index]
                                    contextMenu.popup()
                                } else if (mouse.modifiers & Qt.ControlModifier) {
                                    window.toggleSelect(index)
                                } else if (mouse.modifiers & Qt.ShiftModifier) {
                                    window.selectRange(index)
                                } else {
                                    window.selectOnly(index)
                                }
                            }
                            onDoubleClicked: tile.open_item(index)
                            onReleased: {
                                dragProxy.x = 0
                                dragProxy.y = 0
                            }
                        }
                    }
                }

                // 空白处：点击清空选择，拖拽框选。
                MouseArea {
                    id: marqueeArea
                    anchors.fill: parent
                    z: -1
                    acceptedButtons: Qt.LeftButton
                    onPressed: function (mouse) {
                        var point = mapToItem(grid, mouse.x, mouse.y)
                        window.marqueeX1 = point.x
                        window.marqueeY1 = point.y
                        window.marqueeX2 = point.x
                        window.marqueeY2 = point.y
                        window.marqueeAdditive = (mouse.modifiers & Qt.ControlModifier) !== 0
                        window.marqueeBase = window.marqueeAdditive ? window.selection.slice() : []
                        window.marqueeActive = true
                        grid.forceActiveFocus()
                    }
                    onPositionChanged: function (mouse) {
                        if (!window.marqueeActive)
                            return
                        var point = mapToItem(grid, mouse.x, mouse.y)
                        window.marqueeX2 = point.x
                        window.marqueeY2 = point.y
                        window.applyMarquee(window.marqueeX1, window.marqueeY1, point.x, point.y)
                    }
                    onReleased: {
                        var tiny = Math.abs(window.marqueeX2 - window.marqueeX1) < 4
                                   && Math.abs(window.marqueeY2 - window.marqueeY1) < 4
                        window.marqueeActive = false
                        if (tiny)
                            window.clearSelection()
                    }
                }

                // 框选矩形（视觉）
                Rectangle {
                    visible: window.marqueeActive
                    z: 10
                    x: Math.min(window.marqueeX1, window.marqueeX2)
                    y: Math.min(window.marqueeY1, window.marqueeY2)
                    width: Math.abs(window.marqueeX2 - window.marqueeX1)
                    height: Math.abs(window.marqueeY2 - window.marqueeY1)
                    color: "#223477E8"
                    border.color: "#3477E8"
                    border.width: 1
                }

                Text {
                    anchors.centerIn: parent
                    visible: tile.itemCount === 0
                    text: qsTr("这个文件夹是空的")
                    color: Theme.textTertiary
                    font.pixelSize: 12
                }
            }

            Text {
                Layout.fillWidth: true
                text: tile.statusText
                elide: Text.ElideRight
                font.pixelSize: 11
                color: Theme.textTertiary
            }
        }
    }

    // ------------------------------------------------------------ 快捷键

    Shortcut {
        sequence: "Ctrl+A"
        enabled: !window.dialogOpen
        onActivated: window.selectAll()
    }
    Shortcut {
        sequence: "Ctrl+C"
        enabled: !window.dialogOpen && window.selection.length > 0
        onActivated: tile.copy_urls(window.selectedUrls())
    }
    Shortcut {
        sequence: "Ctrl+X"
        enabled: !window.dialogOpen && window.selection.length > 0
        onActivated: tile.cut_urls(window.selectedUrls())
    }
    Shortcut {
        sequence: "Ctrl+V"
        enabled: !window.dialogOpen
        onActivated: tile.paste()
    }
    Shortcut {
        sequence: "Escape"
        enabled: !window.dialogOpen
        onActivated: window.clearSelection()
    }
    Shortcut {
        sequence: "F2"
        enabled: !window.dialogOpen && grid.currentIndex >= 0
        onActivated: window.beginRename()
    }
    Shortcut {
        sequence: "Delete"
        enabled: !window.dialogOpen && window.selection.length > 0
        onActivated: window.beginDelete()
    }

    // ------------------------------------------------------------ 菜单

    Menu {
        id: contextMenu

        MenuItem {
            text: qsTr("打开")
            enabled: window.selection.length <= 1
            onTriggered: tile.open_item(window.actionIndex)
        }
        MenuItem {
            text: qsTr("复制")
            onTriggered: tile.copy_urls(window.selectedUrls())
        }
        MenuItem {
            text: qsTr("剪切")
            onTriggered: tile.cut_urls(window.selectedUrls())
        }
        MenuItem {
            text: qsTr("粘贴")
            onTriggered: tile.paste()
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("重命名")
            enabled: window.selection.length <= 1
            onTriggered: window.beginRename()
        }
        MenuItem {
            text: qsTr("移到回收站")
            onTriggered: window.beginDelete()
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("新建文件夹")
            onTriggered: newFolderDialog.open()
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("在文件管理器中显示")
            onTriggered: tile.reveal_item(window.actionIndex)
        }
    }

    // 「+」只负责新建另一个文件格子：内容瓦片是单例，
    // 统一由设置页开关开启（见 settings.qml 的「功能格子」页）。
    Menu {
        id: newTileMenu

        MenuItem {
            text: qsTr("新建文件格子…")
            onTriggered: newTileDialog.open()
        }
        MenuItem {
            text: qsTr("更多功能格子…")
            onTriggered: app.open_settings()
        }
    }

    Menu {
        id: settingsMenu

        Menu {
            title: qsTr("图标大小")
            MenuItem {
                text: qsTr("小")
                checkable: true
                checked: tile.iconSize <= 32
                onTriggered: tile.choose_icon_size(32)
            }
            MenuItem {
                text: qsTr("中")
                checkable: true
                checked: tile.iconSize > 32 && tile.iconSize <= 48
                onTriggered: tile.choose_icon_size(48)
            }
            MenuItem {
                text: qsTr("大")
                checkable: true
                checked: tile.iconSize > 48
                onTriggered: tile.choose_icon_size(64)
            }
        }

        Menu {
            title: qsTr("文件名")
            MenuItem {
                text: qsTr("隐藏")
                checkable: true
                checked: tile.nameLines === 0
                onTriggered: tile.choose_name_lines(0)
            }
            MenuItem {
                text: qsTr("单行")
                checkable: true
                checked: tile.nameLines === 1
                onTriggered: tile.choose_name_lines(1)
            }
            MenuItem {
                text: qsTr("双行")
                checkable: true
                checked: tile.nameLines === 2
                onTriggered: tile.choose_name_lines(2)
            }
        }

        Menu {
            title: qsTr("密度")
            MenuItem {
                text: qsTr("紧凑")
                checkable: true
                checked: tile.density === "compact"
                onTriggered: tile.choose_density("compact")
            }
            MenuItem {
                text: qsTr("标准")
                checkable: true
                checked: tile.density === "normal"
                onTriggered: tile.choose_density("normal")
            }
            MenuItem {
                text: qsTr("宽松")
                checkable: true
                checked: tile.density === "comfortable"
                onTriggered: tile.choose_density("comfortable")
            }
        }

        Menu {
            title: qsTr("排序")
            MenuItem {
                text: qsTr("名称")
                checkable: true
                checked: tile.sortMode === "name"
                onTriggered: tile.choose_sort_mode("name")
            }
            MenuItem {
                text: qsTr("修改时间")
                checkable: true
                checked: tile.sortMode === "modified"
                onTriggered: tile.choose_sort_mode("modified")
            }
            MenuItem {
                text: qsTr("大小")
                checkable: true
                checked: tile.sortMode === "size"
                onTriggered: tile.choose_sort_mode("size")
            }
            MenuItem {
                text: qsTr("类型")
                checkable: true
                checked: tile.sortMode === "kind"
                onTriggered: tile.choose_sort_mode("kind")
            }
        }

        MenuSeparator {}

        MenuItem {
            text: qsTr("选择文件夹…")
            onTriggered: folderDialog.open()
        }
        MenuItem {
            text: qsTr("在文件管理器中打开")
            onTriggered: tile.reveal_folder()
        }
        Menu {
            title: qsTr("新建瓦片")
            MenuItem {
                text: qsTr("新建文件格子…")
                onTriggered: newTileDialog.open()
            }
            MenuItem {
                text: qsTr("更多功能格子…")
                onTriggered: app.open_settings()
            }
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("删除这个瓦片…")
            onTriggered: deleteTileDialog.open()
        }
    }

    // ------------------------------------------------------------ 对话框

    FolderPicker {
        id: folderDialog
        appState: app
        title: qsTr("选择本格子要显示的文件夹")
        onFolderChosen: function (path) { tile.set_folder(path) }
    }

    // 已完成待办统一在待办瓦片「筛选 → 已完成」里查看与恢复，
    // 文件格子不再掺入待办功能。

    Dialog {
        id: newFolderDialog
        title: qsTr("新建文件夹")
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        anchors.centerIn: Overlay.overlay
        width: Math.min(360, window.width - 40)

        onOpened: {
            folderField.text = qsTr("新建文件夹")
            folderField.forceActiveFocus()
            folderField.selectAll()
        }
        onAccepted: tile.create_folder(folderField.text)

        TextField {
            id: folderField
            anchors.fill: parent
            onAccepted: newFolderDialog.accept()
            Keys.onReturnPressed: function (event) {
                newFolderDialog.accept()
                event.accepted = true
            }
            Keys.onEnterPressed: function (event) {
                newFolderDialog.accept()
                event.accepted = true
            }
        }
    }

    Dialog {
        id: deleteTileDialog
        title: qsTr("删除瓦片")
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        anchors.centerIn: Overlay.overlay
        width: Math.min(380, window.width - 40)

        onAccepted: {
            app.remove_tile()
            window.close()
        }

        Label {
            anchors.fill: parent
            wrapMode: Text.Wrap
            text: qsTr("确定删除这个瓦片吗？文件夹里的内容不会被删除。")
        }
    }

    Dialog {
        id: renameDialog
        title: qsTr("重命名")
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        anchors.centerIn: Overlay.overlay
        width: Math.min(360, window.width - 40)

        onOpened: {
            renameField.text = window.actionName
            renameField.forceActiveFocus()
            renameField.selectAll()
        }
        onAccepted: tile.rename_item(window.actionIndex, renameField.text)

        TextField {
            id: renameField
            anchors.fill: parent
            placeholderText: qsTr("新名称")
            onAccepted: renameDialog.accept()
            Keys.onReturnPressed: function (event) {
                renameDialog.accept()
                event.accepted = true
            }
            Keys.onEnterPressed: function (event) {
                renameDialog.accept()
                event.accepted = true
            }
        }
    }

    Dialog {
        id: deleteDialog
        title: window.selection.length > 1 ? qsTr("移到回收站") : qsTr("移到回收站")
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        anchors.centerIn: Overlay.overlay
        width: Math.min(380, window.width - 40)

        onOpened: window.pendingDeletePaths = window.selectedPaths()
        onAccepted: {
            if (window.pendingDeletePaths.length === 0)
                return
            tile.trash_paths(window.pendingDeletePaths)
            window.clearSelection()
        }

        Label {
            anchors.fill: parent
            wrapMode: Text.Wrap
            text: window.selection.length > 1
                  ? qsTr("确定要把选中的 %1 项移到回收站吗？").arg(window.selection.length)
                  : qsTr("确定要把「%1」移到回收站吗？").arg(window.actionName)
        }
    }

    // ------------------------------------------------------------ 动作

    // 把 DropArea 坐标换算成网格里的格子下标；不在格子上返回 -1。
    function cellIndexAt(px, py) {
        var point = grid.mapFromItem(dropArea, px, py)
        if (point.x < 0 || point.y < 0 || point.x > grid.width || point.y > grid.height)
            return -1
        return grid.indexAt(point.x, point.y)
    }

    function beginRename() {
        if (window.selection.length > 1)
            return
        var index = grid.currentIndex >= 0 ? grid.currentIndex : window.actionIndex
        if (index < 0)
            return
        window.actionIndex = index
        window.actionName = tile.names[index]
        renameDialog.open()
    }

    function beginDelete() {
        if (window.selection.length === 0)
            return
        window.actionName = tile.names[grid.currentIndex >= 0 ? grid.currentIndex : window.actionIndex]
        deleteDialog.open()
    }
}
