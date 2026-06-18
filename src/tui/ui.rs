use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, BorderType, Clear, List, ListItem, Padding, Paragraph, Wrap},
};

use crate::tui::app::{App, InputMode, Screen};
use crate::tui::theme::MOCHA;

pub fn draw(frame: &mut Frame, app: &App) {
    frame.render_widget(
        Block::default().style(Style::default().bg(MOCHA.base)),
        frame.area(),
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_main(frame, app, chunks[1]);
    draw_footer(frame, app, chunks[2]);
}

fn base_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(MOCHA.surface1))
        .style(Style::default().bg(MOCHA.base).fg(MOCHA.text))
        .padding(Padding::horizontal(1))
        .title(Span::styled(title, Style::default().fg(MOCHA.lavender).add_modifier(Modifier::BOLD)))
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let cells: Vec<Line> = Screen::all()
        .iter()
        .enumerate()
        .map(|(i, screen)| {
            let active = *screen == app.screen;
            let (fg, bg) = if active {
                (MOCHA.base, MOCHA.mauve)
            } else {
                (MOCHA.subtext0, MOCHA.surface0)
            };
            let style = Style::default()
                .fg(fg)
                .bg(bg)
                .add_modifier(if active { Modifier::BOLD } else { Modifier::empty() });
            Line::from(vec![
                Span::styled(format!(" {}.{}", i + 1, screen.title()), style),
                Span::raw(" "),
            ])
        })
        .collect();

    let header = Paragraph::new(Text::from(cells))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(MOCHA.surface1))
                .style(Style::default().bg(MOCHA.base))
                .title(Span::styled(
                    " Lighthouse ",
                    Style::default().fg(MOCHA.rosewater).add_modifier(Modifier::BOLD),
                )),
        );
    frame.render_widget(header, area);
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect) {
    match app.screen {
        Screen::Status => draw_status(frame, app, area),
        Screen::Thresholds => draw_thresholds(frame, app, area),
        Screen::Colors => draw_colors(frame, app, area),
        Screen::Effects => draw_effects(frame, app, area),
        Screen::Daemon => draw_daemon(frame, app, area),
    }
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let temp = app
        .current_temp
        .map(|t| format!("{:.1}°C", t))
        .unwrap_or_else(|| "n/a".to_string());
    let usage = format!("{:.1}%", app.current_usage);
    let color = format!(
        "rgb({},{},{})",
        app.current_color[0], app.current_color[1], app.current_color[2]
    );
    let daemon = format!("{:?}", app.daemon_status);

    let lines = vec![
        Line::from(vec![
            Span::styled("CPU Temp", Style::default().fg(MOCHA.blue)),
            Span::raw(": "),
            Span::styled(temp, Style::default().fg(MOCHA.green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("CPU Usage", Style::default().fg(MOCHA.blue)),
            Span::raw(": "),
            Span::styled(usage, Style::default().fg(MOCHA.green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Color", Style::default().fg(MOCHA.blue)),
            Span::raw(": "),
            Span::styled(color, Style::default().fg(MOCHA.peach).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Profile", Style::default().fg(MOCHA.blue)),
            Span::raw(": "),
            Span::styled(
                app.current_profile.clone(),
                Style::default().fg(MOCHA.mauve).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Daemon", Style::default().fg(MOCHA.blue)),
            Span::raw(": "),
            Span::styled(daemon, Style::default().fg(MOCHA.sky).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(base_block("Status"))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_thresholds(frame: &mut Frame, app: &App, area: Rect) {
    let labels = ["Cold", "Warm", "Hot"];
    let rows: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let value = &app.thresholds[i];
            let selected = app.selected_field == i;
            let style = if selected {
                Style::default().fg(MOCHA.base).bg(MOCHA.yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MOCHA.text)
            };
            let marker = if selected { "▸ " } else { "  " };
            ListItem::new(format!("{}{}: {}°C", marker, label, value)).style(style)
        })
        .collect();

    let list = List::new(rows).block(base_block("Temperature Thresholds"));
    frame.render_widget(list, area);

    if app.input_mode == InputMode::Editing {
        draw_input_popup(frame, area, &app.thresholds[app.selected_field], "Edit threshold");
    }
}

fn draw_colors(frame: &mut Frame, app: &App, area: Rect) {
    let labels = ["Cold", "Warm", "Hot"];
    let rows: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let value = &app.colors[i];
            let selected = app.selected_field == i;
            let style = if selected {
                Style::default().fg(MOCHA.base).bg(MOCHA.yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MOCHA.text)
            };
            let marker = if selected { "▸ " } else { "  " };
            ListItem::new(format!("{}{}: [{}]", marker, label, value)).style(style)
        })
        .collect();

    let list = List::new(rows).block(base_block("Colors [r, g, b]"));
    frame.render_widget(list, area);

    if app.input_mode == InputMode::Editing {
        draw_input_popup(frame, area, &app.colors[app.selected_field], "Edit color as r, g, b");
    }
}

fn draw_effects(frame: &mut Frame, app: &App, area: Rect) {
    let fields = [
        ("Active Profile", app.active_profile.as_str()),
        ("Temp Smoothing", app.temp_smoothing.as_str()),
        ("Transition Steps", app.transition_steps.as_str()),
        ("Transition Interval (ms)", app.transition_interval.as_str()),
    ];
    let rows: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let selected = app.selected_field == i;
            let style = if selected {
                Style::default().fg(MOCHA.base).bg(MOCHA.yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MOCHA.text)
            };
            let marker = if selected { "▸ " } else { "  " };
            ListItem::new(format!("{}{}: {}", marker, label, value)).style(style)
        })
        .collect();

    let list = List::new(rows).block(base_block("Effects"));
    frame.render_widget(list, area);

    if app.input_mode == InputMode::Editing {
        let (title, value) = match app.selected_field {
            0 => ("Edit active profile", app.active_profile.as_str()),
            1 => ("Edit temp smoothing", app.temp_smoothing.as_str()),
            2 => ("Edit transition steps", app.transition_steps.as_str()),
            _ => ("Edit transition interval", app.transition_interval.as_str()),
        };
        draw_input_popup(frame, area, value, title);
    }
}

fn draw_daemon(frame: &mut Frame, app: &App, area: Rect) {
    let status = format!("{:?}", app.daemon_status);
    let lines = vec![
        Line::from(vec![
            Span::styled("Current status", Style::default().fg(MOCHA.blue)),
            Span::raw(": "),
            Span::styled(status, Style::default().fg(MOCHA.sky).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "Shortcuts",
            Style::default().fg(MOCHA.lavender).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("s", Style::default().fg(MOCHA.peach).add_modifier(Modifier::BOLD)),
            Span::raw(" start daemon"),
        ]),
        Line::from(vec![
            Span::styled("S", Style::default().fg(MOCHA.peach).add_modifier(Modifier::BOLD)),
            Span::raw(" stop daemon"),
        ]),
        Line::from(vec![
            Span::styled("r", Style::default().fg(MOCHA.peach).add_modifier(Modifier::BOLD)),
            Span::raw(" restart daemon"),
        ]),
        Line::from(vec![
            Span::styled("u", Style::default().fg(MOCHA.peach).add_modifier(Modifier::BOLD)),
            Span::raw(" refresh status"),
        ]),
    ];

    let paragraph = Paragraph::new(Text::from(lines)).block(base_block("Daemon Control"));
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help = if app.input_mode == InputMode::Editing {
        "Enter: confirm | Esc: cancel".to_string()
    } else if app.status_message.starts_with("error:") {
        app.status_message.clone()
    } else {
        format!(
            "Tab / 1-5: switch screen | ↑↓: select | Enter: edit | q: quit | Ctrl+S: save | {}",
            app.status_message
        )
    };

    let style = if app.input_mode == InputMode::Editing {
        Style::default().fg(MOCHA.yellow)
    } else if app.status_message.starts_with("error:") {
        Style::default().fg(MOCHA.red)
    } else {
        Style::default().fg(MOCHA.subtext0)
    };

    let paragraph = Paragraph::new(help)
        .style(style.patch(Style::default().bg(MOCHA.surface0)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(MOCHA.surface1)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_input_popup(frame: &mut Frame, area: Rect, value: &str, title: &str) {
    let popup = centered_rect(60, 25, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(MOCHA.mauve))
        .style(Style::default().bg(MOCHA.surface0).fg(MOCHA.text))
        .title(Span::styled(
            title,
            Style::default().fg(MOCHA.lavender).add_modifier(Modifier::BOLD),
        ));
    let paragraph = Paragraph::new(value)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, popup);
    frame.render_widget(paragraph, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
