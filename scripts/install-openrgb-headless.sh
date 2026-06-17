#!/usr/bin/env bash
# Install OpenRGB from source on a headless Debian/Proxmox host.
# This installs only the OpenRGB SDK server, no desktop GUI is required.

set -euo pipefail

OPENRGB_VERSION="0.9"
OPENRGB_TAG="release_${OPENRGB_VERSION}"
INSTALL_PREFIX="/usr/local"
SERVICE_USER="openrgb"
SERVICE_NAME="openrgb-server"
SERVICE_HOME="/var/lib/openrgb"

log() {
    echo "[lighthouse-openrgb-install] $*"
}

# Detect privilege escalation tool
if command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
elif [[ $EUID -eq 0 ]]; then
    SUDO=""
else
    log "this script must be run as root or with sudo installed"
    exit 1
fi

run() {
    if [[ -n $SUDO ]]; then
        $SUDO "$@"
    else
        "$@"
    fi
}

log "installing build dependencies..."
run apt-get update
run apt-get install -y \
    git \
    build-essential \
    pkg-config \
    qtbase5-dev \
    qttools5-dev \
    libusb-1.0-0-dev \
    libusb-1.0-0 \
    libhidapi-dev \
    libhidapi-hidraw0 \
    i2c-tools \
    libgl1-mesa-dev \
    libpulse-dev \
    libmbedtls-dev

log "loading i2c kernel module..."
if ! run modprobe i2c-dev; then
    log "warning: failed to load i2c-dev module"
fi
if ! run grep -q "^i2c-dev$" /etc/modules-load.d/*.conf 2>/dev/null; then
    run sh -c "echo i2c-dev > /etc/modules-load.d/i2c.conf"
fi

BUILD_DIR="$(mktemp -d)"
trap "rm -rf ${BUILD_DIR}" EXIT

cd "${BUILD_DIR}"

log "cloning OpenRGB ${OPENRGB_VERSION}..."
git clone --depth 1 --branch "${OPENRGB_TAG}" https://gitlab.com/CalcProgrammer1/OpenRGB.git
cd OpenRGB

log "building OpenRGB (this may take a while)..."
qmake OpenRGB.pro
make -j"$(nproc)"

log "installing OpenRGB binary..."
run cp openrgb "${INSTALL_PREFIX}/bin/openrgb"
run chmod +x "${INSTALL_PREFIX}/bin/openrgb"

log "creating service user ${SERVICE_USER}..."
run useradd -r -s /usr/sbin/nologin "${SERVICE_USER}" 2>/dev/null || true
run usermod -aG i2c "${SERVICE_USER}" 2>/dev/null || true
run getent group plugdev >/dev/null && run usermod -aG plugdev "${SERVICE_USER}" 2>/dev/null || true

log "creating service home directory ${SERVICE_HOME}..."
run mkdir -p "${SERVICE_HOME}/.config"
run chown -R "${SERVICE_USER}:${SERVICE_USER}" "${SERVICE_HOME}"

log "installing OpenRGB udev rules..."
run sh -c "cat > /etc/udev/rules.d/60-openrgb.rules <<'UDEOF'
# OpenRGB udev rules for ASUS and other RGB controllers
SUBSYSTEM==\"usb\", ATTR{idVendor}==\"0b05\", MODE=\"0666\", TAG+=\"uaccess\"
SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"0b05\", MODE=\"0666\", TAG+=\"uaccess\"
UDEOF"
run udevadm control --reload-rules
run udevadm trigger

log "installing systemd service..."
run sh -c "cat > /etc/systemd/system/${SERVICE_NAME}.service <<EOF
[Unit]
Description=OpenRGB SDK Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/openrgb --server --server-port 6742
Restart=on-failure
RestartSec=5
User=${SERVICE_USER}
Group=${SERVICE_USER}
Environment=HOME=${SERVICE_HOME}
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF"

run systemctl daemon-reload
run systemctl enable "${SERVICE_NAME}.service"

log "starting OpenRGB SDK server..."
run systemctl start "${SERVICE_NAME}.service" || {
    log "warning: failed to start OpenRGB server automatically"
    log "check logs with: journalctl -u ${SERVICE_NAME}.service"
}

log "OpenRGB ${OPENRGB_VERSION} installed."
log "Verify with: systemctl status ${SERVICE_NAME}.service"
log "List devices with: ${SUDO:+sudo }openrgb --list-devices"
