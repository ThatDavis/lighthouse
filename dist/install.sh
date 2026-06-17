#!/usr/bin/env bash
set -euo pipefail
SRC="$(cd "$(dirname "$0")" && pwd)"
echo "Installing Lighthouse from $SRC"
sudo cp "$SRC/lighthouse/usr/local/bin/lighthouse" /usr/local/bin/
sudo mkdir -p /etc/lighthouse
sudo cp -n "$SRC/lighthouse/etc/lighthouse/config.toml" /etc/lighthouse/config.toml || true
sudo cp "$SRC/lighthouse/etc/systemd/system/lighthouse.service" /etc/systemd/system/
sudo systemctl daemon-reload
sudo useradd -r -s /usr/sbin/nologin lighthouse || true
echo "Lighthouse installed. Edit /etc/lighthouse/config.toml, then run:"
echo "  sudo systemctl enable --now lighthouse"
