// 最小 C++ 胶水：只做 cxx-qt 0.10 没有暴露的窗口操作。
// 业务规则一律在 Rust；这里不得出现任何判断逻辑。
//
// 为什么需要它：
// - QQuickWindow 没有公开的 "set X11 window type" 接口；
// - cxx-qt-lib 0.10 没有 QWindow / QQmlComponent / rootObjects 绑定。

#include <QClipboard>
#include <QCursor>
#include <QPalette>
#include <QQuickStyle>
#include <QCoreApplication>
#include <QMimeData>
#include <QUrl>
#include <QGuiApplication>
#include <QLibraryInfo>
#include <QLocale>
#include <QTranslator>
#include <QQmlApplicationEngine>
#include <QQmlComponent>
#include <QQuickWindow>
#include <QUrl>
#include <QVariantMap>
#include <QGuiApplication>
#include <QPointer>
#include <QScreen>
#include <QWindow>
#include <QEvent>
#include <QTimer>
#include <QFont>
#include <QFontMetrics>
#include <QMap>
#include <QMetaObject>
#include <QElapsedTimer>

#include <cstdint>
#include <cstdlib>
#include <cstring>

extern "C" bool dm_apply_desktop_layer_atoms(uint64_t window_id);
extern "C" bool dm_apply_tile_layer_atoms(uint64_t window_id, bool accepts_input);
extern "C" bool dm_apply_window_blur(uint64_t window_id);
extern "C" void dm_keep_window_below_x11(uint64_t window_id);
extern "C" void dm_focus_window_x11(uint64_t window_id);
extern "C" bool dm_skip_taskbar(uint64_t window_id);
// 定义在文件后半部分；跨线程请求窗口时需要提前声明。
extern "C" uint64_t dm_spawn_settings_window(void *engine_ptr);
extern "C" uint64_t dm_spawn_search_window(void *engine_ptr);

// QQmlApplicationEngine 指针：由第一次创建瓦片时记录，供托盘线程跨线程请求
// 开设置 / 搜索窗口使用。
static void *g_engine = nullptr;

// 瓦片窗口的标志必须在 winId()（原生窗口创建）**之前**定好，映射后绝不再
// 改动。两个理由：
// 1) 任务栏（关键）：映射后任何 setFlag 都会触发 Qt 异步重写 _NET_WM_STATE
//    与 _NET_WM_WINDOW_TYPE，抹掉映射前写入的 SKIP_TASKBAR / UTILITY。
//    dde-dock 在窗口首次出现时跑一次 shouldSkip() 并缓存判定，此后补全属性
//    也撤不掉已经生成的图标。所以标志只能在原生窗口创建前定好。
// 2) 焦点：WindowDoesNotAcceptFocus 会让 Qt 写出 WM_HINTS.input=False，
//    窗管据此拒绝把键盘焦点交给瓦片——输入框点了没反应。因此只有纯展示
//    瓦片才设这个标志；输入瓦片（待办/随记/天气）不设，由 Rust 侧把
//    WM_HINTS.input 写成 True，点击时 KWin 正常聚焦。
//
// 界面上所有瓦片仍是 FramelessWindowHint + Tool：Tool 让 Qt 自己写
// _NET_WM_WINDOW_TYPE_UTILITY（dde-dock 白名单类型），Frameless 去掉窗框。
class TileFocusFilter : public QObject {
public:
    bool eventFilter(QObject *watched, QEvent *event) override {
        if (event->type() == QEvent::MouseButtonPress) {
            auto *window = qobject_cast<QWindow *>(watched);
            if (window && window->property("tileAcceptsFocus").toBool()) {
                dm_focus_window_x11(static_cast<uint64_t>(window->winId()));
            }
        }
        // 注意：这里不能拦截 QEvent::Close——设置页关闭功能瓦片走
        // dm_close_tile → window->close()，拦掉会永久失去移除瓦片的能力。
        // 防外部关闭靠两层：QML onClosing 拒绝 + Motif 提示禁用关闭
        // （窗管的礼貌协商在 KWin 侧就被拒绝；强杀路径不产生 Close 事件，
        // 软件层拦不住，只能靠 Motif 让 KWin 不发起强杀）。
        return false;
    }
};

static void dm_install_tile_focus_filter(QWindow *window) {
    static TileFocusFilter *filter = nullptr;
    if (!filter) {
        filter = new TileFocusFilter();
        filter->setParent(QCoreApplication::instance());
    }
    window->installEventFilter(filter);
}

