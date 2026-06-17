# Lighthouse

A lightweight Rust daemon and TUI that maps system telemetry (CPU temperature, load) to OpenRGB-controlled motherboard Aurora lighting. Designed to run on a Proxmox server.

## Stack

- **Language:** Rust (edition 2024, MSRV 1.85)
- **Runtime:** Tokio
- **TUI:** ratatui + crossterm
- **System metrics:** sysinfo
- **OpenRGB control:** Custom minimal OpenRGB SDK client
- **Config:** TOML
- **Logging:** tracing + tracing-journald
- **CI:** GitHub Actions

## Getting Started

### Prerequisites

- Rust 1.85+ (install via [rustup](https://rustup.rs))
- An OpenRGB server running and reachable
- (Optional) `lm-sensors` for richer temperature data

### Install OpenRGB (headless)

Lighthouse depends on an OpenRGB server to control the motherboard RGB hardware. Run the helper script on your Proxmox host:

```bash
sudo ./scripts/install-openrgb-headless.sh
```

This builds OpenRGB from source, installs the `openrgb` binary, and sets up a headless `openrgb-server` systemd service on port `6742`.

Verify it works:

```bash
sudo systemctl status openrgb-server
sudo openrgb --list-devices
```

### Install from Release

Download the latest `lighthouse-install.tar.gz` from the [releases page](https://github.com/ThatDavis/lighthouse/releases), then:

```bash
tar xzf lighthouse-install.tar.gz
sudo ./install.sh
```

### Build from Source

```bash
git clone https://github.com/ThatDavis/lighthouse.git
cd lighthouse
cargo build --release --target x86_64-unknown-linux-gnu
sudo cp target/x86_64-unknown-linux-gnu/release/lighthouse /usr/local/bin/
```

### Install

The CI artifact contains an install script. Alternatively:

```bash
sudo cp target/x86_64-unknown-linux-gnu/release/lighthouse /usr/local/bin/
sudo mkdir -p /etc/lighthouse
sudo cp assets/config.example.toml /etc/lighthouse/config.toml
sudo cp assets/systemd/lighthouse.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now lighthouse
```

### Configuration

Edit `/etc/lighthouse/config.toml`:

- `openrgb_host` / `openrgb_port` — OpenRGB server address
- `openrgb_device_id` — Device to control (default: 0)
- `poll_interval` — Seconds between updates
- `temperature` — `cold`, `warm`, `hot` thresholds in °C
- `colors` — RGB values for each threshold
- `dry_run` — Log intended colors without contacting OpenRGB (default: false)

### Running Tests

```bash
cargo test
```

## Project Structure

```
.
├── .agent/              # Agent continuity briefings
├── assets/              # Config templates and systemd units
├── docs/                # Specifications, architecture, ADRs
├── src/                 # Rust source
│   ├── config/          # TOML config parsing and validation
│   ├── daemon/          # Headless daemon loop
│   ├── metrics/         # System telemetry readers
│   ├── openrgb/         # OpenRGB SDK client
│   ├── tui/             # Interactive terminal UI
│   └── main.rs          # CLI entry point
└── tests/               # Integration tests
```

## Commands

- `lighthouse daemon` — Run the lighting daemon
- `lighthouse tui` — Launch the interactive TUI
- `lighthouse validate` — Validate the config file

## Features

- CPU temperature monitoring via `sysinfo`
- Temperature-to-color mapping with configurable thresholds
- OpenRGB server control
- Headless daemon mode with systemd service
- Dry-run mode for testing without hardware
- Config validation with `lighthouse validate`

See [PLAN.md](PLAN.md) for the full roadmap.

## License

MIT
