import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 时光瓦片：日期、星期、农历、节气、节日。
ApplicationWindow {
    id: window
    property string widgetId: ""

    visible: false
    flags: Qt.FramelessWindowHint
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

    title: qsTr("DeskMason 时光")

    AppState { id: app }
    NewFileTileDialog { id: newTileDialog; appState: app }
    GlanceContent { id: glance }

    property int hostEpoch: -1

    Component.onCompleted: {
        app.attach(window.widgetId)
        window.x = app.windowX
        window.y = app.windowY
        window.width = app.windowW
        window.height = app.windowH
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
        id: clockTimer
        interval: 1000
        running: true
        repeat: true
        onTriggered: glance.refresh()
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
        id: panel
        anchors.fill: parent
        radius: 14
        color: Theme.panel
        border.width: 1
        border.color: "#1A000000"
        clip: true

        // 背景默认取当前桌面壁纸，设置 → 外观里可关闭或换成自定义图片。
        // 设置改动通过 1.5s 轮询同步（跨窗口没有广播通道）。
        property string bgSource: ""
        Image {
            anchors.fill: parent
            source: panel.bgSource
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            visible: panel.bgSource !== ""
        }
        // 压暗背景，保证白色文字在任何壁纸上都清晰。
        Rectangle {
            anchors.fill: parent
            color: "#99000000"
            visible: panel.bgSource !== ""
        }

        Component.onCompleted: panel.bgSource = app.glance_background()
        Timer {
            interval: 1500
            running: true
            repeat: true
            onTriggered: {
                var next = app.glance_background()
                if (next !== panel.bgSource)
                    panel.bgSource = next
            }
        }

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
            anchors.margins: 18
            spacing: 6

            Item { Layout.fillHeight: true }

            // 背景是桌面壁纸：所有文字统一白色/近白色 + 深色描边，
            // 避免灰色小字在深色壁纸上糊成一片。
            Text {
                Layout.alignment: Qt.AlignHCenter
                text: glance.timeText
                font.pixelSize: Math.min(64, window.height * 0.28)
                font.bold: true
                color: "#FFFFFF"
                style: Text.Outline
                styleColor: "#B3000000"
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 8

                Text {
                    text: glance.dateText
                    font.pixelSize: 16
                    color: "#FFFFFF"
                    style: Text.Outline
                    styleColor: "#B3000000"
                }

                Text {
                    text: glance.weekdayText
                    font.pixelSize: 16
                    color: "#FFFFFF"
                    style: Text.Outline
                    styleColor: "#B3000000"
                }
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 8

                Text {
                    text: glance.lunarText
                    font.pixelSize: 13
                    color: "#F0F0F4"
                    style: Text.Outline
                    styleColor: "#B3000000"
                }

                // 节气 / 节日是整块瓦片里最小的字：不能只靠描边，否则在
                // 深色壁纸上会糊成一团。给近乎不透明的深色底片 + 纯白字。
                Rectangle {
                    visible: glance.termText !== ""
                    radius: 4
                    color: "#E0000000"
                    implicitWidth: termLabel.implicitWidth + 14
                    implicitHeight: termLabel.implicitHeight + 6

                    Text {
                        id: termLabel
                        anchors.centerIn: parent
                        text: glance.termText
                        font.pixelSize: 13
                        color: "#FFFFFF"
                    }
                }

                Rectangle {
                    visible: glance.festivalText !== ""
                    radius: 4
                    color: "#E0000000"
                    implicitWidth: festivalLabel.implicitWidth + 14
                    implicitHeight: festivalLabel.implicitHeight + 6

                    Text {
                        id: festivalLabel
                        anchors.centerIn: parent
                        text: glance.festivalText
                        font.pixelSize: 13
                        color: "#FFFFFF"
                    }
                }
            }

            Item { Layout.fillHeight: true }
        }
    }
}
