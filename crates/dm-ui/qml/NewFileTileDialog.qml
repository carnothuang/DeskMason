import QtQuick
import com.deskmason.probe

// 新建文件格子：必须先由用户选定要引用的文件夹。
// 直接复用当前格子的目录会产生两个内容完全一样的格子，没有意义。
FolderPicker {
    id: root

    title: qsTr("为新的文件格子选择文件夹")
    onFolderChosen: function (path) {
        if (root.appState)
            root.appState.create_file_tile(path)
    }
}
