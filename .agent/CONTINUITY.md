# CONTINUITY — Lighthouse

> Canonical project briefing. Read at session start.
> Stack: Rust + Tokio + custom OpenRGB SDK client
> Type: CLI daemon + TUI for Proxmox

## [PLANS]

### Milestone 1: Core Daemon (Complete)
Goal: Read CPU temperature and map it to OpenRGB lighting via configurable thresholds, running as a headless daemon.
- [x] Read CPU temperature using `sysinfo`
- [x] Map temperature to color via configurable thresholds
- [x] Connect to OpenRGB server and update lighting
- [x] Load config from default or `--config` path
- [x] Run as a systemd service
- [x] `--dry-run` mode
- [x] `lighthouse validate` command


### Future Milestones
- Milestone 2: Effects engine (pulse, breathe, cycle, scheduling)
- Milestone 3: Interactive TUI for status, config, daemon control
- Milestone 4: Home Assistant integration (MQTT/REST)

### Open Questions
- [ ] Which Home Assistant protocol: MQTT or REST? — Due: Milestone 4

## [DECISIONS]

- 2026-06-16: Initial stack — Rust + Tokio + custom OpenRGB SDK client. Rationale: lightweight, reliable, avoids unmaintained third-party crates.
- 2026-06-16: System metrics via `sysinfo` for simplicity; may add lm-sensors later if needed.
- 2026-06-16: GitHub Actions will produce an install-ready artifact (binary + systemd service + config template).
- 2026-06-17: Lighthouse depends on a separate OpenRGB server for hardware control; added `scripts/install-openrgb-headless.sh` to build and install OpenRGB headless on Proxmox.
- 2026-06-17: Added `openrgb_zone_id` config option to control a specific OpenRGB zone (e.g. CPU fan on addressable header).

## [PROGRESS]

| Date | What was done |
|------|---------------|
| 2026-06-16 | Initial scaffold. Stack: Rust + Tokio + OpenRGB client. |
| 2026-06-17 | Completed feature #1: core daemon with CPU temp, color mapping, OpenRGB control. PR #2 opened.

## [DISCOVERIES]

*None yet.*

## [OUTCOMES]

### Core Daemon (2026-06-17)
- CPU temperature reading via `sysinfo` with fallback to any available sensor.
- Smooth RGB color interpolation between configurable cold/warm/hot thresholds.
- Minimal OpenRGB SDK client that sends `UpdateLeds` commands.
- TOML config loading with `/etc/lighthouse/config.toml` fallback and `lighthouse validate`.
- Dry-run mode for safe testing without an OpenRGB server.
- Systemd service unit with auto-restart.
- Milestone 1 complete.
