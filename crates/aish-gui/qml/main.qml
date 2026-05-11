import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    visible: true
    width: 960
    height: 680
    title: "AISH — AI Agent Shell"

    property int currentPage: 0

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
                text: "Daemon: 127.0.0.1:9876"
                color: "green"
                font.pixelSize: 13
            }
        }
    }

    // Main content
    RowLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 12

        // --- Left sidebar ---
        Frame {
            Layout.preferredWidth: 260
            Layout.fillHeight: true
            padding: 12

            ColumnLayout {
                anchors.fill: parent
                spacing: 8

                // Nav buttons
                Repeater {
                    model: ["Dashboard", "Agents", "Tasks", "Activity", "Models", "Permissions", "Bands"]
                    Button {
                        Layout.fillWidth: true
                        text: modelData
                        flat: currentPage !== index
                        highlighted: currentPage === index
                        onClicked: currentPage = index
                    }
                }

                Item { Layout.fillHeight: true }

                // Quick actions
                Label {
                    text: "Quick Actions"
                    font.bold: true
                    font.pixelSize: 13
                    color: "gray"
                }
                Button {
                    Layout.fillWidth: true
                    text: "+ New Task"
                    onClicked: taskDialog.open()
                }
                Button {
                    Layout.fillWidth: true
                    text: "+ Add Agent"
                    onClicked: agentDialog.open()
                }
                Button {
                    Layout.fillWidth: true
                    text: "+ Create Band"
                    onClicked: bandDialog.open()
                }
            }
        }

        // --- Right content area ---
        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: currentPage

            // Page 0: Dashboard
            DashboardPage {}

            // Page 1: Agents
            AgentsPage {}

            // Page 2: Tasks
            TasksPage {}

            // Page 3: Activity
            ActivityPage {}

            // Page 4: Models
            ModelsPage {}

            // Page 5: Permissions
            PermissionsPage {}

            // Page 6: Bands
            BandsPage {}
        }
    }

    footer: ToolBar {
        RowLayout {
            Label { text: pagesNavText() }
            Item { Layout.fillWidth: true }
            Label { text: "v0.1.0" }
        }
    }

    function pagesNavText() {
        var names = ["Dashboard", "Agents", "Tasks", "Activity", "Models", "Permissions", "Bands"];
        return names[currentPage];
    }

    // ── Dialogs ──

    Dialog {
        id: taskDialog
        title: "New Task"
        standardButtons: Dialog.Ok | Dialog.Cancel
        width: 500
        ColumnLayout {
            spacing: 8
            anchors.fill: parent
            Label { text: "Prompt:" }
            TextArea {
                Layout.fillWidth: true
                Layout.preferredHeight: 120
                placeholderText: "Describe the task..."
            }
            RowLayout {
                Label { text: "Agent:" }
                ComboBox {
                    model: ["Claude Desktop", "Claude Server", "Gemini Agent"]
                }
            }
            RowLayout {
                Label { text: "Model:" }
                ComboBox {
                    model: ["claude-opus-4-7", "claude-sonnet-4-6", "gemini-3.0-pro"]
                }
            }
        }
        onAccepted: statusToast.show("Task submitted")
    }

    Dialog {
        id: agentDialog
        title: "Add Agent"
        standardButtons: Dialog.Ok | Dialog.Cancel
        width: 450
        ColumnLayout {
            spacing: 8
            anchors.fill: parent
            RowLayout {
                Label { text: "Name:"; Layout.preferredWidth: 60 }
                TextField { placeholderText: "My Agent" }
            }
            RowLayout {
                Label { text: "Type:"; Layout.preferredWidth: 60 }
                ComboBox { model: ["claude", "gemini", "mock"] }
            }
            RowLayout {
                Label { text: "Model:"; Layout.preferredWidth: 60 }
                TextField { placeholderText: "claude-sonnet-4-6" }
            }
            RowLayout {
                Label { text: "Connect:"; Layout.preferredWidth: 60 }
                ComboBox { model: ["stdio", "ssh", "tcp", "unix"] }
            }
        }
        onAccepted: statusToast.show("Agent added")
    }

    Dialog {
        id: bandDialog
        title: "Create Band"
        standardButtons: Dialog.Ok | Dialog.Cancel
        width: 450
        ColumnLayout {
            spacing: 8
            anchors.fill: parent
            RowLayout {
                Label { text: "Name:"; Layout.preferredWidth: 60 }
                TextField { placeholderText: "my-project" }
            }
            RowLayout {
                Label { text: "Isolation:"; Layout.preferredWidth: 60 }
                ComboBox { model: ["full", "shared-store", "shared-config"] }
            }
            RowLayout {
                Label { text: "Root:"; Layout.preferredWidth: 60 }
                TextField {
                    Layout.fillWidth: true
                    placeholderText: "~/bands/my-project"
                    text: "~/bands/"
                }
            }
        }
        onAccepted: statusToast.show("Band created")
    }

    // ── Toast ──
    Popup {
        id: statusToast
        parent: Overlay.overlay
        x: (parent.width - width) / 2
        y: parent.height - height - 40
        padding: 12
        background: Rectangle { color: "#333"; radius: 6 }

        property string msg: ""
        function show(message) {
            msg = message;
            open();
            toastTimer.restart();
        }

        Label { text: statusToast.msg; color: "white" }
        Timer { id: toastTimer; interval: 2000; onTriggered: statusToast.close() }
    }

    // ── Page Components ──

    component DashboardPage: Frame {
        ColumnLayout {
            anchors.fill: parent
            spacing: 16

            Label { text: "Dashboard"; font.pixelSize: 24; font.bold: true }
            Label { text: "AI Agent Shell — Unified Terminal Manager"; color: "gray"; font.pixelSize: 14 }

            RowLayout {
                spacing: 16
                Layout.fillWidth: true

                DashboardCard { title: "Agents"; value: "3"; subtitle: "2 online"; color: "#4CAF50" }
                DashboardCard { title: "Tasks"; value: "17"; subtitle: "5 running"; color: "#2196F3" }
                DashboardCard { title: "Bands"; value: "4"; subtitle: "3 active"; color: "#FF9800" }
                DashboardCard { title: "Tokens"; value: "128K"; subtitle: "today"; color: "#9C27B0" }
            }

            GroupBox {
                title: "System Info"
                GridLayout {
                    columns: 2; rowSpacing: 4; columnSpacing: 24
                    Label { text: "Daemon:" }   ; Label { text: "127.0.0.1:9876 ✓" }
                    Label { text: "Platform:" } ; Label { text: "macOS · Qt 6.11 · Rust" }
                    Label { text: "Uptime:" }   ; Label { text: "2h 34m" }
                }
            }

            Item { Layout.fillHeight: true }
        }
    }

    component DashboardCard: Rectangle {
        property string title: ""
        property string value: ""
        property string subtitle: ""
        property color cardColor: "#4CAF50"

        Layout.fillWidth: true
        height: 100
        radius: 8
        color: "#f8f8f8"
        border { color: "#ddd"; width: 1 }
        ColumnLayout {
            anchors.centerIn: parent
            Label { text: value; font.pixelSize: 28; font.bold: true; color: cardColor }
            Label { text: title; font.pixelSize: 12; color: "gray" }
            Label { text: subtitle; font.pixelSize: 11; color: "gray" }
        }
    }

    component AgentsPage: Frame {
        ColumnLayout {
            anchors.fill: parent
            Label { text: "Agents"; font.pixelSize: 20; font.bold: true }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true
                model: ListModel {
                    ListElement { name: "Claude Desktop"; status: "Online"; model: "claude-opus-4-7"; connect: "stdio"; tokens: "45.2K" }
                    ListElement { name: "Claude Server"; status: "Online"; model: "claude-sonnet-4-6"; connect: "tcp"; tokens: "12.8K" }
                    ListElement { name: "Gemini Agent"; status: "Offline"; model: "gemini-3.0-pro"; connect: "stdio"; tokens: "0" }
                }
                delegate: Frame {
                    width: ListView.view.width
                    RowLayout {
                        anchors.fill: parent
                        Rectangle { width: 10; height: 10; radius: 5; color: status === "Online" ? "green" : "gray" }
                        ColumnLayout {
                            Label { text: name; font.bold: true }
                            Label { text: status + " · " + model + " · " + connect; font.pixelSize: 12; color: "gray" }
                        }
                        Item { Layout.fillWidth: true }
                        Label { text: tokens + " tokens"; font.pixelSize: 12; color: "gray" }
                        Button { text: "Edit"; flat: true }
                        Button {
                            text: status === "Online" ? "Stop" : "Start"
                            flat: true
                            onClicked: statusToast.show(name + (status === "Online" ? " stopped" : " started"))
                        }
                    }
                }
            }
        }
    }

    component TasksPage: Frame {
        ColumnLayout {
            anchors.fill: parent
            Label { text: "Tasks"; font.pixelSize: 20; font.bold: true }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true
                model: ListModel {
                    ListElement { id: "t1"; prompt: "Refactor database layer"; agent: "Claude Desktop"; status: "Running"; progress: 65 }
                    ListElement { id: "t2"; prompt: "Write unit tests for API"; agent: "Claude Server"; status: "Running"; progress: 30 }
                    ListElement { id: "t3"; prompt: "Fix login redirect bug"; agent: "Claude Desktop"; status: "Done"; progress: 100 }
                    ListElement { id: "t4"; prompt: "Update dependencies"; agent: "Gemini Agent"; status: "Failed"; progress: 45 }
                    ListElement { id: "t5"; prompt: "Code review PR #42"; agent: "Claude Server"; status: "Queued"; progress: 0 }
                }
                delegate: Frame {
                    width: ListView.view.width
                    RowLayout {
                        anchors.fill: parent
                        ColumnLayout {
                            Layout.fillWidth: true
                            Label { text: prompt; font.bold: true }
                            Label { text: agent + " · " + status; font.pixelSize: 12; color: "gray" }
                            ProgressBar {
                                Layout.fillWidth: true
                                value: progress / 100.0
                            }
                        }
                        Button {
                            text: status === "Running" ? "Cancel" : "Re-run"
                            flat: true
                            onClicked: statusToast.show(prompt + " action triggered")
                        }
                    }
                }
            }
        }
    }

    component ActivityPage: Frame {
        ColumnLayout {
            anchors.fill: parent
            Label { text: "Activity"; font.pixelSize: 20; font.bold: true }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true
                model: ListModel {
                    ListElement { time: "11:02:15"; agent: "Claude Desktop"; tool: "read_file"; path: "src/main.rs"; result: "✓" }
                    ListElement { time: "11:02:18"; agent: "Claude Desktop"; tool: "edit_file"; path: "src/main.rs"; result: "✓" }
                    ListElement { time: "11:03:01"; agent: "Claude Server"; tool: "run_test"; path: "test_auth"; result: "✗" }
                    ListElement { time: "11:03:45"; agent: "Gemini Agent"; tool: "write_file"; path: "docs/readme.md"; result: "✓" }
                    ListElement { time: "11:04:12"; agent: "Claude Desktop"; tool: "web_fetch"; path: "docs.rs"; result: "✓" }
                }
                delegate: RowLayout {
                    width: ListView.view.width
                    Label { text: time; font.pixelSize: 12; color: "gray"; Layout.preferredWidth: 60 }
                    Label { text: agent; Layout.preferredWidth: 120; font.pixelSize: 13 }
                    Label { text: tool; Layout.preferredWidth: 100; font.pixelSize: 13 }
                    Label {
                        text: path
                        font.pixelSize: 13
                        color: "gray"
                        Layout.fillWidth: true
                        elide: Text.ElideMiddle
                    }
                    Label {
                        text: result
                        color: result === "✓" ? "green" : "red"
                        font.bold: true
                        font.pixelSize: 14
                    }
                }
            }
        }
    }

    component ModelsPage: Frame {
        ColumnLayout {
            anchors.fill: parent
            Label { text: "Models"; font.pixelSize: 20; font.bold: true }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true
                model: ListModel {
                    ListElement { model: "claude-opus-4-7"; provider: "Anthropic"; context: "200K"; cost: "$15/$75" }
                    ListElement { model: "claude-sonnet-4-6"; provider: "Anthropic"; context: "200K"; cost: "$3/$15" }
                    ListElement { model: "gemini-3.0-pro"; provider: "Google"; context: "2M"; cost: "$1.25/$5" }
                }
                delegate: Frame {
                    width: ListView.view.width
                    RowLayout {
                        anchors.fill: parent
                        ColumnLayout {
                            Layout.fillWidth: true
                            Label { text: model; font.bold: true }
                            Label { text: provider + " · " + context + " ctx · " + cost + " per MTok"; font.pixelSize: 12; color: "gray" }
                        }
                        Button {
                            text: "Set Default"
                            flat: true
                            onClicked: statusToast.show(model + " set as default")
                        }
                    }
                }
            }
        }
    }

    component PermissionsPage: Frame {
        ColumnLayout {
            anchors.fill: parent
            Label { text: "Permissions"; font.pixelSize: 20; font.bold: true }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true
                model: ListModel {
                    ListElement { tool: "read_file"; permit: "Allow"; desc: "Read any file on disk"; changed: "2026-05-10" }
                    ListElement { tool: "write_file"; permit: "Ask"; desc: "Write or modify files"; changed: "2026-05-10" }
                    ListElement { tool: "execute_command"; permit: "Ask"; desc: "Run shell commands"; changed: "2026-05-09" }
                    ListElement { tool: "web_fetch"; permit: "Allow"; desc: "Fetch URLs over HTTP"; changed: "2026-05-08" }
                    ListElement { tool: "delete_file"; permit: "Deny"; desc: "Delete files"; changed: "2026-05-08" }
                }
                delegate: Frame {
                    width: ListView.view.width
                    RowLayout {
                        anchors.fill: parent
                        ColumnLayout {
                            Layout.fillWidth: true
                            Label { text: tool; font.bold: true }
                            Label { text: desc + " · Last changed: " + changed; font.pixelSize: 12; color: "gray" }
                        }
                        Label {
                            text: permit
                            font.bold: true
                            font.pixelSize: 13
                            color: permit === "Allow" ? "green" : (permit === "Deny" ? "red" : "orange")
                        }
                        Button {
                            text: "Change"
                            flat: true
                            onClicked: statusToast.show("Permission for " + tool + " toggled")
                        }
                    }
                }
            }
        }
    }

    component BandsPage: Frame {
        ColumnLayout {
            anchors.fill: parent
            Label { text: "Bands"; font.pixelSize: 20; font.bold: true }
            ListView {
                Layout.fillWidth: true; Layout.fillHeight: true
                model: ListModel {
                    ListElement { name: "frontend-dev"; isolation: "full"; root: "~/bands/frontend-dev"; status: "Active" }
                    ListElement { name: "api-refactor"; isolation: "shared-store"; root: "~/bands/api-refactor"; status: "Active" }
                    ListElement { name: "docs-update"; isolation: "shared-config"; root: "~/bands/docs-update"; status: "Idle" }
                    ListElement { name: "archived-project"; isolation: "full"; root: "~/bands/archived"; status: "Archived" }
                }
                delegate: Frame {
                    width: ListView.view.width
                    RowLayout {
                        anchors.fill: parent
                        ColumnLayout {
                            Layout.fillWidth: true
                            Label { text: name; font.bold: true }
                            Label { text: isolation + " · " + root; font.pixelSize: 12; color: "gray" }
                        }
                        Label {
                            text: status
                            font.pixelSize: 12
                            color: status === "Active" ? "green" : "gray"
                        }
                        Button {
                            text: "Open"
                            flat: true
                            onClicked: statusToast.show("Opening band: " + name)
                        }
                        Button {
                            text: "Delete"
                            flat: true
                            onClicked: statusToast.show("Band " + name + " deleted")
                        }
                    }
                }
            }
        }
    }
}
