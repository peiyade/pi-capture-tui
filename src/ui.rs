use crate::app::{App, Mode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

const MIN_WIDTH: u16 = 15;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Enforce minimum width
    let width = area.width.max(MIN_WIDTH);
    let height = area.height;
    let area = Rect::new(area.x, area.y, width, height);

    // Calculate layout
    let chunks = Layout::vertical([
        Constraint::Min(3),                   // History
        Constraint::Length(4),                // Secretary
        Constraint::Length(6),                // Input
        Constraint::Length(1),                // Status
    ])
    .split(area);

    render_history(frame, app, chunks[0]);
    render_secretary(frame, app, chunks[1]);
    render_input(frame, app, chunks[2]);
    render_status(frame, app, chunks[3]);

    // Render overlays based on mode
    match app.mode {
        Mode::Command => render_command_overlay(frame, area),
        Mode::Help => render_help_overlay(frame, area),
        Mode::Searching => render_search_overlay(frame, app, area),
        _ => {}
    }
}

/// Get display width (CJK chars = 2, others = 1)
pub fn display_width(s: &str) -> usize {
    s.chars().map(|c| if c.len_utf8() > 1 { 2 } else { 1 }).sum()
}

/// Wrap text to fit within max_width (display width)
fn wrap_text_to_width(s: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || s.is_empty() {
        return vec![s.to_string()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for c in s.chars() {
        let char_width = if c.len_utf8() > 1 { 2 } else { 1 };

        if current_width + char_width > max_width && !current_line.is_empty() {
            // Current line is full, push it and start new line
            lines.push(current_line.clone());
            current_line.clear();
            current_width = 0;
        }

        current_line.push(c);
        current_width += char_width;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Highlight search matches in text
fn highlight_search_matches(text: &str, query: &str) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::raw(text.to_string())];
    }

    let mut spans = Vec::new();
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut last_end = 0;

    for (start, _) in lower_text.match_indices(&lower_query) {
        // Add text before match
        if start > last_end {
            spans.push(Span::raw(text[last_end..start].to_string()));
        }

        // Add highlighted match
        let match_end = start + query.len();
        spans.push(Span::styled(
            text[start..match_end].to_string(),
            Style::default().bg(Color::Yellow).fg(Color::Black),
        ));

        last_end = match_end;
    }

    // Add remaining text
    if last_end < text.len() {
        spans.push(Span::raw(text[last_end..].to_string()));
    }

    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }

    spans
}

