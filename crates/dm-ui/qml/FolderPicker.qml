import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 纯 Qt Quick 的文件夹选择器。
//
// 为什么不用 QtQuick.Dialogs.FolderDialog：本机 DDE 的 Qt 缺少
// Qt.labs.folderlistmodel，非原生实现加载失败，原生实现又依赖 portal，
// 结果是点了"选择文件夹"什么都不弹。这里自带目录浏览：路径输入框 +
// 子目录列表，全部走 AppState 的 list_directories，不依赖任何外部模块。
Dialog {
    id: root

    property var appState
    property string currentPath: ""
    property var subfolders: []
    signal folderChosen(string path)

    title: qsTr("选择文件夹")
    modal: true
    standardButtons: Dialog.NoButton
    anchors.centerIn: Overlay.overlay
    width: 440
    height: 440

    function refresh() {
        if (!appState)
            return
        subfolders = appState.list_directories(root.currentPath)
        pathField.text = root.currentPath
    }

    function goUp() {
        var trimmed = root.currentPath.replace(/\/+$/, "")
        var parts = trimmed.split("/")
        parts.pop()
        root.currentPath = parts.join("/") || "/"
        root.refresh()
    }

    onOpened: {
        if (root.currentPath === "" && appState)
            root.currentPath = appState.home_folder()
        root.refresh()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        Label {
            Layout.fillWidth: true
            text: qsTr("输入文件夹路径，或在下方列表中浏览")
            color: Theme.textSecondary
            font.pixelSize: 12
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            TextField {
                id: pathField
                Layout.fillWidth: true
                placeholderText: qsTr("例如 /home/can/Documents")
                color: Theme.textPrimary
                placeholderTextColor: Theme.textTertiary
                background: Rectangle {
                    color: Theme.panelSubtle
                    radius: 6
                    border.width: 1
                    border.color: Theme.borderStrong
                }
                onAccepted: {
                    root.currentPath = text
                    root.refresh()
                }
            }

            Button {
                text: qsTr("转到")
                onClicked: {
                    root.currentPath = pathField.text
                    root.refresh()
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Button {
                text: qsTr("↑ 上级")
                onClicked: root.goUp()
            }

            Text {
                Layout.fillWidth: true
                text: root.currentPath
                elide: Text.ElideMiddle
                color: Theme.textTertiary
                font.pixelSize: 11
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 6
            color: Theme.panelSubtle
            border.width: 1
            border.color: Theme.border

            ListView {
                id: dirList
                anchors.fill: parent
                anchors.margins: 4
                clip: true
                model: root.subfolders
                ScrollBar.vertical: ScrollBar {}

                delegate: ItemDelegate {
                    width: dirList.width
                    text: {
                        var parts = modelData.split("/")
                        return parts[parts.length - 1] || modelData
                    }
                    onClicked: {
                        root.currentPath = modelData
                        root.refresh()
                    }
                }

                Text {
                    anchors.centerIn: parent
                    visible: root.subfolders.length === 0
                    text: qsTr("这里没有子文件夹")
                    color: Theme.textTertiary
                    font.pixelSize: 12
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Item { Layout.fillWidth: true }

            Button {
                text: qsTr("取消")
                onClicked: root.reject()
            }

            Button {
                text: qsTr("选择此文件夹")
                enabled: appState && appState.is_directory(root.currentPath)
                onClicked: {
                    root.folderChosen(root.currentPath)
                    root.accept()
                }
            }
        }
    }
}
