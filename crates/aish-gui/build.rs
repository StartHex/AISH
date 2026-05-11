fn main() {
    #[cfg(feature = "gui-native")]
    cxx_qt_build::CxxQtBuilder::new()
        .file("src/native.rs")
        .build();
}
