# AdGuard VPN GUI

Графическая оболочка для управления [adguardvpn-cli](https://adguard-vpn.com/en/adguardvpn-cli/overview.html) на Linux.
Тёмная тема Catppuccin Mocha. Протестировано на **CachyOS / Arch Linux** с KDE Plasma.

![Python](https://img.shields.io/badge/Python-3.8%2B-blue)
![PyQt5](https://img.shields.io/badge/PyQt5-5.15%2B-green)
![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)
![iced](https://img.shields.io/badge/iced-0.14-purple)
![License](https://img.shields.io/badge/License-MIT-yellow)

---

## 📦 Доступные версии

Проект доступен в двух реализациях:

| Версия | Технологии | Папка | Описание |
|--------|-----------|-------|----------|
| 🐍 **Python** | Python 3 + PyQt5 | [корень репозитория](./) | Оригинальная версия |
| 🦀 **Rust** | Rust + iced 0.14 | [rust-version/](./rust-version/) | Переписана на Rust, нативная производительность |

Обе версии предоставляют одинаковый функционал и используют `adguardvpn-cli` в качестве бэкенда.

---

## Возможности

| Вкладка | Функционал |
|---------|-----------|
| 🌐 **Подключение** | Connect / Disconnect, выбор локации, самая быстрая, IPv4/IPv6, автообновление статуса |
| 👤 **Аккаунт** | Login / Logout, просмотр лицензии |
| ⚙ **Настройки** | Режим (TUN/SOCKS), DNS, SOCKS-параметры, протокол, канал обновлений, флаги |
| 🚫 **Исключения** | Режим General/Selective, добавление/удаление доменов, очистка |
| 🔄 **Обновления и логи** | Проверка/установка обновлений, экспорт логов |

---

## 🐍 Python версия

### Требования

- Linux (Arch / CachyOS / любой дистрибутив)
- Python 3.8+
- PyQt5
- [adguardvpn-cli](https://adguard-vpn.com/en/adguardvpn-cli/overview.html) — установлен и доступен в `$PATH`

### Установка

#### 1. Установить adguardvpn-cli

**Arch / CachyOS (через AUR):**
```bash
paru -S adguardvpn-cli-bin
# или
yay -S adguardvpn-cli-bin
```

**Официальный скрипт:**
```bash
curl -fsSL https://raw.githubusercontent.com/AdguardTeam/AdGuardVPNCLI/master/scripts/release/install.sh | sh -s -- -v
```

#### 2. Установить зависимости GUI

**Arch / CachyOS:**
```bash
sudo pacman -S python-pyqt5
```

**Ubuntu / Debian:**
```bash
sudo apt install python3-pyqt5
```

**pip:**
```bash
pip install -r requirements.txt
```

#### 3. Настроить sudo без пароля

`adguardvpn-cli connect` в режиме TUN требует root-привилегий внутри.
Чтобы GUI работал без запроса пароля, добавь NOPASSWD правило:

```bash
echo "$USER ALL=(ALL) NOPASSWD: ALL" | sudo tee /etc/sudoers.d/99-$USER-nopasswd
sudo chmod 440 /etc/sudoers.d/99-$USER-nopasswd
```

> Стандартная практика для личных однопользовательских машин.
> Для минимальных прав замени `NOPASSWD: ALL` на `NOPASSWD: /usr/bin/adguardvpn-cli`.

#### 4. Войти в аккаунт AdGuard

```bash
adguardvpn-cli login
```

Введи email и пароль от [my.adguard.com](https://my.adguard.com).

#### 5. Клонировать репозиторий

```bash
git clone https://github.com/mukti645/adguard-vpn-gui.git
cd adguard-vpn-gui
```

### Запуск

```bash
# Напрямую
python3 main.py

# Или через скрипт (автоматически установит PyQt5 если нет)
./run.sh
```

### Установка глобально (опционально)

```bash
sudo mkdir -p /usr/lib/adguard-vpn-gui
sudo cp -r . /usr/lib/adguard-vpn-gui/

printf '#!/usr/bin/env bash\nexec python3 /usr/lib/adguard-vpn-gui/main.py "$@"\n' \
  | sudo tee /usr/local/bin/adguard-vpn-gui
sudo chmod +x /usr/local/bin/adguard-vpn-gui

# Запуск
adguard-vpn-gui
```

---

## 🦀 Rust версия

Полная документация: [rust-version/README.md](./rust-version/README.md)

### Быстрый старт

```bash
cd rust-version

# Установить Rust (если ещё не установлен)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Собрать и запустить
cargo run --release

# Или через скрипт
./run.sh
```

### Технологии

- **Rust** — системный язык программирования
- **iced 0.14** — кроссплатформенный GUI фреймворк (Elm-архитектура)
- **tokio** — асинхронный runtime
- **regex** — очистка ANSI-кодов из вывода CLI

---

## Примечания

- Кнопка **Login** открывает терминал для интерактивной авторизации.
- Статус VPN обновляется автоматически каждые 15 секунд.
- Все команды выполняются через `adguardvpn-cli` — GUI не хранит никаких данных самостоятельно.

---

## Структура

```
adguard-vpn-gui/
├── main.py              # GUI (Python 3 + PyQt5)
├── requirements.txt     # Python зависимости
├── run.sh               # Скрипт запуска Python версии
├── README.md            # Этот файл
└── rust-version/        # Rust версия
    ├── Cargo.toml       # Зависимости (iced, tokio, regex)
    ├── Cargo.lock
    ├── src/
    │   └── main.rs      # GUI приложение (Rust + iced)
    ├── run.sh           # Скрипт запуска Rust версии
    ├── README.md        # Документация Rust версии
    ├── TEST_REPORT.md   # Отчёт о тестировании
    └── TEST_REPORT.pdf
```

---

## Лицензия

MIT
