#!/bin/bash
# Eka Man Page Installation Script

# Get the directory where the script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if running as root or with sudo
if [ "$EUID" -eq 0 ]; then
    # Install to system man directory
    MAN_DIR="/usr/share/man/man1"
    
    if [ -d "$MAN_DIR" ]; then
        cp "$SCRIPT_DIR/docs/eka.1" "$MAN_DIR/"
        gzip -f "$MAN_DIR/eka.1"
        echo "Man page installed to $MAN_DIR/eka.1.gz"
    else
        echo "Error: Man directory $MAN_DIR does not exist"
        exit 1
    fi
else
    # Install to user man directory
    MAN_DIR="$HOME/.local/share/man/man1"
    mkdir -p "$MAN_DIR"
    cp "$SCRIPT_DIR/docs/eka.1" "$MAN_DIR/"
    gzip -f "$MAN_DIR/eka.1"
    echo "Man page installed to $MAN_DIR/eka.1.gz"
    echo "You may need to add ~/.local/share/man to your MANPATH"
fi

echo "Installation complete!"
echo "View the man page with: man eka"
