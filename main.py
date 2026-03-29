#!/usr/bin/env python3
"""
AdGuard VPN CLI GUI — графическая оболочка для adguardvpn-cli на Linux.
Требует: Python 3.8+, PyQt5, установленный adguardvpn-cli.
"""

import sys
import os
import re
import subprocess
import shlex
from functools import partial

def strip_ansi(text: str) -> str:
    return re.sub(r'\x1b\[[0-9;]*m', '', text)

from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QTabWidget, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QPushButton, QComboBox, QLineEdit, QTextEdit, QCheckBox,
    QGroupBox, QFormLayout, QMessageBox, QFileDialog, QListWidget,
    QRadioButton, QButtonGroup, QSplitter, QFrame, QSpinBox, QSizePolicy,
    QGridLayout, QStatusBar,
)
from PyQt5.QtCore import Qt, QTimer, QThread, pyqtSignal, QSize
from PyQt5.QtGui import QFont, QIcon, QColor, QPalette


# ─── helpers ────────────────────────────────────────────────────────────────

CLI = "adguardvpn-cli"


class CommandWorker(QThread):
    """Run a CLI command in a background thread so the GUI stays responsive."""
    finished = pyqtSignal(str, str)  # stdout, stderr

    def __init__(self, cmd: list[str], parent=None):
        super().__init__(parent)
        self.cmd = cmd

    def run(self):
        try:
            proc = subprocess.run(
                self.cmd, capture_output=True, text=True, timeout=60
            )
            self.finished.emit(proc.stdout.strip(), proc.stderr.strip())
        except FileNotFoundError:
            self.finished.emit("", f"Команда не найдена: {self.cmd[0]}\n"
                               "Убедитесь, что adguardvpn-cli установлен и доступен в PATH.")
        except subprocess.TimeoutExpired:
            self.finished.emit("", "Команда превысила тайм-аут (60 сек).")
        except Exception as exc:
            self.finished.emit("", str(exc))


def run_cmd(args: list[str]) -> tuple[str, str]:
    """Synchronous command run (for quick queries)."""
    try:
        proc = subprocess.run(
            [CLI] + args, capture_output=True, text=True, timeout=30
        )
        return proc.stdout.strip(), proc.stderr.strip()
    except FileNotFoundError:
        return "", f"{CLI} не найден в PATH."
    except subprocess.TimeoutExpired:
        return "", "Тайм-аут команды."
    except Exception as exc:
        return "", str(exc)


def run_cmd_async(parent, args: list[str], callback):
    """Run CLI command asynchronously; returns the worker thread."""
    worker = CommandWorker([CLI] + args, parent)
    worker.finished.connect(callback)
    worker.start()
    return worker


# ─── Stylesheet ─────────────────────────────────────────────────────────────

