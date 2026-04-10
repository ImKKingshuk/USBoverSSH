//! TUI Widgets and Rendering

use super::app::{App, Pane, Popup};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
};

/// Main render function
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);

    // Render popup if active
    match &app.popup {
        Popup::Help => render_help_popup(frame),
        Popup::Connect => render_connect_popup(frame),
        Popup::Error(msg) => render_error_popup(frame, msg),
        Popup::Confirm { title, message } => render_confirm_popup(frame, title, message),
        Popup::None => {}
    }
}

/// Render header with tabs
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["Local Devices", "Remote Devices", "Attached", "Hosts", "Pool Status", "Cache Status"];
    let selected = match app.active_pane {
        Pane::LocalDevices => 0,
        Pane::RemoteDevices => 1,
        Pane::AttachedDevices => 2,
        Pane::Hosts => 3,
        Pane::PoolStatus => 4,
        Pane::CacheStatus => 5,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" 🔌 USBoverSSH ")
                .title_style(Style::default().bold().fg(Color::Cyan)),
        )
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" │ ");

    frame.render_widget(tabs, area);
}

/// Render main content area
fn render_main(frame: &mut Frame, app: &App, area: Rect) {
    match app.active_pane {
        Pane::LocalDevices => render_local_devices(frame, app, area),
        Pane::RemoteDevices => render_remote_devices(frame, app, area),
        Pane::AttachedDevices => render_attached_devices(frame, app, area),
        Pane::Hosts => render_hosts(frame, app, area),
        Pane::PoolStatus => render_pool_status(frame, app, area),
        Pane::CacheStatus => render_cache_status(frame, app, area),
    }
}

/// Render local devices list
fn render_local_devices(frame: &mut Frame, app: &App, area: Rect) {
    let selected = app.selected.get(&Pane::LocalDevices).copied().unwrap_or(0);

    let header = Row::new(vec![
        Cell::from("Bus ID").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("VID:PID").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Class").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Product").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Speed").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .local_devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            let style = if i == selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            let status = if device.is_attached {
                "●"
            } else if device.is_bound {
                "○"
            } else {
                " "
            };

            Row::new(vec![
                Cell::from(format!("{} {}", status, device.bus_id)),
                Cell::from(device.vid_pid()),
                Cell::from(format!("[{}]", device.device_class.short_name())),
                Cell::from(device.display_name()),
                Cell::from(device.speed.as_str()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(11),
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(18),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(format!(" Local Devices ({}) ", app.local_devices.len()))
            .title_style(Style::default().bold()),
    );

    frame.render_widget(table, area);
}

/// Render remote devices
fn render_remote_devices(frame: &mut Frame, app: &App, area: Rect) {
    let total: usize = app.remote_devices.values().map(|v| v.len()).sum();

    let items: Vec<ListItem> = if app.hosts.iter().any(|h| h.connected) {
        app.remote_devices
            .iter()
            .flat_map(|(host, devices)| {
                std::iter::once(
                    ListItem::new(format!("── {} ──", host))
                        .style(Style::default().fg(Color::Cyan).bold()),
                )
                .chain(devices.iter().map(|d| {
                    ListItem::new(format!(
                        "  {} {} {}",
                        d.bus_id,
                        d.vid_pid(),
                        d.display_name()
                    ))
                }))
            })
            .collect()
    } else {
        vec![ListItem::new("  No hosts connected").style(Style::default().fg(Color::DarkGray))]
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .title(format!(" Remote Devices ({}) ", total))
            .title_style(Style::default().bold()),
    );

    frame.render_widget(list, area);
}

/// Render attached devices
fn render_attached_devices(frame: &mut Frame, app: &App, area: Rect) {
    let selected = app
        .selected
        .get(&Pane::AttachedDevices)
        .copied()
        .unwrap_or(0);

    let header = Row::new(vec![
        Cell::from("Port").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Bus ID").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Host").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Speed").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .attached_devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            let style = if i == selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(device.port.to_string()),
                Cell::from(device.bus_id.clone()),
                Cell::from(device.host.clone()),
                Cell::from(device.speed.clone()),
            ])
            .style(style)
        })
        .collect();

    let content = if rows.is_empty() {
        Table::new(
            vec![Row::new(vec![Cell::from("  No devices attached")])
                .style(Style::default().fg(Color::DarkGray))],
            [Constraint::Percentage(100)],
        )
    } else {
        Table::new(
            rows,
            [
                Constraint::Length(6),
                Constraint::Length(15),
                Constraint::Min(15),
                Constraint::Length(15),
            ],
        )
        .header(header)
    };

    let table = content.block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(format!(
                " Attached Devices ({}) ",
                app.attached_devices.len()
            ))
            .title_style(Style::default().bold()),
    );

    frame.render_widget(table, area);
}

