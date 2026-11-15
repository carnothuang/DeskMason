import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 天气瓦片：当前温度、天气、今日区间，城市可改。
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

    title: qsTr("DeskMason 天气")

    AppState { id: app }
    NewFileTileDialog { id: newTileDialog; appState: app }
    WeatherContent { id: weather }

    property int hostEpoch: -1

    Component.onCompleted: {
        app.attach(window.widgetId)
        weather.attach(window.widgetId)
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

    // 后台结果轮询
    Timer {
        id: pollTimer
        interval: 200
        running: true
        repeat: true
        onTriggered: weather.poll()
    }

    // 30 分钟自动刷新
    Timer {
        id: refreshTimer
        interval: 30 * 60 * 1000
        running: true
        repeat: true
        onTriggered: weather.refresh()
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
            anchors.margins: 14
            spacing: 8

            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Text {
                    Layout.fillWidth: true
                    text: weather.city
                    font.pixelSize: 14
                    font.bold: true
                    color: Theme.textPrimary
                    elide: Text.ElideRight
                }

                HeaderButton {
                    id: settingsButton
                    glyph: "⚙"
                    tip: qsTr("设置城市")
                    onActivated: {
                        cityField.text = weather.city
                        cityDialog.open()
                    }
                }
            }

            Item { Layout.fillHeight: true }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 10

                Text {
                    id: weatherIcon
                    text: weather.symbolText
                    font.pixelSize: Math.min(56, window.height * 0.32)
                    // 晴/云/雨按 emoji 原色渲染。
                    color: Theme.textPrimary
                }

                Text {
                    text: weather.temperatureText
                    font.pixelSize: Math.min(52, window.height * 0.3)
                    font.bold: true
                    color: Theme.textPrimary
                }
            }

            Text {
                Layout.alignment: Qt.AlignHCenter
                text: weather.conditionText + (weather.rangeText !== ""
                                               ? "  " + weather.rangeText : "")
                font.pixelSize: 14
                color: Theme.textSecondary
            }

            Item { Layout.fillHeight: true }

            Text {
                Layout.fillWidth: true
                text: weather.statusText !== "" ? weather.statusText : weather.updatedText
                font.pixelSize: 11
                color: weather.statusText !== "" ? Theme.danger : Theme.textTertiary
                elide: Text.ElideRight
            }
        }
    }

    Dialog {
        id: cityDialog
        title: qsTr("设置城市")
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        anchors.centerIn: Overlay.overlay
        width: Math.min(360, window.width - 40)

        onOpened: {
            cityField.forceActiveFocus()
            cityField.selectAll()
        }
        onAccepted: weather.choose_city(cityField.text)

        TextField {
            id: cityField
            anchors.fill: parent
            placeholderText: qsTr("城市名，例如 北京")
            // 桌面层窗口默认不抢 X 焦点；城市输入框聚焦时显式接管，
            // 否则中文/英文键盘事件都会落到别的应用。
            onActiveFocusChanged: {
                if (activeFocus)
                    app.focus_tile()
            }
            onAccepted: cityDialog.accept()
            Keys.onReturnPressed: function (event) {
                cityDialog.accept()
                event.accepted = true
            }
        }
    }
}
