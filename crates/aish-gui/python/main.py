#!/usr/bin/env python3
"""AISH GUI — PySide6 fallback client that connects to the AISH daemon.

Connects to aishd via Unix socket (default: ~/.aish/daemon.sock)
or TCP (default: 127.0.0.1:9876) using the MCP JSON-RPC 2.0 protocol.

Requires: pip install PySide6
"""

import json
import os
import socket
import sys
from pathlib import Path

try:
    from PySide6.QtCore import QTimer, Qt
    from PySide6.QtWidgets import (
        QApplication,
        QHBoxLayout,
        QLabel,
        QListWidget,
        QListWidgetItem,
        QMainWindow,
        QPushButton,
        QSplitter,
        QTabWidget,
        QTableWidget,
        QTableWidgetItem,
        QTextEdit,
        QVBoxLayout,
        QWidget,
        QLineEdit,
        QHeaderView,
        QStatusBar,
    )
except ImportError:
    print("PySide6 is required. Install with: pip install PySide6")
    sys.exit(1)


class McpClient:
    """Minimal MCP JSON-RPC client over Unix socket or TCP."""

    def __init__(self, socket_path=None, tcp_host="127.0.0.1", tcp_port=9876):
        self.socket_path = socket_path or os.path.expanduser("~/.aish/daemon.sock")
        self.tcp_host = tcp_host
        self.tcp_port = tcp_port
        self.sock = None
        self.buffer = b""
        self._id = 0

    def _next_id(self):
        self._id += 1
        return self._id

    def connect(self):
        # Try Unix socket first
        if Path(self.socket_path).exists():
            self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            self.sock.connect(self.socket_path)
        else:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.connect((self.tcp_host, self.tcp_port))

        # Handshake: initialize
        self._send(
            {
                "jsonrpc": "2.0",
                "id": self._next_id(),
                "method": "initialize",
                "params": {
                    "protocolVersion": "0.1",
                    "capabilities": {},
                    "clientInfo": {"name": "aish-gui", "version": "0.1.0"},
                },
            }
        )
        resp = self._recv()
        if "result" in resp:
            server = resp["result"].get("serverInfo", {})
            print(f"Connected to {server.get('name', 'unknown')} v{server.get('version', '?')}")
            # Send initialized notification
            self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        else:
            print(f"Handshake warning: {resp}")

    def _send(self, msg):
        data = json.dumps(msg).encode() + b"\n"
        self.sock.sendall(data)

    def _recv(self):
        while b"\n" not in self.buffer:
            chunk = self.sock.recv(4096)
            if not chunk:
                raise ConnectionError("Connection closed")
            self.buffer += chunk
        line, self.buffer = self.buffer.split(b"\n", 1)
        return json.loads(line)

    def call(self, method, params=None):
        req = {
            "jsonrpc": "2.0",
            "id": self._next_id(),
            "method": method,
            "params": params or {},
        }
        self._send(req)
        return self._recv()

    def close(self):
        if self.sock:
            self.sock.close()
            self.sock = None


