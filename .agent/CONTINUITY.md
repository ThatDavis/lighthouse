# CONTINUITY — Lighthouse

> Canonical project briefing. Read at session start.
> Stack: Rust + Tokio + custom OpenRGB SDK client
> Type: CLI daemon + TUI for Proxmox

## [PLANS]

### Milestone 1: Core Daemon (In Progress)
Goal: Read CPU temperature and map it to OpenRGB lighting via configurable thresholds, running as a headless daemon.
- [ ] Read CPU temperature using `sysinfo`
- [ ] Map temperature to color via configurable thresholds
- [ ] Connect to OpenRGB server and update lighting
- [ ] Load config from default or `--config` path
- [ ] Run as a systemd service
- [ ] `--dry-run` mode
- [ ] `lighthouse validate` command

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