static constexpr auto kQmlUrl = "qrc:/qt/qml/com/deskmason/probe/qml/main.qml";

// 返回 0 表示失败。窗口在 setVisible(true) 之前就已经有了原生 id，
// 所以桌面层属性可以在映射前设置好，避免"先闪一下普通窗口"。
extern "C" uint64_t dm_pin_root_window(void *engine_ptr) {
    auto *engine = static_cast<QQmlApplicationEngine *>(engine_ptr);
    if (!engine || engine->rootObjects().isEmpty()) {
        return 0;
    }
    auto *window = qobject_cast<QWindow *>(engine->rootObjects().first());
    if (!window) {
        return 0;
    }

    const WId id = window->winId();
    if (!dm_apply_desktop_layer_atoms(static_cast<uint64_t>(id))) {
        return 0;
    }

    // 同 dm_spawn_tile：桌面组件不抢 X 激活焦点，避免 fcitx5 上下文错乱。
    window->setFlag(Qt::WindowDoesNotAcceptFocus, true);

    window->setVisible(true);
    return static_cast<uint64_t>(id);
}

// 按瓦片 id 创建一个独立的 QML 窗口。每个瓦片一个窗口，互不影响。
// qml_path 决定用哪个界面（文件瓦片 / 时光 / 占位……）。
extern "C" uint64_t dm_spawn_tile(void *engine_ptr, const char *widget_id,
                                  const char *qml_path, bool accepts_focus) {
    auto *engine = static_cast<QQmlApplicationEngine *>(engine_ptr);
    g_engine = engine_ptr;   // 托盘线程请求开窗时要用到
    if (!engine || !widget_id || !qml_path) {
        return 0;
    }

    QQmlComponent component(engine, QUrl(QString::fromUtf8(qml_path)));
    if (component.isError()) {
        for (const auto &error : component.errors()) {
            qWarning("DeskMason: QML 组件加载失败: %s", qPrintable(error.toString()));
        }
        return 0;
    }

    QVariantMap properties;
    properties.insert(QStringLiteral("widgetId"), QString::fromUtf8(widget_id));
    QObject *object = component.createWithInitialProperties(properties);
    if (!object) {
        return 0;
    }

    auto *window = qobject_cast<QWindow *>(object);
    if (!window) {
        delete object;
        return 0;
    }

    // 标志必须在 winId()（原生窗口创建）**之前**定好：一旦创建后再 setFlag，
    // Qt 会异步重写 EWMH 属性，抹掉 SKIP_TASKBAR / UTILITY，令 dde-dock 在
    // 首次检查时把瓦片当成普通客户收进任务栏（且此后缓存该判定，补写属性
    // 也救不回来）。只有纯展示瓦片才禁止接受焦点；输入瓦片必须让窗管能把
    // 键盘焦点交过来，否则输入框点了没反应。
    if (!accepts_focus) {
        window->setFlag(Qt::WindowDoesNotAcceptFocus, true);
    }

    const WId id = window->winId();
    if (!dm_apply_tile_layer_atoms(static_cast<uint64_t>(id), accepts_focus)) {
        delete window;
        return 0;
    }

    window->setProperty("tileAcceptsFocus", accepts_focus);
    // 关闭拒绝依赖 QML onClosing，但实测 QML 层并不总能兜住（见下方
    // TileFocusFilter 的 Close 分支）。这里给所有瓦片统一装上事件过滤器：
    // 瓦片是桌面常驻组件，任何来源的关闭请求（任务栏/窗管快捷键）都不
    // 允许销毁窗口，退出统一走托盘菜单。
    dm_install_tile_focus_filter(window);

    // 多屏布局合法：按窗口中心所在的屏绑定 screen，Qt 的 DPI / 弹出层
    // 定位跟随瓦片实际所在的屏。setScreen 可能改写未映射窗口的几何，
    // 所以之后恢复 QML 设好的位置。
    if (QScreen *home = QGuiApplication::screenAt(
            QRect(window->x(), window->y(), window->width(), window->height()).center())) {
        const QRect before = window->geometry();
        window->setScreen(home);
        if (before.isValid()) {
            window->setGeometry(before);
        }
    }

    window->setVisible(true);

    // 不再在映射后切换 WindowDoesNotAcceptFocus：任何一次 setFlag 都会让 Qt
    // 异步重写 _NET_WM_STATE，抹掉映射前写入的 SKIP_TASKBAR，导致 dde-dock
    // 首次检查时把瓦片当普通客户收进任务栏并缓存该判定。
    // 输入瓦片的 WM_HINTS.input 已在 dm_apply_tile_layer_atoms 里写死，
    // 窗管点击时会正常聚焦；TileFocusFilter 的 dm_focus_window_x11() 是
    // 兜底，防止窗管不接手时输入框拿不到键盘事件。
    return static_cast<uint64_t>(id);
}