/// Render hosts list
fn render_hosts(frame: &mut Frame, app: &App, area: Rect) {
    let selected = app.selected.get(&Pane::Hosts).copied().unwrap_or(0);

    let items: Vec<ListItem> = app
        .hosts
        .iter()
        .enumerate()
        .map(|(i, host)| {
            let status = if host.connected { "●" } else { "○" };
            let status_color = if host.connected {
                Color::Green
            } else {
                Color::DarkGray
            };

            let style = if i == selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", status), Style::default().fg(status_color)),
                Span::styled(&host.name, Style::default().bold()),
                Span::raw(" "),
                Span::styled(
                    format!("({})", host.hostname),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .style(style)
        })
        .collect();

    let list = if items.is_empty() {
        List::new(vec![
            ListItem::new("  No hosts configured").style(Style::default().fg(Color::DarkGray))
        ])
    } else {
        List::new(items)
    };

    let list = list.block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(format!(" Hosts ({}) ", app.hosts.len()))
            .title_style(Style::default().bold()),
    );

    frame.render_widget(list, area);
}

/// Render status bar
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let message = app
        .status_message
        .as_ref()
        .map(|(m, _)| m.as_str())
        .unwrap_or("");

    let status = Paragraph::new(Line::from(vec![
        Span::styled(" ? ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Help"),
        Span::raw(" │ "),
        Span::styled(" Tab ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Switch Pane"),
        Span::raw(" │ "),
        Span::styled(" r ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Refresh"),
        Span::raw(" │ "),
        Span::styled(" a ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Attach"),
        Span::raw(" │ "),
        Span::styled(" d ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Detach"),
        Span::raw(" │ "),
        Span::styled(" q ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Quit"),
        Span::raw("   "),
        Span::styled(message, Style::default().fg(Color::Cyan)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(status, area);
}

/// Render help popup
fn render_help_popup(frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let help_text = vec![
        "",
        "  Navigation",
        "  ──────────────────────────────────",
        "  Tab / Shift+Tab    Switch panes",
        "  ↑/k  ↓/j           Navigate items",
        "  Enter              Activate item",
        "",
        "  Actions",
        "  ──────────────────────────────────",
        "  a                  Attach device",
        "  d                  Detach device",
        "  r / F5             Refresh devices",
        "  c                  Connect to host",
        "  h                  Hosts panel",
        "  s                  Toggle status",
        "",
        "  General",
        "  ──────────────────────────────────",
        "  ? / F1             Toggle help",
        "  q / Esc            Quit / Close",
        "",
    ];

    let paragraph = Paragraph::new(help_text.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Help ")
                .title_style(Style::default().bold().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render connect popup
fn render_connect_popup(frame: &mut Frame) {
    let area = centered_rect(50, 30, frame.area());
    frame.render_widget(Clear, area);

    let paragraph = Paragraph::new(
        "\n  Enter host: user@hostname[:port]\n\n  Press Enter to connect, Esc to cancel",
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(" Connect to Host ")
            .title_style(Style::default().bold().fg(Color::Green)),
    );

    frame.render_widget(paragraph, area);
}

/// Render error popup
fn render_error_popup(frame: &mut Frame, message: &str) {
    let area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, area);

    let paragraph = Paragraph::new(format!("\n  {}\n\n  Press any key to dismiss", message))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" Error ")
                .title_style(Style::default().bold().fg(Color::Red)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render confirm popup
fn render_confirm_popup(frame: &mut Frame, title: &str, message: &str) {
    let area = centered_rect(50, 20, frame.area());
    frame.render_widget(Clear, area);

    let paragraph = Paragraph::new(format!("\n  {}\n\n  [Y]es  [N]o", message))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(format!(" {} ", title))
                .title_style(Style::default().bold().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Create a centered rect
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
