use crate::checker::checker;
use crate::parser::parse;
use crate::parser::ASTNode;
use crate::transpilier::Transpiler;
use crate::CodeError;
use crate::Editor;
use log::error;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::prelude::Position;
use std::backtrace::Backtrace;
use std::panic;
use std::path::Path;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::time::Duration;

use std::process::Command;

use crate::lexer;

use ratatui::prelude::Alignment;

use ratatui::prelude::Rect;
use ratatui::widgets::Clear;
use std::fs;
use std::io;
use std::io::Write;

use std::sync::MutexGuard;
use std::thread;

use crate::colorizer::colorize_code;

use ratatui::text::Span;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs},
    Frame, Terminal,
};

#[derive(Debug, PartialEq)]
pub enum EditorMessage {
    Shutdown,
    BuildStart,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuildStatus {
    Idle,
    Parsing,
    Transpiling,
    Compiling,
    Complete,
    Failed(String),
}

pub fn lock<T>(arc_mutex: &Arc<Mutex<T>>) -> MutexGuard<T> {
    match arc_mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("Mutex was poisoned, recovering");
            poisoned.into_inner()
        }
    }
}

pub fn resize_thread_logic(terminal_arc: Arc<Mutex<Terminal<CrosstermBackend<io::Stdout>>>>, rx: Receiver<EditorMessage>) {
    loop {
        match rx.try_recv() {
            Ok(EditorMessage::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Shutting down resize thread");
                break;
            }
            Ok(message) => {
                log::info!("{}", format!("Resize thread saw and ignored message: {:?}", message));
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        thread::sleep(Duration::from_millis(100));
        let mut terminal: MutexGuard<'_, Terminal<CrosstermBackend<_>>> = lock(&terminal_arc);
        let result_resize = terminal.autoresize();
        if let Err(err) = result_resize {
            log::error!("Error resizing terminal: {:?}", err);
        }
    }
}

pub fn draw_thread_logic(terminal_arc: Arc<Mutex<Terminal<CrosstermBackend<io::Stdout>>>>, editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>) {
    loop {
        match rx.try_recv() {
            Ok(EditorMessage::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Shutting down draw thread");
                let _ = lock(&terminal_arc).clear();
                let _ = lock(&terminal_arc).show_cursor();
                io::stdout().flush().expect("Failed to flush stdout");
                break;
            }
            Ok(message) => {
                log::info!("Draw thread saw and ignored message: {:?}", message);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }

        thread::sleep(Duration::from_millis(50)); // 20 FPS - balance between smooth UI and mouse selection
        let mut locked_terminal = lock(&terminal_arc);

        // Check if terminal size is valid before drawing
        let size = match locked_terminal.size() {
            Ok(size) => size,
            Err(e) => {
                log::error!("Failed to get terminal size: {}", e);
                continue;
            }
        };

        if size.width == 0 || size.height == 0 {
            log::warn!("Terminal size is too small: {}x{}", size.width, size.height);
            continue;
        }

        let result_draw = locked_terminal.draw(|f| {
            let editor = lock(&editor_arc);

            // Log frame area details
            log::info!("Drawing frame - area: {:?}", f.area());

            // Check if frame area is valid
            if f.area().width == 0 || f.area().height == 0 {
                log::warn!("Frame area is too small: {}x{}", f.area().width, f.area().height);
                return;
            }

            if f.area().height < 5 {
                log::warn!("Frame area height too small for layout: {}", f.area().height);
                return;
            }

            let chunks = Layout::default().direction(Direction::Vertical).margin(0).constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)].as_ref()).split(f.area());
            log::info!("Layout chunks: {:?}", chunks);

            // Render tabs
            let titles = vec!["Tab1", "Tab2", "Tab3", "Tab4"];
            let file_title = if editor.modified { format!("FILES [*] - Press Ctrl+S to save") } else { "FILES".to_string() };
            let tabs = Tabs::new(titles)
                .block(Block::default().borders(Borders::ALL).title(file_title))
                .select(editor.tab_index)
                .style(Style::default().fg(editor.theme.default))
                .highlight_style(Style::default().fg(editor.theme.operator));
            f.render_widget(tabs, chunks[0]);

            // Create a horizontal layout for the main content area
            let content_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Min(0), Constraint::Length(1)].as_ref()).split(chunks[1]);

            // Render main content

            let visible_lines = if chunks[1].height > 2 { chunks[1].height as usize - 2 } else { 0 };

            // First colorize the entire content to properly handle multi-line constructs
            let all_content_lines: Vec<Line> =
                editor.content.iter().map(|line| Line::from(vec![Span::styled(line.clone(), Style::default().fg(editor.theme.default).bg(editor.theme.background))])).collect();
            let all_colorized = colorize_code(all_content_lines, &editor.theme);

            // Then extract the visible portion and apply cursor highlighting
            let mut visible_content: Vec<Line> = all_colorized.into_iter().skip(editor.scroll_position as usize).take(visible_lines).collect();
            
            // Apply special styling to the cursor position to ensure it appears white
            let cursor_y_visible = editor.cursor_y.saturating_sub(editor.scroll_position as usize);
            if cursor_y_visible < visible_content.len() {
                if let Some(line) = visible_content.get_mut(cursor_y_visible) {
                    let mut new_spans = Vec::new();
                    let mut char_pos = 0;
                    
                    for span in line.spans.iter() {
                        let text = span.content.to_string();
                        let span_style = span.style;
                        
                        for ch in text.chars() {
                            if char_pos == editor.cursor_x {
                                // Simply ensure the cursor position has white text
                                // Most terminals will then properly show their cursor on top
                                new_spans.push(Span::styled(
                                    ch.to_string(), 
                                    span_style.fg(Color::White)
                                ));
                            } else {
                                new_spans.push(Span::styled(ch.to_string(), span_style));
                            }
                            char_pos += 1;
                        }
                    }
                    
                    // Handle case where cursor is at the end of the line
                    if char_pos == editor.cursor_x {
                        new_spans.push(Span::styled(" ", Style::default().fg(Color::White)));
                    }
                    
                    *line = Line::from(new_spans);
                }
            }

            let editor_title = if let Some(ref filename) = editor.current_file { format!("NAIL - {}", filename) } else { "NAIL".to_string() };
            let paragraph =
                Paragraph::new(visible_content).block(Block::default().borders(Borders::ALL).title(editor_title)).style(Style::default().bg(editor.theme.background).fg(editor.theme.default));

            f.render_widget(paragraph, content_layout[0]);

            let scrollbar = Scrollbar::default()
                .style(Style::default().fg(editor.theme.default))
                .orientation(ScrollbarOrientation::VerticalRight)
                .symbols(ratatui::symbols::scrollbar::VERTICAL)
                .begin_symbol(None)
                .end_symbol(None);

            let mut scrollbar_state = ScrollbarState::default().content_length(editor.content.len()).position(editor.scroll_position as usize);

            f.render_stateful_widget(scrollbar, content_layout[1], &mut scrollbar_state);

            // Set cursor
            let cursor_y = editor.cursor_y.saturating_sub(editor.scroll_position.into());
            log::info!("Cursor position calculation - editor.cursor_y: {}, editor.cursor_x: {}, scroll: {}, cursor_y: {}", editor.cursor_y, editor.cursor_x, editor.scroll_position, cursor_y);

            if cursor_y < content_layout[0].height.saturating_sub(2) as usize {
                let cursor_pos = Position { x: content_layout[0].x + editor.cursor_x as u16 + 1, y: content_layout[0].y + cursor_y as u16 + 1 };
                log::info!("Setting cursor position: {:?}, content_layout[0]: {:?}", cursor_pos, content_layout[0]);
                f.set_cursor_position(cursor_pos);
            }

            // Always display status
            display_build_status(f, &editor);

            // Check and draw errors FIRST
            if let Some(error) = &editor.code_error {
                display_error(f, error, &editor, content_layout[0]);
            }
            
            // Draw completion popup LAST so it appears on top
            if editor.show_completions && !editor.completions.is_empty() {
                if editor.show_detail_view {
                    display_completion_detail(f, &editor, content_layout[0]);
                } else {
                    display_completions(f, &editor, content_layout[0]);
                }
            }
        });

        match result_draw {
            Ok(_) => {}
            Err(err) => log::error!("Error drawing terminal: {:?}", err),
        }
    }
}

