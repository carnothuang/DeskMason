import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.deskmason.probe

// 设置窗口。
// 左侧导航（常规/外观/文件格子/功能格子/快捷与交互/诊断与维护/关于）
// + 顶部"搜索设置" + 右侧分组设置卡（图标 + 标题 + 描述 + 控件）。
ApplicationWindow {
    id: window
    width: 1000
    height: 760
    visible: false
    title: qsTr("DeskMason 设置")
    color: "transparent"
    // 必须用 Tool：`Qt.Window` 会让 Qt 写出 _NET_WM_WINDOW_TYPE_NORMAL 并在
    // show() 时重写 _NET_WM_STATE，抹掉映射前写入的 SKIP_TASKBAR——dde-dock
    // 的 shouldSkip() 白名单里没有 NORMAL，设置窗就会在任务栏占一条。
    // Tool 让 Qt 写 _NET_WM_WINDOW_TYPE_UTILITY（白名单类型）。
    flags: Qt.Tool | Qt.FramelessWindowHint

    // ---------- 主题（暗紫黑底 + 紫色强调） ----------
    readonly property color cBase: "#1A1116"          // 窗口底
    readonly property color cSide: "#1F141E"          // 侧栏
    readonly property color cTop: "#1C131C"           // 顶栏
    readonly property color cCard: "#2A241E"          // 设置卡
    readonly property color cCardHover: "#332C24"     // 设置卡悬停
    readonly property color cControl: "#2E2820"       // 下拉框等控件
    readonly property color cText: "#F2F0F4"
    readonly property color cTextSub: "#C9C2CC"
    readonly property color cTextDim: "#918A96"
    readonly property color cPurple: "#A94DC1"        // 开关/导航指示
    readonly property color cSelect: "#2C2135"        // 导航选中底

    palette.window: cBase
    palette.windowText: cText
    palette.base: cCard
    palette.text: cText
    palette.button: cControl
    palette.buttonText: cText
    palette.highlight: cPurple
    palette.highlightedText: "#FFFFFF"
    palette.placeholderText: cTextDim

    // 界面字体由 C++ 在启动时实测选出（跳过画不出中文的族），这里显式继承：
    // Qt Quick Controls 的文字默认跟 DDE 主题字体走，主题字体一旦落在没有
    // 字形的字体文件上，整窗就全是空心方框。AppState 只用来取这个族名。
    AppState { id: app }

    font.family: app.ui_font_family() !== "" ? app.ui_font_family()
                                             : Qt.application.font.family

    SettingsPanel {
        id: settings
    }

    // ---------- 搜索 ----------
    property string filterText: ""
    readonly property var searchIndex: [
        { page: 0, title: qsTr("开机自动启动"), desc: qsTr("登录后在后台启动 DeskMason") },
        { page: 0, title: qsTr("待办与随记附件"), desc: qsTr("关联保留原路径；复制会将文件保存到 DeskMason 数据目录") },
        { page: 0, title: qsTr("新手引导"), desc: qsTr("重新查看收纳、映射、文件安全和托盘操作说明") },
        { page: 1, title: qsTr("应用主题"), desc: qsTr("跟随系统，或固定浅色/深色") },
        { page: 1, title: qsTr("托盘图标"), desc: qsTr("选择托盘区域的图标样式") },
        { page: 1, title: qsTr("时光背景"), desc: qsTr("默认使用当前桌面壁纸，可关闭或自定义图片") },
        { page: 2, title: qsTr("文件显示"), desc: qsTr("图片预览、快捷方式标记、列表信息和路径提示") },
        { page: 2, title: qsTr("自动叠放"), desc: qsTr("把同类文件叠成一摞，点击展开") },
        { page: 2, title: qsTr("收纳与路径"), desc: qsTr("备份与数据目录") },
        { page: 2, title: qsTr("文件名"), desc: qsTr("新建文件瓦片的默认外观") },
        { page: 2, title: qsTr("排序"), desc: qsTr("新建文件瓦片的默认外观") },
        { page: 3, title: qsTr("随记"), desc: qsTr("记录剪贴板内容") },
        { page: 3, title: qsTr("待办"), desc: qsTr("任务清单") },
        { page: 3, title: qsTr("音乐"), desc: qsTr("系统媒体控制、进度和封面背景") },
        { page: 3, title: qsTr("天气"), desc: qsTr("天气预报") },
        { page: 3, title: qsTr("时光"), desc: qsTr("日历与时间信息") },
        { page: 4, title: qsTr("全局快捷键"), desc: settings.hotkey },
        { page: 4, title: qsTr("悬浮操作"), desc: qsTr("控制鼠标移入格子时显示的快捷操作") },
        { page: 4, title: qsTr("窗口与鼠标行为"), desc: qsTr("调整打开、缩放吸附和格子层级行为") },
        { page: 5, title: qsTr("本地备份"), desc: qsTr("备份布局、设置与待办/随记数据库") },
        { page: 5, title: qsTr("备份目录"), desc: qsTr("备份保存到 ~/.local/share/deskmason/backups") },
        { page: 6, title: qsTr("关于 DeskMason"), desc: qsTr("Deepin 桌面瓦片应用") },
        { page: 6, title: qsTr("数据目录"), desc: qsTr("~/.config/deskmason 与 ~/.local/share/deskmason") }
    ]
    property var searchResults: []
    onFilterTextChanged: {
        const needle = filterText.toLowerCase()
        if (needle === "") {
            searchResults = []
            return
        }
        searchResults = searchIndex.filter(function (entry) {
            return entry.title.toLowerCase().includes(needle)
                || entry.desc.toLowerCase().includes(needle)
        })
    }

    // ---------- 页面定义 ----------
    readonly property var navItems: [
        { key: "general",  label: qsTr("常规"),        glyph: "⌂" },
        { key: "look",     label: qsTr("外观"),        glyph: "★" },
        { key: "files",    label: qsTr("文件格子"),    glyph: "▤" },
        { key: "tools",    label: qsTr("功能格子"),    glyph: "◧" },
        { key: "interact", label: qsTr("快捷与交互"),  glyph: "➤" },
        { key: "diag",     label: qsTr("诊断与维护"),  glyph: "⚒" },
        { key: "about",    label: qsTr("关于"),        glyph: "ⓘ" }
    ]

    function pageLabel(page) {
        return page >= 0 && page < navItems.length ? navItems[page].label : ""
    }

    function gotoPage(index) {
        settings.currentPage = index
        settings.save_ui_state()
    }

    // =================================================================
    // 可复用部件
    // =================================================================

    // 设置卡：图标 + 标题 + 描述 + 右侧控件。
    component CardRow: Rectangle {
        id: cardRoot
        property string iconGlyph: ""
        property string titleText: ""
        property string descText: ""
        default property alias content: controlSlot.data
        Layout.fillWidth: true
        height: Math.max(66, controlSlot.childrenRect.height + 36)
        radius: 10
        color: cardHover.hovered ? window.cCardHover : window.cCard
        border.width: 1
        border.color: "#1EFFFFFF"

        HoverHandler { id: cardHover }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 18
            anchors.rightMargin: 18
            spacing: 14

            Label {
                text: cardRoot.iconGlyph
                font.pixelSize: 19
                Layout.preferredWidth: 30
                horizontalAlignment: Text.AlignHCenter
                color: window.cTextSub
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4
                Label {
                    text: cardRoot.titleText
                    font.pixelSize: 14
                    font.bold: true
                    color: window.cText
                }
                Label {
                    text: cardRoot.descText
                    visible: text.length > 0
                    font.pixelSize: 11
                    color: window.cTextDim
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                    Layout.maximumWidth: 520
                }
            }

            Item {
                id: controlSlot
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: Math.max(1, childrenRect.width)
                implicitHeight: Math.max(1, childrenRect.height)
            }
        }
    }

    // 分组标题。
    component GroupTitle: Label {
        property string titleText: ""
        text: titleText
        font.pixelSize: 15
        font.bold: true
        color: window.cText
        topPadding: 6
    }

    // 下拉框：暗色样式。labelText 由实例显式绑定显示文字。
    component PrefCombo: ComboBox {
        id: combo
        property string labelText: ""
        implicitWidth: 220
        implicitHeight: 36
        font.pixelSize: 12
        background: Rectangle {
            color: window.cControl
            radius: 8
            border.width: 1
            border.color: combo.hovered || combo.popup.visible
                          ? "#46FFFFFF" : "#24FFFFFF"
        }
        contentItem: Item {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 30
            Text {
                anchors.fill: parent
                text: combo.labelText !== "" ? combo.labelText : combo.displayText
                color: window.cText
                font.pixelSize: 12
                elide: Text.ElideRight
                verticalAlignment: Text.AlignVCenter
            }
            Text {
                anchors.right: parent.right
                anchors.rightMargin: -22
                anchors.verticalCenter: parent.verticalCenter
                text: "⌄"
                color: window.cTextSub
                font.pixelSize: 14
            }
        }
    }

    // 紫色开关。
    component PrefSwitch: Switch {
        id: sw
        indicator: Rectangle {
            implicitWidth: 46
            implicitHeight: 24
            radius: 12
            color: sw.checked ? window.cPurple : "#3A3438"
            border.width: 1
            border.color: sw.checked ? window.cPurple : "#3DFFFFFF"

            Rectangle {
                x: sw.checked ? parent.width - width - 3 : 3
                anchors.verticalCenter: parent.verticalCenter
                width: 18
                height: 18
                radius: 9
                color: sw.checked ? "#FFFFFF" : window.cTextDim
                Behavior on x { NumberAnimation { duration: 120 } }
            }
        }
    }

    // 卡片右侧的进入箭头。
    component Chevron: Label {
        text: "›"
        color: window.cTextDim
        font.pixelSize: 18
    }

    // =================================================================
    // 窗体
    // =================================================================
    Rectangle {
        id: chrome
        anchors.fill: parent
        radius: 10
        color: window.cBase
        border.width: 1
        border.color: "#24FFFFFF"

        // ---------- 顶栏 ----------
        Item {
            id: topBar
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 52

            // 空白区域拖动窗口。
            MouseArea {
                anchors.fill: parent
                onPressed: window.startSystemMove()
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 8
                spacing: 12

                RowLayout {
                    spacing: 8
                    z: 1
                    Label { text: "◆"; color: window.cPurple; font.pixelSize: 15 }
                    Label {
                        text: qsTr("DeskMason")
                        font.pixelSize: 15
                        font.bold: true
                        color: window.cText
                    }
                }

                Item { Layout.preferredWidth: 8 }

                Rectangle {
                    id: searchBox
                    Layout.fillWidth: true
                    Layout.preferredHeight: 34
                    Layout.maximumWidth: 560
                    radius: 17
                    color: window.cTop
                    border.width: 1
                    border.color: searchField.activeFocus ? window.cPurple : "#24FFFFFF"

                    TextField {
                        id: searchField
                        anchors.fill: parent
                        anchors.leftMargin: 14
                        anchors.rightMargin: 40
                        background: null
                        color: window.cText
                        font.pixelSize: 13
                        placeholderText: qsTr("搜索设置")
                        placeholderTextColor: window.cTextDim
                        verticalAlignment: TextInput.AlignVCenter
                        onTextChanged: window.filterText = text.trim()
                    }

                    Label {
                        text: "⌕"
                        color: window.cTextDim
                        font.pixelSize: 16
                        anchors.right: parent.right
                        anchors.rightMargin: 14
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }

                Item { Layout.fillWidth: true }

                HeaderButton {
                    glyph: "–"
                    onActivated: window.showMinimized()
                }
                HeaderButton {
                    glyph: "□"
                    onActivated: window.showMaximized()
                }
                HeaderButton {
                    glyph: "✕"
                    onActivated: window.close()
                }
            }
        }

        // ---------- 主体 ----------
        RowLayout {
            anchors.top: topBar.bottom
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: 0

            // ----- 左侧导航 -----
            Rectangle {
                id: sidebar
                Layout.preferredWidth: settings.menuVisible ? 240 : 64
                Layout.fillHeight: true
                color: window.cSide
                Behavior on Layout.preferredWidth { NumberAnimation { duration: 140 } }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.topMargin: 6
                    spacing: 0

                    HeaderButton {
                        glyph: "≡"
                        Layout.leftMargin: 12
                        onActivated: {
                            settings.menuVisible = !settings.menuVisible
                            settings.save_ui_state()
                        }
                    }

                    Item { Layout.preferredHeight: 6 }

                    Repeater {
                        model: window.navItems

                        delegate: Item {
                            id: navRow
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            Layout.preferredHeight: 44

                            Rectangle {
                                anchors.fill: parent
                                anchors.leftMargin: 8
                                anchors.rightMargin: 8
                                radius: 8
                                color: settings.currentPage === navRow.index
                                       ? window.cSelect
                                       : (navHover.hovered ? "#1EFFFFFF" : "transparent")
                            }

                            // 左侧紫色指示条
                            Rectangle {
                                visible: settings.currentPage === navRow.index
                                anchors.left: parent.left
                                anchors.leftMargin: 2
                                anchors.verticalCenter: parent.verticalCenter
                                width: 3
                                height: 22
                                radius: 1.5
                                color: window.cPurple
                            }

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 16
                                spacing: 12
                                Label {
                                    text: navRow.modelData.glyph
                                    font.pixelSize: 15
                                    color: window.cTextSub
                                    Layout.preferredWidth: 24
                                    horizontalAlignment: Text.AlignHCenter
                                }
                                Label {
                                    visible: settings.menuVisible
                                    text: navRow.modelData.label
                                    font.pixelSize: 13
                                    color: settings.currentPage === navRow.index
                                           ? window.cText : window.cTextSub
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }

                            HoverHandler { id: navHover }

                            TapHandler { onTapped: window.gotoPage(navRow.index) }
                        }
                    }

                    Item { Layout.fillHeight: true }
                }
            }

            // ----- 右侧内容 -----
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                ScrollView {
                    id: contentScroll
                    anchors.fill: parent
                    anchors.leftMargin: 28
                    anchors.rightMargin: 28
                    anchors.topMargin: 18
                    anchors.bottomMargin: 24
                    contentWidth: availableWidth
                    clip: true

                    ColumnLayout {
                        width: contentScroll.availableWidth
                        spacing: 10

                        Label {
                            id: pageTitle
                            text: window.navItems[settings.currentPage].label
                            font.pixelSize: 24
                            font.bold: true
                            color: window.cText
                        }

                        Item { Layout.preferredHeight: 6 }

                        // ============ 常规 ============
                        ColumnLayout {
                            visible: settings.currentPage === 0
                            spacing: 10
                            Layout.fillWidth: true

                            CardRow {
                                iconGlyph: "⚑"
                                titleText: qsTr("开机自动启动")
                                descText: qsTr("登录后在后台启动 DeskMason")
                                PrefSwitch {
                                    checked: settings.autostart
                                    onToggled: {
                                        settings.autostart = checked
                                        settings.save(hotkeyEdit.text, checked)
                                    }
                                }
                            }

                            CardRow {
                                iconGlyph: "📎"
                                titleText: qsTr("待办与随记附件")
                                descText: qsTr("关联保留原路径；复制会将文件保存到 DeskMason 数据目录")
                                PrefCombo {
                                    property var keys: ["linked", "copied"]
                                    labelText: model[currentIndex]
                                    model: [qsTr("关联原文件"), qsTr("复制到数据目录")]
                                    currentIndex: keys.indexOf(settings.attachmentMode)
                                    onActivated: {
                                        settings.attachmentMode = keys[currentIndex]
                                        settings.save_ui_state()
                                    }
                                }
                            }

                            CardRow {
                                iconGlyph: "📄"
                                titleText: qsTr("新手引导")
                                descText: qsTr("重新查看收纳、映射、文件安全和托盘操作说明")
                                Button {
                                    text: qsTr("查看")
                                    onClicked: settings.show_onboarding()
                                }
                            }
                        }

                        // ============ 外观 ============
                        ColumnLayout {
                            visible: settings.currentPage === 1
                            spacing: 10
                            Layout.fillWidth: true

                            CardRow {
                                iconGlyph: "🎨"
                                titleText: qsTr("应用主题")
                                descText: qsTr("跟随系统，或固定浅色/深色")
                                PrefCombo {
                                    property var keys: ["system", "light", "dark"]
                                    labelText: model[currentIndex]
                                    model: [qsTr("跟随系统"), qsTr("浅色"), qsTr("深色")]
                                    currentIndex: keys.indexOf(settings.appTheme)
                                    onActivated: {
                                        settings.appTheme = keys[currentIndex]
                                        settings.save_ui_state()
                                    }
                                }
                            }

                            CardRow {
                                iconGlyph: "⚙"
                                titleText: qsTr("托盘图标")
                                descText: qsTr("选择托盘区域的图标样式")
                                PrefCombo {
                                    property var keys: ["color", "mono"]
                                    labelText: model[currentIndex]
                                    model: [qsTr("彩色"), qsTr("单色")]
                                    currentIndex: keys.indexOf(settings.trayIconStyle)
                                    onActivated: {
                                        settings.trayIconStyle = keys[currentIndex]
                                        settings.save_ui_state()
                                    }
                                }
                            }

                            CardRow {
                                iconGlyph: "◷"
                                titleText: qsTr("时光背景")
                                descText: qsTr("默认使用当前桌面壁纸；可关闭或指定一张本地图片")
                                ColumnLayout {
                                    spacing: 8
                                    RowLayout {
                                        spacing: 8
                                        TextField {
                                            id: glanceBgField
                                            Layout.preferredWidth: 230
                                            text: settings.glanceBgPath
                                            placeholderText: qsTr("留空 = 当前桌面壁纸")
                                            color: window.cText
                                            placeholderTextColor: window.cTextDim
                                            font.pixelSize: 12
                                            background: Rectangle {
                                                color: window.cControl
                                                radius: 8
                                                border.width: 1
                                                border.color: "#24FFFFFF"
                                            }
                                        }
                                        Button {
                                            text: qsTr("应用")
                                            onClicked: settings.set_glance_background(
                                                glanceBgField.text, settings.glanceBgEnabled)
                                        }
                                        Button {
                                            text: qsTr("桌面壁纸")
                                            onClicked: {
                                                glanceBgField.text = ""
                                                settings.set_glance_background("", true)
                                            }
                                        }
                                    }
                                    RowLayout {
                                        spacing: 10
                                        PrefSwitch {
                                            checked: settings.glanceBgEnabled
                                            onToggled: settings.set_glance_background(
                                                settings.glanceBgPath, checked)
                                        }
                                        Label {
                                            text: qsTr("显示背景（关闭后恢复纯色面板）")
                                            color: window.cTextSub
                                            font.pixelSize: 12
                                        }
                                    }
                                }
                            }
                        }

                        // ============ 文件格子 ============
                        ColumnLayout {
                            visible: settings.currentPage === 2
                            spacing: 10
                            Layout.fillWidth: true

                            CardRow {
                                iconGlyph: "📄"
                                titleText: qsTr("文件显示")
                                descText: qsTr("图片预览、快捷方式标记、列表信息和路径提示")
                                PrefSwitch {
                                    checked: settings.fileShowPreviews
                                    onToggled: {
                                        settings.fileShowPreviews = checked
                                        settings.save_ui_state()
                                    }
                                }
                                Chevron {}
                            }

                            CardRow {
                                iconGlyph: "🗂"
                                titleText: qsTr("自动叠放")
                                descText: settings.fileAutoStack ? qsTr("已开启") : qsTr("已关闭")
                                PrefSwitch {
                                    checked: settings.fileAutoStack
                                    onToggled: {
                                        settings.fileAutoStack = checked
                                        settings.save_ui_state()
                                    }
                                }
                                Chevron {}
                            }

                            CardRow {
                                iconGlyph: "📁"
                                titleText: qsTr("收纳与路径")
                                descText: qsTr("备份与数据目录")
                                Button {
                                    text: qsTr("打开")
                                    onClicked: settings.reveal_backup_dir()
                                }
                                Chevron {}
                            }

                            GroupTitle { titleText: qsTr("新建文件瓦片的默认外观") }

                            CardRow {
                                iconGlyph: "☰"
                                titleText: qsTr("文件名")
                                descText: qsTr("新建文件瓦片的默认外观")
                                PrefCombo {
                                    labelText: model[currentIndex]
                                    model: [qsTr("隐藏"), qsTr("单行"), qsTr("双行")]
                                    currentIndex: settings.defaultNameLines
                                    onActivated: {
                                        settings.defaultNameLines = currentIndex
                                        settings.save_ui_state()
                                    }
                                }
                            }

                            CardRow {
                                iconGlyph: "⇅"
                                titleText: qsTr("排序")
                                descText: qsTr("新建文件瓦片的默认外观")
                                PrefCombo {
                                    property var keys: ["name", "modified", "size", "kind"]
                                    labelText: model[currentIndex]
                                    model: [qsTr("名称"), qsTr("修改时间"), qsTr("大小"), qsTr("类型")]
                                    currentIndex: keys.indexOf(settings.defaultSortMode)
                                    onActivated: {
                                        settings.defaultSortMode = keys[currentIndex]
                                        settings.save_ui_state()
                                    }
                                }
                            }
                        }

                        // ============ 功能格子 ============
                        ColumnLayout {
                            visible: settings.currentPage === 3
                            spacing: 10
                            Layout.fillWidth: true

                            CardRow {
                                iconGlyph: "✎"
                                titleText: qsTr("随记")
                                descText: qsTr("记录剪贴板内容；每种功能格子只能有一个")
                                PrefSwitch {
                                    checked: settings.toolCapture
                                    onToggled: settings.set_tool_enabled("quickCapture", checked)
                                }
                                Chevron {}
                            }

                            CardRow {
                                iconGlyph: "☰"
                                titleText: qsTr("待办")
                                descText: qsTr("任务清单；每种功能格子只能有一个")
                                PrefSwitch {
                                    checked: settings.toolTodo
                                    onToggled: settings.set_tool_enabled("todo", checked)
                                }
                                Chevron {}
                            }

                            CardRow {
                                iconGlyph: "♪"
                                titleText: qsTr("音乐")
                                descText: qsTr("系统媒体控制、进度和封面背景；只能有一个")
                                PrefSwitch {
                                    checked: settings.toolMusic
                                    onToggled: settings.set_tool_enabled("music", checked)
                                }
                                Chevron {}
                            }

                            CardRow {
                                iconGlyph: "☀"
                                titleText: qsTr("天气")
                                descText: qsTr("天气预报；只能有一个")
                                PrefSwitch {
                                    checked: settings.toolWeather
                                    onToggled: settings.set_tool_enabled("weather", checked)
                                }
                                Chevron {}
                            }

                            CardRow {
                                iconGlyph: "◷"
                                titleText: qsTr("时光")
                                descText: qsTr("日历与时间信息；只能有一个")
                                PrefSwitch {
                                    checked: settings.toolGlance
                                    onToggled: settings.set_tool_enabled("glance", checked)
                                }
                                Chevron {}
                            }
                        }

                        // ============ 快捷与交互 ============
                        ColumnLayout {
                            visible: settings.currentPage === 4
                            spacing: 10
                            Layout.fillWidth: true

                            CardRow {
                                iconGlyph: "⌨"
                                titleText: qsTr("全局快捷键")
                                descText: settings.hotkey
                                RowLayout {
                                    spacing: 8
                                    TextField {
                                        id: hotkeyEdit
                                        width: 200
                                        text: settings.hotkey
                                        color: window.cText
                                        font.pixelSize: 12
                                        placeholderText: qsTr("例如 ctrl+alt+d")
                                        placeholderTextColor: window.cTextDim
                                        background: Rectangle {
                                            color: window.cControl
                                            radius: 8
                                            border.width: 1
                                            border.color: hotkeyEdit.activeFocus
                                                          ? window.cPurple : "#24FFFFFF"
                                        }
                                    }
                                    Button {
                                        text: qsTr("应用")
                                        onClicked: settings.save(
                                            hotkeyEdit.text, settings.autostart)
                                    }
                                    Button {
                                        text: qsTr("恢复默认")
                                        onClicked: {
                                            hotkeyEdit.text = "ctrl+alt+d"
                                            settings.save(hotkeyEdit.text, settings.autostart)
                                        }
                                    }
                                }
                            }

                            CardRow {
                                iconGlyph: "👆"
                                titleText: qsTr("悬浮操作")
                                descText: qsTr("控制鼠标移入格子时显示的快捷操作")
                                PrefSwitch {
                                    checked: settings.hoverActionsEnabled
                                    onToggled: {
                                        settings.hoverActionsEnabled = checked
                                        settings.save_ui_state()
                                    }
                                }
                                Chevron {}
                            }

                            CardRow {
                                iconGlyph: "🖱"
                                titleText: qsTr("窗口与鼠标行为")
                                descText: qsTr("调整打开、缩放吸附和格子层级行为")
                                Chevron {}
                            }

                            Label {
                                Layout.fillWidth: true
                                text: qsTr("热键修改保存后，在下次启动 DeskMason 时生效。")
                                font.pixelSize: 11
                                color: window.cTextDim
                                wrapMode: Text.Wrap
                            }
                        }

                        // ============ 诊断与维护 ============
                        ColumnLayout {
                            visible: settings.currentPage === 5
                            spacing: 10
                            Layout.fillWidth: true

                            CardRow {
                                iconGlyph: "🗄"
                                titleText: qsTr("本地备份")
                                descText: qsTr("备份布局、设置与待办/随记数据库到 ~/.local/share/deskmason/backups")
                                RowLayout {
                                    spacing: 8
                                    Button {
                                        text: qsTr("立即备份")
                                        onClicked: settings.create_backup()
                                    }
                                    Button {
                                        text: qsTr("恢复最近一次")
                                        onClicked: settings.restore_latest_backup()
                                    }
                                }
                            }

                            CardRow {
                                iconGlyph: "📁"
                                titleText: qsTr("备份目录")
                                descText: settings.backupStatus !== ""
                                          ? settings.backupStatus
                                          : qsTr("备份保存到 ~/.local/share/deskmason/backups")
                                Button {
                                    text: qsTr("打开目录")
                                    onClicked: settings.reveal_backup_dir()
                                }
                            }
                        }

                        // ============ 关于 ============
                        ColumnLayout {
                            visible: settings.currentPage === 6
                            spacing: 10
                            Layout.fillWidth: true

                            CardRow {
                                iconGlyph: "◆"
                                titleText: qsTr("DeskMason")
                                descText: qsTr("Deepin 桌面瓦片应用。把常用文件夹和工具做成「瓦片」放在桌面上，文件仍然在原来的位置。")
                            }

                            CardRow {
                                iconGlyph: "ⓘ"
                                titleText: qsTr("版本")
                                descText: qsTr("版本 %1").arg(settings.version)
                            }

                            CardRow {
                                iconGlyph: "🗂"
                                titleText: qsTr("数据目录")
                                descText: qsTr("~/.config/deskmason 与 ~/.local/share/deskmason")
                                Button {
                                    text: qsTr("打开")
                                    onClicked: settings.reveal_backup_dir()
                                }
                            }
                        }
                    }
                }

                // ---------- 搜索结果浮层 ----------
                Rectangle {
                    anchors.fill: parent
                    color: window.cBase
                    visible: window.filterText !== ""

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 28
                        spacing: 10

                        Label {
                            text: window.searchResults.length > 0
                                  ? qsTr("找到 %1 项设置").arg(window.searchResults.length)
                                  : qsTr("没有匹配「%1」的设置").arg(window.filterText)
                            font.pixelSize: 15
                            color: window.cText
                        }

                        ListView {
                            id: resultList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            spacing: 8
                            model: window.searchResults

                            delegate: Rectangle {
                                required property var modelData
                                width: resultList.width
                                height: 58
                                radius: 10
                                color: resultHover.hovered ? window.cCardHover : window.cCard
                                border.width: 1
                                border.color: "#1EFFFFFF"

                                HoverHandler { id: resultHover }
                                TapHandler {
                                    onTapped: {
                                        window.gotoPage(modelData.page)
                                        searchField.text = ""
                                    }
                                }

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 16
                                    anchors.rightMargin: 16
                                    spacing: 12
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 3
                                        Label {
                                            text: modelData.title
                                            font.pixelSize: 13
                                            font.bold: true
                                            color: window.cText
                                        }
                                        Label {
                                            text: modelData.desc
                                            font.pixelSize: 11
                                            color: window.cTextDim
                                            elide: Text.ElideRight
                                            Layout.fillWidth: true
                                        }
                                    }
                                    Label {
                                        text: window.pageLabel(modelData.page)
                                        font.pixelSize: 11
                                        color: window.cTextDim
                                    }
                                    Chevron {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ---------- 状态提示 ----------
    Rectangle {
        id: statusToast
        property string text: settings.statusText !== ""
                              ? settings.statusText : settings.backupStatus
        visible: text !== ""
        width: childrenRect.width + 32
        height: childrenRect.height + 18
        color: "#E8262628"
        radius: 8
        border.width: 1
        border.color: "#3DFFFFFF"
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 18
        anchors.horizontalCenter: parent.horizontalCenter
        z: 10

        Label {
            x: 16
            y: 9
            text: parent.text
            color: window.cText
            font.pixelSize: 12
        }

        Timer {
            interval: 4000
            running: statusToast.visible
            onTriggered: {
                settings.statusText = ""
                settings.backupStatus = ""
            }
        }
    }

    Shortcut {
        sequence: "Esc"
        onActivated: {
            if (searchField.text !== "")
                searchField.text = ""
            else
                window.close()
        }
    }
}
