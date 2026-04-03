# AdGuard VPN GUI (Rust + iced)

Графическая оболочка для управления [adguardvpn-cli](https://adguard-vpn.com/en/adguardvpn-cli/overview.html) на Linux.
Тёмная тема Catppuccin Mocha. Переписано на **Rust** с использованием фреймворка **iced**.

> Портирование оригинального [Python/PyQt5 проекта](https://github.com/mukti645/adguard-vpn-gui).

---

## Возможности

| Вкладка | Функционал |
|-------|----|
| 🌐 **Подключение** | Connect / Disconnect, выбор локации, самая быстрая, IPv4/IPv6, автообновление статуса каждые 15 сек |
| 👤 **Аккаунт** | Login (открывает терминал) / Logout, просмотр лицензии |
| ⚙ **Настройки** | Режим (TUN/SOCKS), DNS, SOCKS-параметры, протокол, канал обновлений, TUN routing mode, флаги |
| 🚫 **Исключения** | Режим General/Selective, добавление/удаление доменов, очистка |
| 🔄 **Обновления** | Проверка/установка обновлений, экспорт логов |

---

## Требования

- Linux (Arch / Ubuntu / Fedora / любой дистрибутив)
- [Rust](https://www.rust-lang.org/tools/install) (1.75+ рекомендуется)
- [adguardvpn-cli](https://adguard-vpn.com/en/adguardvpn-cli/overview.html) — установлен и доступен в `$PATH`
- Системные библиотеки для GUI: `libxkbcommon`, `libwayland-client` (обычно уже установлены)

---

## Установка

### 1. Установить Rust (если ещё не установлен)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Установить adguardvpn-cli

**Arch / CachyOS (через AUR):**
```bash
paru -S adguardvpn-cli-bin
```

**Официальный скрипт:**
```bash
curl -fsSL https://raw.githubusercontent.com/AdguardTeam/AdGuardVPNCLI/master/scripts/release/install.sh | sh -s -- -v
```

### 3. Настроить sudo без пароля (для TUN режима)

```bash
echo "$USER ALL=(ALL) NOPASSWD: /usr/bin/adguardvpn-cli" | sudo tee /etc/sudoers.d/99-$USER-vpn
sudo chmod 440 /etc/sudoers.d/99-$USER-vpn
```

### 4. Войти в аккаунт AdGuard

```bash
adguardvpn-cli login
```

---

## Сборка и запуск

### Через cargo

```bash
# Клонировать / перейти в директорию
cd adguard-vpn-rust

# Debug-сборка (быстрая компиляция)
cargo run

# Release-сборка (оптимизированная)
cargo run --release
```

### Через скрипт

```bash
chmod +x run.sh
./run.sh
```

---

## Установка глобально (опционально)

```bash
cargo build --release
sudo cp target/release/adguard-vpn-gui /usr/local/bin/

# Запуск
adguard-vpn-gui
```

---

## Структура проекта

```
adguard-vpn-rust/
├── Cargo.toml        # Зависимости (iced, tokio, regex)
├── src/
│   └── main.rs       # GUI приложение (Rust + iced)
├── run.sh            # Скрипт запуска
└── README.md
```

---

## Технологии

- **Rust** — системный язык программирования
- **iced 0.14** — кроссплатформенный GUI фреймворк (Elm-архитектура)
- **tokio** — асинхронный runtime для выполнения CLI команд
- **regex** — очистка ANSI-кодов из вывода CLI

---

## Примечания

- Кнопка **Login** открывает системный терминал (konsole, kitty, alacritty, gnome-terminal, xfce4-terminal, xterm) для интерактивной авторизации.
- Статус VPN обновляется автоматически каждые 15 секунд.
- Все команды выполняются через `adguardvpn-cli` — GUI не хранит никаких данных самостоятельно.
- Тема оформления: Catppuccin Mocha (встроена в iced).

---

## Лицензия

MIT
