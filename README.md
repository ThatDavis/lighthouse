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

### Setup

```bash
git clone https://github.com/yourusername/lighthouse.git
cd lighthouse
cargo build --release
```

### Install

The CI artifact contains an install script. Alternatively:

```bash
sudo cp target/release/lighthouse /usr/local/bin/
sudo mkdir -p /etc/lighthouse
sudo cp assets/config.example.toml /etc/lighthouse/config.toml
sudo cp assets/systemd/lighthouse.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now lighthouse
```

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

*No production features implemented yet. See [PLAN.md](PLAN.md) for the roadmap.*

## License

MIT