fn display_build_status(f: &mut Frame, editor: &Editor) {
    let status_text = match &editor.build_status {
        BuildStatus::Idle => "Ready",
        BuildStatus::Parsing => "Starting",
        BuildStatus::Transpiling => "Transpiling",
        BuildStatus::Compiling => "Compiling",
        BuildStatus::Complete => "Saved!",
        BuildStatus::Failed(err) => err,
    };

    let build_status = Line::from(vec![Span::styled(status_text, Style::default().fg(editor.theme.default))]);

    let build_status_width = build_status.width() as u16;

    let paragraph = Paragraph::new(build_status).style(Style::default().fg(editor.theme.default).bg(editor.theme.background)).alignment(Alignment::Right);

    let status_width = build_status_width;
    let status_height = 1;
    let status_area = Rect::new(f.area().width.saturating_sub(status_width), 0, status_width, status_height);

    log::info!("Build status area: {:?}, frame area: {:?}", status_area, f.area());

    // Check if status area is within frame bounds
    if status_area.x + status_area.width > f.area().width || status_area.y + status_area.height > f.area().height {
        log::warn!("Build status area exceeds frame bounds, skipping render");
        return;
    }

    f.render_widget(Clear, status_area);
    f.render_widget(paragraph, status_area);
}

fn display_error(f: &mut Frame, error: &CodeError, editor: &Editor, content_area: Rect) {
    let error_line = error.code_span.start_line.saturating_sub(editor.scroll_position as usize);
    let error_column = error.code_span.start_column;
    let error_message = error.message.clone();

    log::info!("Displaying error - line: {}, column: {}, message: {}", error.code_span.start_line, error_column, error_message);

    // Only display the error if it's within the visible area

    let error_message = Line::from(vec![Span::styled(error_message, Style::default().fg(editor.theme.error).bg(editor.theme.background))]);

    let paragraph = Paragraph::new(error_message.clone()).style(Style::default().fg(editor.theme.error).bg(editor.theme.background)).alignment(Alignment::Left);

    let error_area = Rect::new(
        content_area.x + error_column as u16,
        content_area.y + error_line as u16 + 1, // +1 for the border
        error_message.width() as u16,
        1,
    );

    log::info!("Error area: {:?}, content_area: {:?}, frame area: {:?}", error_area, content_area, f.area());

    // Check if error area is within frame bounds
    if error_area.x + error_area.width > f.area().width || error_area.y + error_area.height > f.area().height {
        log::warn!("Error area exceeds frame bounds, skipping render");
        return;
    }

    f.render_widget(Clear, error_area);
    f.render_widget(paragraph, error_area);
}

