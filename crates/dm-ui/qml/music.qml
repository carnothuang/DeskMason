import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 音乐瓦片：MPRIS2 当前播放器。
ApplicationWindow {
    id: window
    property string widgetId: ""

    visible: false
    flags: Qt.FramelessWindowHint
    color: "transparent"
    title: qsTr("DeskMason 音乐")

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
    MusicContent { id: music }

    property int hostEpoch: -1

    // 从 "1:23 / 4:56" 解析进度比例；解析不了就 0。
    readonly property real progressRatio: {
        var parts = music.progressText.split("/")
        if (parts.length !== 2)
            return 0
        function toSeconds(text) {
            var chunks = text.trim().split(":")
            var seconds = 0
            for (var i = 0; i < chunks.length; i++)
                seconds = seconds * 60 + (parseInt(chunks[i], 10) || 0)
            return seconds
        }
        var total = toSeconds(parts[1])
        if (total <= 0)
            return 0
        return Math.min(1, toSeconds(parts[0]) / total)
    }

    Component.onCompleted: {
        app.attach(window.widgetId)
        music.refresh()
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
        // 瓦片是桌面常驻组件，不允许被窗管关闭。退出走托盘。
        close.accepted = false
    }

    Timer {
        id: geometryTimer
        interval: 500
        onTriggered: app.save_geometry(window.x, window.y, window.width, window.height)
    }

    Timer {
        id: refreshTimer
        interval: 2000
        running: true
        repeat: true
        onTriggered: music.refresh()
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
            else if (command === "search")
                app.open_search()
            else if (command === "settings")
                app.open_settings()
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

        // 封面做背景（音乐瓦片样式）。
        Image {
            anchors.fill: parent
            fillMode: Image.PreserveAspectCrop
            source: music.artUrl
            opacity: 0.18
            visible: music.artUrl !== ""
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 10

            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Rectangle {
                    width: 64
                    height: 64
                    radius: 10
                    color: Theme.panelSubtle
                    clip: true

                    Image {
                        anchors.fill: parent
                        fillMode: Image.PreserveAspectCrop
                        source: music.artUrl
                        visible: music.artUrl !== ""
                    }

                    Text {
                        anchors.centerIn: parent
                        visible: music.artUrl === ""
                        text: "♪"
                        font.pixelSize: 28
                        color: Theme.accent
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Text {
                        Layout.fillWidth: true
                        text: music.title
                        elide: Text.ElideRight
                        font.pixelSize: 14
                        font.bold: true
                        color: Theme.textPrimary
                    }

                    Text {
                        Layout.fillWidth: true
                        text: music.artist
                        elide: Text.ElideRight
                        font.pixelSize: 12
                        color: Theme.textSecondary
                    }

                    Text {
                        Layout.fillWidth: true
                        text: music.album
                        elide: Text.ElideRight
                        font.pixelSize: 11
                        color: Theme.textTertiary
                        visible: text.length > 0
                    }
                }
            }

            // 进度条（主题粉色）。
            Rectangle {
                Layout.fillWidth: true
                height: 4
                radius: 2
                color: Theme.panelActive

                Rectangle {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: parent.width * window.progressRatio
                    radius: 2
                    color: "#E85A8A"
                }
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 18

                HeaderButton {
                    glyph: "⏮"
                    tip: qsTr("上一首")
                    enabled: music.canPrevious
                    onActivated: music.previous_track()
                }

                HeaderButton {
                    glyph: music.playing ? "⏸" : "▶"
                    tip: music.playing ? qsTr("暂停") : qsTr("播放")
                    onActivated: music.play_pause()
                }

                HeaderButton {
                    glyph: "⏭"
                    tip: qsTr("下一首")
                    enabled: music.canNext
                    onActivated: music.next_track()
                }
            }

            Text {
                Layout.alignment: Qt.AlignHCenter
                text: music.progressText
                font.pixelSize: 11
                color: Theme.textTertiary
                visible: text.length > 0
            }

            Text {
                Layout.fillWidth: true
                text: music.statusText
                elide: Text.ElideRight
                font.pixelSize: 10
                color: Theme.textTertiary
                visible: text.length > 0
            }
        }
    }
}
