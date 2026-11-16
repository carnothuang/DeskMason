import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 首次运行引导：三步，只显示一次。
ApplicationWindow {
    id: window
    width: 520
    height: 380
    visible: false
    title: qsTr("欢迎使用 DeskMason")
    // 与设置/搜索窗同理：默认 Window 类型（NORMAL）不在 dde-dock 白名单，
    // 会在任务栏占一条。Tool → UTILITY，自带 SKIP_TASKBAR。
    flags: Qt.Tool | Qt.FramelessWindowHint

    palette.window: "#202022"
    palette.windowText: "#F2F2F2"
    palette.base: "#262628"
    palette.text: "#F2F2F2"
    palette.button: "#2E2E30"
    palette.buttonText: "#F2F2F2"
    palette.highlight: "#3477E8"
    palette.highlightedText: "#FFFFFF"

    Onboarding { id: onboarding }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 16

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: onboarding.page

            ColumnLayout {
                spacing: 12
                Label {
                    text: qsTr("欢迎使用 DeskMason")
                    font.pixelSize: 22
                    font.bold: true
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    text: qsTr("DeskMason 把常用文件夹和工具做成「瓦片」放在桌面上。\n文件仍然在原来的位置，不会被搬走或改变用法。")
                    font.pixelSize: 13
                }
                Item { Layout.fillHeight: true }
            }

            ColumnLayout {
                spacing: 12
                Label {
                    text: qsTr("瓦片怎么用")
                    font.pixelSize: 22
                    font.bold: true
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    text: qsTr("· 拖动标题栏移动，拖右下角调整大小\n· 拖入文件即移动到该文件夹，拖出即交还给文件管理器\n· 右键有复制 / 剪切 / 重命名 / 删除\n· 空白处拖拽可框选，Delete 移到回收站")
                    font.pixelSize: 13
                }
                Item { Layout.fillHeight: true }
            }

            ColumnLayout {
                spacing: 12
                Label {
                    text: qsTr("托盘与快捷键")
                    font.pixelSize: 22
                    font.bold: true
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    text: qsTr("· 托盘图标可显示 / 隐藏全部瓦片\n· 默认快捷键 Ctrl+Alt+D，可在设置里修改\n· 新建瓦片、设置都在托盘菜单里\n· 设置里可以打开开机自启")
                    font.pixelSize: 13
                }
                Item { Layout.fillHeight: true }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Label {
                text: qsTr("第 %1 / %2 步").arg(onboarding.page + 1).arg(onboarding.pageCount)
                font.pixelSize: 11
                opacity: 0.7
            }

            Item { Layout.fillWidth: true }

            Button {
                text: qsTr("跳过")
                palette.buttonText: "#F2F2F2"
                onClicked: window.complete()
            }

            Button {
                text: qsTr("上一步")
                palette.buttonText: "#F2F2F2"
                enabled: onboarding.page > 0
                onClicked: onboarding.previous_page()
            }

            Button {
                text: onboarding.page === onboarding.pageCount - 1 ? qsTr("开始使用") : qsTr("下一步")
                palette.buttonText: "#F2F2F2"
                onClicked: {
                    if (onboarding.page === onboarding.pageCount - 1)
                        window.complete()
                    else
                        onboarding.next_page()
                }
            }
        }
    }

    function complete() {
        onboarding.finish()
        window.close()
    }
}