class MainWindow(QMainWindow):
    def __init__(self, client: McpClient):
        super().__init__()
        self.client = client
        self.setWindowTitle("AISH — AI Agent Shell")
        self.resize(1200, 800)

        central = QWidget()
        self.setCentralWidget(central)
        layout = QVBoxLayout(central)

        # Toolbar
        toolbar = QHBoxLayout()
        self.refresh_btn = QPushButton("Refresh")
        self.refresh_btn.clicked.connect(self.refresh_all)
        toolbar.addWidget(self.refresh_btn)
        toolbar.addStretch()
        layout.addLayout(toolbar)

        splitter = QSplitter(Qt.Horizontal)

        # Agent list (left)
        agent_panel = QWidget()
        agent_layout = QVBoxLayout(agent_panel)
        agent_layout.addWidget(QLabel("Agents"))
        self.agent_list = QListWidget()
        self.agent_list.currentRowChanged.connect(self.on_agent_selected)
        agent_layout.addWidget(self.agent_list)
        splitter.addWidget(agent_panel)

        # Tab widget (right)
        self.tabs = QTabWidget()
        self.task_table = QTableWidget(0, 4)
        self.task_table.setHorizontalHeaderLabels(["ID", "Agent", "Prompt", "Status"])
        self.task_table.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        self.tabs.addTab(self.task_table, "Tasks")

        self.activity_text = QTextEdit()
        self.activity_text.setReadOnly(True)
        self.tabs.addTab(self.activity_text, "Activity")

        self.tools_table = QTableWidget(0, 3)
        self.tools_table.setHorizontalHeaderLabels(["Tool", "Description", "Schema"])
        self.tools_table.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        self.tabs.addTab(self.tools_table, "Tools")

        splitter.addWidget(self.tabs)
        splitter.setSizes([300, 900])
        layout.addWidget(splitter)

        # Status bar
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)
        self.status_bar.showMessage("Ready")

        # Command bar
        cmd_layout = QHBoxLayout()
        self.cmd_input = QLineEdit()
        self.cmd_input.setPlaceholderText("Type a command (e.g., :agent.list, :task.list)...")
        self.cmd_input.returnPressed.connect(self.on_command)
        cmd_layout.addWidget(self.cmd_input)
        self.send_btn = QPushButton("Send")
        self.send_btn.clicked.connect(self.on_command)
        cmd_layout.addWidget(self.send_btn)
        layout.addLayout(cmd_layout)

        self.refresh_all()

    def refresh_all(self):
        self.load_agents()
        self.load_tasks()

    def load_agents(self):
        try:
            resp = self.client.call("agent.list")
            agents = json.loads(resp.get("result", {}).get("content", [{}])[0].get("text", "[]"))
            self.agent_list.clear()
            for agent in agents:
                item = QListWidgetItem(f"{agent['id']} — {agent['status']}")
                self.agent_list.addItem(item)
            self.status_bar.showMessage(f"{len(agents)} agents loaded")
        except Exception as e:
            self.status_bar.showMessage(f"Error: {e}")

    def load_tasks(self):
        try:
            resp = self.client.call("task.list")
            tasks = json.loads(resp.get("result", {}).get("content", [{}])[0].get("text", "[]"))
            self.task_table.setRowCount(len(tasks))
            for i, task in enumerate(tasks):
                self.task_table.setItem(i, 0, QTableWidgetItem(task.get("id", "")))
                self.task_table.setItem(i, 1, QTableWidgetItem(task.get("agent", "")))
                self.task_table.setItem(i, 2, QTableWidgetItem(task.get("prompt", "")))
                self.task_table.setItem(i, 3, QTableWidgetItem(task.get("status", "")))
        except Exception as e:
            self.status_bar.showMessage(f"Error: {e}")

    def on_agent_selected(self, row):
        if row >= 0:
            item = self.agent_list.item(row)
            self.status_bar.showMessage(f"Selected: {item.text()}")

    def on_command(self):
        cmd = self.cmd_input.text().strip()
        if not cmd:
            return

        # Parse simple commands: :method
        if cmd.startswith(":agent.list"):
            self.load_agents()
        elif cmd.startswith(":task.list"):
            self.load_tasks()
        elif cmd.startswith(":ping"):
            try:
                resp = self.client.call("daemon.ping")
                text = resp.get("result", {}).get("content", [{}])[0].get("text", "?")
                self.status_bar.showMessage(f"Ping: {text}")
            except Exception as e:
                self.status_bar.showMessage(f"Ping failed: {e}")
        elif cmd.startswith(":version"):
            try:
                resp = self.client.call("daemon.version")
                text = resp.get("result", {}).get("content", [{}])[0].get("text", "?")
                self.activity_text.append(text)
            except Exception as e:
                self.status_bar.showMessage(f"Error: {e}")
        else:
            self.status_bar.showMessage(f"Unknown command: {cmd}")

        self.cmd_input.clear()


def main():
    app = QApplication(sys.argv)
    client = McpClient()

    try:
        client.connect()
    except Exception as e:
        print(f"Failed to connect to daemon: {e}")
        print("Start the daemon first: cargo run -p aish-daemon")
        sys.exit(1)

    window = MainWindow(client)
    window.show()

    # Auto-refresh every 5 seconds
    timer = QTimer()
    timer.timeout.connect(window.refresh_all)
    timer.start(5000)

    result = app.exec()
    client.close()
    return result


if __name__ == "__main__":
    sys.exit(main())