STYLE = """
QMainWindow {
    background-color: #1e1e2e;
}
QTabWidget::pane {
    border: 1px solid #313244;
    background: #1e1e2e;
    border-radius: 6px;
}
QTabBar::tab {
    background: #313244;
    color: #cdd6f4;
    padding: 8px 18px;
    margin-right: 2px;
    border-top-left-radius: 6px;
    border-top-right-radius: 6px;
    font-size: 13px;
}
QTabBar::tab:selected {
    background: #45475a;
    color: #89b4fa;
    font-weight: bold;
}
QTabBar::tab:hover {
    background: #585b70;
}
QGroupBox {
    border: 1px solid #45475a;
    border-radius: 8px;
    margin-top: 14px;
    padding: 14px 10px 10px 10px;
    color: #cdd6f4;
    font-weight: bold;
    font-size: 13px;
}
QGroupBox::title {
    subcontrol-origin: margin;
    left: 12px;
    padding: 0 6px;
}
QLabel {
    color: #cdd6f4;
    font-size: 13px;
}
QPushButton {
    background-color: #45475a;
    color: #cdd6f4;
    border: 1px solid #585b70;
    border-radius: 6px;
    padding: 7px 18px;
    font-size: 13px;
    min-height: 20px;
}
QPushButton:hover {
    background-color: #585b70;
    border-color: #89b4fa;
}
QPushButton:pressed {
    background-color: #313244;
}
QPushButton#connectBtn {
    background-color: #a6e3a1;
    color: #1e1e2e;
    font-weight: bold;
}
QPushButton#connectBtn:hover {
    background-color: #94e2d5;
}
QPushButton#disconnectBtn {
    background-color: #f38ba8;
    color: #1e1e2e;
    font-weight: bold;
}
QPushButton#disconnectBtn:hover {
    background-color: #eba0ac;
}
QLineEdit, QSpinBox {
    background-color: #313244;
    color: #cdd6f4;
    border: 1px solid #45475a;
    border-radius: 5px;
    padding: 5px 8px;
    font-size: 13px;
}
QLineEdit:focus, QSpinBox:focus {
    border-color: #89b4fa;
}
QComboBox {
    background-color: #313244;
    color: #cdd6f4;
    border: 1px solid #45475a;
    border-radius: 5px;
    padding: 5px 8px;
    font-size: 13px;
}
QComboBox::drop-down {
    border: none;
}
QComboBox QAbstractItemView {
    background-color: #313244;
    color: #cdd6f4;
    selection-background-color: #45475a;
}
QTextEdit {
    background-color: #181825;
    color: #a6adc8;
    border: 1px solid #45475a;
    border-radius: 6px;
    padding: 6px;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
}
QCheckBox, QRadioButton {
    color: #cdd6f4;
    font-size: 13px;
    spacing: 6px;
}
QCheckBox::indicator, QRadioButton::indicator {
    width: 16px;
    height: 16px;
}
QListWidget {
    background-color: #181825;
    color: #cdd6f4;
    border: 1px solid #45475a;
    border-radius: 6px;
    padding: 4px;
    font-size: 13px;
}
QListWidget::item:selected {
    background-color: #45475a;
}
QStatusBar {
    background-color: #181825;
    color: #a6adc8;
    font-size: 12px;
}
"""


# ─── Connection Tab ─────────────────────────────────────────────────────────

