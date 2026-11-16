import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 尚未实现的内容瓦片：保证窗口能创建、能改尺寸、能删除，
// 让 M2/M3 的新类型可以一个一个补上，而不是一次性接完。
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

    title: qsTr("DeskMason 瓦片")

    AppState { id: app }
    NewFileTileDialog { id: newTileDialog; appState: app }

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

    property int hostEpoch: -1

    Rectangle {
        anchors.fill: parent
        radius: 14
        color: Theme.panel
        border.width: 1
        border.color: "#1A000000"

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 8

            Text {
                Layout.fillWidth: true
                text: qsTr("这个瓦片类型还在开发中")
                font.pixelSize: 14
                font.bold: true
                color: Theme.textPrimary
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("窗口已经就绪：位置、尺寸、显隐都正常。内容会在后续版本补上。")
                wrapMode: Text.Wrap
                font.pixelSize: 12
                color: Theme.textTertiary
            }

            Item { Layout.fillHeight: true }
        }
    }
}