// 按 widget_id 找到瓦片窗口（QML 侧所有窗口都带这个动态属性）。
static QWindow *dm_find_tile(const char *widget_id) {
    if (!widget_id) {
        return nullptr;
    }
    const QString wanted = QString::fromUtf8(widget_id);
    const auto windows = QGuiApplication::topLevelWindows();
    for (QWindow *window : windows) {
        const QVariant id = window->property("widgetId");
        if (id.isValid() && id.toString() == wanted) {
            return window;
        }
    }
    return nullptr;
}

// 按 widget_id 找到瓦片窗口并把它放回 below 层（普通窗口之下）。
// 瓦片绝不能置顶：待办/随记/天气/时光曾经盖住微信、文件管理器等窗口。
extern "C" void dm_keep_tile_below(const char *widget_id) {
    if (auto *window = dm_find_tile(widget_id)) {
        dm_keep_window_below_x11(static_cast<uint64_t>(window->winId()));
    }
}

// 一次按固定顺序把所有瓦片固定回 below 层。
//
// 之前每个瓦片各自 200ms 调 StackMode::ABOVE，6 个窗口轮流抢全局栈顶，
// 既让重叠区域闪烁，也会盖住桌面上的普通窗口。现在统一走 BELOW：
// KWin 把瓦片留在 DESKTOP 层之上、普通窗口之下，点击桌面空白处也不会
// 被桌面盖住。按 widgetId 排序保证 stacking 顺序稳定，节流避免频繁请求。
extern "C" void dm_keep_tiles_below() {
    static QElapsedTimer throttle;
    if (throttle.isValid() && throttle.elapsed() < 1000) {
        return;
    }
    throttle.restart();

    QMap<QString, uint64_t> ordered;
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        const QVariant id = window->property("widgetId");
        if (id.isValid() && !id.toString().isEmpty()) {
            ordered.insert(id.toString(), static_cast<uint64_t>(window->winId()));
        }
    }
    for (auto it = ordered.constBegin(); it != ordered.constEnd(); ++it) {
        dm_keep_window_below_x11(it.value());
    }
}

// 把 Rust 侧全局「松手留缝」结算后的几何写回瓦片窗口。
// 系统拖动由 KWin 直接改窗口位置，只有回写才能让 10px 缝真正生效。
extern "C" void dm_set_tile_geometry(const char *widget_id, int x, int y,
                                     int w, int h) {
    if (auto *window = dm_find_tile(widget_id)) {
        window->setGeometry(x, y, w, h);
    }
}

// 让瓦片窗口拿到 X 输入焦点。桌面层窗口设了 WindowDoesNotAcceptFocus，
// 窗管不会激活它；用户点击输入框时由 QML 调这里，显式把键盘/输入法
// 事件路由到该窗口。只在用户明确要输入时发生，不会造成焦点反复横跳。
extern "C" void dm_focus_tile(const char *widget_id) {
    if (auto *window = dm_find_tile(widget_id)) {
        // requestActivate() 会被 Qt 对 WindowDoesNotAcceptFocus 窗口直接拒绝；
        // 真正起作用的是 X11 set_input_focus：绕过窗管把键盘/输入法事件
        // 路由过来，同时不改变窗口类型与 stacking。
        dm_focus_window_x11(static_cast<uint64_t>(window->winId()));
    }
}

// 设置页关闭某类功能瓦片时，把已开的窗口一起关掉。
extern "C" void dm_close_tile(const char *widget_id) {
    if (auto *window = dm_find_tile(widget_id)) {
        window->close();
        window->deleteLater();
    }
}