class ConnectionTab(QWidget):
    def __init__(self, status_bar: QStatusBar):
        super().__init__()
        self.status_bar = status_bar
        self._worker = None
        self._init_ui()
        self._refresh_status()
        self._load_locations()

        self.timer = QTimer(self)
        self.timer.timeout.connect(self._refresh_status)
        self.timer.start(15_000)

    # -- UI --
    def _init_ui(self):
        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        # Status group
        grp_status = QGroupBox("Статус VPN")
        sl = QVBoxLayout(grp_status)
        self.lbl_status = QLabel("Получение статуса…")
        self.lbl_status.setWordWrap(True)
        self.lbl_status.setTextInteractionFlags(Qt.TextSelectableByMouse)
        sl.addWidget(self.lbl_status)
        btn_row = QHBoxLayout()
        self.btn_connect = QPushButton("⚡  Подключиться")
        self.btn_connect.setObjectName("connectBtn")
        self.btn_disconnect = QPushButton("⏹  Отключиться")
        self.btn_disconnect.setObjectName("disconnectBtn")
        self.btn_refresh = QPushButton("🔄  Обновить статус")
        btn_row.addWidget(self.btn_connect)
        btn_row.addWidget(self.btn_disconnect)
        btn_row.addWidget(self.btn_refresh)
        sl.addLayout(btn_row)
        layout.addWidget(grp_status)

        # Location group
        grp_loc = QGroupBox("Локация")
        gl = QGridLayout(grp_loc)
        gl.addWidget(QLabel("Выберите локацию:"), 0, 0)
        self.combo_loc = QComboBox()
        self.combo_loc.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Fixed)
        gl.addWidget(self.combo_loc, 0, 1, 1, 2)
        self.btn_reload_loc = QPushButton("🔄  Обновить список")
        gl.addWidget(self.btn_reload_loc, 0, 3)

        self.chk_fastest = QCheckBox("Самая быстрая локация (-f)")
        gl.addWidget(self.chk_fastest, 1, 0, 1, 2)

        self.rb_default = QRadioButton("По умолчанию")
        self.rb_ipv4 = QRadioButton("Только IPv4 (-4)")
        self.rb_ipv6 = QRadioButton("Только IPv6 (-6)")
        self.rb_default.setChecked(True)
        ip_group = QButtonGroup(self)
        ip_group.addButton(self.rb_default)
        ip_group.addButton(self.rb_ipv4)
        ip_group.addButton(self.rb_ipv6)
        ip_row = QHBoxLayout()
        ip_row.addWidget(QLabel("IP-версия:"))
        ip_row.addWidget(self.rb_default)
        ip_row.addWidget(self.rb_ipv4)
        ip_row.addWidget(self.rb_ipv6)
        ip_row.addStretch()
        gl.addLayout(ip_row, 2, 0, 1, 4)
        layout.addWidget(grp_loc)

        # Output
        grp_out = QGroupBox("Вывод команд")
        ol = QVBoxLayout(grp_out)
        self.txt_output = QTextEdit()
        self.txt_output.setReadOnly(True)
        self.txt_output.setMaximumHeight(180)
        ol.addWidget(self.txt_output)
        layout.addWidget(grp_out)
        layout.addStretch()

        # Signals
        self.btn_connect.clicked.connect(self._connect)
        self.btn_disconnect.clicked.connect(self._disconnect)
        self.btn_refresh.clicked.connect(self._refresh_status)
        self.btn_reload_loc.clicked.connect(self._load_locations)

    # -- Logic --
    def _set_busy(self, busy: bool):
        self.btn_connect.setEnabled(not busy)
        self.btn_disconnect.setEnabled(not busy)

    def _append(self, text: str):
        if text:
            self.txt_output.append(text)

    def _handle_result(self, stdout, stderr):
        self._set_busy(False)
        self._append(strip_ansi(stdout))
        if stderr:
            self._append(f"⚠ {strip_ansi(stderr)}")
        self.status_bar.showMessage("Готово", 5000)
        self._refresh_status()

    def _refresh_status(self):
        out, err = run_cmd(["status"])
        if out:
            self.lbl_status.setText(strip_ansi(out))
        elif err:
            self.lbl_status.setText(f"⚠ {strip_ansi(err)}")

    def _load_locations(self):
        out, err = run_cmd(["list-locations"])
        self.combo_loc.clear()
        if out:
            for raw_line in out.splitlines():
                line = strip_ansi(raw_line).strip()
                parts = line.split()
                # skip header and empty lines
                if not parts or parts[0] in ("ISO", "--", "ESTIMATE"):
                    continue
                # format: ISO  COUNTRY...  CITY...  PING
                if len(parts) >= 3 and len(parts[0]) == 2 and parts[0].isupper() and parts[0].isalpha():
                    iso = parts[0]
                    ping = parts[-1] if parts[-1].isdigit() else ""
                    middle = parts[1:-1] if ping else parts[1:]
                    label = f"{iso}  {' '.join(middle)}"
                    if ping:
                        label += f"  ({ping} ms)"
                    self.combo_loc.addItem(label, userData=iso)
        if err:
            self._append(f"⚠ {err}")

    def _connect(self):
        self._set_busy(True)
        self.status_bar.showMessage("Подключение…")
        args = ["connect"]
        if self.chk_fastest.isChecked():
            args.append("-f")
        else:
            iso = self.combo_loc.currentData()  # ISO code stored as userData
            if iso:
                args += ["-l", iso]
        if self.rb_ipv4.isChecked():
            args.append("-4")
        elif self.rb_ipv6.isChecked():
            args.append("-6")
        self._worker = run_cmd_async(self, args, self._handle_result)

    def _disconnect(self):
        self._set_busy(True)
        self.status_bar.showMessage("Отключение…")
        self._worker = run_cmd_async(self, ["disconnect"], self._handle_result)


# ─── Account Tab ────────────────────────────────────────────────────────────

