#!/usr/bin/env bash
# Uninstall Lighthouse from the system.

set -euo pipefail

log() {
    echo "[lighthouse-uninstall] $*"
}

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

log "stopping and disabling lighthouse service..."
run systemctl stop lighthouse.service 2>/dev/null || true
run systemctl disable lighthouse.service 2>/dev/null || true

log "removing installed files..."
run rm -f /usr/local/bin/lighthouse
run rm -f /etc/systemd/system/lighthouse.service
run rm -rf /etc/lighthouse

log "reloading systemd..."
run systemctl daemon-reload

log "Lighthouse has been uninstalled."
log "Configuration backups were not kept."