// 全局指针坐标（屏幕坐标）。磁吸拖动的参考系必须与窗口位置无关，
// QML mapToItem(null) 给的是窗口内场景坐标，窗口一动它就跟着变。
extern "C" void dm_cursor_pos(int *out) {
    const QPoint p = QCursor::pos();
    out[0] = p.x();
    out[1] = p.y();
}

// 对指定窗口发起窗口管理器原生拖动（_NET_WM_MOVERESIZE，客户端发起）。
// QML MouseArea 手动改 x/y 每步都要等 X 往返，会一顿一顿；
// 交给 KWin 后由合成器按指针同步移动，帧率与桌面一致。
// widget_id 找不到窗口时返回 false，QML 侧可退回手动拖动。
extern "C" bool dm_start_system_move(const char *widget_id) {
    if (!widget_id) {
        return false;
    }
    const QString wanted = QString::fromUtf8(widget_id);
    const auto windows = QGuiApplication::topLevelWindows();
    for (QWindow *window : windows) {
        const QVariant id = window->property("widgetId");
        if (id.isValid() && id.toString() == wanted) {
            return window->startSystemMove();
        }
    }
    return false;
}
extern "C" void dm_install_translations() {
    static QTranslator translator;
    const QLocale locale = QLocale::system();
    const QString path = QLibraryInfo::path(QLibraryInfo::TranslationsPath);
    if (translator.load(locale, QStringLiteral("qtbase"), QStringLiteral("_"), path)) {
        QCoreApplication::installTranslator(&translator);
    }
}

// ---------------------------------------------------------------- 剪贴板
//
// Linux 没有"文件剪贴板"标准类型，实际约定是：
// - text/uri-list：文件管理器都认；
// - x-special/gnome-copied-files：首行 copy/cut，决定粘贴时是复制还是移动。
// 两个都写，读的时候优先看后者。

extern "C" bool dm_clipboard_write(const char *uri_list, bool cut) {
    auto *clipboard = QGuiApplication::clipboard();
    if (!clipboard) {
        return false;
    }

    const QStringList lines =
        QString::fromUtf8(uri_list).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
    QList<QUrl> urls;
    urls.reserve(lines.size());
    for (const QString &line : lines) {
        const QUrl url(line.trimmed());
        if (url.isValid()) {
            urls.append(url);
        }
    }
    if (urls.isEmpty()) {
        // 空列表用于"粘贴剪切内容后清空剪贴板"。
        clipboard->clear();
        return true;
    }

    auto *data = new QMimeData();
    data->setUrls(urls);
    QString gnome = cut ? QStringLiteral("cut\n") : QStringLiteral("copy\n");
    for (const QUrl &url : urls) {
        gnome += url.toString() + QLatin1Char('\n');
    }
    data->setData(QStringLiteral("x-special/gnome-copied-files"), gnome.toUtf8());
    clipboard->setMimeData(data);
    return true;
}

// 返回 "copy|cut\nuri\nuri..."，由调用方用 dm_free_string 释放。无内容返回 nullptr。
extern "C" char *dm_clipboard_read() {
    auto *clipboard = QGuiApplication::clipboard();
    if (!clipboard) {
        return nullptr;
    }
    const QMimeData *data = clipboard->mimeData();
    if (!data) {
        return nullptr;
    }

    QString mode = QStringLiteral("copy");
    if (data->hasFormat(QStringLiteral("x-special/gnome-copied-files"))) {
        const QString text =
            QString::fromUtf8(data->data(QStringLiteral("x-special/gnome-copied-files")));
        if (text.startsWith(QStringLiteral("cut"))) {
            mode = QStringLiteral("cut");
        }
    }

    QString out = mode + QLatin1Char('\n');
    if (data->hasUrls()) {
        for (const QUrl &url : data->urls()) {
            out += url.toString() + QLatin1Char('\n');
        }
    } else if (data->hasFormat(QStringLiteral("x-special/gnome-copied-files"))) {
        // 只带个格式的情况（部分文件管理器这样写）：首行是 copy/cut，其余是 URI。
        const QString payload =
            QString::fromUtf8(data->data(QStringLiteral("x-special/gnome-copied-files")));
        for (const QString &line : payload.split(QLatin1Char('\n'), Qt::SkipEmptyParts)) {
            const QString trimmed = line.trimmed();
            if (trimmed.startsWith(QStringLiteral("file://"))
                || trimmed.startsWith(QLatin1Char('/'))) {
                out += trimmed + QLatin1Char('\n');
            }
        }
    } else if (data->hasText()) {
        for (const QString &line : data->text().split(QLatin1Char('\n'), Qt::SkipEmptyParts)) {
            const QString trimmed = line.trimmed();
            if (trimmed.startsWith(QStringLiteral("file://"))
                || trimmed.startsWith(QLatin1Char('/'))) {
                out += trimmed + QLatin1Char('\n');
            }
        }
    }
    if (out.size() <= mode.size() + 1) {
        return nullptr;
    }
    return strdup(out.toUtf8().constData());
}