class AccountTab(QWidget):
    def __init__(self, status_bar: QStatusBar):
        super().__init__()
        self.status_bar = status_bar
        self._worker = None
        self._init_ui()
        self._refresh_license()

    def _init_ui(self):
        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        grp_auth = QGroupBox("Авторизация")
        al = QHBoxLayout(grp_auth)
        self.btn_login = QPushButton("🔑  Войти (Login)")
        self.btn_logout = QPushButton("🚪  Выйти (Logout)")
        self.btn_refresh_lic = QPushButton("🔄  Обновить лицензию")
        al.addWidget(self.btn_login)
        al.addWidget(self.btn_logout)
        al.addWidget(self.btn_refresh_lic)
        layout.addWidget(grp_auth)

        grp_lic = QGroupBox("Информация о лицензии")
        ll = QVBoxLayout(grp_lic)
        self.lbl_license = QLabel("Нажмите «Обновить лицензию» для получения данных.")
        self.lbl_license.setWordWrap(True)
        self.lbl_license.setTextInteractionFlags(Qt.TextSelectableByMouse)
        ll.addWidget(self.lbl_license)
        layout.addWidget(grp_lic)

        grp_out = QGroupBox("Вывод")
        ol = QVBoxLayout(grp_out)
        self.txt_output = QTextEdit()
        self.txt_output.setReadOnly(True)
        self.txt_output.setMaximumHeight(200)
        ol.addWidget(self.txt_output)
        layout.addWidget(grp_out)
        layout.addStretch()

        self.btn_login.clicked.connect(self._login)
        self.btn_logout.clicked.connect(self._logout)
        self.btn_refresh_lic.clicked.connect(self._refresh_license)

    def _append(self, text):
        if text:
            self.txt_output.append(text)

    def _handle(self, stdout, stderr):
        self._append(stdout)
        if stderr:
            self._append(f"⚠ {stderr}")
        self.status_bar.showMessage("Готово", 5000)

    def _login(self):
        self.status_bar.showMessage("Вход…")
        self._append("Открываю терминал для авторизации…")
        terminals = [
            ["konsole", "-e", f"{CLI} login"],
            ["kitty", "--", f"{CLI} login"],
            ["alacritty", "-e", f"{CLI} login"],
            ["gnome-terminal", "--", f"{CLI} login"],
            ["xfce4-terminal", "-e", f"{CLI} login"],
            ["xterm", "-e", f"{CLI} login"],
        ]
        for cmd in terminals:
            try:
                subprocess.Popen(cmd, start_new_session=True)
                self._append(f"Открыт: {cmd[0]}\nВведите email и пароль AdGuard.")
                return
            except FileNotFoundError:
                continue
        self._append("⚠ Терминал не найден. Выполните вручную:\n  adguardvpn-cli login")

    def _logout(self):
        self.status_bar.showMessage("Выход…")
        self._worker = run_cmd_async(self, ["logout"], self._handle)

    def _refresh_license(self):
        out, err = run_cmd(["license"])
        if out:
            self.lbl_license.setText(out)
        elif err:
            self.lbl_license.setText(f"⚠ {err}")


# ─── Settings Tab ───────────────────────────────────────────────────────────

