import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 随记瓦片：快速记录文本/链接，可置顶。
ApplicationWindow {
    id: window
    property string widgetId: ""

    visible: false
    flags: Qt.FramelessWindowHint
    color: "transparent"
    title: qsTr("DeskMason 随记")

    palette.window: "#202022"
    palette.windowText: "#F2F2F2"
    palette.base: "#262628"
    palette.text: "#F2F2F2"
    palette.button: "#2E2E30"
    palette.buttonText: "#F2F2F2"
    palette.highlight: "#3477E8"
    palette.highlightedText: "#FFFFFF"
    palette.placeholderText: "#8A8A8E"

    AppState { id: app }
    NewFileTileDialog { id: newTileDialog; appState: app }
    CaptureContent { id: capture }

    property int hostEpoch: -1

    Component.onCompleted: {
        app.attach(window.widgetId)
        capture.refresh()
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

                Text {
                    text: qsTr("随记")
                    font.pixelSize: 15
                    font.bold: true
                    color: Theme.textPrimary
                }

                Text {
                    text: capture.countText
                    font.pixelSize: 12
                    color: Theme.textSecondary
                }

                Item { Layout.fillWidth: true }
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
                    placeholderText: qsTr("记一条，回车确认")
                    // 桌面层窗口不抢 X 焦点：点击输入框时显式接管键盘/输入法。
                    onActiveFocusChanged: {
                        if (activeFocus)
                            app.focus_tile()
                    }
                    onAccepted: window.commit()
                    Keys.onReturnPressed: function (event) {
                        window.commit()
                        event.accepted = true
                    }
                }

                Button {
                    text: qsTr("记录")
                    palette.buttonText: "#F2F2F2"
                    contentItem: Text {
                        text: parent.text
                        color: Theme.textPrimary
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        font.pixelSize: 13
                    }
                    onClicked: window.commit()
                }
            }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: capture.ids
                spacing: 2
                ScrollBar.vertical: ScrollBar {}

                delegate: Rectangle {
                    width: list.width
                    height: 40
                    radius: 8
                    color: rowMouse.containsMouse ? Theme.accentSoft : "transparent"

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 4
                        spacing: 6

                        Text {
                            text: capture.kinds[index] === "link" ? "🔗" : "✎"
                            font.pixelSize: 13
                            color: Theme.textSecondary
                        }

                        Item {
                            Layout.fillWidth: true
                            Layout.fillHeight: true

                            Text {
                                anchors.fill: parent
                                text: capture.previews[index]
                                elide: Text.ElideRight
                                verticalAlignment: Text.AlignVCenter
                                font.pixelSize: 13
                                color: Theme.textPrimary
                            }

                            // 列表只显示一行摘要，点击这里看完整随记。
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: window.openDetail(index)
                            }
                        }

                        Text {
                            visible: capture.pinnedFlags[index] === "1"
                            text: "📌"
                            font.pixelSize: 11
                        }

                        HeaderButton {
                            glyph: capture.pinnedFlags[index] === "1" ? "★" : "☆"
                            tip: qsTr("置顶")
                            onActivated: capture.toggle_pin(capture.ids[index])
                        }

                        HeaderButton {
                            glyph: "×"
                            tip: qsTr("删除")
                            onActivated: capture.delete_item(capture.ids[index])
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
                    visible: capture.ids.length === 0
                    text: qsTr("还没有随记")
                    color: Theme.textTertiary
                    font.pixelSize: 12
                }
            }

            Text {
                Layout.fillWidth: true
                text: capture.statusText
                elide: Text.ElideRight
                font.pixelSize: 11
                color: Theme.textTertiary
            }
        }
    }

    // 列表只显示一行摘要，这里看完整正文（长文本、换行都不截断）。
    property int detailIndex: -1

    function openDetail(index) {
        window.detailIndex = index
        detailDialog.open()
    }

    Dialog {
        id: detailDialog
        title: qsTr("随记全文")
        modal: true
        standardButtons: Dialog.Close
        anchors.centerIn: Overlay.overlay
        width: Math.min(440, window.width - 30)
        height: Math.min(420, window.height - 40)

        ColumnLayout {
            anchors.fill: parent
            spacing: 8

            TextArea {
                id: detailText
                Layout.fillWidth: true
                Layout.fillHeight: true
                readOnly: true
                wrapMode: TextArea.Wrap
                selectByMouse: true
                text: window.detailIndex >= 0 ? (capture.contents[window.detailIndex] || "") : ""
                color: Theme.textPrimary
                font.pixelSize: 13
                background: Rectangle {
                    color: Theme.panelSubtle
                    radius: 6
                    border.width: 1
                    border.color: Theme.border
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Text {
                    Layout.fillWidth: true
                    text: window.detailIndex >= 0
                          ? (capture.kinds[window.detailIndex] === "link"
                             ? qsTr("链接") : qsTr("文本"))
                          : ""
                    color: Theme.textTertiary
                    font.pixelSize: 11
                }
                Button {
                    text: qsTr("复制全文")
                    onClicked: {
                        detailText.selectAll()
                        detailText.copy()
                        detailText.deselect()
                    }
                }
            }
        }
    }

    function commit() {
        var text = input.text
        if (text.trim() === "")
            return
        capture.add_text(text)
        input.text = ""
        input.forceActiveFocus()
    }
}