fn display_completion_detail(f: &mut Frame, editor: &Editor, content_area: Rect) {
    use crate::CompletionKind;
    use ratatui::widgets::Wrap;
    
    // Get the selected completion
    if editor.completion_index >= editor.completions.len() {
        return;
    }
    
    let selected = &editor.completions[editor.completion_index];
    
    // Build the detailed content
    let mut lines = vec![];
    
    // Title with function signature
    lines.push(Line::from(vec![
        Span::styled("Function: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(&selected.label, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));
    
    lines.push(Line::from(""));
    
    // Signature
    lines.push(Line::from(vec![
        Span::styled("Signature: ", Style::default().fg(Color::Cyan)),
        Span::styled(&selected.detail, Style::default().fg(Color::White)),
    ]));
    
    lines.push(Line::from(""));
    
    // Description
    if !selected.description.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Description:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(&selected.description, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }
    
    // Example
    if !selected.example.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Example:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(&selected.example, Style::default().fg(Color::Gray)),
        ]));
        lines.push(Line::from(""));
    }
    
    // Help text
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("ESC", Style::default().fg(Color::Yellow)),
        Span::styled(" to go back, ", Style::default().fg(Color::DarkGray)),
        Span::styled("TAB", Style::default().fg(Color::Yellow)),
        Span::styled(" to insert", Style::default().fg(Color::DarkGray)),
    ]));
    
    // Calculate popup size
    let width = lines.iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(40)
        .max(50)
        .min(100) as u16;
    
    let height = (lines.len() + 2).min(20) as u16; // +2 for borders
    
    // Center the popup
    let popup_x = content_area.x + (content_area.width.saturating_sub(width)) / 2;
    let popup_y = content_area.y + (content_area.height.saturating_sub(height)) / 2;
    
    let popup_area = Rect::new(
        popup_x,
        popup_y,
        width,
        height,
    );
    
    // Clear the area and draw the detailed view
    f.render_widget(Clear, popup_area);
    
    let detail_paragraph = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Documentation (F1 to toggle) ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(editor.theme.background))
        .wrap(Wrap { trim: true });
    
    f.render_widget(detail_paragraph, popup_area);
}

