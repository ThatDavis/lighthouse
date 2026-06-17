# CONTINUITY — Lighthouse

> Canonical project briefing. Read at session start.
> Stack: Rust + Tokio + custom OpenRGB SDK client
> Type: CLI daemon + TUI for Proxmox

## [PLANS]

### Milestone 1: Core Daemon (In Progress)
Goal: Read CPU temperature and map it to OpenRGB lighting via configurable thresholds, running as a headless daemon.
**Active Feature:** #1 — Implement core daemon: CPU temperature reading, color mapping, and OpenRGB control (branch: `feature/core-daemon`)
- [x] Read CPU temperature using `sysinfo`
- [x] Map temperature to color via configurable thresholds
- [x] Connect to OpenRGB server and update lighting
- [x] Load config from default or `--config` path
- [x] Run as a systemd service
- [x] `--dry-run` mode
- [x] `lighthouse validate` command

### Sub-tasks
- [x] Implement CPU temperature reading
- [x] Implement temperature-to-color mapping
- [x] Implement OpenRGB control
- [x] Implement config loading and validation
- [x] Implement systemd service and dry-run mode
- [x] Run full test suite and verify acceptance criteria


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

## [PROGRESS]

| Date | What was done |
|------|---------------|
| 2026-06-16 | Initial scaffold. Stack: Rust + Tokio + OpenRGB client. |

## [DISCOVERIES]

*None yet.*

## [OUTCOMES]

*None yet.*
