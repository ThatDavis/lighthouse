#!/usr/bin/env bash
# Install OpenRGB from source on a headless Debian/Proxmox host.
# This installs only the OpenRGB SDK server, no desktop GUI is required.

set -euo pipefail

OPENRGB_VERSION="0.9"
OPENRGB_TAG="release_${OPENRGB_VERSION}"
INSTALL_PREFIX="/usr/local"
SERVICE_USER="openrgb"
SERVICE_NAME="openrgb-server"

log() {
    echo "[lighthouse-openrgb-install] $*"
}

if [[ $EUID -ne 0 ]]; then
    log "this script must be run as root"
    exit 1
fi

log "installing build dependencies..."
apt-get update
apt-get install -y \
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
modprobe i2c-dev || true
if ! grep -q "^i2c-dev$" /etc/modules-load.d/*.conf 2>/dev/null; then
    echo "i2c-dev" > /etc/modules-load.d/i2c.conf
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
cp openrgb "${INSTALL_PREFIX}/bin/openrgb"
chmod +x "${INSTALL_PREFIX}/bin/openrgb"

log "creating service user ${SERVICE_USER}..."
useradd -r -s /usr/sbin/nologin "${SERVICE_USER}" 2>/dev/null || true
usermod -aG i2c "${SERVICE_USER}" 2>/dev/null || true

log "installing systemd service..."
cat > "/etc/systemd/system/${SERVICE_NAME}.service" <<'EOF'
[Unit]
Description=OpenRGB SDK Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/openrgb --server --server-port 6742
Restart=on-failure
RestartSec=5
User=openrgb
Group=openrgb
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "${SERVICE_NAME}.service"

log "starting OpenRGB SDK server..."
systemctl start "${SERVICE_NAME}.service" || {
    log "warning: failed to start OpenRGB server automatically"
    log "check logs with: journalctl -u ${SERVICE_NAME}.service"
}

log "OpenRGB ${OPENRGB_VERSION} installed."
log "Verify with: systemctl status ${SERVICE_NAME}.service"
log "List devices with: sudo openrgb --list-devices"
