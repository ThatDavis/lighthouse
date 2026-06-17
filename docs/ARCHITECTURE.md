# Architecture — Lighthouse

> Last updated: 2026-06-16

## Stack Decisions

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Language | Rust | Systems-friendly, lightweight binary, safe concurrency for daemon |
| Runtime | Tokio | Async runtime for daemon loop and future network integrations |
| TUI | ratatui + crossterm | Standard Rust TUI stack |
| System metrics | sysinfo | Cross-platform, simple API, sufficient for CPU temp/load |
| OpenRGB | Custom minimal client | Avoids unmaintained crates; protocol is small |
| Config | TOML via serde | Idiomatic for Rust CLI tools |
| Logging | tracing + tracing-journald | Structured logs, journald integration on Linux |
| CI | GitHub Actions | Builds release binary and produces install artifact |

## Project Structure

```
.
├── src/
│   ├── config/        # TOML config parsing, validation, color interpolation
│   ├── daemon/        # Daemon loop: poll metrics, compute color, send to OpenRGB
│   ├── metrics/       # sysinfo-based telemetry readers
│   ├── openrgb/       # Minimal OpenRGB SDK network client
│   ├── tui/           # Interactive terminal UI (placeholder)
│   └── main.rs        # CLI entry point with clap subcommands
├── assets/
│   ├── config.example.toml
│   └── systemd/lighthouse.service
├── docs/
│   ├── SPEC.md
│   ├── ARCHITECTURE.md
│   └── adr/
├── tests/             # Integration tests
└── .github/workflows/ # CI/CD
```

## Key Design Decisions

- **Single-threaded async daemon.** Keeps resource usage low; sufficient for polling every second.
- **Dry-run mode.** Allows development and testing without an OpenRGB server.
- **Minimal OpenRGB client.** We implement only the commands needed to set a single LED color.

## Architecture Decision Records

- `docs/adr/` — *No ADRs yet.*

## Open Design Questions

- How should we handle multiple OpenRGB devices?
- Should the daemon support dynamic config reloading?
