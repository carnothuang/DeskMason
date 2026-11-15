import QtQuick
import QtQuick.Controls

// 标题栏里的小按钮：只负责外观与点击，不含业务。
Item {
    id: root
    property string glyph: "?"
    property string tip: ""
    signal activated()

    implicitWidth: 24
    implicitHeight: 24

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: hover.hovered ? Theme.panelHover : "transparent"
    }

    Text {
        anchors.centerIn: parent
        text: root.glyph
        font.pixelSize: 14
        color: Theme.textSecondary
    }

    MouseArea {
        id: hover
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.activated()
    }

    ToolTip.visible: hover.containsMouse && root.tip !== ""
    ToolTip.text: root.tip
    ToolTip.delay: 600
}
