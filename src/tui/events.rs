use anyhow::Context;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use std::time::Duration;

use crate::tui::app::{App, InputMode, Screen};
use crate::tui::daemon;

pub async fn run(mut app: App) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;

    let result = run_loop(&mut terminal, &mut app).await;

    ratatui::restore();
    result
}

async fn run_loop(terminal: &mut DefaultTerminal, app: &mut App) -> anyhow::Result<()> {
    let mut last_status_check = std::time::Instant::now();

    loop {
        app.tick();

        if last_status_check.elapsed() >= Duration::from_secs(5) {
            app.daemon_status = match daemon::status() {
                Ok(true) => crate::tui::app::DaemonStatus::Running,
                Ok(false) => crate::tui::app::DaemonStatus::Stopped,
                Err(_) => crate::tui::app::DaemonStatus::Unknown,
            };
            last_status_check = std::time::Instant::now();
        }

        terminal.draw(|f| crate::tui::ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(app, key).await? {
                    return Ok(());
                }
            }
        }
    }
}

async fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

    if app.input_mode == InputMode::Editing {
        return Ok(handle_edit_key(app, key));
    }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('s') if app.screen == Screen::Daemon => match daemon::start() {
            Ok(_) => app.set_status("daemon started"),
            Err(e) => app.set_status(format!("start failed: {}", e)),
        },
        KeyCode::Char('S') if app.screen == Screen::Daemon => match daemon::stop() {
            Ok(_) => app.set_status("daemon stopped"),
            Err(e) => app.set_status(format!("stop failed: {}", e)),
        },
        KeyCode::Char('r') if app.screen == Screen::Daemon => match daemon::restart() {
            Ok(_) => app.set_status("daemon restarted"),
            Err(e) => app.set_status(format!("restart failed: {}", e)),
        },
        KeyCode::Char('u') if app.screen == Screen::Daemon => {
            app.daemon_status = match daemon::status() {
                Ok(true) => crate::tui::app::DaemonStatus::Running,
                Ok(false) => crate::tui::app::DaemonStatus::Stopped,
                Err(_) => crate::tui::app::DaemonStatus::Unknown,
            };
        }
        KeyCode::Char('1') => app.screen = Screen::Status,
        KeyCode::Char('2') => app.screen = Screen::Thresholds,
        KeyCode::Char('3') => app.screen = Screen::Colors,
        KeyCode::Char('4') => app.screen = Screen::Effects,
        KeyCode::Char('5') => app.screen = Screen::Daemon,
        KeyCode::Tab => {
            let screens = Screen::all();
            let idx = screens.iter().position(|s| *s == app.screen).unwrap_or(0);
            app.screen = screens[(idx + 1) % screens.len()];
        }
        KeyCode::BackTab => {
            let screens = Screen::all();
            let idx = screens.iter().position(|s| *s == app.screen).unwrap_or(0);
            app.screen = screens[(idx + screens.len() - 1) % screens.len()];
        }
        KeyCode::Up => move_selection(app, -1),
        KeyCode::Down => move_selection(app, 1),
        KeyCode::Enter => app.input_mode = InputMode::Editing,
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => save_config(app)?,
        _ => {}
    }

    Ok(false)
}

fn handle_edit_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => {
            apply_edit(app);
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            revert_edit(app);
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char(c) => push_edit(app, c),
        KeyCode::Backspace => backspace_edit(app),
        _ => {}
    }
    false
}

fn move_selection(app: &mut App, delta: isize) {
    let max = match app.screen {
        Screen::Thresholds => 2,
        Screen::Colors => 2,
        Screen::Effects => 3,
        _ => return,
    };
    let new = app.selected_field as isize + delta;
    app.selected_field = new.clamp(0, max as isize) as usize;
}

fn push_edit(app: &mut App, c: char) {
    match app.screen {
        Screen::Thresholds => app.thresholds[app.selected_field].push(c),
        Screen::Effects => match app.selected_field {
            0 => app.active_profile.push(c),
            1 => app.temp_smoothing.push(c),
            2 => app.transition_steps.push(c),
            _ => app.transition_interval.push(c),
        },
        Screen::Colors => app.colors[app.selected_field].push(c),
        _ => {}
    }
}

fn backspace_edit(app: &mut App) {
    match app.screen {
        Screen::Thresholds => {
            app.thresholds[app.selected_field].pop();
        }
        Screen::Colors => {
            app.colors[app.selected_field].pop();
        }
        Screen::Effects => match app.selected_field {
            0 => {
                let _ = app.active_profile.pop();
            }
            1 => {
                let _ = app.temp_smoothing.pop();
            }
            2 => {
                let _ = app.transition_steps.pop();
            }
            _ => {
                let _ = app.transition_interval.pop();
            }
        },
        _ => {}
    }
}

fn apply_edit(app: &mut App) {
    match app.screen {
        Screen::Thresholds => {
            if let Ok(value) = app.thresholds[app.selected_field].parse() {
                match app.selected_field {
                    0 => app.config.temperature.cold = value,
                    1 => app.config.temperature.warm = value,
                    2 => app.config.temperature.hot = value,
                    _ => {}
                }
            }
        }
        Screen::Colors => {
            let parts: Vec<&str> = app.colors[app.selected_field].split(',').collect();
            if parts.len() == 3 {
                if let Some(color) = crate::tui::app::parse_color([parts[0], parts[1], parts[2]]) {
                    match app.selected_field {
                        0 => app.config.colors.cold = color,
                        1 => app.config.colors.warm = color,
                        2 => app.config.colors.hot = color,
                        _ => {}
                    }
                }
            }
        }
        Screen::Effects => match app.selected_field {
            0 => app.config.effects.active_profile = app.active_profile.clone(),
            1 => {
                if let Ok(value) = app.temp_smoothing.parse() {
                    app.config.temp_smoothing = value;
                }
            }
            2 => {
                if let Ok(value) = app.transition_steps.parse() {
                    app.config.transition_steps = value;
                }
            }
            _ => {
                if let Ok(value) = app.transition_interval.parse() {
                    app.config.transition_interval_ms = value;
                }
            }
        },
        _ => {}
    }
}

fn revert_edit(app: &mut App) {
    match app.screen {
        Screen::Thresholds => {
            app.thresholds = [
                app.config.temperature.cold.to_string(),
                app.config.temperature.warm.to_string(),
                app.config.temperature.hot.to_string(),
            ];
        }
        Screen::Colors => {
            app.colors = [
                color_to_string(app.config.colors.cold),
                color_to_string(app.config.colors.warm),
                color_to_string(app.config.colors.hot),
            ];
        }
        Screen::Effects => {
            app.active_profile = app.config.effects.active_profile.clone();
            app.temp_smoothing = app.config.temp_smoothing.to_string();
            app.transition_steps = app.config.transition_steps.to_string();
            app.transition_interval = app.config.transition_interval_ms.to_string();
        }
        _ => {}
    }
}

fn color_to_string(color: [u8; 3]) -> String {
    format!("{}, {}, {}", color[0], color[1], color[2])
}

fn save_config(app: &mut App) -> anyhow::Result<()> {
    app.config.validate()?;
    let contents = toml::to_string_pretty(&app.config).context("failed to serialize config")?;
    std::fs::write(&app.config_path, contents).context("failed to write config")?;
    app.set_status("config saved");
    Ok(())
}
