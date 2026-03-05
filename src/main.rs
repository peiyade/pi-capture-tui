use anyhow::Result;
use crossterm::{
    event::{
        DisableBracketedPaste, EnableBracketedPaste,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{
    io::stdout,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

mod app;
mod ui;
mod events;
mod ai;
mod config;

use app::{App, AppEvent};
use events::EventHandler;
use ai::AISecretary;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if exists
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                unsafe { std::env::set_var(key, value); }
            }
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableBracketedPaste)?;

    // Try to enable keyboard enhancement protocol
    // This helps distinguish key press/repeat/release events
    let kb_enhancement_enabled = stdout
        .execute(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
        ))
        .is_ok();

    // For terminals without keyboard enhancement (or with IME issues),
    // we'll handle events more conservatively
    if !kb_enhancement_enabled {
        eprintln!("Note: Keyboard enhancement not supported, using basic input handling");
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channels
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let (ai_tx, mut ai_rx) = mpsc::unbounded_channel();

    // Setup event handler
    let event_handler = EventHandler::new(event_tx.clone(), Duration::from_millis(100));
    tokio::spawn(event_handler.run());

    // Load config first
    let config = config::load_config()?;

    // Setup AI secretary with config
    let ai_secretary = AISecretary::new(config.ai.clone(), ai_tx);

    // Create app
    let mut app = App::new(config, ai_secretary);

    // Main loop
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(50);

    loop {
        // Render
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Handle events
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());

        tokio::select! {
            Some(event) = event_rx.recv() => {
                match event {
                    AppEvent::Tick => {}
                    AppEvent::Key(key) => {
                        if !app.handle_key_event(key).await? {
                            break;
                        }
                    }
                }
            }
            Some(text) = ai_rx.recv() => {
                app.update_secretary(text);
            }
            _ = tokio::time::sleep(timeout) => {
                last_tick = Instant::now();
            }
        }
    }

    // Cleanup
    let mut stdout_out = std::io::stdout();
    let _ = stdout_out.execute(PopKeyboardEnhancementFlags);
    stdout_out.execute(DisableBracketedPaste)?;
    stdout_out.execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
