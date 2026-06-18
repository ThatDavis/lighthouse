use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::tui::app::{App, InputMode, Screen};

pub fn draw(frame: &mut Frame, app: &App) {
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

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Screen::all()
        .iter()
        .enumerate()
        .map(|(i, screen)| {
            let style = if *screen == app.screen {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            let prefix = if i < 9 {
                format!("{}.", i + 1)
            } else {
                String::new()
            };
            Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("{}{}", prefix, screen.title()), style),
                Span::raw(" "),
            ])
        })
        .collect();

    let tabs = Text::from(titles);
    let paragraph = Paragraph::new(tabs).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Lighthouse")
            .title_alignment(Alignment::Center),
    );
    frame.render_widget(paragraph, area);
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
        Line::from(vec![Span::styled(
            "Current Status",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        )]),
        Line::raw(""),
        Line::from(vec![Span::raw(format!("CPU Temp:    {}", temp))]),
        Line::from(vec![Span::raw(format!("CPU Usage:   {}", usage))]),
        Line::from(vec![Span::raw(format!("Color:       {}", color))]),
        Line::from(vec![Span::raw(format!(
            "Profile:     {}",
            app.current_profile
        ))]),
        Line::from(vec![Span::raw(format!("Daemon:      {}", daemon))]),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Status"))
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
            let style = if app.selected_field == i {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}: {}", label, value)).style(style)
        })
        .collect();

    let list = List::new(rows).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Temperature Thresholds (°C)"),
    );
    frame.render_widget(list, area);

    if app.input_mode == InputMode::Editing {
        draw_input_popup(frame, area, &app.thresholds[app.selected_field]);
    }
}

fn draw_colors(frame: &mut Frame, app: &App, area: Rect) {
    let labels = ["Cold", "Warm", "Hot"];
    let rows: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let value = &app.colors[i];
            let style = if app.selected_field == i {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(format!("{}: [{}]", label, value)).style(style)
        })
        .collect();

    let list = List::new(rows).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Colors [r, g, b]"),
    );
    frame.render_widget(list, area);

    if app.input_mode == InputMode::Editing {
        draw_input_popup(frame, area, &app.colors[app.selected_field]);
    }
}

fn draw_effects(frame: &mut Frame, app: &App, area: Rect) {
    let fields = [
        format!("Active Profile: {}", app.active_profile),
        format!("Temp Smoothing: {}", app.temp_smoothing),
        format!("Transition Steps: {}", app.transition_steps),
        format!("Transition Interval (ms): {}", app.transition_interval),
    ];
    let rows: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let style = if app.selected_field == i {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(text.clone()).style(style)
        })
        .collect();

    let list = List::new(rows).block(Block::default().borders(Borders::ALL).title("Effects"));
    frame.render_widget(list, area);

    if app.input_mode == InputMode::Editing {
        let value = match app.selected_field {
            0 => app.active_profile.clone(),
            1 => app.temp_smoothing.clone(),
            2 => app.transition_steps.clone(),
            _ => app.transition_interval.clone(),
        };
        draw_input_popup(frame, area, &value);
    }
}

fn draw_daemon(frame: &mut Frame, _app: &App, area: Rect) {
    let help = Paragraph::new(Text::from(vec![
        Line::from(vec![Span::raw("s: start daemon")]),
        Line::from(vec![Span::raw("S: stop daemon")]),
        Line::from(vec![Span::raw("r: restart daemon")]),
        Line::from(vec![Span::raw("u: refresh status")]),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Daemon Control"),
    );
    frame.render_widget(help, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help = if app.input_mode == InputMode::Editing {
        "Enter: confirm | Esc: cancel"
    } else {
        "Tab/1-5: switch screen | ↑↓: select | Enter: edit | q: quit | Ctrl+S: save config"
    };
    let status = format!("{} | {}", help, app.status_message);
    let paragraph = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_input_popup(frame: &mut Frame, area: Rect, value: &str) {
    let popup = centered_rect(60, 20, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Edit")
        .title_alignment(Alignment::Center);
    let paragraph = Paragraph::new(value).block(block).wrap(Wrap { trim: true });
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
