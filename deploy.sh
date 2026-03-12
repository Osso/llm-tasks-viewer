#!/usr/bin/env bash

set -euo pipefail

APP_NAME="llm-tasks-viewer"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"
APPLICATIONS_DIR="${DATA_DIR}/applications"
ICON_DIR="${DATA_DIR}/icons/hicolor/scalable/apps"

DESKTOP_SRC="${SCRIPT_DIR}/assets/${APP_NAME}.desktop"
ICON_SRC="${SCRIPT_DIR}/assets/${APP_NAME}.svg"
BIN_SRC="${SCRIPT_DIR}/target/release/${APP_NAME}"

mkdir -p "${BIN_DIR}" "${APPLICATIONS_DIR}" "${ICON_DIR}"

echo "Building ${APP_NAME}..."
cargo build --release --locked --manifest-path "${SCRIPT_DIR}/Cargo.toml"

echo "Installing binary..."
install -m755 "${BIN_SRC}" "${BIN_DIR}/${APP_NAME}"

echo "Installing icon..."
install -m644 "${ICON_SRC}" "${ICON_DIR}/${APP_NAME}.svg"

echo "Installing desktop entry..."
sed "s|^Exec=.*$|Exec=${BIN_DIR}/${APP_NAME}|" "${DESKTOP_SRC}" \
    > "${APPLICATIONS_DIR}/${APP_NAME}.desktop"
chmod 644 "${APPLICATIONS_DIR}/${APP_NAME}.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${APPLICATIONS_DIR}"
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1 \
    && [ -f "${DATA_DIR}/icons/hicolor/index.theme" ]; then
    gtk-update-icon-cache -q "${DATA_DIR}/icons/hicolor" || true
fi

echo "Installed ${APP_NAME}"
echo "Binary: ${BIN_DIR}/${APP_NAME}"
echo "Desktop entry: ${APPLICATIONS_DIR}/${APP_NAME}.desktop"
