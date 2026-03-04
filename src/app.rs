use crate::ai::AISecretary;
use crate::config::Config;
use anyhow::Result;
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Entry {
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Collecting,
    Command,        // 命令模式（:q 退出, :h 帮助）
    Searching,      // 搜索模式
    Help,           // 帮助模式
}

// 用于 undo/redo 的状态快照
#[derive(Debug, Clone)]
pub struct InputSnapshot {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

impl InputSnapshot {
    pub fn from_state(state: &InputState) -> Self {
        Self {
            lines: state.lines.clone(),
            cursor_line: state.cursor_line,
            cursor_col: state.cursor_col,
        }
    }

    pub fn apply_to(&self, state: &mut InputState) {
        state.lines = self.lines.clone();
        state.cursor_line = self.cursor_line;
        state.cursor_col = self.cursor_col;
    }
}

#[derive(Debug)]
pub struct InputState {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub last_activity: Instant,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            last_activity: Instant::now(),
        }
    }

    pub fn current_line(&self) -> &String {
        &self.lines[self.cursor_line]
    }

    pub fn current_line_mut(&mut self) -> &mut String {
        &mut self.lines[self.cursor_line]
    }

    pub fn to_string(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
    }
}

#[derive(Debug)]
pub enum AppEvent {
    Tick,
    Key(KeyEvent),
}

pub struct App {
    pub config: Config,
    pub mode: Mode,
    pub history: Vec<Entry>,
    pub input: InputState,
    pub secretary_content: String,
    pub ai_pending: bool,
    pub ai_secretary: AISecretary,
    pub entries_count: usize,
    pub last_input_time: Instant,

    // Undo/Redo
    undo_stack: Vec<InputSnapshot>,
    redo_stack: Vec<InputSnapshot>,
    max_undo_size: usize,

    // Search
    pub search_query: String,
    pub search_results: Vec<(usize, usize)>, // (entry_index, match_start_pos)
    pub current_search_idx: usize,
}

impl App {
    pub fn new(config: Config, ai_secretary: AISecretary) -> Self {
        Self {
            config,
            mode: Mode::Collecting,
            history: Vec::new(),
            input: InputState::new(),
            secretary_content: String::from("🤖 秘书已就绪，开始记录你的想法..."),
            ai_pending: false,
            ai_secretary,
            entries_count: 0,
            last_input_time: Instant::now(),

            // Undo/Redo
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_size: 100,

            // Search
            search_query: String::new(),
            search_results: Vec::new(),
            current_search_idx: 0,
        }
    }