class SettingsTab(QWidget):
    def __init__(self, status_bar: QStatusBar):
        super().__init__()
        self.status_bar = status_bar
        self._worker = None
        self._init_ui()

    def _init_ui(self):
        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        # ---- Mode / Protocol / Update channel ----
        grp_main = QGroupBox("Основные параметры")
        fl = QFormLayout(grp_main)

        self.combo_mode = QComboBox()
        self.combo_mode.addItems(["TUN", "SOCKS"])
        fl.addRow("Режим работы:", self.combo_mode)

        self.combo_proto = QComboBox()
        self.combo_proto.addItems(["auto", "http2", "quic"])
        fl.addRow("Протокол:", self.combo_proto)

        self.combo_channel = QComboBox()
        self.combo_channel.addItems(["release", "beta", "nightly"])
        fl.addRow("Канал обновлений:", self.combo_channel)

        self.combo_tun_route = QComboBox()
        self.combo_tun_route.addItems(["AUTO", "SCRIPT", "NONE"])
        fl.addRow("TUN routing mode:", self.combo_tun_route)

        layout.addWidget(grp_main)

        # ---- DNS ----
        grp_dns = QGroupBox("DNS")
        dl = QFormLayout(grp_dns)
        self.edit_dns = QLineEdit()
        self.edit_dns.setPlaceholderText("например: 1.1.1.1 или tls://dns.adguard.com")
        dl.addRow("DNS-сервер:", self.edit_dns)
        self.chk_sys_dns = QCheckBox("Изменять системный DNS")
        dl.addRow(self.chk_sys_dns)
        layout.addWidget(grp_dns)

        # ---- SOCKS ----
        grp_socks = QGroupBox("SOCKS")
        sl = QFormLayout(grp_socks)
        self.spin_socks_port = QSpinBox()
        self.spin_socks_port.setRange(1, 65535)
        self.spin_socks_port.setValue(1080)
        sl.addRow("Порт:", self.spin_socks_port)
        self.edit_socks_host = QLineEdit()
        self.edit_socks_host.setPlaceholderText("127.0.0.1")
        sl.addRow("Хост:", self.edit_socks_host)
        self.edit_socks_user = QLineEdit()
        sl.addRow("Username:", self.edit_socks_user)
        self.edit_socks_pass = QLineEdit()
        self.edit_socks_pass.setEchoMode(QLineEdit.Password)
        sl.addRow("Password:", self.edit_socks_pass)
        self.btn_clear_socks = QPushButton("Очистить SOCKS-авторизацию")
        sl.addRow(self.btn_clear_socks)
        layout.addWidget(grp_socks)

        # ---- Checkboxes ----
        grp_flags = QGroupBox("Флаги")
        fg = QVBoxLayout(grp_flags)
        self.chk_reports = QCheckBox("Отправка отчётов (send-reports)")
        self.chk_hints = QCheckBox("Показывать подсказки (show-hints)")
        self.chk_debug = QCheckBox("Debug-логирование (debug-logging)")
        self.chk_notif = QCheckBox("Уведомления (show-notifications)")
        for cb in (self.chk_reports, self.chk_hints, self.chk_debug, self.chk_notif):
            fg.addWidget(cb)
        layout.addWidget(grp_flags)

        # ---- Buttons ----
        btn_row = QHBoxLayout()
        self.btn_apply = QPushButton("✅  Применить настройки")
        self.btn_apply.setStyleSheet("background-color:#a6e3a1;color:#1e1e2e;font-weight:bold;")
        self.btn_show_conf = QPushButton("📋  Показать текущую конфигурацию")
        btn_row.addWidget(self.btn_apply)
        btn_row.addWidget(self.btn_show_conf)
        layout.addLayout(btn_row)

        # Output
        grp_out = QGroupBox("Вывод")
        ol = QVBoxLayout(grp_out)
        self.txt_output = QTextEdit()
        self.txt_output.setReadOnly(True)
        self.txt_output.setMaximumHeight(180)
        ol.addWidget(self.txt_output)
        layout.addWidget(grp_out)

        # Signals
        self.btn_apply.clicked.connect(self._apply)
        self.btn_show_conf.clicked.connect(self._show_config)
        self.btn_clear_socks.clicked.connect(self._clear_socks)

    def _append(self, text):
        if text:
            self.txt_output.append(text)

    def _run(self, args):
        out, err = run_cmd(["config"] + args)
        self._append(out)
        if err:
            self._append(f"⚠ {err}")

    def _apply(self):
        self.txt_output.clear()
        self.status_bar.showMessage("Применение настроек…")
        self._run(["set-mode", self.combo_mode.currentText()])
        self._run(["set-protocol", self.combo_proto.currentText()])
        self._run(["set-update-channel", self.combo_channel.currentText()])
        self._run(["set-tun-routing-mode", self.combo_tun_route.currentText()])

        dns = self.edit_dns.text().strip()
        if dns:
            self._run(["set-dns", dns])

        sys_dns = "on" if self.chk_sys_dns.isChecked() else "off"
        self._run(["set-system-dns", sys_dns])

        self._run(["set-socks-port", str(self.spin_socks_port.value())])
        host = self.edit_socks_host.text().strip()
        if host:
            self._run(["set-socks-host", host])
        user = self.edit_socks_user.text().strip()
        if user:
            self._run(["set-socks-username", user])
        pw = self.edit_socks_pass.text().strip()
        if pw:
            self._run(["set-socks-password", pw])

        self._run(["send-reports", "on" if self.chk_reports.isChecked() else "off"])
        self._run(["set-show-hints", "on" if self.chk_hints.isChecked() else "off"])
        self._run(["set-debug-logging", "on" if self.chk_debug.isChecked() else "off"])
        self._run(["set-show-notifications", "on" if self.chk_notif.isChecked() else "off"])

        self.status_bar.showMessage("Настройки применены", 5000)

    def _show_config(self):
        self.txt_output.clear()
        out, err = run_cmd(["config", "show"])
        self._append(out or err)

    def _clear_socks(self):
        self._run(["clear-socks-auth"])


# ─── Exclusions Tab ─────────────────────────────────────────────────────────