fn display_completions(f: &mut Frame, editor: &Editor, content_area: Rect) {
    use ratatui::widgets::{List, ListItem};
    use crate::CompletionKind;
    
    // Calculate popup position (below current cursor line)
    let cursor_y = editor.cursor_y.saturating_sub(editor.scroll_position as usize);
    // Place popup BELOW the current line: +1 for border, +1 to go to next line, +1 for spacing
    let popup_y = content_area.y + cursor_y as u16 + 3;
    
    // Position popup to the right of the current word being typed
    let word_start = {
        let mut start = editor.cursor_x;
        if editor.cursor_y < editor.content.len() {
            let line = &editor.content[editor.cursor_y];
            while start > 0 && line.chars().nth(start - 1).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                start -= 1;
            }
        }
        start
    };
    let popup_x = content_area.x + word_start as u16 + 1;
    
    // Limit completions shown
    let max_items = 10;
    let items_to_show = editor.completions.len().min(max_items);
    
    // Create list items with highlighting for selected item
    let items: Vec<ListItem> = editor.completions
        .iter()
        .take(items_to_show)
        .enumerate()
        .map(|(i, item)| {
            let icon = match item.kind {
                CompletionKind::Function => "ƒ ",
                CompletionKind::Variable => "v ",
                CompletionKind::Keyword => "k ",
            };
            
            let content = if i == editor.completion_index {
                Line::from(vec![
                    Span::styled(icon, Style::default().fg(Color::Yellow)),
                    Span::styled(&item.label, Style::default().fg(Color::White).bg(Color::Blue)),
                    Span::raw(" "),
                    Span::styled(&item.detail, Style::default().fg(Color::Gray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(icon, Style::default().fg(Color::DarkGray)),
                    Span::styled(&item.label, Style::default().fg(editor.theme.default)),
                    Span::raw(" "),
                    Span::styled(&item.detail, Style::default().fg(Color::DarkGray)),
                ])
            };
            ListItem::new(content)
        })
        .collect();
    
    // Calculate popup width based on longest item - make it wider to show full signatures
    let max_width = editor.completions
        .iter()
        .take(items_to_show)
        .map(|item| {
            // Consider label, detail, and description for width
            let label_detail_len = item.label.len() + item.detail.len() + 5;
            let desc_len = item.description.len();
            label_detail_len.max(desc_len)
        })
        .max()
        .unwrap_or(40)
        .min(100) as u16; // Increased max width from 60 to 100
    
    let popup_area = Rect::new(
        popup_x.min(f.area().width.saturating_sub(max_width + 2)),
        popup_y.min(f.area().height.saturating_sub(items_to_show as u16 + 2)),
        max_width + 2,
        items_to_show as u16 + 2,
    );
    
    // Clear the area first
    f.render_widget(Clear, popup_area);
    
    // Check if selected item has documentation
    let has_docs = if editor.completion_index < editor.completions.len() {
        let selected = &editor.completions[editor.completion_index];
        !selected.description.is_empty() || !selected.example.is_empty()
    } else {
        false
    };
    
    // Create title with hint about F1 if docs are available
    let title = if has_docs {
        " Completions (F1 for docs) "
    } else {
        " Completions "
    };
    
    // Create and render the list
    let completions_list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(editor.theme.operator))
            .title(title)
            .title_style(if has_docs {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(editor.theme.operator)
            }))
        .style(Style::default().bg(editor.theme.background));
    
    f.render_widget(completions_list, popup_area);
}

