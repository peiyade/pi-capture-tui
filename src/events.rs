use crate::app::AppEvent;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct EventHandler {
    sender: mpsc::UnboundedSender<AppEvent>,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(sender: mpsc::UnboundedSender<AppEvent>, tick_rate: Duration) -> Self {
        Self { sender, tick_rate }
    }

    pub async fn run(self) {
        let mut last_tick = tokio::time::Instant::now();

        loop {
            let timeout = self.tick_rate.saturating_sub(last_tick.elapsed());

            if event::poll(timeout).unwrap_or(false) {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        // Filter: only handle key press events (not Repeat or Release)
                        // This helps with IME input methods (e.g., Rime/Squirrel)
                        // which may generate intermediate events during composition
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }

                        if self.sender.send(AppEvent::Key(key)).is_err() {
                            break;
                        }
                    }
                    Ok(Event::Paste(text)) => {
                        // Handle paste events - send as a single paste event
                        // The app will append this at cursor position
                        for c in text.chars() {
                            if c == '\n' || c == '\r' {
                                // Send Enter for newlines
                                let key = KeyEvent::from(KeyCode::Enter);
                                if self.sender.send(AppEvent::Key(key)).is_err() {
                                    break;
                                }
                            } else {
                                let key = KeyEvent::from(KeyCode::Char(c));
                                if self.sender.send(AppEvent::Key(key)).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if last_tick.elapsed() >= self.tick_rate {
                if self.sender.send(AppEvent::Tick).is_err() {
                    break;
                }
                last_tick = tokio::time::Instant::now();
            }
        }
    }
}
