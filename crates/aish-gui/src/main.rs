//! AISH GUI — Qt6-based graphical interface for AI agent management.
//!
//! With `gui-native` feature (requires Qt6 dev libraries):
//!   - Agent list with status indicators
//!   - Task monitor with progress
//!   - MCP server topology view
//!   - System tray integration
//!
//! Without Qt, falls back to suggesting TUI or Python/PySide6 GUI.

#[cfg(feature = "gui-native")]
mod native;

fn main() {
    #[cfg(feature = "gui-native")]
    {
        println!("Starting AISH GUI (native Qt6)...");
        native::run();
        return;
    }

    #[cfg(not(feature = "gui-native"))]
    {
        eprintln!("AISH GUI: native Qt build not available.");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  1. Use the TUI:  aish tui");
        eprintln!("  2. Start the daemon + Python GUI:");
        eprintln!("     aish daemon &");
        eprintln!("     python3 crates/aish-gui/python/main.py");
        eprintln!();
        eprintln!("  3. Build with Qt support:");
        eprintln!("     brew install qt@6");
        eprintln!("     cargo build -p aish-gui --features gui-native");
    }
}