class ExclusionsTab(QWidget):
    def __init__(self, status_bar: QStatusBar):
        super().__init__()
        self.status_bar = status_bar
        self._worker = None
        self._init_ui()
        self._refresh()

    def _init_ui(self):
        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        # Mode
        grp_mode = QGroupBox("Режим исключений")
        ml = QHBoxLayout(grp_mode)
        self.rb_general = QRadioButton("General")
        self.rb_selective = QRadioButton("Selective")
        self.rb_general.setChecked(True)
        mg = QButtonGroup(self)
        mg.addButton(self.rb_general)
        mg.addButton(self.rb_selective)
        self.btn_set_mode = QPushButton("Применить режим")
        ml.addWidget(self.rb_general)
        ml.addWidget(self.rb_selective)
        ml.addStretch()
        ml.addWidget(self.btn_set_mode)
        layout.addWidget(grp_mode)

        # List
        grp_list = QGroupBox("Текущие исключения")
        ll = QVBoxLayout(grp_list)
        self.list_excl = QListWidget()
        ll.addWidget(self.list_excl)
        self.btn_refresh_excl = QPushButton("🔄  Обновить список")
        ll.addWidget(self.btn_refresh_excl)
        layout.addWidget(grp_list)

        # Add/Remove
        grp_manage = QGroupBox("Управление")
        mg_l = QGridLayout(grp_manage)
        mg_l.addWidget(QLabel("Домен:"), 0, 0)
        self.edit_domain = QLineEdit()
        self.edit_domain.setPlaceholderText("example.com")
        mg_l.addWidget(self.edit_domain, 0, 1)
        self.btn_add = QPushButton("➕ Добавить")
        self.btn_remove = QPushButton("➖ Удалить")
        self.btn_clear = QPushButton("🗑  Очистить все")
        self.btn_clear.setStyleSheet("background-color:#f38ba8;color:#1e1e2e;font-weight:bold;")
        mg_l.addWidget(self.btn_add, 0, 2)
        mg_l.addWidget(self.btn_remove, 0, 3)
        mg_l.addWidget(self.btn_clear, 1, 2, 1, 2)
        layout.addWidget(grp_manage)

        # Output
        grp_out = QGroupBox("Вывод")
        ol = QVBoxLayout(grp_out)
        self.txt_output = QTextEdit()
        self.txt_output.setReadOnly(True)
        self.txt_output.setMaximumHeight(150)
        ol.addWidget(self.txt_output)
        layout.addWidget(grp_out)
        layout.addStretch()

        # Signals
        self.btn_set_mode.clicked.connect(self._set_mode)
        self.btn_refresh_excl.clicked.connect(self._refresh)
        self.btn_add.clicked.connect(self._add)
        self.btn_remove.clicked.connect(self._remove)
        self.btn_clear.clicked.connect(self._clear)

    def _append(self, text):
        if text:
            self.txt_output.append(text)

    def _refresh(self):
        self.list_excl.clear()
        out, err = run_cmd(["site-exclusions", "show"])
        if out:
            for line in out.splitlines():
                line = line.strip()
                if line and not line.startswith("--"):
                    self.list_excl.addItem(line)
        if err:
            self._append(f"⚠ {err}")

    def _set_mode(self):
        mode = "general" if self.rb_general.isChecked() else "selective"
        out, err = run_cmd(["site-exclusions", "mode", mode])
        self._append(out)
        if err:
            self._append(f"⚠ {err}")
        self._refresh()

    def _add(self):
        domain = self.edit_domain.text().strip()
        if not domain:
            return
        out, err = run_cmd(["site-exclusions", "add", domain])
        self._append(out)
        if err:
            self._append(f"⚠ {err}")
        self.edit_domain.clear()
        self._refresh()

    def _remove(self):
        domain = self.edit_domain.text().strip()
        if not domain:
            sel = self.list_excl.currentItem()
            if sel:
                domain = sel.text().strip()
        if not domain:
            return
        out, err = run_cmd(["site-exclusions", "remove", domain])
        self._append(out)
        if err:
            self._append(f"⚠ {err}")
        self.edit_domain.clear()
        self._refresh()

    def _clear(self):
        reply = QMessageBox.question(
            self, "Подтверждение",
            "Вы уверены, что хотите очистить все исключения?",
            QMessageBox.Yes | QMessageBox.No
        )
        if reply == QMessageBox.Yes:
            out, err = run_cmd(["site-exclusions", "clear"])
            self._append(out)
            if err:
                self._append(f"⚠ {err}")
            self._refresh()


# ─── Updates & Logs Tab ─────────────────────────────────────────────────────