pub fn key_thread_logic(editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>, tx: Sender<EditorMessage>, tx_build: Sender<EditorMessage>) {
    loop {
        // Check for messages
        match rx.try_recv() {
            Ok(EditorMessage::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Shutting down key thread");
                break;
            }
            _ => {}
        }

        // Check for key input
        match event::poll(Duration::from_millis(100)) {
            Ok(true) => {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        let mut editor = lock(&editor_arc);
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // SEND SHUTDOWN SIGNAL
                                let _ = tx.send(EditorMessage::Shutdown);
                                break;
                            }
                            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Save file with formatting
                                log::info!("Ctrl+S detected - saving file...");
                                editor.build_status = BuildStatus::Failed("Saving...".to_string());
                                drop(editor); // Release lock before save

                                let mut editor = lock(&editor_arc);
                                match editor.save_file() {
                                    Ok(_) => {
                                        editor.build_status = BuildStatus::Complete;
                                        log::info!("File saved successfully");
                                    }
                                    Err(e) => {
                                        editor.build_status = BuildStatus::Failed(format!("Save failed: {}", e));
                                        log::error!("Failed to save file: {}", e);
                                    }
                                }
                            }
                            KeyCode::F(5) => {
                                // Load example files
                                log::info!("F5 pressed - cycling through example files");

                                // List of example files to cycle through
                                // get all nail files from examples folder

                                let example_files = fs::read_dir("examples")
                                    .unwrap_or_else(|_| panic!("Failed to read examples directory"))
                                    .filter_map(Result::ok)
                                    .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "nail"))
                                    .map(|entry| entry.path().to_string_lossy().to_string())
                                    .collect::<Vec<String>>();

                                // Find current file index
                                let current_index = if let Some(ref current) = editor.current_file { example_files.iter().position(|f| f == current).unwrap_or(0) } else { 0 };

                                // Try to load files until we find one that exists
                                let mut attempts = 0;
                                let mut loaded = false;
                                while attempts < example_files.len() && !loaded {
                                    let next_index = (current_index + 1 + attempts) % example_files.len();
                                    let next_file = &example_files[next_index];

                                    match editor.load_file(next_file) {
                                        Ok(_) => {
                                            editor.build_status = BuildStatus::Idle;
                                            editor.code_error = Some(format!("Loaded: {}", next_file).into());
                                            log::info!("Successfully loaded file: {}", next_file);
                                            loaded = true;
                                        }
                                        Err(e) => {
                                            log::warn!("Failed to load file {}: {}", next_file, e);
                                            attempts += 1;
                                        }
                                    }
                                }

                                if !loaded {
                                    editor.build_status = BuildStatus::Failed("No example files found".to_string());
                                }
                            }
                            KeyCode::F(6) => editor.toggle_theme(),
                            KeyCode::F(7) => {
                                match editor.build_status {
                                    BuildStatus::Idle | BuildStatus::Failed(_) | BuildStatus::Complete => {
                                        let _ = tx_build.send(EditorMessage::BuildStart);
                                    }
                                    _ => {
                                        // Don't allow new builds while one is in progress
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                editor.insert_char(c);
                                editor.update_completions();
                            },
                            KeyCode::Up => {
                                if editor.show_completions {
                                    editor.previous_completion();
                                } else {
                                    editor.move_cursor_up();
                                }
                            },
                            KeyCode::Down => {
                                if editor.show_completions {
                                    editor.next_completion();
                                } else {
                                    editor.move_cursor_down();
                                }
                            },
                            KeyCode::PageDown => editor.scroll_down(),
                            KeyCode::PageUp => editor.scroll_up(),
                            KeyCode::Tab => {
                                if editor.show_completions {
                                    editor.accept_completion();
                                } else {
                                    // Trigger completion
                                    editor.update_completions();
                                    if !editor.show_completions {
                                        // If no completions, switch tabs as before
                                        editor.next_tab();
                                    }
                                }
                            },
                            KeyCode::BackTab => editor.previous_tab(),
                            KeyCode::Backspace => {
                                editor.delete_char();
                                editor.update_completions();
                            },
                            KeyCode::Enter => {
                                if editor.show_completions {
                                    editor.accept_completion();
                                } else {
                                    editor.insert_newline();
                                }
                            },
                            KeyCode::Esc => {
                                if editor.show_detail_view {
                                    // Go back to completion list from detail view
                                    editor.show_detail_view = false;
                                } else if editor.show_completions {
                                    // Close completions entirely
                                    editor.show_completions = false;
                                    editor.show_detail_view = false;  // Reset detail view too
                                    editor.completions.clear();
                                }
                            },
                            KeyCode::F(1) => {
                                // Toggle detail view for selected completion
                                if editor.show_completions && !editor.completions.is_empty() {
                                    editor.show_detail_view = !editor.show_detail_view;
                                }
                            },
                            KeyCode::Left => editor.move_cursor_left(),
                            KeyCode::Right => editor.move_cursor_right(),
                            _ => {}
                        }
                    }
                    Ok(_) => {
                        // Other events (mouse, resize, etc.) - ignore for now
                    }
                    Err(e) => {
                        log::error!("Error reading key event: {}", e);
                    }
                }
            }
            Ok(false) => {
                // No events available, continue
            }
            Err(e) => {
                log::error!("Error polling for events: {}", e);
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

pub fn build_thread_logic(editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>, _tx: Sender<EditorMessage>) {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::capture();
        error!("Panic occurred: {:?}", panic_info);
        error!("Backtrace:\n{:?}", backtrace);
    }));

    loop {
        let recv_result = match rx.try_recv() {
            Ok(EditorMessage::Shutdown) => {
                log::info!("Shutdown message. Shutting down build thread");
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Thread disconnected. Shutting down build thread.");
                break;
            }
            Ok(message) => message,
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(20));
                continue;
            }
        };

        if recv_result == EditorMessage::BuildStart {
            log::info!("Received build signal");

            // Step 1: Parse the content
            let mut editor = editor_arc.lock().unwrap();
            editor.build_status = BuildStatus::Parsing;
            let tokens = lexer::lexer(&editor.content.join("\n"));
            drop(editor);

            let mut ast = match parse(tokens) {
                Ok((ast, _used_functions)) => {
                    log::info!("AST (parsed): {:#?}", ast);
                    ast
                }
                Err(e) => {
                    let mut editor = editor_arc.lock().unwrap();
                    editor.build_status = BuildStatus::Failed(e.message.clone());
                    log::error!("Parsing failed: {:?}", e);
                    continue;
                }
            };

            let ast = match checker(&mut ast) {
                Ok(_) => {
                    log::info!("AST (type checked): {:#?}", ast);
                    ast
                }
                Err(errors) => {
                    let mut editor = editor_arc.lock().unwrap();
                    editor.build_status = BuildStatus::Failed(errors[0].message.clone());
                    log::error!("Checker failed: {:?}", errors);
                    continue;
                }
            };

            // Step 2: Transpile to Rust
            let mut editor = editor_arc.lock().unwrap();
            editor.build_status = BuildStatus::Transpiling;
            drop(editor); // Release the lock
            let mut transpiler = Transpiler::new();
            let rust_code = match transpiler.transpile(&ast) {
                Ok(code) => {
                    log::info!("Transpiled Rust pre-format code:\n{}", code);
                    code
                }
                Err(e) => {
                    let mut editor = editor_arc.lock().unwrap();
                    editor.build_status = BuildStatus::Failed(e.to_string());
                    log::error!("Transpilation failed: {}", e);
                    continue;
                }
            };

            // Step 3: Write Rust code to a temporary file
            let transpilation_dir = Path::new("./transpilation");
            let _ = fs::remove_dir_all(transpilation_dir);
            let _ = fs::create_dir(transpilation_dir);

            let transpilation_src_dir = transpilation_dir.join("src");
            if let Err(e) = fs::create_dir_all(&transpilation_src_dir) {
                let mut editor = editor_arc.lock().unwrap();
                editor.build_status = BuildStatus::Failed(format!("Failed to create src directory: {}", e));
                log::error!("Failed to create src directory: {}", e);
                continue;
            }

            let transpilation_toml = crate::utils::create_transpilation_cargo_toml();
            let transpilation_toml_path = transpilation_dir.join("Cargo.toml");
            if let Err(e) = fs::write(&transpilation_toml_path, &transpilation_toml) {
                let mut editor = editor_arc.lock().unwrap();
                editor.build_status = BuildStatus::Failed(format!("Failed to write Cargo.toml file: {}", e));
                log::error!("Failed to write Cargo.toml file: {}", e);
                continue;
            }

            let temp_file_path = transpilation_src_dir.join("main.rs");
            if let Err(e) = fs::write(&temp_file_path, &rust_code) {
                let mut editor = editor_arc.lock().unwrap();
                editor.build_status = BuildStatus::Failed(format!("Failed to write Rust code to file: {}", e));
                log::error!("Failed to write Rust code to file: {}", e);

                continue;
            }

            // Step 4: Compile the Rust code
            let mut editor = editor_arc.lock().unwrap();
            editor.build_status = BuildStatus::Compiling;
            drop(editor); // Release the lock
            let output = Command::new("cargo")
                .arg("build")
                .arg("--release")
                // run rustfmt or something
                .current_dir(transpilation_dir)
                .output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        log::debug!("Compiler stdout: {}", String::from_utf8_lossy(&output.stdout));

                        let binary_path = transpilation_dir.join("target/release/nail_transpilation");
                        let destination_path = Path::new("./build");
                        if let Err(e) = fs::rename(&binary_path, &destination_path) {
                            log::error!("Failed to move binary: {}", e);
                            let mut editor = editor_arc.lock().unwrap();
                            editor.build_status = BuildStatus::Failed(format!("Failed to move binary: {}", e));
                        } else {
                            let mut editor = editor_arc.lock().unwrap();
                            editor.build_status = BuildStatus::Complete;
                        }
                        if let Err(e) = fs::remove_dir_all(transpilation_dir) {
                            log::error!("Failed to remove transpilation directory: {}", e);
                        }
                    } else {
                        log::error!("Compiler stderr: {}", String::from_utf8_lossy(&output.stderr));
                        let mut editor = editor_arc.lock().unwrap();
                        editor.build_status = BuildStatus::Failed(format!("Compiler failed: {}", String::from_utf8_lossy(&output.stderr)));
                    }
                }
                Err(e) => {
                    log::error!("Failed to execute cargo: {}", e);
                    let mut editor = editor_arc.lock().unwrap();
                    editor.build_status = BuildStatus::Failed(format!("Failed to execute cargo: {}", e));
                    log::error!("Failed to execute cargo: {}", e);
                }
            }

            // sleep for 1000 ms to display complete message before reset
            thread::sleep(std::time::Duration::from_millis(1000));
            let mut editor = editor_arc.lock().unwrap();
            editor.build_status = BuildStatus::Idle;
        }

        thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub fn lex_and_parse_thread_logic(editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>) {
    loop {
        // Check for shutdown message
        match rx.try_recv() {
            Ok(EditorMessage::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Shutting down syntax error thread");
                break;
            }
            _ => {}
        }

        // Lock the editor to access its content
        let content = {
            let editor = lock(&editor_arc);
            editor.content.join("\n")
        };

        // Run the lexer on the content
        let tokens = lexer::lexer(&content);

        {
            let mut editor = lock(&editor_arc);
            editor.tokens = tokens.clone();
        }

        // Check for error tokens
        let mut lexing_error = None;
        for token in tokens.clone() {
            if let lexer::TokenType::LexerError(message) = token.token_type {
                lexing_error = Some(CodeError { message: format!("^ {}", message), code_span: token.code_span });
                break;
            }
        }

        {
            let mut editor = lock(&editor_arc);
            editor.code_error = lexing_error.clone();
        }

        if lexing_error.is_some() {
            log::info!("Lexer error detected: {:?}", lexing_error);
            // Sleep for a while to avoid excessive CPU usage, no need to parse if there are lexer errors
            thread::sleep(Duration::from_millis(250));
            continue;
        }

        // if the above is successful, get the parser errors and do the same thing

        let (mut ast, parse_succeeded) = match parse(tokens) {
            Ok((ast, _used_functions)) => (ast, true),
            Err(e) => {
                let mut editor = lock(&editor_arc);
                editor.code_error = Some(CodeError { message: format!("^ {}", e.message), code_span: e.code_span });
                (ASTNode::default(), false)
            }
        };

        // Only check types if parsing succeeded
        if parse_succeeded {
            let _ = match checker(&mut ast) {
                Ok(_) => {
                    // Clear any previous errors if everything is successful
                    let mut editor = lock(&editor_arc);
                    editor.code_error = None;
                }
                Err(errors) => {
                    let mut editor = lock(&editor_arc);
                    editor.code_error = Some(CodeError { message: format!("^ {}", errors[0].message), code_span: errors[0].code_span.clone() });
                }
            };
        }

        // Sleep to avoid excessive CPU usage
        thread::sleep(Duration::from_millis(250));
    }
}

pub fn create_transpilation_cargo_toml() -> String {
    r#"
    [package]
    name = "nail_transpilation"
    edition = "2021"

    [dependencies]
    tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
    nail = { path = ".." }

    # Binary target for the project
    [[bin]]
    name = "nail_transpilation"
    path = "src/main.rs"
    "#
    .to_string()
}

static WELCOME_MESSAGE: &str = include_str!("../examples/hello_world.nail");

pub fn create_welcome_message() -> Vec<String> {
    WELCOME_MESSAGE.lines().map(String::from).collect()
}
