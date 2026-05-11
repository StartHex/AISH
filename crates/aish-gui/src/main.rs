//! AISH GUI — Qt-based graphical interface for AI agent management.
//!
//! When built with cxx-qt (requires Qt6 dev libraries), this provides:
//!   - Agent list with drag-drop fan-out
//!   - Task monitor with progress charts (Qt Charts)
//!   - MCP topology graph
//!   - System tray integration
//!
//! Without Qt, falls back to launching the Python/PySide6 GUI client
//! that connects to the AISH daemon via Unix socket/TCP.

fn main() {
    // Try to import and run the native cxx-qt GUI
    #[cfg(feature = "gui-native")]
    {
        println!("Starting AISH GUI (native)...");
        aish_gui::native::run();
        return;
    }

    // Fallback: try PySide6 GUI, then suggest daemon + TUI
    eprintln!("AISH GUI: native Qt build not available.");
    eprintln!("");
    eprintln!("Options:");
    eprintln!("  1. Use the TUI:  aish tui");
    eprintln!("  2. Start the daemon + Python GUI:");
    eprintln!("     aish daemon &");
    eprintln!("     python3 crates/aish-gui/python/main.py");
    eprintln!("");
    eprintln!("  3. Build with Qt support:");
    eprintln!("     brew install qt@6");
    eprintln!("     cargo build -p aish-gui --features gui-native");
}
