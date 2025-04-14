#!/bin/bash

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Cursor Rust Tools Installer ===${NC}"
echo -e "${BLUE}This script will install Cursor Rust Tools and create a desktop entry${NC}"
echo ""

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed.${NC}"
    echo -e "Please install Rust and Cargo first by running:"
    echo -e "${GREEN}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    exit 1
fi

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd "$SCRIPT_DIR"
cd ..

echo -e "${BLUE}Building and installing Cursor Rust Tools...${NC}"
cargo install --path .

if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to install Cursor Rust Tools${NC}"
    exit 1
fi

echo -e "${BLUE}Creating desktop entry...${NC}"

EXECUTABLE_PATH=$(which cursor-rust-tools)

mkdir -p ~/.local/share/icons/hicolor/256x256/apps/

if [ -f "./assets/icon.png" ]; then
    echo -e "${BLUE}Installing application icon...${NC}"
    cp "./assets/icon.png" ~/.local/share/icons/hicolor/256x256/apps/cursor-rust-tools.png
    ICON_PATH="cursor-rust-tools"
else
    echo -e "${RED}Warning: Icon file not found, using default icon${NC}"
    ICON_PATH="utilities-terminal"
fi

mkdir -p ~/.local/share/applications

cat > ~/.local/share/applications/cursor-rust-tools.desktop << EOF
[Desktop Entry]
Name=Cursor Rust Tools
Comment=Rust development tools for Cursor.
Exec=${EXECUTABLE_PATH}
Icon=${ICON_PATH}
Terminal=false
Type=Application
Categories=Development;Utility;
Keywords=rust;cursor;tools;
EOF

chmod +x ~/.local/share/applications/cursor-rust-tools.desktop

echo -e "${GREEN}Desktop entry created at ~/.local/share/applications/cursor-rust-tools.desktop${NC}"

if command -v update-desktop-database &> /dev/null; then
    update-desktop-database ~/.local/share/applications
fi

if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor
fi

echo -e "${GREEN}Installation complete!${NC}"
echo -e "You can now launch Cursor Rust Tools from your application menu or run '${GREEN}cursor-rust-tools${NC}' in the terminal." 