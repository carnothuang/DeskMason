import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 待办瓦片：列表 + 添加 + 完成/删除 + 筛选。
ApplicationWindow {
    id: window
    property string widgetId: ""

    visible: false
    // Qt::Tool → Qt 自己写 _NET_WM_WINDOW_TYPE_UTILITY，dde-dock 的
    // shouldSkip() 明确跳过该类型，瓦片不会进任务栏（只靠 SKIP_TASKBAR
    // 不可靠：Qt 在 show() 时会重写状态，而 dock 只在映射时判定一次）。
    flags: Qt.Tool | Qt.FramelessWindowHint
    color: "transparent"

    // 桌面瓦片是浅色面板，必须把控件调色板也固定成浅色，
    // 否则跟随系统深色主题时会出现"白底白字"。
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

    title: qsTr("DeskMason 待办")

    AppState { id: app }
    NewFileTileDialog { id: newTileDialog; appState: app }
    TodoContent { id: todo }

    property int hostEpoch: -1

    Component.onCompleted: {
        app.attach(window.widgetId)
        todo.refresh()
        window.x = app.windowX
        window.y = app.windowY
        window.width = app.windowW
        window.height = app.windowH
    }

    onXChanged: geometryTimer.restart()
    onYChanged: geometryTimer.restart()
    onWidthChanged: geometryTimer.restart()
    onHeightChanged: geometryTimer.restart()
    onClosing: function (close) {
        app.save_geometry(window.x, window.y, window.width, window.height)
        // 瓦片是桌面常驻组件，不允许被窗管/任务栏关闭。退出走托盘。
        close.accepted = false
    }

    Timer {
        id: geometryTimer
        interval: 500
        onTriggered: app.save_geometry(window.x, window.y, window.width, window.height)
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
            // 队列里只剩"新建文件格子"这类所有窗口处理一致的请求；退出 /
            // 设置 / 搜索由 Rust 直接排到 GUI 线程，不走这个队列。
            var command = app.take_host_command()
            if (command === "new-tile")
                newTileDialog.open()
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: 14
        color: Theme.panel
        border.width: 1
        border.color: "#1A000000"

        // 顶部拖动把手：桌面层窗口窗管不管移动。
        WindowDragEdge {
            targetWindow: window
            appState: app
        }

        // 四边缩放边条：靠近其他瓦片边界时磁吸并留出间隔。
        TileResizeEdge {
            targetWindow: window
            appState: app
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Text {
                    text: todo.filter === "done" ? qsTr("待办 · 已完成") : qsTr("待办")
                    font.pixelSize: 15
                    font.bold: true
                    color: Theme.textPrimary
                }

                Text {
                    text: todo.countText
                    font.pixelSize: 12
                    color: Theme.textSecondary
                }

                Item { Layout.fillWidth: true }

                // 未完成 / 全部 / 已完成 三种视图统一走这个齿轮菜单，
                // 不再另设「已完成」按钮，避免两个入口做同一件事。
                HeaderButton {
                    id: filterButton
                    glyph: "⚙"
                    tip: qsTr("筛选：未完成 / 全部 / 已完成")
                    onActivated: filterMenu.popup(filterButton, 0, filterButton.height + 4)
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                TextField {
                    id: input
                    Layout.fillWidth: true
                    color: Theme.textPrimary
                    placeholderTextColor: Theme.textTertiary
                    selectionColor: "#3477E8"
                    selectedTextColor: "#FFFFFF"
                    background: Rectangle {
                        color: Theme.panelSubtle
                        radius: 6
                        border.width: 1
                        border.color: input.activeFocus ? "#3477E8" : Theme.borderStrong
                    }
                    placeholderText: qsTr("添加待办，回车确认")
                    // 桌面层窗口不抢 X 焦点：点击输入框时显式接管，
                    // 否则键盘/输入法事件会落到别的应用（表现为无法输入）。
                    onActiveFocusChanged: {
                        if (activeFocus)
                            app.focus_tile()
                    }
                    onAccepted: window.commitTask()
                    Keys.onReturnPressed: function (event) {
                        window.commitTask()
                        event.accepted = true
                    }
                }

                Button {
                    text: qsTr("添加")
                    palette.buttonText: "#F2F2F2"
                    contentItem: Text {
                        text: parent.text
                        color: Theme.textPrimary
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        font.pixelSize: 13
                    }
                    onClicked: window.commitTask()
                }
            }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: todo.ids
                spacing: 2

                ScrollBar.vertical: ScrollBar {}

                delegate: Rectangle {
                    width: list.width
                    height: 36
                    radius: 8
                    color: rowMouse.containsMouse ? Theme.accentSoft : "transparent"

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 4
                        spacing: 10

                        // 自绘圆形勾选（主题紫色）。
                        Item {
                            width: 20
                            height: 20
                            Layout.alignment: Qt.AlignVCenter

                            Rectangle {
                                anchors.fill: parent
                                radius: 10
                                color: todo.doneFlags[index] === "1"
                                       ? "#A94DC1" : "transparent"
                                border.width: 1.5
                                border.color: todo.doneFlags[index] === "1"
                                              ? "#A94DC1" : Theme.borderStrong
                            }

                            Text {
                                anchors.centerIn: parent
                                text: "✓"
                                font.pixelSize: 12
                                font.bold: true
                                color: "#FFFFFF"
                                visible: todo.doneFlags[index] === "1"
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: todo.toggle_task(todo.ids[index])
                            }
                        }

                        // 颜色标记：未设色时显示细描边，设色后显示色条。
                        Rectangle {
                            Layout.fillHeight: true
                            Layout.topMargin: 7
                            Layout.bottomMargin: 7
                            width: 4
                            radius: 2
                            color: (todo.colors[index] || "") !== "" ? todo.colors[index] : "transparent"
                            border.width: (todo.colors[index] || "") !== "" ? 0 : 1
                            border.color: Theme.borderStrong
                        }

                        Item {
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            Text {
                                anchors.fill: parent
                                text: todo.titles[index]
                                elide: Text.ElideRight
                                verticalAlignment: Text.AlignVCenter
                                font.pixelSize: 13
                                color: todo.doneFlags[index] === "1"
                                       ? Theme.textTertiary : Theme.textPrimary
                                font.strikeout: todo.doneFlags[index] === "1"
                            }

                            // 点击整行查看完整标题、备注、到期与步骤。
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: window.openDetail(index)
                            }
                        }

                        Text {
                            text: todo.dueTexts[index]
                            visible: todo.dueTexts[index] !== ""
                            font.pixelSize: 11
                            color: Theme.dueText
                        }

                        HeaderButton {
                            glyph: "×"
                            tip: qsTr("删除")
                            onActivated: todo.delete_task(todo.ids[index])
                        }
                    }

                    MouseArea {
                        id: rowMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }
                }

                Text {
                    anchors.centerIn: parent
                    visible: todo.ids.length === 0
                    text: todo.filter === "done" ? qsTr("还没有完成的任务") : qsTr("暂无待办")
                    color: Theme.textTertiary
                    font.pixelSize: 12
                }
            }

            Text {
                Layout.fillWidth: true
                text: todo.statusText
                elide: Text.ElideRight
                font.pixelSize: 11
                color: Theme.textTertiary
            }
        }
    }

    Menu {
        id: filterMenu
        MenuItem {
            text: qsTr("未完成")
            checkable: true
            checked: todo.filter === "open"
            onTriggered: todo.choose_filter("open")
        }
        MenuItem {
            text: qsTr("全部")
            checkable: true
            checked: todo.filter === "all"
            onTriggered: todo.choose_filter("all")
        }
        MenuItem {
            text: qsTr("已完成")
            checkable: true
            checked: todo.filter === "done"
            onTriggered: todo.choose_filter("done")
        }
    }

    // 单条详情：完整标题 / 备注 / 到期 / 步骤 + 颜色色板。
    property int detailIndex: -1
    readonly property var paletteColors: [
        "", "#A94DC1", "#E85A8A", "#F0A35C", "#4CAF6D", "#3477E8", "#8A8A8E"
    ]

    function openDetail(index) {
        window.detailIndex = index
        detailDialog.open()
    }

    function pickColor(color) {
        if (window.detailIndex < 0)
            return
        todo.set_task_color(todo.ids[window.detailIndex], color)
        detailDialog.close()
    }

    Dialog {
        id: detailDialog
        title: qsTr("待办详情")
        modal: true
        standardButtons: Dialog.Close
        anchors.centerIn: Overlay.overlay
        width: Math.min(420, window.width - 30)
        height: Math.min(460, window.height - 40)

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            Text {
                Layout.fillWidth: true
                text: window.detailIndex >= 0 ? (todo.titles[window.detailIndex] || "") : ""
                wrapMode: Text.Wrap
                font.pixelSize: 16
                font.bold: true
                color: Theme.textPrimary
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Rectangle {
                    width: 10
                    height: 10
                    radius: 5
                    color: (window.detailIndex >= 0
                            && (todo.colors[window.detailIndex] || "") !== "")
                           ? todo.colors[window.detailIndex] : Theme.borderStrong
                }
                Text {
                    Layout.fillWidth: true
                    text: (window.detailIndex >= 0
                           && (todo.dueTexts[window.detailIndex] || "") !== "")
                          ? qsTr("到期：%1").arg(todo.dueTexts[window.detailIndex])
                          : qsTr("无到期时间")
                    color: Theme.dueText
                    font.pixelSize: 12
                }
            }

            TextArea {
                Layout.fillWidth: true
                Layout.fillHeight: true
                readOnly: true
                wrapMode: TextArea.Wrap
                selectByMouse: true
                text: (window.detailIndex >= 0
                       && (todo.notes[window.detailIndex] || "") !== "")
                      ? todo.notes[window.detailIndex] : qsTr("（无备注）")
                color: Theme.textPrimary
                font.pixelSize: 13
                background: Rectangle {
                    color: Theme.panelSubtle
                    radius: 6
                    border.width: 1
                    border.color: Theme.border
                }
            }

            Text {
                Layout.fillWidth: true
                visible: window.detailIndex >= 0
                         && (todo.stepTexts[window.detailIndex] || "") !== ""
                text: window.detailIndex >= 0
                      ? (todo.stepTexts[window.detailIndex] || "") : ""
                wrapMode: Text.Wrap
                color: Theme.textSecondary
                font.pixelSize: 12
            }

            Text {
                text: qsTr("颜色标记（× 表示清除）")
                color: Theme.textTertiary
                font.pixelSize: 11
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Repeater {
                    model: window.paletteColors
                    delegate: Rectangle {
                        required property var modelData
                        width: 26
                        height: 26
                        radius: 13
                        color: modelData === "" ? "transparent" : modelData
                        border.width: 1
                        border.color: modelData === "" ? Theme.borderStrong : "#33FFFFFF"
                        Text {
                            anchors.centerIn: parent
                            visible: modelData === ""
                            text: "×"
                            color: Theme.textTertiary
                            font.pixelSize: 14
                        }
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: window.pickColor(modelData)
                        }
                    }
                }
            }
        }
    }

    function commitTask() {
        var text = input.text
        if (text.trim() === "")
            return
        todo.add_task(text)
        input.text = ""
        input.forceActiveFocus()
    }
}