    // Save current state for undo
    pub fn save_undo_state(&mut self) {
        let snapshot = InputSnapshot::from_state(&self.input);
        self.undo_stack.push(snapshot);

        // Limit undo stack size
        if self.undo_stack.len() > self.max_undo_size {
            self.undo_stack.remove(0);
        }

        // Clear redo stack when new action is performed
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            // Save current state to redo stack
            let current = InputSnapshot::from_state(&self.input);
            self.redo_stack.push(current);

            // Apply undo
            snapshot.apply_to(&mut self.input);
        }
    }

    pub fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop() {
            // Save current state to undo stack
            let current = InputSnapshot::from_state(&self.input);
            self.undo_stack.push(current);

            // Apply redo
            snapshot.apply_to(&mut self.input);
        }
    }

    // Search functionality
    pub fn start_search(&mut self) {
        self.mode = Mode::Searching;
        self.search_query.clear();
        self.search_results.clear();
        self.current_search_idx = 0;
    }

    pub fn cancel_search(&mut self) {
        self.mode = Mode::Collecting;
        self.search_query.clear();
        self.search_results.clear();
    }

    pub fn update_search(&mut self) {
        self.search_results.clear();

        if self.search_query.is_empty() {
            return;
        }

        // Search in history entries
        for (idx, entry) in self.history.iter().enumerate() {
            if let Some(pos) = entry.content.to_lowercase()
                .find(&self.search_query.to_lowercase()) {
                self.search_results.push((idx, pos));
            }
        }

        // Reset current index
        self.current_search_idx = 0;
    }

    pub fn next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_idx = (self.current_search_idx + 1) % self.search_results.len();
        }
    }

    pub fn prev_search_result(&mut self) {
        if !self.search_results.is_empty() {
            if self.current_search_idx == 0 {
                self.current_search_idx = self.search_results.len() - 1;
            } else {
                self.current_search_idx -= 1;
            }
        }
    }

    pub fn update_secretary(&mut self, text: String) {
        self.secretary_content = text;
        self.ai_pending = false;
    }

    pub fn on_input_changed(&mut self, _text: &str) {
        self.last_input_time = Instant::now();
        // AI 不再在输入过程中触发，而是在提交后评价
    }

    pub fn submit_entry(&mut self) -> Result<()> {
        if self.input.is_empty() {
            return Ok(());
        }

        let content = self.input.to_string();
        let entry = Entry {
            content: content.clone(),
            timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        };

        // Save to file
        self.save_to_file(&entry)?;

        // Add to history
        self.history.push(entry);
        self.entries_count += 1;

        // Clear input and undo/redo stacks for new entry
        self.input.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();

        // Update secretary - AI 将对刚提交的想法发出感慨
        self.secretary_content = format!("✅ 已收集第 {} 个想法，秘书思考中...", self.entries_count);

        // 请求 AI 对刚提交的内容发出感慨
        self.ai_pending = true;
        let _ = self.ai_secretary.request_analysis(content, self.history.clone());

        Ok(())
    }

    fn save_to_file(&self, entry: &Entry) -> Result<()> {
        use std::fs::{OpenOptions, File};
        use std::io::{Read, Write};
        use chrono::{DateTime, Local, Datelike, Weekday};

        let file_path = &self.config.capture_path;

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Parse entry timestamp to extract date components
        // timestamp format: 2026-03-04T16:01:27
        let dt = DateTime::parse_from_rfc3339(&format!("{}+08:00", entry.timestamp))
            .unwrap_or_else(|_| Local::now().into());

        let year = dt.year();
        let month_num = dt.month();
        let day = dt.day();
        let weekday = dt.weekday();

        // Month names in English (3-letter abbrev)
        let month_names = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun",
                          "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let month_name = month_names[month_num as usize];

        // Weekday names in English (3-letter abbrev)
        let weekday_name = match weekday {
            Weekday::Mon => "Mon",
            Weekday::Tue => "Tue",
            Weekday::Wed => "Wed",
            Weekday::Thu => "Thu",
            Weekday::Fri => "Fri",
            Weekday::Sat => "Sat",
            Weekday::Sun => "Sun",
        };

        // Build headings
        let year_heading = format!("# {}", year);
        let month_heading = format!("## {} {}", year, month_name);
        let day_heading = format!("### {} {} {} {}", year, month_name, day, weekday_name);

        // Entry line with task checkbox and timezone
        // Format: - [ ] content [created:: 2026-03-04T16:01:27+08:00]
        let entry_line = format!(
            "- [ ] {} [created:: {}+08:00]\n",
            entry.content.replace('\n', " "),
            entry.timestamp
        );

        // Read existing file content
        let mut existing_content = String::new();
        let file_exists = file_path.exists();

        if file_exists {
            let mut file = File::open(file_path)?;
            file.read_to_string(&mut existing_content)?;
        }

        // Determine where to insert the new entry
        let mut new_content = existing_content.clone();

        if !file_exists || existing_content.trim().is_empty() {
            // New file - create full structure
            new_content = format!(
                "{}\n\n{}\n\n{}\n\n{}",
                year_heading,
                month_heading,
                day_heading,
                entry_line
            );
        } else {
            // Check if year heading exists
            if !existing_content.contains(&year_heading) {
                // Add year, month, day at the end
                new_content.push_str(&format!("\n\n{}\n\n{}\n\n{}\n\n{}",
                    year_heading, month_heading, day_heading, entry_line));
            } else if !existing_content.contains(&month_heading) {
                // Year exists but not month - insert after year heading
                let year_pos = existing_content.find(&year_heading).unwrap();
                let insert_pos = existing_content[year_pos..]
                    .find('\n')
                    .map(|p| year_pos + p)
                    .unwrap_or(existing_content.len());

                new_content = format!(
                    "{}\n\n{}\n\n{}\n\n{}{}",
                    &existing_content[..insert_pos],
                    month_heading,
                    day_heading,
                    entry_line,
                    &existing_content[insert_pos..]
                );
            } else if !existing_content.contains(&day_heading) {
                // Year and month exist but not day - insert after month heading
                let month_pos = existing_content.rfind(&month_heading).unwrap();
                let insert_pos = existing_content[month_pos..]
                    .find('\n')
                    .map(|p| month_pos + p)
                    .unwrap_or(existing_content.len());

                new_content = format!(
                    "{}\n\n{}\n\n{}{}",
                    &existing_content[..insert_pos],
                    day_heading,
                    entry_line,
                    &existing_content[insert_pos..]
                );
            } else {
                // All headings exist - append to the end of today's section
                // Find the day heading and append after the last entry under it
                let day_pos = existing_content.rfind(&day_heading).unwrap();
                let after_day = &existing_content[day_pos + day_heading.len()..];

                // Find the next heading (if any) or end of file
                let next_heading_pos = after_day.find("\n#").unwrap_or(after_day.len());

                let insert_pos = day_pos + day_heading.len() + next_heading_pos;

                new_content = format!(
                    "{}\n{}{}",
                    &existing_content[..insert_pos],
                    entry_line,
                    &existing_content[insert_pos..]
                );
            }
        }

        // Write the updated content
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)?;

        file.write_all(new_content.as_bytes())?;

        Ok(())
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match self.mode {
            Mode::Collecting => self.handle_collecting_mode(key).await,
            Mode::Command => self.handle_command_mode(key).await,
            Mode::Help => self.handle_help_mode(key).await,
            Mode::Searching => self.handle_search_mode(key).await,
        }
    }

    async fn handle_collecting_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            // Command mode: :q 退出, :h 帮助
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                return Ok(true);
            }

            // Search with Ctrl+S or Cmd+S (macOS)
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL
                || key.modifiers == KeyModifiers::SUPER => {
                self.start_search();
                return Ok(true);
            }

            // Undo with Ctrl+Z / Cmd+Z
            KeyCode::Char('z') if key.modifiers == KeyModifiers::CONTROL
                || key.modifiers == KeyModifiers::SUPER => {
                self.undo();
            }

            // Redo with Ctrl+Shift+Z / Cmd+Shift+Z
            KeyCode::Char('Z') if key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT)
                || key.modifiers == (KeyModifiers::SUPER | KeyModifiers::SHIFT) => {
                self.redo();
            }

            // Enter handling: submit or newline
            KeyCode::Enter => {
                if key.modifiers == KeyModifiers::SHIFT {
                    // Shift+Enter: insert newline (折行)
                    self.save_undo_state();
                    self.insert_newline();
                } else if key.modifiers == KeyModifiers::CONTROL {
                    // Ctrl+Enter: submit
                    self.submit_entry()?;
                } else if key.modifiers == KeyModifiers::SUPER {
                    // Cmd+Enter: submit (macOS)
                    self.submit_entry()?;
                } else {
                    // Plain Enter: submit
                    self.submit_entry()?;
                }
            }

            // Ctrl+J: insert newline (same as Shift+Enter, 折行)
            KeyCode::Char('j') if key.modifiers == KeyModifiers::CONTROL => {
                self.save_undo_state();
                self.insert_newline();
            }

            // Emacs navigation
            KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                self.move_cursor_down();
            }
            KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                self.move_cursor_up();
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                self.move_cursor_right();
            }
            KeyCode::Char('b') if key.modifiers == KeyModifiers::CONTROL => {
                self.move_cursor_left();
            }
            KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
                self.input.cursor_col = 0;
            }
            KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
                self.input.cursor_col = self.input.current_line().chars().count();
            }
            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                self.save_undo_state();
                self.delete_forward();
            }
            KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
                self.save_undo_state();
                self.kill_line();
            }

            // Backspace
            KeyCode::Backspace => {
                if !self.input.is_empty() {
                    self.save_undo_state();
                }
                self.handle_backspace();
            }

            // Delete
            KeyCode::Delete => {
                self.save_undo_state();
                self.delete_forward();
            }

            // Character input
            KeyCode::Char(c) => {
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                    self.save_undo_state();
                    self.insert_char(c);
                }
            }

            // Arrow keys
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),

            _ => {}
        }

        self.input.last_activity = Instant::now();

        let text = self.input.to_string();
        self.on_input_changed(&text);

        Ok(true)
    }

    async fn handle_command_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                return Ok(false);
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.mode = Mode::Help;
            }
            _ => {
                self.mode = Mode::Collecting;
            }
        }
        Ok(true)
    }

    async fn handle_help_mode(&mut self, key: KeyEvent) -> Result<bool> {
        // 按任意键退出帮助模式
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                self.mode = Mode::Collecting;
            }
            _ => {
                self.mode = Mode::Collecting;
            }
        }
        Ok(true)
    }

    async fn handle_search_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.cancel_search();
            }
            KeyCode::Enter => {
                // Jump to selected result or exit search
                self.cancel_search();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_search();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search();
            }
            KeyCode::Tab | KeyCode::Down => {
                self.next_search_result();
            }
            KeyCode::Up => {
                self.prev_search_result();
            }
            _ => {}
        }
        Ok(true)
    }

    // Input handling methods
    fn insert_char(&mut self, c: char) {
        let col = self.input.cursor_col;
        let line = self.input.current_line_mut();
        let char_count = line.chars().count();

        if col >= char_count {
            line.push(c);
        } else {
            let byte_pos = line.char_indices().nth(col).map(|(i, _)| i).unwrap_or(line.len());
            line.insert(byte_pos, c);
        }
        self.input.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        let col = self.input.cursor_col;
        let line = self.input.current_line().clone();
        let after_cursor: String = line.chars().skip(col).collect();
        let before_cursor: String = line.chars().take(col).collect();

        *self.input.current_line_mut() = before_cursor;
        self.input.lines.insert(self.input.cursor_line + 1, after_cursor);

        self.input.cursor_line += 1;
        self.input.cursor_col = 0;
    }

    fn handle_backspace(&mut self) {
        if self.input.cursor_col > 0 {
            let col = self.input.cursor_col;
            let line = self.input.current_line_mut();
            let byte_pos = line.char_indices().nth(col - 1).map(|(i, _)| i);

            if let Some(pos) = byte_pos {
                if line.char_indices().nth(col - 1).is_some() {
                    line.remove(pos);
                }
            }
            self.input.cursor_col -= 1;
        } else if self.input.cursor_line > 0 {
            let current = self.input.lines.remove(self.input.cursor_line);
            self.input.cursor_line -= 1;
            self.input.cursor_col = self.input.current_line().chars().count();
            self.input.current_line_mut().push_str(&current);
        }
    }

    fn delete_forward(&mut self) {
        let col = self.input.cursor_col;
        let line = self.input.current_line();
        let char_count = line.chars().count();

        if col < char_count {
            let line = self.input.current_line_mut();
            let byte_pos = line.char_indices().nth(col).map(|(i, _)| i).unwrap_or(line.len());
            line.remove(byte_pos);
        } else if self.input.cursor_line < self.input.lines.len() - 1 {
            let next = self.input.lines.remove(self.input.cursor_line + 1);
            self.input.current_line_mut().push_str(&next);
        }
    }

    fn kill_line(&mut self) {
        let col = self.input.cursor_col;
        let line = self.input.current_line();
        let char_count = line.chars().count();

        if col < char_count {
            let line = self.input.current_line_mut();
            let before_cursor: String = line.chars().take(col).collect();
            *line = before_cursor;
        } else if self.input.cursor_line < self.input.lines.len() - 1 {
            let next = self.input.lines.remove(self.input.cursor_line + 1);
            self.input.current_line_mut().push_str(&next);
        }
    }

    fn move_cursor_up(&mut self) {
        if self.input.cursor_line > 0 {
            self.input.cursor_line -= 1;
            let prev_len = self.input.current_line().chars().count();
            self.input.cursor_col = self.input.cursor_col.min(prev_len);
        }
    }

    fn move_cursor_down(&mut self) {
        if self.input.cursor_line < self.input.lines.len() - 1 {
            self.input.cursor_line += 1;
            let next_len = self.input.current_line().chars().count();
            self.input.cursor_col = self.input.cursor_col.min(next_len);
        }
    }

    fn move_cursor_left(&mut self) {
        if self.input.cursor_col > 0 {
            self.input.cursor_col -= 1;
        } else if self.input.cursor_line > 0 {
            self.input.cursor_line -= 1;
            self.input.cursor_col = self.input.current_line().chars().count();
        }
    }

    fn move_cursor_right(&mut self) {
        let line_len = self.input.current_line().chars().count();
        if self.input.cursor_col < line_len {
            self.input.cursor_col += 1;
        } else if self.input.cursor_line < self.input.lines.len() - 1 {
            self.input.cursor_line += 1;
            self.input.cursor_col = 0;
        }
    }
}
