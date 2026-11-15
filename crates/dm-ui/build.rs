use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    // Theme.qml 是单例（pragma Singleton），必须单独注册。
    let theme_singleton = QmlFile::from("qml/Theme.qml").singleton(true);
    CxxQtBuilder::new_qml_module(QmlModule::new("com.deskmason.probe").qml_files([
        theme_singleton,
        QmlFile::from("qml/main.qml"),
        QmlFile::from("qml/HeaderButton.qml"),
        QmlFile::from("qml/FolderPicker.qml"),
        QmlFile::from("qml/NewFileTileDialog.qml"),
        QmlFile::from("qml/WindowDragEdge.qml"),
        QmlFile::from("qml/TileSnap.qml"),
        QmlFile::from("qml/TileResizeEdge.qml"),
        QmlFile::from("qml/settings.qml"),
        QmlFile::from("qml/placeholder.qml"),
        QmlFile::from("qml/glance.qml"),
        QmlFile::from("qml/weather.qml"),
        QmlFile::from("qml/todo.qml"),
        QmlFile::from("qml/quickcapture.qml"),
        QmlFile::from("qml/search.qml"),
        QmlFile::from("qml/music.qml"),
        QmlFile::from("qml/onboarding.qml"),
    ]))
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .cpp_file("cpp/window_glue.cpp")
    .files(["src/lib.rs"])
    .build();
}
