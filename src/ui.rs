use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let size = f.size();

    // Split layout vertically:
    // 1. Header (status, search/mode)
    // 2. Game list (scrollable)
    // 3. Footer (keybindings help)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header + subheader
            Constraint::Min(1),    // Main list
            Constraint::Length(2), // Blank + Footer
        ])
        .split(size);

    render_header(f, app, chunks[0]);
    render_list(f, app, chunks[1]);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = if app.steam_online {
        ("Online", Color::Green)
    } else {
        ("Offline", Color::Red)
    };

    let title_line = Line::from(vec![
        Span::raw("Stui: A TUI Steam Game Launcher | Steam Status: "),
        Span::styled(
            status_text,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let sub_line = if app.searching {
        Line::from(vec![
            Span::styled(
                "Search: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&app.search_query),
        ])
    } else if !app.search_query.is_empty() {
        Line::from(vec![
            Span::styled(
                "Filtered: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&app.search_query),
        ])
    } else if app.show_hidden {
        Line::from(vec![Span::styled(
            "Showing Hidden Games",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )])
    } else {
        Line::default()
    };

    let header_paragraph = Paragraph::new(vec![title_line, sub_line, Line::default()]);
    f.render_widget(header_paragraph, area);

    if app.searching {
        let cursor_x = area.x + 8 + app.search_query.len() as u16;
        let cursor_y = area.y + 1;
        if cursor_x < area.right() && cursor_y < area.bottom() {
            f.set_cursor(cursor_x, cursor_y);
        }
    }
}

fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let visual_indices = app.filtered_indices();
    let available_height = area.height as usize;

    if visual_indices.is_empty() {
        let empty_msg = if app.show_hidden {
            "  (No hidden games)"
        } else if !app.search_query.is_empty() {
            "  (No games matching search)"
        } else {
            "  (No games found)"
        };

        let p = Paragraph::new(vec![Line::from(Span::styled(
            empty_msg,
            Style::default().fg(Color::DarkGray),
        ))]);
        f.render_widget(p, area);
        return;
    }

    let cursor_visual_idx = visual_indices
        .iter()
        .position(|&idx| idx == app.cursor)
        .unwrap_or(0);

    let start = if visual_indices.len() <= available_height {
        0
    } else {
        let half = available_height / 2;
        if cursor_visual_idx < half {
            0
        } else if cursor_visual_idx + (available_height - half) >= visual_indices.len() {
            visual_indices.len().saturating_sub(available_height)
        } else {
            cursor_visual_idx - half
        }
    };

    let end = (start + available_height).min(visual_indices.len());

    let mut lines = Vec::new();
    for &game_idx in &visual_indices[start..end] {
        let game = &app.games[game_idx];
        let is_selected = game_idx == app.cursor;

        let cursor_str = if is_selected { ">" } else { " " };
        let fav_str = if game.favorite { "*" } else { " " };
        let hours = game.playtime / 60;

        let cursor_style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let fav_style = if game.favorite {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let name_style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if game.favorite {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Reset)
        };

        let playtime_style = Style::default().fg(Color::DarkGray);

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", cursor_str), cursor_style),
            Span::styled(format!("{} ", fav_str), fav_style),
            Span::styled(format!("{} ", game.name), name_style),
            Span::styled(format!("[{} hours]", hours), playtime_style),
        ]));
    }

    let list_paragraph = Paragraph::new(lines).block(Block::default());
    f.render_widget(list_paragraph, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::DarkGray);

    let footer_line = Line::from(vec![
        Span::styled("q", key_style),
        Span::styled(":quit  ", desc_style),
        Span::styled("/", key_style),
        Span::styled(":search  ", desc_style),
        Span::styled("h", key_style),
        Span::styled(":hide  ", desc_style),
        Span::styled("H", key_style),
        Span::styled(":toggle view  ", desc_style),
        Span::styled("f", key_style),
        Span::styled(":fav  ", desc_style),
        Span::styled("O", key_style),
        Span::styled(":toggle status", desc_style),
    ]);

    let footer_paragraph = Paragraph::new(vec![Line::default(), footer_line]);
    f.render_widget(footer_paragraph, area);
}