extern "C" void dm_free_string(char *ptr) {
    free(ptr);
}

// QML 的 Qt.quit() 依赖 QQmlApplicationEngine::load() 建立的连接；我们用
// QQmlComponent 逐窗口创建，没有那条连接，所以直接退应用。
extern "C" void dm_app_quit() {
    // 必须回到 GUI 线程再退：QCoreApplication::quit() 只对调用线程自己的事件
    // 循环有效，从托盘线程直接调用会被静默忽略（表现就是"点退出没反应"）。
    // QueuedConnection 把这次调用排进主线程队列，由 exec() 所在的循环处理。
    QMetaObject::invokeMethod(
        qApp,
        [] {
            QCoreApplication::quit();
            // 实测：quit() 在这里并不能让 exec() 返回（事件循环照常派发事件），
            // 只调它等于"点了退出没反应"。所以加一道兜底：600ms 内事件循环
            // 若仍未退出，直接结束进程。
            //
            // 这么做是安全的：布局、待办、随记、偏好都是变更即时落盘，退出
            // 路径上没有待写的缓冲；窗口也不需要 onClosing（它只负责存几何，
            // 而几何每次变动都已存过）。
            QTimer::singleShot(600, qApp, [] {
                qWarning("DeskMason: 事件循环未退出，强制结束进程");
                std::_Exit(0);
            });
        },
        Qt::QueuedConnection);
}

// 从任意线程请求打开设置 / 搜索窗口。
//
// 和退出同一个道理：QQmlComponent 只能在 GUI 线程用，托盘线程直接建窗口会
// 崩。这里统一排到主线程执行。engine 指针在创建瓦片时记录到 g_engine。

extern "C" void dm_request_settings_window() {
    QMetaObject::invokeMethod(
        qApp,
        [] {
            if (g_engine) {
                dm_spawn_settings_window(g_engine);
            }
        },
        Qt::QueuedConnection);
}

extern "C" void dm_request_search_window() {
    QMetaObject::invokeMethod(
        qApp,
        [] {
            if (g_engine) {
                dm_spawn_search_window(g_engine);
            }
        },
        Qt::QueuedConnection);
}

// ---------------------------------------------------------------- 界面字体
//
// 中文界面最怕"豆腐块"：整窗文字变成一排空心方框。
//
// 成因不是编码，而是字体族解析到了一个**读不出字形**的字体文件——DDE 主题
// 给的默认族（如 Source Han Sans SC）在字体缓存重建、文件瞬时不可读、或
// 只装了不完整的子集时，Qt 拿到的是一个 0 覆盖的字体引擎，而 Qt 不会因为
// "这个族画不出中文"就自动换族，于是整窗都成了方框。
//
// 这里在启动时把候选族逐个装上、用真实字形探测（中 + A），取第一个真正有
// 覆盖的族写回应用字体。纯展示瓦片用 Text 走的就是应用字体，Qt Quick
// Controls 的控件也跟随它，所以一处设定覆盖全部窗口。
static QString dm_pick_ui_font_family() {
    // 顺序有讲究：真实的 TrueType 轮廓字体（TTC）优先，CFF/OTF 的思源黑体
    // 放后面。这台机器上 SourceHanSansSC-Regular.otf 会偶发加载失败，Qt 退化
    // 成 QFontEngineBox（每个字符都画空心方框），而且 Qt Quick Controls 一旦
    // 继承到这个族，整个窗口的文字全成了方块——这就是"设置页乱码"的成因。
    const QStringList candidates = {
        QStringLiteral("Noto Sans CJK SC"),
        QStringLiteral("Noto Sans SC"),
        QGuiApplication::font().family(),
        QStringLiteral("Source Han Sans SC"),
        QStringLiteral("思源黑体"),
        QStringLiteral("WenQuanYi Zen Hei"),
        QStringLiteral("Sans Serif"),
    };
    for (const QString &family : candidates) {
        if (family.isEmpty()) {
            continue;
        }
        QFont probe(family);
        probe.setPixelSize(16);
        const QFontMetrics metrics(probe);
        // 两个判据缺一不可：
        // 1) 中文有字形（inFont）；
        // 2) 不是"方框字体"——字体引擎完全加载失败时 Qt 会退化成 QFontEngineBox，
        //    它对任何字符都返回同一个宽度的空心方框，inFont 仍然返回 true。
        //    真实字体的 'i' 与 'W' 宽度必然不同，据此识别方框引擎。
        const bool has_cjk = metrics.inFont(QChar(0x4E2D));
        const bool is_box = metrics.horizontalAdvance(QChar('i'))
                            == metrics.horizontalAdvance(QChar('W'));
        if (has_cjk && !is_box) {
            return family;
        }
        qWarning("DeskMason: 字体族「%s」不可用（cjk=%d box=%d），试用下一个",
                 qPrintable(family), int(has_cjk), int(is_box));
    }
    return QGuiApplication::font().family();
}

