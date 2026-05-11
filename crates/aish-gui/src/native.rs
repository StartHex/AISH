//! Native Qt6 GUI via cxx-qt.
//!
//! Uses cxx-qt-lib's built-in wrappers for QGuiApplication, QQmlApplicationEngine,
//! and QUrl — no custom C++ headers needed. The UI is defined in QML.

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

// Minimal bridge — no custom QObjects yet.
// cxx-qt-lib types (QGuiApplication, QQmlApplicationEngine, QUrl)
// are pre-built and don't need bridge declarations.
#[cxx_qt::bridge]
pub mod bridge {
    // Reserved for custom QObject definitions (agent model, task list, etc.)
}

/// Launch the native Qt6 GUI.
pub fn run() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    // Load QML from the qml/ directory relative to Cargo.toml
    let qml_dir = format!("file://{}/qml/main.qml", env!("CARGO_MANIFEST_DIR"));
    let url = QUrl::from(&qml_dir);

    if let Some(engine) = engine.as_mut() {
        engine.load(&url);
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