fn render_history(frame: &mut Frame, app: &App, area: Rect) {
    let is_searching = app.mode == Mode::Searching;

    let title = if is_searching && !app.search_query.is_empty() {
        format!(" 历史 (搜索: {} - {}/{})",
            app.search_query,
            if app.search_results.is_empty() { 0 } else { app.current_search_idx + 1 },
            app.search_results.len()
        )
    } else {
        " 历史 ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if is_searching {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let inner = block.inner(area);
    // Available width for content (accounting for borders and padding)
    let available_width = (inner.width as usize).saturating_sub(2);
    // Width for first line (with timestamp "[HH:MM] ")
    let first_line_width = available_width.saturating_sub(7); // "[HH:MM] " = 7 chars
    // Width for continuation lines (indent to align with content)
    let cont_line_width = available_width.saturating_sub(6);  // 6 spaces indent

    // Build list items from history with automatic line wrapping
    let mut items: Vec<ListItem> = Vec::new();

    for (idx, entry) in app.history.iter().enumerate() {
        // Extract HH:MM from ISO8601 timestamp
        let display_time = if entry.timestamp.len() >= 16 {
            entry.timestamp[11..16].to_string()
        } else {
            entry.timestamp.clone()
        };
        let timestamp = format!("[{}] ", display_time);

        // Check if this entry is in search results
        let is_match = app.search_results.iter().any(|(i, _)| *i == idx);

        // Split content by newlines first
        let content_lines: Vec<&str> = entry.content.lines().collect();
        let mut all_lines: Vec<Line> = Vec::new();

        for (line_idx, content_line) in content_lines.iter().enumerate() {
            // Wrap this line to fit available width
            let max_width = if line_idx == 0 && all_lines.is_empty() {
                first_line_width
            } else {
                cont_line_width
            };

            let wrapped = wrap_text_to_width(content_line, max_width);

            for (wrap_idx, wrapped_part) in wrapped.iter().enumerate() {
                let is_first = line_idx == 0 && wrap_idx == 0 && all_lines.is_empty();

                let content_spans = if is_searching && is_match {
                    highlight_search_matches(wrapped_part, &app.search_query)
                } else {
                    vec![Span::raw(wrapped_part.clone())]
                };

                if is_first {
                    // First line has timestamp
                    let mut spans = vec![Span::styled(timestamp.clone(), Style::default().fg(Color::DarkGray))];
                    spans.extend(content_spans);
                    all_lines.push(Line::from(spans));
                } else {
                    // Continuation lines have indentation
                    let mut spans = vec![Span::styled("      ".to_string(), Style::default().fg(Color::DarkGray))];
                    spans.extend(content_spans);
                    all_lines.push(Line::from(spans));
                }
            }
        }

        items.push(ListItem::new(Text::from(all_lines)));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

fn render_secretary(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.ai_pending {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let desk_name = &app.config.ai.desk_name;
    let block = Block::default()
        .title(format!(" {} ", desk_name))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    for line_text in app.secretary_content.lines() {
        if lines.len() >= inner.height as usize {
            break;
        }
        lines.push(Line::from(line_text.to_string()));
    }

    if app.ai_pending && lines.len() < inner.height as usize {
        lines.push(Line::styled("⏳ 思考中...", Style::default().fg(Color::Yellow)));
    }

    // Use Paragraph with built-in wrap
    let paragraph = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: true });

    frame.render_widget(block, area);
    frame.render_widget(paragraph, inner);

    // Render secretary name at bottom right
    let secretary_name = &app.config.ai.secretary_name;
    let suffix = format!("——{}", secretary_name);
    let suffix_width = display_width(&suffix);
    let inner_width = inner.width as usize;

    if suffix_width <= inner_width {
        let x = inner.x + (inner_width - suffix_width) as u16;
        let y = inner.y + inner.height.saturating_sub(1);
        let suffix_span = Span::styled(suffix, Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new(Line::from(vec![suffix_span])),
            Rect::new(x, y, suffix_width as u16, 1),
        );
    }
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" 输入 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);

    // Build display text with prefix
    let prefix = "> ";
    let content = app.input.to_string();

    // Create lines with prefix
    let mut lines: Vec<Line> = Vec::new();

    if content.is_empty() {
        lines.push(Line::from(prefix));
    } else {
        for (i, line) in content.lines().enumerate() {
            let line_prefix = if i == 0 { prefix } else { "  " };
            lines.push(Line::from(format!("{}{}", line_prefix, line)));
        }
    }

    // Use Paragraph with wrap for automatic line breaking
    let paragraph = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false });

    frame.render_widget(block, area);
    frame.render_widget(paragraph, inner);

    // Calculate cursor position
    let cursor_line = app.input.cursor_line;
    let cursor_col = app.input.cursor_col;

    // Count how many wrapped lines before cursor
    let mut visual_line = 0;
    for (i, line) in app.input.lines.iter().enumerate() {
        if i == cursor_line {
            // Calculate wrapped lines within this logical line
            let text_before: String = line.chars().take(cursor_col).collect();
            let wrapped_width = (inner.width as usize).saturating_sub(2); // -2 for "> "

            if wrapped_width > 0 {
                visual_line += display_width(&text_before) / wrapped_width;
            }
            break;
        } else {
            // Full line contributes to visual lines
            let wrapped_width = (inner.width as usize).saturating_sub(2);
            if wrapped_width > 0 {
                visual_line += (display_width(line) + wrapped_width - 1) / wrapped_width;
            }
        }
    }

    // Calculate cursor column within wrapped line
    let current_line_text = &app.input.lines[cursor_line];
    let text_before_cursor: String = current_line_text.chars().take(cursor_col).collect();
    let wrapped_width = (inner.width as usize).saturating_sub(2);
    let cursor_offset = if wrapped_width > 0 {
        display_width(&text_before_cursor) % wrapped_width
    } else {
        0
    };

    let cursor_x = inner.x + 2 + cursor_offset as u16;
    let cursor_y = inner.y + visual_line as u16;

    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let status_text = match app.mode {
        Mode::Collecting => {
            let connectivity = if app.config.ai.provider == "mock" {
                "离线"
            } else if app.ai_pending {
                "思考中"
            } else {
                "在线"
            };
            format!(
                " :q 退出 | :h 帮助 | {} | {} ",
                app.config.ai.model,
                connectivity
            )
        }
        Mode::Command => {
            " :q 退出 | :h 帮助 | 其他键取消 ".to_string()
        }
        Mode::Help => {
            " 帮助 | 按任意键返回 ".to_string()
        }
        Mode::Searching => {
            format!(
                " 搜索: {} | Tab/↑↓ 切换结果 | Enter 确认 | Esc 取消 ",
                app.search_query
            )
        }
    };

    let style = match app.mode {
        Mode::Collecting => Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        Mode::Command => Style::default()
            .bg(Color::Red)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        Mode::Help => Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
        Mode::Searching => Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    };

    let status = Paragraph::new(status_text)
        .style(style)
        .alignment(Alignment::Left);

    frame.render_widget(status, area);
}

