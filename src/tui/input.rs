use crate::tui::app::App;
use crossterm::event::{KeyEvent, KeyModifiers};

pub async fn handle_key(app: &mut App, key: KeyEvent) {
    use crossterm::event::KeyCode::*;

    match key.code {
        Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            match c {
                'c' | 'q' => app.should_quit = true,
                'l' => app.clear_chat(),
                _ => {}
            }
        }
        Char(c) => {
            app.handle_char(c);
        }
        Backspace => {
            app.handle_backspace();
        }
        Enter => {
            let _ = app.submit_input().await;
        }
        Esc => {
            app.should_quit = true;
        }
        Up => {
            app.scroll_up();
        }
        Down => {
            app.scroll_down();
        }
        _ => {}
    }
}
