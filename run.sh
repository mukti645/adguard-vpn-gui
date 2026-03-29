#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Check Python
if ! command -v python3 &>/dev/null; then
    echo "Python3 not found. Please install Python 3.8+"
    exit 1
fi

# Check/install deps
if ! python3 -c "import PyQt5" 2>/dev/null; then
    echo "Installing PyQt5..."
    pip install --user -r requirements.txt
fi

python3 main.py "$@"
