import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    visible: true
    width: 960
    height: 680
    title: "AISH — AI Agent Shell"

    // Header
    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            Label {
                text: "AISH"
                font.pixelSize: 22
                font.bold: true
            }
            Label {
                text: "AI Agent Shell v0.1.0"
                font.pixelSize: 14
                color: "gray"
            }
            Item { Layout.fillWidth: true }
            Label {
                id: statusLabel
                text: "Daemon: disconnected"
                color: "orange"
                font.pixelSize: 13
            }
        }
    }

    // Main content
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        // Toolbar buttons
        RowLayout {
            spacing: 8
            Button { text: "Agents" }
            Button { text: "Tasks" }
            Button { text: "Activity" }
            Button { text: "Models" }
            Button { text: "Permissions" }
            Button { text: "Bands" }
        }

        // Split view
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            // Agent list (left)
            Frame {
                Layout.preferredWidth: 280
                Layout.fillHeight: true
                ColumnLayout {
                    anchors.fill: parent
                    Label {
                        text: "Agents"
                        font.pixelSize: 16
                        font.bold: true
                    }
                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        model: ListModel {
                            ListElement { name: "Claude Desktop"; status: "Online"; model: "claude-opus-4-7" }
                            ListElement { name: "Claude Server"; status: "Online"; model: "claude-sonnet-4-6" }
                            ListElement { name: "Gemini Agent"; status: "Offline"; model: "gemini-3.0-pro" }
                        }
                        delegate: ItemDelegate {
                            width: ListView.view.width
                            Column {
                                Text { text: model.name; font.bold: true }
                                Text {
                                    text: model.status + " · " + model.model
                                    color: model.status === "Online" ? "green" : "gray"
                                    font.pixelSize: 12
                                }
                            }
                        }
                    }
                }
            }

            // Detail panel (right)
            Frame {
                Layout.fillWidth: true
                Layout.fillHeight: true
                ColumnLayout {
                    anchors.fill: parent
                    spacing: 16

                    Label {
                        text: "Welcome to AISH"
                        font.pixelSize: 28
                        font.bold: true
                    }

                    Label {
                        text: "AI Agent Shell — Unified Terminal Manager for AI Coding Agents"
                        font.pixelSize: 14
                        color: "gray"
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }

                    GroupBox {
                        title: "Quick Actions"
                        Layout.fillWidth: true
                        RowLayout {
                            spacing: 8
                            Button { text: "New Task" }
                            Button { text: "Add Agent" }
                            Button { text: "Create Band" }
                            Button { text: "Fan-Out" }
                        }
                    }

                    GroupBox {
                        title: "System Info"
                        Layout.fillWidth: true
                        GridLayout {
                            columns: 2
                            rowSpacing: 4
                            columnSpacing: 16
                            Label { text: "Agents:" }
                            Label { text: "3 registered, 2 online" }
                            Label { text: "Tasks:" }
                            Label { text: "5 running, 12 completed" }
                            Label { text: "Daemon:" }
                            Label { text: "127.0.0.1:9876" }
                            Label { text: "Platform:" }
                            Label { text: "macOS (Qt6)" }
                        }
                    }

                    Item { Layout.fillHeight: true }
                }
            }
        }
    }

    footer: StatusBar {
        RowLayout {
            Label { text: "Ready" }
            Item { Layout.fillWidth: true }
            Label { text: "FPS: 60" }
        }
    }
}
