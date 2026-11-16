import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 搜索窗：托盘"搜索…"打开，也可作为搜索格子贴在桌面。
ApplicationWindow {
    id: window
    property string widgetId: ""

    width: 620
    height: 460
    visible: false
    flags: Qt.FramelessWindowHint
    color: "transparent"
    title: qsTr("DeskMason 搜索")
    font.family: app.ui_font_family() !== "" ? app.ui_font_family()
                                             : Qt.application.font.family

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
    SearchContent { id: search }
    // 只读 prefs 拿材质档位（acrylic/mica/opaque），无副作用。
    SettingsPanel { id: settings }

    // 亚克力透亮（模糊+低不透明度），云母暗调（模糊+较高不透明度），
    // 弹窗亮度：acrylic≈102，mica≈58。
    property color panelBg: {
        if (settings.windowMaterial === "acrylic")
            return Qt.rgba(0x26/255, 0x26/255, 0x28/255, 0.30)
        if (settings.windowMaterial === "opaque")
            return Qt.rgba(0x26/255, 0x26/255, 0x28/255, 0.97)
        return Qt.rgba(0x26/255, 0x26/255, 0x28/255, 0.72)
    }

    property int hostEpoch: -1

    Component.onCompleted: {
        app.attach(window.widgetId)
        window.x = app.windowX
        window.y = app.windowY
        if (window.widgetId !== "")
            window.width = app.windowW
        if (window.widgetId !== "")
            window.height = app.windowH
        input.forceActiveFocus()
    }

    onXChanged: geometryTimer.restart()
    onYChanged: geometryTimer.restart()
    onWidthChanged: geometryTimer.restart()
    onHeightChanged: geometryTimer.restart()
    onClosing: app.save_geometry(window.x, window.y, window.width, window.height)

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

    Shortcut {
        sequence: "Escape"
        onActivated: window.close()
    }

    Rectangle {
        anchors.fill: parent
        radius: 14
        color: window.panelBg
        border.width: 1
        border.color: Theme.border

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            TextField {
                id: input
                Layout.fillWidth: true
                font.pixelSize: 16
                color: Theme.textPrimary
                placeholderTextColor: Theme.textTertiary
                selectionColor: "#3477E8"
                selectedTextColor: "#FFFFFF"
                background: Rectangle {
                    color: Theme.panelSubtle
                    radius: 8
                    border.width: 1
                    border.color: input.activeFocus ? Theme.accentBorder : Theme.border
                }
                placeholderText: qsTr("搜索文件，回车打开选中项")
                onTextChanged: {
                    // 输入即搜（300ms 防抖）。
                    debounceTimer.restart()
                }
                onAccepted: {
                    if (search.countText !== "0")
                        search.open_result(list.currentIndex >= 0 ? list.currentIndex : 0)
                    else
                        search.run(text)
                }
                Keys.onDownPressed: {
                    list.currentIndex = Math.min(list.count - 1, list.currentIndex + 1)
                    event.accepted = true
                }
                Keys.onUpPressed: {
                    list.currentIndex = Math.max(0, list.currentIndex - 1)
                    event.accepted = true
                }
                Keys.onReturnPressed: function (event) {
                    if (search.countText !== "0")
                        search.open_result(list.currentIndex >= 0 ? list.currentIndex : 0)
                    event.accepted = true
                }

                Timer {
                    id: debounceTimer
                    interval: 300
                    onTriggered: search.run(input.text)
                }
            }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: search.names
                spacing: 2
                ScrollBar.vertical: ScrollBar {}

                delegate: Rectangle {
                    width: list.width
                    height: 40
                    radius: 6
                    color: list.currentIndex === index ? Theme.accentSoft : "transparent"

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        anchors.rightMargin: 6
                        spacing: 8

                        Image {
                            Layout.preferredWidth: 18
                            Layout.preferredHeight: 18
                            Layout.alignment: Qt.AlignVCenter
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            source: (search.iconPaths[index] && search.iconPaths[index] !== "")
                                    ? "file://" + search.iconPaths[index]
                                    : ""
                            sourceSize.width: 36
                            sourceSize.height: 36
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0

                            Text {
                                Layout.fillWidth: true
                                text: search.names[index]
                                elide: Text.ElideRight
                                font.pixelSize: 13
                                color: Theme.textPrimary
                            }

                            Text {
                                Layout.fillWidth: true
                                text: search.paths[index]
                                elide: Text.ElideMiddle
                                font.pixelSize: 10
                                color: Theme.textTertiary
                            }
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: list.currentIndex = index
                        onDoubleClicked: search.open_result(index)
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                text: search.statusText !== ""
                      ? search.statusText
                      : qsTr("共 %1 项 · 双击打开").arg(search.countText)
                font.pixelSize: 11
                color: Theme.textTertiary
                elide: Text.ElideRight
            }
        }
    }
}