extern "C" const char *dm_apply_ui_font() {
    static QByteArray chosen;
    const QString family = dm_pick_ui_font_family();
    QFont font = QGuiApplication::font();
    if (font.family() != family) {
        font.setFamily(family);
        QGuiApplication::setFont(font);
        qWarning("DeskMason: 界面字体切换为「%s」", qPrintable(family));
    }
    chosen = family.toUtf8();
    return chosen.constData();
}

// ---------------------------------------------------------------- 设置窗口
//
// 设置窗口是普通窗口（不贴桌面层）。重复点"设置"时复用同一个实例。

static QPointer<QWindow> g_settingsWindow;

extern "C" uint64_t dm_spawn_settings_window(void *engine_ptr) {
    auto *engine = static_cast<QQmlApplicationEngine *>(engine_ptr);
    if (!engine) {
        return 0;
    }

    if (g_settingsWindow) {
        g_settingsWindow->setVisible(true);
        g_settingsWindow->raise();
        g_settingsWindow->requestActivate();
        return static_cast<uint64_t>(g_settingsWindow->winId());
    }

    QQmlComponent component(
        engine, QUrl(QStringLiteral("qrc:/qt/qml/com/deskmason/probe/qml/settings.qml")));
    if (component.isError()) {
        for (const auto &error : component.errors()) {
            qWarning("DeskMason: 设置窗口加载失败: %s", qPrintable(error.toString()));
        }
        return 0;
    }

    QObject *object = component.create();
    auto *window = qobject_cast<QWindow *>(object);
    if (!window) {
        delete object;
        return 0;
    }

    if (auto *screen = window->screen()) {
        const QRect area = screen->availableGeometry();
        window->setPosition(area.center().x() - window->width() / 2,
                            area.center().y() - window->height() / 2);
    }

    // 亚克力 / 云母：映射前挂 KWin 模糊 atom。
    dm_apply_window_blur(static_cast<uint64_t>(window->winId()));
    // 常驻入口是托盘：设置窗不进任务栏。
    dm_skip_taskbar(static_cast<uint64_t>(window->winId()));

    g_settingsWindow = window;
    window->setVisible(true);
    return static_cast<uint64_t>(window->winId());
}

// ---------------------------------------------------------------- 搜索窗口

static QPointer<QWindow> g_searchWindow;

extern "C" uint64_t dm_spawn_search_window(void *engine_ptr) {
    auto *engine = static_cast<QQmlApplicationEngine *>(engine_ptr);
    if (!engine) {
        return 0;
    }
    if (g_searchWindow) {
        g_searchWindow->raise();
        g_searchWindow->requestActivate();
        return static_cast<uint64_t>(g_searchWindow->winId());
    }

    QQmlComponent component(
        engine, QUrl(QStringLiteral("qrc:/qt/qml/com/deskmason/probe/qml/search.qml")));
    if (component.isError()) {
        for (const auto &error : component.errors()) {
            qWarning("DeskMason: 搜索窗口加载失败: %s", qPrintable(error.toString()));
        }
        return 0;
    }

    QObject *object = component.create();
    auto *window = qobject_cast<QWindow *>(object);
    if (!window) {
        delete object;
        return 0;
    }

    if (auto *screen = window->screen()) {
        const QRect area = screen->availableGeometry();
        window->setPosition(area.center().x() - window->width() / 2,
                            area.center().y() - window->height() / 2);
    }

    // 亚克力 / 云母：映射前挂 KWin 模糊 atom。
    dm_apply_window_blur(static_cast<uint64_t>(window->winId()));
    // 常驻入口是托盘：搜索窗不进任务栏。
    dm_skip_taskbar(static_cast<uint64_t>(window->winId()));

    g_searchWindow = window;
    window->setVisible(true);
    return static_cast<uint64_t>(window->winId());
}