class UpdatesLogsTab(QWidget):
    def __init__(self, status_bar: QStatusBar):
        super().__init__()
        self.status_bar = status_bar
        self._worker = None
        self._init_ui()

    def _init_ui(self):
        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        grp_upd = QGroupBox("Обновления")
        ul = QHBoxLayout(grp_upd)
        self.btn_check = QPushButton("🔍  Проверить обновления")
        self.btn_update = QPushButton("⬆  Обновить")
        self.btn_update.setStyleSheet("background-color:#a6e3a1;color:#1e1e2e;font-weight:bold;")
        ul.addWidget(self.btn_check)
        ul.addWidget(self.btn_update)
        layout.addWidget(grp_upd)

        grp_logs = QGroupBox("Логи")
        ll = QHBoxLayout(grp_logs)
        self.btn_export = QPushButton("📁  Экспортировать логи")
        ll.addWidget(self.btn_export)
        layout.addWidget(grp_logs)

        grp_out = QGroupBox("Результат")
        ol = QVBoxLayout(grp_out)
        self.txt_output = QTextEdit()
        self.txt_output.setReadOnly(True)
        ol.addWidget(self.txt_output)
        layout.addWidget(grp_out)

        self.btn_check.clicked.connect(self._check_update)
        self.btn_update.clicked.connect(self._do_update)
        self.btn_export.clicked.connect(self._export_logs)

    def _append(self, text):
        if text:
            self.txt_output.append(text)

    def _handle(self, stdout, stderr):
        self._append(stdout)
        if stderr:
            self._append(f"⚠ {stderr}")
        self.status_bar.showMessage("Готово", 5000)

    def _check_update(self):
        self.txt_output.clear()
        self.status_bar.showMessage("Проверка обновлений…")
        self._worker = run_cmd_async(self, ["check-update"], self._handle)

    def _do_update(self):
        self.txt_output.clear()
        self.status_bar.showMessage("Обновление…")
        self._worker = run_cmd_async(self, ["update"], self._handle)

    def _export_logs(self):
        path, _ = QFileDialog.getSaveFileName(
            self, "Сохранить логи", os.path.expanduser("~/adguardvpn_logs.zip"),
            "ZIP-файлы (*.zip);;Все файлы (*)"
        )
        if not path:
            return
        self.txt_output.clear()
        self.status_bar.showMessage("Экспорт логов…")
        self._worker = run_cmd_async(self, ["export-logs", "-o", path], self._handle)


# ─── Main Window ────────────────────────────────────────────────────────────

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("AdGuard VPN — GUI Manager")
        self.setMinimumSize(780, 640)
        self.resize(860, 700)

        self.statusBar().showMessage("Готов к работе")

        tabs = QTabWidget()
        tabs.addTab(ConnectionTab(self.statusBar()), "🌐 Подключение")
        tabs.addTab(AccountTab(self.statusBar()), "👤 Аккаунт")
        tabs.addTab(SettingsTab(self.statusBar()), "⚙ Настройки")
        tabs.addTab(ExclusionsTab(self.statusBar()), "🚫 Исключения")
        tabs.addTab(UpdatesLogsTab(self.statusBar()), "🔄 Обновления и логи")

        self.setCentralWidget(tabs)


# ─── Entry point ────────────────────────────────────────────────────────────

def main():
    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    app.setStyleSheet(STYLE)

    # Dark palette as fallback
    palette = QPalette()
    palette.setColor(QPalette.Window, QColor("#1e1e2e"))
    palette.setColor(QPalette.WindowText, QColor("#cdd6f4"))
    palette.setColor(QPalette.Base, QColor("#181825"))
    palette.setColor(QPalette.AlternateBase, QColor("#313244"))
    palette.setColor(QPalette.ToolTipBase, QColor("#45475a"))
    palette.setColor(QPalette.ToolTipText, QColor("#cdd6f4"))
    palette.setColor(QPalette.Text, QColor("#cdd6f4"))
    palette.setColor(QPalette.Button, QColor("#45475a"))
    palette.setColor(QPalette.ButtonText, QColor("#cdd6f4"))
    palette.setColor(QPalette.Highlight, QColor("#89b4fa"))
    palette.setColor(QPalette.HighlightedText, QColor("#1e1e2e"))
    app.setPalette(palette)

    win = MainWindow()
    win.show()
    sys.exit(app.exec_())


if __name__ == "__main__":
    main()
