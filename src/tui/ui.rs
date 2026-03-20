use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crate::tui::app::{App, AppState};

pub fn render<B: Backend>(f: &mut Frame<B>, app: &App) {
    let size = f.size();

    // Three sections: messages (top), input (middle), status (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(size);

    // Chat messages pane
    render_messages(f, chunks[0], app);

    // Input pane
    render_input(f, chunks[1], app);

    // Status pane
    render_status(f, chunks[2], app);
}

fn render_messages<B: Backend>(f: &mut Frame<B>, area: ratatui::layout::Rect, app: &App) {
    let mut chat_text = String::new();

    for msg in &app.messages {
        // Message header with role
        let role = if msg.role == "you" { "👤 You" } else { "🤖 Assistant" };
        chat_text.push_str(&format!("{}\n", role));
        chat_text.push_str(&msg.content);
        chat_text.push_str("\n\n");
    }

    // Show streaming indicator
    if app.state == AppState::Streaming || app.state == AppState::Processing {
        if !app.streaming_buf.is_empty() {
            chat_text.push_str("🤖 Assistant\n");
            chat_text.push_str(&app.streaming_buf);
            chat_text.push_str("\n\n");
        } else {
            chat_text.push_str("⏳ Processing...\n");
        }
    }

    let paragraph = Paragraph::new(chat_text)
        .scroll((app.scroll, 0))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_input<B: Backend>(f: &mut Frame<B>, area: ratatui::layout::Rect, app: &App) {
    let input_text = if app.input.is_empty() {
        "Type your prompt... (Ctrl+C to quit)".to_string()
    } else {
        app.input.clone()
    };

    let input_style = if app.input.is_empty() {
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
    } else {
        Style::default().fg(Color::White)
    };

    let input_block = Block::default()
        .borders(Borders::TOP)
        .style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(input_text).block(input_block).style(input_style);
    f.render_widget(paragraph, area);
}

fn render_status<B: Backend>(f: &mut Frame<B>, area: ratatui::layout::Rect, app: &App) {
    let status_text = if app.state == AppState::Idle {
        format!(
            " [LOCAL] ready • depth: {} ",
            app.status.session_depth
        )
    } else {
        format!(
            " [LOCAL] processing... • depth: {} ",
            app.status.session_depth
        )
    };

    let paragraph = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}