// 桌面瓦片是浅色面板，但 DDE 默认是深色主题。Qt Quick Controls 的 palette
// 在 QML 里改不干净（Window 背景与控件样式都还跟系统走），所以在进程启动时
// 统一切成 Fusion 样式 + 浅色调色板，一次覆盖所有窗口与对话框。
extern "C" void dm_apply_light_theme() {
    QQuickStyle::setStyle(QStringLiteral("Fusion"));

    QPalette palette;
    palette.setColor(QPalette::Window, QColor(0xF7, 0xF7, 0xF7));
    palette.setColor(QPalette::WindowText, QColor(0x1F, 0x1F, 0x1F));
    palette.setColor(QPalette::Base, QColor(0xFF, 0xFF, 0xFF));
    palette.setColor(QPalette::AlternateBase, QColor(0xF2, 0xF2, 0xF2));
    palette.setColor(QPalette::Text, QColor(0x20, 0x20, 0x20));
    palette.setColor(QPalette::Button, QColor(0xED, 0xED, 0xED));
    palette.setColor(QPalette::ButtonText, QColor(0x20, 0x20, 0x20));
    palette.setColor(QPalette::Highlight, QColor(0x34, 0x77, 0xE8));
    palette.setColor(QPalette::HighlightedText, QColor(0xFF, 0xFF, 0xFF));
    palette.setColor(QPalette::PlaceholderText, QColor(0x9A, 0x9A, 0x9A));
    palette.setColor(QPalette::ToolTipBase, QColor(0x2B, 0x2B, 0x2B));
    palette.setColor(QPalette::ToolTipText, QColor(0xFF, 0xFF, 0xFF));
    palette.setColor(QPalette::Disabled, QPalette::Text, QColor(0x9A, 0x9A, 0x9A));
    palette.setColor(QPalette::Disabled, QPalette::ButtonText, QColor(0x9A, 0x9A, 0x9A));
    QGuiApplication::setPalette(palette);
}

// 加载应用自己的翻译文件（.qm 由 lrelease 生成，编译进二进制后运行时落盘再加载）。
extern "C" bool dm_install_translation_file(const char *path) {
    static QTranslator translator;
    if (!translator.load(QString::fromUtf8(path))) {
        return false;
    }
    QCoreApplication::installTranslator(&translator);
    return true;
}

// ---------------------------------------------------------------- 首次引导

static QPointer<QWindow> g_onboardingWindow;

extern "C" uint64_t dm_spawn_onboarding_window(void *engine_ptr) {
    auto *engine = static_cast<QQmlApplicationEngine *>(engine_ptr);
    if (!engine) {
        return 0;
    }
    if (g_onboardingWindow) {
        g_onboardingWindow->raise();
        return static_cast<uint64_t>(g_onboardingWindow->winId());
    }

    QQmlComponent component(
        engine, QUrl(QStringLiteral("qrc:/qt/qml/com/deskmason/probe/qml/onboarding.qml")));
    if (component.isError()) {
        for (const auto &error : component.errors()) {
            qWarning("DeskMason: 引导窗口加载失败: %s", qPrintable(error.toString()));
        }
        return 0;
    }

    QObject *object = component.create();
    auto *window = qobject_cast<QWindow *>(object);
    if (!window) {
        delete object;
        return 0;
    }
    if (auto *screen = window->screen()) {
        const QRect area = screen->availableGeometry();
        window->setPosition(area.center().x() - window->width() / 2,
                            area.center().y() - window->height() / 2);
    }

    g_onboardingWindow = window;
    // 常驻入口是托盘：引导窗不进任务栏。
    dm_skip_taskbar(static_cast<uint64_t>(window->winId()));
    window->setVisible(true);
    return static_cast<uint64_t>(window->winId());
}
