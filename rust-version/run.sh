#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Check Rust
if ! command -v cargo &>/dev/null; then
    echo "Rust/Cargo не найден. Устанавливаю..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Check adguardvpn-cli
if ! command -v adguardvpn-cli &>/dev/null; then
    echo "⚠ adguardvpn-cli не найден в PATH."
    echo "Установите его: https://adguard-vpn.com/en/adguardvpn-cli/overview.html"
    echo "Приложение запустится, но команды VPN не будут работать."
fi

# Build and run
echo "Сборка и запуск AdGuard VPN GUI..."
cargo run --release "$@"
