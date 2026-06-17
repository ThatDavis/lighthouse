# Specification — Lighthouse

> Last updated: 2026-06-16

## Overview

Lighthouse is a Rust daemon and TUI for Proxmox servers that maps system telemetry (primarily CPU temperature) to OpenRGB-controlled motherboard RGB lighting. Users can configure temperature thresholds and colors, run the tool headlessly, or manage it through an interactive terminal UI.

## Milestone 1 — Core Daemon

### Feature: CPU Temperature Reading
**Description:** Read the host CPU temperature on Linux/Proxmox.
**Acceptance Criteria:**
- [ ] Daemon obtains a CPU temperature reading at the configured interval
- [ ] Falls back gracefully if no temperature sensor is available
- [ ] Logs a warning when temperature cannot be read

### Feature: Temperature-to-Color Mapping
**Description:** Map the current CPU temperature to an RGB color based on configurable thresholds.
**Acceptance Criteria:**
- [ ] Config defines `cold`, `warm`, and `hot` temperature thresholds
- [ ] Config defines RGB colors for each threshold
- [ ] Colors interpolate smoothly between thresholds
- [ ] Invalid threshold order is rejected at config load time

### Feature: OpenRGB Control
**Description:** Send the computed color to an OpenRGB server.
**Acceptance Criteria:**
- [ ] Connect to OpenRGB server at configured host/port
- [ ] Send the computed color to device 0
- [ ] Log connection failures clearly
- [ ] Gracefully degrade in `--dry-run` mode

### Feature: Config File
**Description:** Load runtime configuration from a TOML file.
**Acceptance Criteria:**
- [ ] Load from `~/.config/lighthouse/config.toml` by default
- [ ] Allow override via `--config`
- [ ] Validate config on `lighthouse validate`
- [ ] Include an example config in `assets/config.example.toml`

### Feature: Systemd Service
**Description:** Run the daemon as a systemd service.
**Acceptance Criteria:**
- [ ] Provide a systemd unit file
- [ ] Service runs as a dedicated `lighthouse` user
- [ ] Service restarts automatically on failure

### Feature: Dry-Run Mode
**Description:** Test color mapping without contacting OpenRGB.
**Acceptance Criteria:**
- [ ] `dry_run = true` in config skips OpenRGB connection
- [ ] Intended color changes are logged
- [ ] Daemon continues running and updating logs

---

## Future Milestones

See [PLAN.md](PLAN.md) for milestone descriptions. Detailed acceptance criteria will be added when each milestone starts.

## Non-Functional Requirements

- Lightweight binary
- Suitable for long-running daemon on Proxmox
- Logs compatible with journald

## Open Questions

- [ ] Should the TUI allow live editing of thresholds? — Owner: TBD, Due: Milestone 3
- [ ] Which Home Assistant protocol is preferred: MQTT or REST? — Owner: TBD, Due: Milestone 4