fn render_command_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(40, 20, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 命令 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let text = Text::from(vec![
        Line::from(""),
        Line::from("可用命令："),
        Line::from(""),
        Line::styled("  q   - 退出程序", Style::default().fg(Color::Red)),
        Line::styled("  h   - 显示帮助", Style::default().fg(Color::Cyan)),
        Line::from(""),
        Line::styled("按其他键取消", Style::default().fg(Color::Gray)),
    ]);

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);

    frame.render_widget(block.clone(), popup_area);
    frame.render_widget(paragraph, block.inner(popup_area));
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 70, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 帮助 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let text = Text::from(vec![
        Line::from(""),
        Line::from("【基本操作】"),
        Line::from("  Enter        - 提交想法"),
        Line::from("  Shift+Enter  - 插入新行"),
        Line::from("  Ctrl+J       - 插入新行"),
        Line::from(""),
        Line::from("【编辑】"),
        Line::from("  ⌘Z           - 撤销"),
        Line::from("  ⌘Shift+Z     - 重做"),
        Line::from(""),
        Line::from("【导航】"),
        Line::from("  Ctrl+N/P     - 上/下行"),
        Line::from("  Ctrl+F/B     - 右/左移"),
        Line::from("  Ctrl+A/E     - 行首/行尾"),
        Line::from("  Ctrl+D       - 删除字符"),
        Line::from("  Ctrl+K       - 删除到行尾"),
        Line::from(""),
        Line::from("【其他】"),
        Line::from("  ⌘S           - 搜索历史"),
        Line::from("  :q           - 退出"),
        Line::from("  :h           - 帮助"),
        Line::from(""),
        Line::styled("按任意键返回", Style::default().fg(Color::Gray)),
    ]);

    let paragraph = Paragraph::new(text).alignment(Alignment::Left);

    frame.render_widget(block.clone(), popup_area);
    frame.render_widget(paragraph, block.inner(popup_area));
}

fn render_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    // Show search info at the top
    let search_area = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        3,
    );

    let block = Block::default()
        .title(" 搜索模式 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(search_area);

    let mut lines = vec![
        Line::from(vec![
            Span::raw("查询: "),
            Span::styled(&app.search_query, Style::default().fg(Color::Yellow)),
        ]),
    ];

    if !app.search_results.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(format!("结果: {}/{}", app.current_search_idx + 1, app.search_results.len())),
        ]));
    } else if !app.search_query.is_empty() {
        lines.push(Line::styled("无匹配结果", Style::default().fg(Color::Gray)));
    }

    let paragraph = Paragraph::new(Text::from(lines));

    frame.render_widget(Clear, search_area);
    frame.render_widget(block, search_area);
    frame.render_widget(paragraph, inner);
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
