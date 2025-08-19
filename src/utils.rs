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

// Timeout-based lock function to prevent deadlocks
pub fn try_lock_with_timeout<T>(arc_mutex: &Arc<Mutex<T>>, timeout_ms: u64) -> Option<MutexGuard<T>> {
    let start = std::time::Instant::now();
    let timeout_duration = std::time::Duration::from_millis(timeout_ms);
    
    loop {
        match arc_mutex.try_lock() {
            Ok(guard) => return Some(guard),
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                log::warn!("Mutex was poisoned during timeout lock, recovering");
                return Some(poisoned.into_inner());
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                if start.elapsed() > timeout_duration {
                    log::error!("Lock timeout after {}ms", timeout_ms);
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }
}

fn normalize_selection_positions(start: (usize, usize), end: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    // Return (start_pos, end_pos) where start_pos is before end_pos
    if start.1 < end.1 || (start.1 == end.1 && start.0 <= end.0) {
        (start, end)
    } else {
        (end, start)
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
    log::info!("Draw thread started");
    
    // Set up panic handler for this thread
    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("DRAW THREAD PANICKED: {:?}", panic_info);
        eprintln!("DRAW THREAD PANICKED: {:?}", panic_info);
    }));
    
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
        
        // Use timeout-based lock to prevent deadlocks
        let mut locked_terminal = match try_lock_with_timeout(&terminal_arc, 100) {
            Some(terminal) => terminal,
            None => {
                log::warn!("Draw thread: terminal lock timeout, skipping frame");
                continue;
            }
        };

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
            // Use timeout-based lock to prevent deadlocks during drawing
            let editor = match try_lock_with_timeout(&editor_arc, 50) {
                Some(editor) => editor,
                None => {
                    log::warn!("Draw thread: editor lock timeout, skipping frame");
                    return;
                }
            };

            // Only log frame area details in debug mode
            if editor.debug_mode {
                log::info!("Drawing frame - area: {:?}", f.area());
            }

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
            if editor.debug_mode {
                log::info!("Layout chunks: {:?}", chunks);
            }

            // Render tabs
            let tab_titles: Vec<String> = editor.tabs.iter().enumerate().map(|(i, tab)| {
                let mut title = if let Some(filename) = &tab.filename {
                    std::path::Path::new(filename)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                } else {
                    format!("Untitled {}", i + 1)
                };
                
                if tab.modified {
                    title.push('*');
                }
                
                title
            }).collect();
            
            let current_tab = editor.get_current_tab();
            let file_title = if current_tab.modified { 
                format!("FILES [*] - Press Ctrl+S to save") 
            } else { 
                "FILES".to_string() 
            };
            
            let tabs = Tabs::new(tab_titles)
                .block(Block::default().borders(Borders::ALL).title(file_title))
                .select(editor.tab_index)
                .style(Style::default().fg(editor.theme.default))
                .highlight_style(Style::default().fg(editor.theme.operator));
            f.render_widget(tabs, chunks[0]);

            // Create a horizontal layout for the main content area
            let current_tab = editor.get_current_tab();
            let gutter_width = if editor.show_line_numbers {
                calculate_line_number_width(current_tab.content.len())
            } else {
                0
            };
            let minimap_width = if editor.show_minimap { 15 } else { 0 };
            
            let content_layout = match (editor.show_line_numbers, editor.show_minimap) {
                (true, true) => {
                    Layout::default().direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(gutter_width),  // Line number gutter
                            Constraint::Min(0),                // Main content
                            Constraint::Length(minimap_width), // Minimap
                            Constraint::Length(1)              // Scrollbar
                        ].as_ref())
                        .split(chunks[1])
                },
                (true, false) => {
                    Layout::default().direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(gutter_width), // Line number gutter
                            Constraint::Min(0),               // Main content
                            Constraint::Length(1)             // Scrollbar
                        ].as_ref())
                        .split(chunks[1])
                },
                (false, true) => {
                    Layout::default().direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Min(0),                // Main content
                            Constraint::Length(minimap_width), // Minimap
                            Constraint::Length(1)              // Scrollbar
                        ].as_ref())
                        .split(chunks[1])
                },
                (false, false) => {
                    Layout::default().direction(Direction::Horizontal)
                        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
                        .split(chunks[1])
                }
            };

            // Render main content

            let visible_lines = if chunks[1].height > 2 { chunks[1].height as usize - 2 } else { 0 };

            // First colorize the entire content to properly handle multi-line constructs
            let current_tab = editor.get_current_tab();
            let all_content_lines: Vec<Line> =
                current_tab.content.iter().map(|line| Line::from(vec![Span::raw(line.clone())])).collect();
            let all_colorized = colorize_code(all_content_lines, &editor.theme);

            // Then extract the visible portion and apply cursor and selection highlighting
            let mut visible_content: Vec<Line> = all_colorized.into_iter().skip(current_tab.scroll_position as usize).take(visible_lines).collect();
            
            // Apply selection highlighting first, then cursor highlighting
            for (visible_line_idx, line) in visible_content.iter_mut().enumerate() {
                // Add bounds check to prevent potential issues
                if visible_line_idx >= 1000 {
                    log::warn!("Draw thread: visible line index too high ({}), breaking to prevent infinite loop", visible_line_idx);
                    break;
                }
                
                let actual_line_idx = visible_line_idx + current_tab.scroll_position as usize;
                let mut new_spans = Vec::new();
                let mut char_pos = 0;
                
                // Check if this is the current line for highlighting
                let is_current_line = actual_line_idx == current_tab.cursor_y && editor.highlight_current_line;
                
                // Check if this line has an error
                // Error lines are 1-based, actual_line_idx is 0-based
                let has_error_line = if let Some(error) = &editor.code_error {
                    actual_line_idx + 1 == error.code_span.start_line  // Convert 0-based index to 1-based for comparison
                } else {
                    false
                };
                
                for span in line.spans.iter() {
                    let text = span.content.to_string();
                    let mut span_style = span.style;
                    
                    // Apply current line background highlighting
                    if is_current_line {
                        span_style = span_style.bg(Color::Rgb(40, 40, 40)); // Dark gray background for current line
                    }
                    
                    // Apply error line background highlighting (overrides current line)
                    if has_error_line {
                        span_style = span_style.bg(Color::Rgb(60, 20, 20)); // Dark red background for error line
                    }
                    
                    for ch in text.chars() {
                        // Safety check to prevent infinite character processing
                        if char_pos > 10000 {
                            log::warn!("Draw thread: character position too high ({}), breaking to prevent infinite loop", char_pos);
                            break;
                        }
                        
                        let mut style = span_style;
                        
                        // Add indentation guides
                        if editor.show_indentation_guides && ch == ' ' {
                            // Calculate indentation level based on line content (with bounds check)
                            if actual_line_idx < current_tab.content.len() {
                                let line_content = &current_tab.content[actual_line_idx];
                                let leading_spaces = line_content.len() - line_content.trim_start().len();
                                
                                // Show guide at every 4 spaces or at tab boundaries
                                if char_pos < leading_spaces && char_pos > 0 && char_pos % 4 == 0 {
                                    style = style.fg(editor.theme.comment);
                                }
                            }
                        }
                        
                        // Add whitespace visualization
                        if editor.show_whitespace {
                            match ch {
                                ' ' => {
                                    // Show spaces as middle dots (only if not covered by indentation guides)
                                    if !editor.show_indentation_guides || char_pos % 4 != 0 {
                                        style = style.fg(editor.theme.comment);
                                    }
                                },
                                '\t' => {
                                    // Show tabs as arrows - replace the character
                                    style = style.fg(Color::Red);
                                },
                                _ => {}
                            }
                            
                            // Highlight trailing whitespace in red (with bounds check)
                            if actual_line_idx < current_tab.content.len() {
                                let line_content = &current_tab.content[actual_line_idx];
                                let trimmed_len = line_content.trim_end().len();
                                if char_pos >= trimmed_len && (ch == ' ' || ch == '\t') {
                                    style = style.bg(Color::Red).fg(Color::White);
                                }
                            }
                        }
                        
                        // Check if this character is within a search result (highlight all matches dimly)
                        let mut is_current_match = false;
                        for (match_idx, &(line, start, end)) in editor.search_results.iter().enumerate() {
                            if actual_line_idx == line && char_pos >= start && char_pos < end {
                                if match_idx == editor.current_match_index {
                                    // Current match - bright highlight
                                    style = style.bg(Color::Yellow).fg(Color::Black);
                                    is_current_match = true;
                                } else {
                                    // Other matches - dim highlight
                                    style = style.bg(Color::DarkGray).fg(Color::White);
                                }
                                break;
                            }
                        }
                        
                        // Check if this character is within selection (but not if it's a search match)
                        if !is_current_match && current_tab.selection_start.is_some() && current_tab.selection_end.is_some() {
                            let start = current_tab.selection_start.expect("selection_start checked to be Some");
                            let end = current_tab.selection_end.expect("selection_end checked to be Some");
                            let (start_pos, end_pos) = normalize_selection_positions(start, end);
                            
                            let is_selected = if start_pos.1 == end_pos.1 {
                                // Single line selection
                                actual_line_idx == start_pos.1 && char_pos >= start_pos.0 && char_pos < end_pos.0
                            } else {
                                // Multi-line selection
                                if actual_line_idx == start_pos.1 {
                                    char_pos >= start_pos.0
                                } else if actual_line_idx == end_pos.1 {
                                    char_pos < end_pos.0
                                } else {
                                    actual_line_idx > start_pos.1 && actual_line_idx < end_pos.1
                                }
                            };
                            
                            if is_selected {
                                style = style.bg(Color::Blue).fg(Color::White);
                            }
                        }
                        
                        // Check if this character is a matching bracket
                        if editor.highlight_matching_brackets {
                            let current_pos = (char_pos, actual_line_idx);
                            let cursor_pos = (current_tab.cursor_x, current_tab.cursor_y);
                            
                            // Highlight current bracket (at cursor position) and its match
                            if current_pos == cursor_pos || Some(current_pos) == editor.matching_bracket_pos {
                                // Check if this is actually a bracket character
                                if matches!(ch, '(' | ')' | '[' | ']' | '{' | '}') {
                                    style = style.bg(Color::Magenta).fg(Color::White).add_modifier(Modifier::BOLD);
                                }
                            }
                        }
                        
                        // Apply cursor highlighting (make cursor position white)
                        if actual_line_idx == current_tab.cursor_y && char_pos == current_tab.cursor_x {
                            style = style.fg(Color::White);
                        }
                        
                        new_spans.push(Span::styled(ch.to_string(), style));
                        char_pos += 1;
                    }
                }
                
                // Handle case where cursor is at the end of the line
                let cursor_y_visible = current_tab.cursor_y.saturating_sub(current_tab.scroll_position as usize);
                if visible_line_idx == cursor_y_visible && char_pos == current_tab.cursor_x {
                    new_spans.push(Span::styled(" ", Style::default().fg(Color::White)));
                }
                
                *line = Line::from(new_spans);
            }

            // Render line numbers if enabled
            if editor.show_line_numbers {
                render_line_numbers(f, &editor, content_layout[0]);
            }

            let current_tab = editor.get_current_tab();
            let editor_title = if let Some(ref filename) = &current_tab.filename { 
                format!("NAIL - {}", filename) 
            } else { 
                "NAIL".to_string() 
            };
            
            let content_area = match (editor.show_line_numbers, editor.show_minimap) {
                (true, true) => content_layout[1],  // Line numbers + content + minimap + scrollbar
                (true, false) => content_layout[1], // Line numbers + content + scrollbar
                (false, true) => content_layout[0], // Content + minimap + scrollbar
                (false, false) => content_layout[0], // Content + scrollbar
            };
            
            if editor.debug_mode {
                log::info!("Rendering {} lines of content to area {:?}", visible_content.len(), content_area);
                if !visible_content.is_empty() {
                    let first_line_text: String = visible_content[0].spans.iter()
                        .map(|s| s.content.to_string()).collect();
                    log::info!("First visible line: '{}'", first_line_text);
                }
            }
            
            let paragraph =
                Paragraph::new(visible_content).block(Block::default().borders(Borders::ALL).title(editor_title)).style(Style::default().bg(editor.theme.background).fg(editor.theme.default));

            f.render_widget(paragraph, content_area);
            
            if editor.debug_mode {
                log::info!("Content rendered successfully");
            }

            // Render minimap if enabled
            if editor.show_minimap {
                let minimap_area = match editor.show_line_numbers {
                    true => content_layout[2],  // After line numbers and content
                    false => content_layout[1], // After content
                };
                // render_minimap(f, editor, minimap_area); // Function not implemented yet
            }

            let scrollbar = Scrollbar::default()
                .style(Style::default().fg(editor.theme.default))
                .orientation(ScrollbarOrientation::VerticalRight)
                .symbols(ratatui::symbols::scrollbar::VERTICAL)
                .begin_symbol(None)
                .end_symbol(None);

            let current_tab = editor.get_current_tab();
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(current_tab.content.len())
                .position(current_tab.scroll_position as usize);

            let scrollbar_area = match (editor.show_line_numbers, editor.show_minimap) {
                (true, true) => content_layout[3],  // Line numbers + content + minimap + scrollbar
                (true, false) => content_layout[2], // Line numbers + content + scrollbar
                (false, true) => content_layout[2], // Content + minimap + scrollbar
                (false, false) => content_layout[1], // Content + scrollbar
            };
            
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);

            // Set cursor
            let current_tab = editor.get_current_tab();
            let cursor_y = current_tab.cursor_y.saturating_sub(current_tab.scroll_position.into());
            if editor.debug_mode {
                log::info!("Cursor position - y: {}, x: {}, scroll: {}, visible_y: {}", 
                    current_tab.cursor_y, current_tab.cursor_x, current_tab.scroll_position, cursor_y);
            }

            if cursor_y < content_area.height.saturating_sub(2) as usize {
                // Account for line numbers gutter when positioning cursor
                let gutter_offset = if editor.show_line_numbers {
                    calculate_line_number_width(current_tab.content.len())
                } else {
                    0
                };
                
                let cursor_pos = Position { 
                    x: content_area.x + current_tab.cursor_x as u16 + 1, 
                    y: content_area.y + cursor_y as u16 + 1 
                };
                if editor.debug_mode {
                    log::info!("Setting cursor at: {:?}, content_area: {:?}", cursor_pos, content_area);
                }
                f.set_cursor_position(cursor_pos);
            }

            // Render status bar at the bottom
            display_status_bar(f, &editor, chunks[2]);
            
            // Always display build status overlay
            display_build_status(f, &editor);

            // Check and draw errors FIRST
            if let Some(error) = &editor.code_error {
                display_error(f, error, &editor, content_area);
            }
            
            // Draw completion popup LAST so it appears on top
            if editor.show_completions && !editor.completions.is_empty() {
                if editor.show_detail_view {
                    display_completion_detail(f, &editor, content_area);
                } else {
                    display_completions(f, &editor, content_area);
                }
            }
            
            // Draw dialog LAST so it appears on top of everything
            if editor.dialog_mode != crate::DialogMode::None {
                display_dialog(f, &editor);
            }
        });

        match result_draw {
            Ok(_) => {}
            Err(err) => log::error!("Error drawing terminal: {:?}", err),
        }
    }
}

fn display_status_bar(f: &mut Frame, editor: &Editor, area: Rect) {
    let current_tab = editor.get_current_tab();
    
    // Create status bar content
    let file_info = if let Some(filename) = &current_tab.filename {
        format!(" {} ", filename)
    } else {
        " Untitled ".to_string()
    };
    
    let cursor_info = format!(" {}:{} ", current_tab.cursor_y + 1, current_tab.cursor_x + 1);
    let line_count = format!(" {} lines ", current_tab.content.len());
    let modified_indicator = if current_tab.modified { " [*] " } else { " " };
    
    // Selection info
    let selection_info = if current_tab.selection_start.is_some() && current_tab.selection_end.is_some() {
        let start = current_tab.selection_start.expect("selection_start checked to be Some");
        let end = current_tab.selection_end.expect("selection_end checked to be Some");
        let (start_pos, end_pos) = normalize_selection_positions(start, end);
        
        let selected_chars = if start_pos.1 == end_pos.1 {
            // Single line selection
            end_pos.0 - start_pos.0
        } else {
            // Multi-line selection - rough estimate
            let lines = end_pos.1 - start_pos.1 + 1;
            let chars_in_first_line = current_tab.content[start_pos.1].len() - start_pos.0;
            let chars_in_last_line = end_pos.0;
            let chars_in_middle_lines: usize = if lines > 2 {
                current_tab.content[(start_pos.1 + 1)..end_pos.1]
                    .iter()
                    .map(|line| line.len() + 1) // +1 for newline
                    .sum()
            } else {
                0
            };
            chars_in_first_line + chars_in_middle_lines + chars_in_last_line
        };
        
        format!(" {} selected ", selected_chars)
    } else {
        String::new()
    };
    
    // File size info
    let file_size: usize = current_tab.content.iter().map(|line| line.len() + 1).sum(); // +1 for newlines
    let size_info = format!(" {} bytes ", file_size);
    
    // Visual features status
    let mut visual_features = Vec::new();
    if editor.show_line_numbers { visual_features.push("LN"); }
    if editor.highlight_current_line { visual_features.push("HL"); }
    if editor.highlight_matching_brackets { visual_features.push("BR"); }
    if editor.show_whitespace { visual_features.push("WS"); }
    if editor.show_indentation_guides { visual_features.push("IG"); }
    let features_info = if visual_features.is_empty() {
        String::new()
    } else {
        format!(" [{}] ", visual_features.join(","))
    };
    
    // Tab info
    let tab_info = format!(" Tab {}/{} ", editor.tab_index + 1, editor.tabs.len());
    
    // Keyboard shortcuts hint
    let shortcuts = " Ctrl+L: Line# | Ctrl+Shift+H: Highlight | Ctrl+Shift+B: Brackets | Ctrl+Shift+W: Whitespace ";
    
    // Create spans for different parts
    let mut spans = vec![
        Span::styled(file_info, Style::default().fg(Color::Cyan).bg(Color::Black)),
        Span::styled(modified_indicator, Style::default().fg(Color::Red).bg(Color::Black)),
        Span::styled(cursor_info, Style::default().fg(Color::Green).bg(Color::Black)),
        Span::styled(line_count, Style::default().fg(Color::Yellow).bg(Color::Black)),
        Span::styled(size_info, Style::default().fg(Color::Blue).bg(Color::Black)),
        Span::styled(tab_info, Style::default().fg(Color::Magenta).bg(Color::Black)),
    ];
    
    // Add selection info if there's a selection
    if !selection_info.is_empty() {
        spans.push(Span::styled(selection_info, Style::default().fg(Color::LightBlue).bg(Color::Black)));
    }
    
    // Add visual features info if any are enabled
    if !features_info.is_empty() {
        spans.push(Span::styled(features_info, Style::default().fg(Color::LightGreen).bg(Color::Black)));
    }
    
    // Add padding to push shortcuts to the right
    let current_width: usize = spans.iter().map(|s| s.content.len()).sum();
    let shortcuts_width = shortcuts.len();
    let total_available = area.width as usize;
    
    if current_width + shortcuts_width < total_available {
        let padding_needed = total_available - current_width - shortcuts_width;
        spans.push(Span::styled(" ".repeat(padding_needed), Style::default().bg(Color::Black)));
    }
    
    spans.push(Span::styled(shortcuts, Style::default().fg(Color::DarkGray).bg(Color::Black)));
    
    let status_line = Line::from(spans);
    let status_paragraph = Paragraph::new(vec![status_line])
        .style(Style::default().bg(Color::Black));
    
    f.render_widget(status_paragraph, area);
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
    let current_tab = editor.get_current_tab();
    // Error line is 1-based, convert to 0-based for array indexing
    let error_line_0based = error.code_span.start_line.saturating_sub(1);
    let error_line = error_line_0based.saturating_sub(current_tab.scroll_position as usize);
    let error_column = error.code_span.start_column.saturating_sub(1);  // Column is also 1-based
    let error_message = error.message.clone();

    log::info!("Displaying error - line: {}, column: {}, message: {}", error.code_span.start_line, error_column, error_message);

    // Only display the error if it's within the visible area

    let error_message = Line::from(vec![Span::styled(error_message, Style::default().fg(editor.theme.error).bg(editor.theme.background))]);

    let paragraph = Paragraph::new(error_message.clone()).style(Style::default().fg(editor.theme.error).bg(editor.theme.background)).alignment(Alignment::Left);

    // Calculate current cursor line relative to scroll position
    let current_cursor_line = current_tab.cursor_y.saturating_sub(current_tab.scroll_position as usize);
    
    // Position error to avoid covering the current cursor line
    let error_display_y = if error_line == current_cursor_line {
        // If error is on current line, position it below unless we're at the bottom
        if content_area.y + error_line as u16 + 2 < content_area.y + content_area.height {
            content_area.y + error_line as u16 + 2 // Position below current line
        } else {
            // If no space below, position above
            content_area.y + (error_line as u16).saturating_sub(1)
        }
    } else {
        content_area.y + error_line as u16 + 1 // Normal positioning
    };

    let error_area = Rect::new(
        content_area.x + error_column as u16,
        error_display_y,
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
    let current_tab = editor.get_current_tab();
    let cursor_y = current_tab.cursor_y.saturating_sub(current_tab.scroll_position as usize);
    // Place popup BELOW the current line: +1 for border, +1 to go to next line, +1 for spacing
    let popup_y = content_area.y + cursor_y as u16 + 3;
    
    // Position popup to the right of the current word being typed
    let word_start = {
        let mut start = current_tab.cursor_x;
        if current_tab.cursor_y < current_tab.content.len() {
            let line = &current_tab.content[current_tab.cursor_y];
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
                CompletionKind::Struct => "s ",
                CompletionKind::Enum => "e ",
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

fn display_dialog(f: &mut Frame, editor: &Editor) {
    use crate::DialogMode;
    
    match editor.dialog_mode {
        DialogMode::GoToLine => {
            display_goto_line_dialog(f, editor);
        }
        DialogMode::Find => {
            display_find_dialog(f, editor);
        }
        DialogMode::Replace => {
            display_replace_dialog(f, editor);
        }
        DialogMode::OpenFile => {
            display_file_dialog(f, editor);
        }
        DialogMode::StdLibBrowser => {
            display_stdlib_dialog(f, editor);
        }
        DialogMode::None => {
            // No dialog to display
        }
    }
}

fn display_goto_line_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::Wrap;
    
    let current_line = editor.get_current_line_number();
    let total_lines = editor.get_total_lines();
    
    // Build dialog content
    let mut lines = vec![];
    
    // Title
    lines.push(Line::from(vec![
        Span::styled("Go to Line", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    
    // Input field
    lines.push(Line::from(vec![
        Span::styled("Line number: ", Style::default().fg(Color::White)),
        Span::styled(&editor.goto_line_input, Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::styled("_", Style::default().fg(Color::White).bg(Color::DarkGray)), // Cursor
    ]));
    lines.push(Line::from(""));
    
    // Info
    lines.push(Line::from(vec![
        Span::styled(format!("Current: {} / {}", current_line, total_lines), Style::default().fg(Color::Gray)),
    ]));
    lines.push(Line::from(""));
    
    // Help text
    lines.push(Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("ENTER", Style::default().fg(Color::Green)),
        Span::styled(" to go, ", Style::default().fg(Color::DarkGray)),
        Span::styled("ESC", Style::default().fg(Color::Red)),
        Span::styled(" to cancel", Style::default().fg(Color::DarkGray)),
    ]));
    
    // Calculate dialog size
    let width = 40;
    let height = lines.len() + 2; // +2 for borders
    
    // Center the dialog
    let popup_x = (f.area().width.saturating_sub(width)) / 2;
    let popup_y = (f.area().height.saturating_sub(height as u16)) / 2;
    
    let dialog_area = Rect::new(
        popup_x,
        popup_y,
        width,
        height as u16,
    );
    
    // Clear the area and draw the dialog
    f.render_widget(Clear, dialog_area);
    
    let dialog_paragraph = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Go to Line (Ctrl+G) ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(editor.theme.background))
        .wrap(Wrap { trim: true });
    
    f.render_widget(dialog_paragraph, dialog_area);
}

fn display_find_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::{Wrap, Clear};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Color, Style, Modifier};
    use ratatui::layout::Rect;
    use ratatui::widgets::{Block, Borders, Paragraph};
    
    let search_status = editor.get_search_status();
    
    // Build dialog content
    let mut lines = vec![];
    
    // Title
    lines.push(Line::from(vec![
        Span::styled("Find", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    
    // Search input field
    lines.push(Line::from(vec![
        Span::styled("Find: ", Style::default().fg(Color::White)),
        Span::styled(&editor.search_query, Style::default().fg(Color::White).bg(Color::DarkGray)),
        Span::styled("_", Style::default().fg(Color::White).bg(Color::DarkGray)), // Cursor
    ]));
    lines.push(Line::from(""));
    
    // Case sensitivity status
    let case_text = if editor.case_sensitive { "Case sensitive" } else { "Case insensitive" };
    lines.push(Line::from(vec![
        Span::styled(case_text, Style::default().fg(Color::Gray)),
    ]));
    
    // Search results
    if !search_status.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(search_status, Style::default().fg(Color::Green)),
        ]));
    }
    lines.push(Line::from(""));
    
    // Help text
    lines.push(Line::from(vec![
        Span::styled("ENTER", Style::default().fg(Color::Green)),
        Span::styled(": next, ", Style::default().fg(Color::DarkGray)),
        Span::styled("F3", Style::default().fg(Color::Green)),
        Span::styled(": next, ", Style::default().fg(Color::DarkGray)),
        Span::styled("Shift+F3", Style::default().fg(Color::Green)),
        Span::styled(": prev", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+I", Style::default().fg(Color::Green)),
        Span::styled(": toggle case, ", Style::default().fg(Color::DarkGray)),
        Span::styled("ESC", Style::default().fg(Color::Red)),
        Span::styled(": close", Style::default().fg(Color::DarkGray)),
    ]));
    
    // Calculate dialog size
    let width = 50;
    let height = lines.len() + 2; // +2 for borders
    
    // Center the dialog
    let popup_x = (f.area().width.saturating_sub(width)) / 2;
    let popup_y = (f.area().height.saturating_sub(height as u16)) / 2;
    
    let dialog_area = Rect::new(
        popup_x,
        popup_y,
        width,
        height as u16,
    );
    
    // Clear the area and draw the dialog
    f.render_widget(Clear, dialog_area);
    
    let dialog_paragraph = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Find (Ctrl+F) ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(editor.theme.background))
        .wrap(Wrap { trim: true });
    
    f.render_widget(dialog_paragraph, dialog_area);
}

fn display_replace_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::{Wrap, Clear};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Color, Style, Modifier};
    use ratatui::layout::Rect;
    use ratatui::widgets::{Block, Borders, Paragraph};
    
    let search_status = editor.get_search_status();
    
    // Build dialog content
    let mut lines = vec![];
    
    // Title
    lines.push(Line::from(vec![
        Span::styled("Find and Replace", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    
    // Find input field
    if editor.replace_field_active {
        // Find field inactive
        lines.push(Line::from(vec![
            Span::styled("Find: ", Style::default().fg(Color::White)),
            Span::styled(&editor.search_query, Style::default().fg(Color::White).bg(Color::Gray)),
        ]));
    } else {
        // Find field active
        lines.push(Line::from(vec![
            Span::styled("Find: ", Style::default().fg(Color::White)),
            Span::styled(&editor.search_query, Style::default().fg(Color::White).bg(Color::DarkGray)),
            Span::styled("_", Style::default().fg(Color::White).bg(Color::DarkGray)), // Cursor
        ]));
    }
    
    // Replace input field
    if editor.replace_field_active {
        // Replace field active
        lines.push(Line::from(vec![
            Span::styled("Replace: ", Style::default().fg(Color::White)),
            Span::styled(&editor.replace_text, Style::default().fg(Color::White).bg(Color::DarkGray)),
            Span::styled("_", Style::default().fg(Color::White).bg(Color::DarkGray)), // Cursor
        ]));
    } else {
        // Replace field inactive
        lines.push(Line::from(vec![
            Span::styled("Replace: ", Style::default().fg(Color::White)),
            Span::styled(&editor.replace_text, Style::default().fg(Color::White).bg(Color::Gray)),
        ]));
    }
    lines.push(Line::from(""));
    
    // Case sensitivity status
    let case_text = if editor.case_sensitive { "Case sensitive" } else { "Case insensitive" };
    lines.push(Line::from(vec![
        Span::styled(case_text, Style::default().fg(Color::Gray)),
    ]));
    
    // Search results
    if !search_status.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(search_status, Style::default().fg(Color::Green)),
        ]));
    }
    lines.push(Line::from(""));
    
    // Help text
    lines.push(Line::from(vec![
        Span::styled("ENTER", Style::default().fg(Color::Green)),
        Span::styled(": replace current, ", Style::default().fg(Color::DarkGray)),
        Span::styled("Alt+ENTER", Style::default().fg(Color::Green)),
        Span::styled(": replace all", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("TAB", Style::default().fg(Color::Green)),
        Span::styled(": switch field, ", Style::default().fg(Color::DarkGray)),
        Span::styled("F3", Style::default().fg(Color::Green)),
        Span::styled(": next, ", Style::default().fg(Color::DarkGray)),
        Span::styled("Shift+F3", Style::default().fg(Color::Green)),
        Span::styled(": prev", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+I", Style::default().fg(Color::Green)),
        Span::styled(": toggle case, ", Style::default().fg(Color::DarkGray)),
        Span::styled("ESC", Style::default().fg(Color::Red)),
        Span::styled(": close", Style::default().fg(Color::DarkGray)),
    ]));
    
    // Calculate dialog size
    let width = 60;
    let height = lines.len() + 2; // +2 for borders
    
    // Center the dialog
    let popup_x = (f.area().width.saturating_sub(width)) / 2;
    let popup_y = (f.area().height.saturating_sub(height as u16)) / 2;
    
    let dialog_area = Rect::new(
        popup_x,
        popup_y,
        width,
        height as u16,
    );
    
    // Clear the area and draw the dialog
    f.render_widget(Clear, dialog_area);
    
    let dialog_paragraph = Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Find and Replace (Ctrl+H) ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(editor.theme.background))
        .wrap(Wrap { trim: true });
    
    f.render_widget(dialog_paragraph, dialog_area);
}

pub fn key_thread_logic(editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>, tx: Sender<EditorMessage>, tx_build: Sender<EditorMessage>) {
    log::info!("Key thread started");
    
    // Set up panic handler for this thread
    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("KEY THREAD PANICKED: {:?}", panic_info);
        eprintln!("KEY THREAD PANICKED: {:?}", panic_info);
    }));
    
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
                log::debug!("Event available from poll");
                match event::read() {
                    Ok(Event::Key(key)) => {
                        log::warn!("==> KEY EVENT: {:?}", key);
                        // Use timeout-based lock to prevent deadlocks
                        let mut editor = match try_lock_with_timeout(&editor_arc, 500) {
                            Some(editor) => editor,
                            None => {
                                log::warn!("Key thread: editor lock timeout, skipping key event");
                                continue;
                            }
                        };
                        
                        // Handle dialog modes first
                        match editor.dialog_mode {
                            crate::DialogMode::OpenFile => {
                                if editor.handle_file_dialog_key(key) {
                                    continue;
                                }
                            },
                            crate::DialogMode::StdLibBrowser => {
                                if editor.handle_stdlib_browser_input(key) {
                                    continue;
                                }
                            },
                            _ => {}
                        }
                        
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

                                let mut editor = match try_lock_with_timeout(&editor_arc, 1000) {
                                    Some(editor) => editor,
                                    None => {
                                        log::error!("Key thread: editor lock timeout during save operation");
                                        continue;
                                    }
                                };
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

                                let example_files = match fs::read_dir("examples") {
                                    Ok(dir) => dir
                                        .filter_map(Result::ok)
                                        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "nail"))
                                        .map(|entry| entry.path().to_string_lossy().to_string())
                                        .collect::<Vec<String>>(),
                                    Err(e) => {
                                        log::warn!("Failed to read examples directory: {}", e);
                                        editor.build_status = BuildStatus::Failed("Examples directory not found".to_string());
                                        continue;
                                    }
                                };

                                if example_files.is_empty() {
                                    editor.build_status = BuildStatus::Failed("No example files found".to_string());
                                    continue;
                                }

                                // Find current file index
                                let current_tab = editor.get_current_tab();
                                let current_index = if let Some(ref current) = current_tab.filename { 
                                    example_files.iter().position(|f| f == current).unwrap_or(0) 
                                } else { 
                                    0 
                                };

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
                            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+L - Toggle line numbers
                                editor.show_line_numbers = !editor.show_line_numbers;
                                log::info!("Line numbers toggled: {}", editor.show_line_numbers);
                            },
                            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Shift+H - Toggle current line highlighting
                                editor.highlight_current_line = !editor.highlight_current_line;
                                log::info!("Current line highlighting toggled: {}", editor.highlight_current_line);
                            },
                            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Shift+B - Toggle bracket matching
                                editor.highlight_matching_brackets = !editor.highlight_matching_brackets;
                                if !editor.highlight_matching_brackets {
                                    editor.matching_bracket_pos = None;
                                }
                                log::info!("Bracket matching toggled: {}", editor.highlight_matching_brackets);
                            },
                            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Shift+W - Toggle whitespace visualization
                                editor.show_whitespace = !editor.show_whitespace;
                                log::info!("Whitespace visualization toggled: {}", editor.show_whitespace);
                            },
                            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Shift+G - Toggle indentation guides
                                editor.show_indentation_guides = !editor.show_indentation_guides;
                                log::info!("Indentation guides toggled: {}", editor.show_indentation_guides);
                            },
                            KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Shift+M - Toggle minimap
                                editor.show_minimap = !editor.show_minimap;
                                log::info!("Minimap toggled: {}", editor.show_minimap);
                            },
                            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+A - Select all
                                editor.select_all();
                            },
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+C - Copy selection
                                if let Err(e) = editor.copy_selection() {
                                    log::error!("Failed to copy to clipboard: {}", e);
                                }
                            },
                            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+X - Cut selection
                                if let Err(e) = editor.cut_selection() {
                                    log::error!("Failed to cut to clipboard: {}", e);
                                }
                            },
                            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+V - Paste from clipboard
                                if let Err(e) = editor.paste_from_clipboard() {
                                    log::error!("Failed to paste from clipboard: {}", e);
                                }
                            },
                            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Z - Undo
                                if editor.undo() {
                                    log::info!("Undo operation performed");
                                } else {
                                    log::info!("Nothing to undo");
                                }
                            },
                            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+Y - Redo
                                if editor.redo() {
                                    log::info!("Redo operation performed");
                                } else {
                                    log::info!("Nothing to redo");
                                }
                            },
                            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Shift+Z - Alternative Redo
                                if editor.redo() {
                                    log::info!("Redo operation performed (Ctrl+Shift+Z)");
                                } else {
                                    log::info!("Nothing to redo");
                                }
                            },
                            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+G - Go to Line dialog
                                editor.show_goto_line_dialog();
                            },
                            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+F - Find dialog
                                editor.show_find_dialog();
                            },
                            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+H - Replace dialog
                                editor.show_replace_dialog();
                            },
                            KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+I - Toggle case sensitivity in find/replace
                                if matches!(editor.dialog_mode, crate::DialogMode::Find | crate::DialogMode::Replace) {
                                    editor.toggle_case_sensitivity();
                                }
                            },
                            KeyCode::F(3) => {
                                // F3 - Find next
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    editor.search_direction_forward = false;
                                    editor.find_previous();
                                } else {
                                    editor.search_direction_forward = true;
                                    editor.find_next();
                                }
                            },
                            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+O - Open file dialog
                                editor.open_file_dialog();
                            },
                            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+W - Close current tab
                                let tab_index = editor.tab_index;
                                editor.close_tab(tab_index);
                            },
                            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+N - New tab
                                editor.new_tab();
                            },
                            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Shift+P - Open standard library browser
                                editor.open_stdlib_browser();
                            },
                            KeyCode::Tab if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+Tab - Next tab
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    editor.prev_tab();
                                } else {
                                    editor.next_tab();
                                }
                            },
                            KeyCode::Char(n) if key.modifiers.contains(KeyModifiers::CONTROL) && n.is_ascii_digit() => {
                                // Ctrl+1-9 - Switch to specific tab
                                if let Some(digit) = n.to_digit(10) {
                                    let tab_index = (digit as usize).saturating_sub(1);
                                    editor.switch_to_tab(tab_index);
                                }
                            },
                            KeyCode::Char('/') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+/ - Toggle comment
                                editor.toggle_comment();
                            },
                            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+D - Duplicate line
                                editor.duplicate_line();
                            },
                            KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
                                // Alt+Up - Move line up
                                editor.move_line_up();
                            },
                            KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
                                // Alt+Down - Move line down
                                editor.move_line_down();
                            },
                            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
                                // Ctrl+Shift+K - Delete line
                                editor.delete_line();
                            },
                            KeyCode::Char(']') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+] - Jump to matching bracket
                                editor.jump_to_matching_bracket();
                            },
                            KeyCode::Char(c) => {
                                log::warn!("Received KeyCode::Char('{}'), dialog_mode: {:?}", c, editor.dialog_mode);
                                // Check if we're in a dialog mode
                                match editor.dialog_mode {
                                    crate::DialogMode::GoToLine => {
                                        editor.handle_goto_line_input(c);
                                    },
                                    crate::DialogMode::Find => {
                                        editor.handle_find_input(c);
                                    },
                                    crate::DialogMode::Replace => {
                                        editor.handle_replace_dialog_input(c);
                                    },
                                    crate::DialogMode::OpenFile => {
                                        editor.handle_file_dialog_input(c);
                                    },
                                    crate::DialogMode::StdLibBrowser => {
                                        editor.handle_stdlib_dialog_input(c);
                                    },
                                    crate::DialogMode::None => {
                                        log::warn!("Calling insert_char('{}') in normal mode", c);
                                        editor.insert_char(c);
                                        log::warn!("After insert_char, current line: '{}'", editor.get_current_tab().content[editor.get_current_tab().cursor_y]);
                                        // Only update completions for single character input
                                        // to avoid overwhelming the system during paste operations
                                        if !key.modifiers.contains(KeyModifiers::SHIFT) {
                                            editor.update_completions();
                                        }
                                    }
                                }
                            },
                            KeyCode::Up => {
                                if editor.show_completions {
                                    editor.previous_completion();
                                } else {
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_cursor_up_with_selection(extend_selection);
                                }
                            },
                            KeyCode::Down => {
                                if editor.show_completions {
                                    editor.next_completion();
                                } else {
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_cursor_down_with_selection(extend_selection);
                                }
                            },
                            KeyCode::PageDown => editor.scroll_down(),
                            KeyCode::PageUp => editor.scroll_up(),
                            KeyCode::Tab => {
                                if editor.dialog_mode == crate::DialogMode::Replace {
                                    // In replace mode, Tab switches between find and replace fields
                                    editor.switch_replace_field();
                                } else if editor.show_completions {
                                    editor.accept_completion();
                                } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    // Shift+Tab - Dedent
                                    editor.dedent_selection();
                                } else {
                                    // Tab - Indent (if there's selection) or trigger completion
                                    let has_selection = {
                                        let current_tab = editor.get_current_tab();
                                        current_tab.has_selection()
                                    };
                                    
                                    if has_selection {
                                        editor.indent_selection();
                                    } else {
                                        // Trigger completion
                                        editor.update_completions();
                                        if !editor.show_completions {
                                            // If no completions, switch tabs as before
                                            editor.next_tab();
                                        }
                                    }
                                }
                            },
                            KeyCode::BackTab => {
                                // BackTab (Shift+Tab without explicit modifiers check) - Dedent
                                let has_selection = {
                                    let current_tab = editor.get_current_tab();
                                    current_tab.has_selection()
                                };
                                
                                if has_selection {
                                    editor.dedent_selection();
                                } else {
                                    editor.previous_tab();
                                }
                            },
                            KeyCode::Backspace => {
                                match editor.dialog_mode {
                                    crate::DialogMode::GoToLine => {
                                        editor.handle_goto_line_backspace();
                                    },
                                    crate::DialogMode::Find => {
                                        editor.handle_find_backspace();
                                    },
                                    crate::DialogMode::Replace => {
                                        editor.handle_replace_dialog_backspace();
                                    },
                                    crate::DialogMode::OpenFile => {
                                        editor.handle_file_dialog_backspace();
                                    },
                                    crate::DialogMode::StdLibBrowser => {
                                        editor.handle_stdlib_dialog_backspace();
                                    },
                                    crate::DialogMode::None => {
                                        editor.delete_char();
                                        editor.update_completions();
                                    }
                                }
                            },
                            KeyCode::Delete => {
                                editor.delete_forward();
                                editor.update_completions();
                            },
                            KeyCode::Enter => {
                                match editor.dialog_mode {
                                    crate::DialogMode::GoToLine => {
                                        editor.execute_goto_line();
                                    },
                                    crate::DialogMode::Find => {
                                        // Enter in find mode - find next
                                        editor.find_next();
                                    },
                                    crate::DialogMode::Replace => {
                                        // Enter in replace mode - replace current and find next
                                        if key.modifiers.contains(KeyModifiers::ALT) {
                                            // Alt+Enter - Replace all
                                            editor.replace_all();
                                        } else {
                                            // Regular Enter - Replace current
                                            editor.replace_current();
                                        }
                                    },
                                    crate::DialogMode::OpenFile => {
                                        editor.handle_file_dialog_enter();
                                    },
                                    crate::DialogMode::StdLibBrowser => {
                                        editor.handle_stdlib_dialog_enter();
                                    },
                                    crate::DialogMode::None => {
                                        if editor.show_completions {
                                            editor.accept_completion();
                                        } else {
                                            editor.insert_newline();
                                        }
                                    }
                                }
                            },
                            KeyCode::Esc => {
                                if editor.dialog_mode != crate::DialogMode::None {
                                    // Close any open dialog
                                    editor.close_dialog();
                                } else if editor.show_detail_view {
                                    // Go back to completion list from detail view
                                    editor.show_detail_view = false;
                                } else if editor.show_completions {
                                    // Close completions entirely
                                    editor.show_completions = false;
                                    editor.show_detail_view = false;  // Reset detail view too
                                    editor.completions.clear();
                                } else if editor.has_selection() {
                                    // Clear selection if no completions are showing
                                    editor.clear_selection();
                                }
                            },
                            KeyCode::F(1) => {
                                // Toggle detail view for selected completion
                                if editor.show_completions && !editor.completions.is_empty() {
                                    editor.show_detail_view = !editor.show_detail_view;
                                }
                            },
                            KeyCode::Left => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl+Left - Move to previous word
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_cursor_left_word_with_selection(extend_selection);
                                } else {
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_cursor_left_with_selection(extend_selection);
                                }
                            },
                            KeyCode::Right => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl+Right - Move to next word
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_cursor_right_word_with_selection(extend_selection);
                                } else {
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_cursor_right_with_selection(extend_selection);
                                }
                            },
                            KeyCode::Home => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl+Home - Move to start of file
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_to_file_start_with_selection(extend_selection);
                                } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    // Shift+Home - Extend selection to line start (keep old behavior for selection)
                                    editor.move_to_line_start_with_selection(true);
                                } else {
                                    // Home - Smart home (toggle between first non-whitespace and line start)
                                    editor.smart_home();
                                }
                            },
                            KeyCode::End => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    // Ctrl+End - Move to end of file
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_to_file_end_with_selection(extend_selection);
                                } else {
                                    // End - Move to end of line
                                    let extend_selection = key.modifiers.contains(KeyModifiers::SHIFT);
                                    editor.move_to_line_end_with_selection(extend_selection);
                                }
                            },
                            _ => {}
                        }
                    }
                    Ok(Event::Paste(data)) => {
                        // Handle paste event - insert text as single operation
                        let mut editor = match try_lock_with_timeout(&editor_arc, 500) {
                            Some(editor) => editor,
                            None => {
                                log::warn!("Key thread: editor lock timeout during paste, skipping paste");
                                continue;
                            }
                        };
                        editor.paste_text(&data);
                        // Don't update completions during paste to avoid lag
                    }
                    Ok(Event::Mouse(_)) => {
                        // Ignore mouse events for now to prevent issues
                        // Mouse events can cause problems if not handled properly
                    }
                    Ok(_) => {
                        // Other events (resize, etc.) - ignore
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
            let current_tab = editor.get_current_tab();
            let tokens = lexer::lexer(&current_tab.content.join("\n"));
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
            editor.get_current_tab().content.join("\n")
        };

        // Run the lexer on the content
        let tokens = lexer::lexer(&content);

        {
            let mut editor = lock(&editor_arc);
            let current_tab = editor.get_current_tab_mut();
            current_tab.tokens = tokens.clone();
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

        // Update AST and extract symbols for intellisense if parsing succeeded
        if parse_succeeded {
            {
                let mut editor = lock(&editor_arc);
                let scope_symbols = editor.extract_symbols_from_ast(&ast);
                let current_tab = editor.get_current_tab_mut();
                current_tab.ast = Some(ast.clone());
                // Extract symbols from AST for autocompletion
                current_tab.scope_symbols = scope_symbols;
            }
            
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

fn display_file_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::{Clear, List, ListItem, ListState};
    
    // Create the dialog area
    let area = f.area();
    let width = std::cmp::min(80, area.width.saturating_sub(4));
    let height = std::cmp::min(20, area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);
    
    // Clear the area
    f.render_widget(Clear, dialog_area);
    
    // Create file list items
    let items: Vec<ListItem> = editor.file_entries.iter().map(|entry| {
        let style = if entry.is_recent {
            Style::default().fg(Color::Yellow)
        } else if entry.is_directory {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };
        ListItem::new(entry.name.clone()).style(style)
    }).collect();
    
    // Create the list widget
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" Open File - {} ", editor.current_directory))
            .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::White));
    
    // Create list state
    let mut list_state = ListState::default();
    list_state.select(Some(editor.file_dialog_index));
    
    f.render_stateful_widget(list, dialog_area, &mut list_state);
    
    // Show search input if any
    if !editor.file_dialog_input.is_empty() {
        let input_area = Rect::new(dialog_area.x + 1, dialog_area.y + dialog_area.height - 2, dialog_area.width - 2, 1);
        let input_paragraph = Paragraph::new(format!("Filter: {}", editor.file_dialog_input))
            .style(Style::default().fg(Color::Yellow).bg(Color::Black));
        f.render_widget(input_paragraph, input_area);
    }
}

fn display_stdlib_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::{Clear, List, ListItem, ListState};
    
    // Create the dialog area
    let area = f.area();
    let width = std::cmp::min(100, area.width.saturating_sub(4));
    let height = std::cmp::min(25, area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);
    
    // Clear the area
    f.render_widget(Clear, dialog_area);
    
    // Split dialog into list and detail areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(8)])
        .split(dialog_area);
    
    // Create function list items
    let items: Vec<ListItem> = editor.stdlib_functions.iter().map(|func| {
        let display = format!("[{}] {}", func.category, func.name);
        ListItem::new(display).style(Style::default().fg(Color::White))
    }).collect();
    
    // Create the list widget
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Standard Library Functions ")
            .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::White));
    
    // Create list state
    let mut list_state = ListState::default();
    list_state.select(Some(editor.stdlib_index));
    
    f.render_stateful_widget(list, chunks[0], &mut list_state);
    
    // Show function details
    if !editor.stdlib_functions.is_empty() && editor.stdlib_index < editor.stdlib_functions.len() {
        let func = &editor.stdlib_functions[editor.stdlib_index];
        let details = vec![
            Line::from(vec![
                Span::styled("Signature: ", Style::default().fg(Color::Yellow)),
                Span::styled(&func.signature, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Description: ", Style::default().fg(Color::Yellow)),
                Span::styled(&func.description, Style::default().fg(Color::White)),
            ]),
        ];
        
        let detail_paragraph = Paragraph::new(details)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" Function Details ")
                .title_style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
            )
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .wrap(ratatui::widgets::Wrap { trim: true });
        
        f.render_widget(detail_paragraph, chunks[1]);
    }
    
    // Show search input if any
    if !editor.stdlib_filter.is_empty() {
        let input_area = Rect::new(dialog_area.x + 1, dialog_area.y + dialog_area.height - 2, dialog_area.width - 2, 1);
        let input_paragraph = Paragraph::new(format!("Filter: {}", editor.stdlib_filter))
            .style(Style::default().fg(Color::Yellow).bg(Color::Black));
        f.render_widget(input_paragraph, input_area);
    }
}

// Helper function to calculate line number gutter width
pub fn calculate_line_number_width(total_lines: usize) -> u16 {
    if total_lines == 0 {
        return 3; // minimum width
    }
    let digits = total_lines.to_string().len();
    (digits + 1).max(3) as u16 // at least 3 characters wide for padding
}

// Function to render line numbers gutter
pub fn render_line_numbers(f: &mut Frame, editor: &Editor, gutter_area: Rect) {
    if !editor.show_line_numbers {
        return;
    }
    
    let current_tab = editor.get_current_tab();
    // Account for the border of the content area - line numbers should align with content inside the border
    // The content has a 1-pixel border on all sides, so we need to skip the first line and reduce visible lines by 2
    let visible_lines = gutter_area.height.saturating_sub(2) as usize;  // -2 for top and bottom borders
    let start_line = current_tab.scroll_position as usize;
    let total_lines = current_tab.content.len();
    
    let mut line_number_content = Vec::new();
    
    for i in 0..visible_lines {
        let actual_line_idx = start_line + i;
        if actual_line_idx >= total_lines {
            break;
        }
        
        let line_number = actual_line_idx + 1; // 1-based line numbers
        let is_current_line = actual_line_idx == current_tab.cursor_y;
        
        // Check if this line has an error
        // Line numbers in errors are 1-based, but we need to compare with displayed line numbers
        let has_error = if let Some(error) = &editor.code_error {
            line_number == error.code_span.start_line  // Compare 1-based line number with 1-based error line
        } else {
            false
        };
        
        let (style, line_text) = if has_error {
            // Show red error marker in gutter
            let style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
            let text = format!("{:>width$}", "●", width = (gutter_area.width - 1) as usize);
            (style, text)
        } else if is_current_line && editor.highlight_current_line {
            let style = Style::default()
                .fg(editor.theme.operator) // Use operator color for current line number
                .add_modifier(Modifier::BOLD);
            let text = format!("{:>width$}", line_number, width = (gutter_area.width - 1) as usize);
            (style, text)
        } else {
            let style = Style::default()
                .fg(editor.theme.comment); // Use comment color for regular line numbers
            let text = format!("{:>width$}", line_number, width = (gutter_area.width - 1) as usize);
            (style, text)
        };
        line_number_content.push(Line::from(vec![
            Span::styled(line_text, style)
        ]));
    }
    
    let line_numbers_paragraph = Paragraph::new(line_number_content)
        .style(Style::default().bg(editor.theme.background))
        .block(Block::default().borders(Borders::NONE));
    
    // Adjust the gutter area to align with content inside the border
    let adjusted_gutter_area = Rect {
        x: gutter_area.x,
        y: gutter_area.y + 1,  // Skip the top border line
        width: gutter_area.width,
        height: gutter_area.height.saturating_sub(2),  // Account for top and bottom borders
    };
    
    f.render_widget(line_numbers_paragraph, adjusted_gutter_area);
}

// Function to find matching bracket position
pub fn find_matching_bracket(content: &[String], cursor_y: usize, cursor_x: usize) -> Option<(usize, usize)> {
    if cursor_y >= content.len() {
        return None;
    }
    
    let line = &content[cursor_y];
    if cursor_x >= line.len() {
        return None;
    }
    
    let chars: Vec<char> = line.chars().collect();
    let bracket = chars[cursor_x];
    
    let (opening, closing, direction) = match bracket {
        '(' => ('(', ')', 1),   // forward
        ')' => ('(', ')', -1),  // backward
        '[' => ('[', ']', 1),   // forward
        ']' => ('[', ']', -1),  // backward
        '{' => ('{', '}', 1),   // forward
        '}' => ('{', '}', -1),  // backward
        _ => return None,
    };
    
    let mut count = 0;
    
    if direction == 1 {
        // Search forward
        for (line_idx, search_line) in content.iter().enumerate().skip(cursor_y) {
            let start_x = if line_idx == cursor_y { cursor_x } else { 0 };
            let line_chars: Vec<char> = search_line.chars().collect();
            
            for (char_idx, &ch) in line_chars.iter().enumerate().skip(start_x) {
                if ch == opening {
                    count += 1;
                } else if ch == closing {
                    count -= 1;
                    if count == 0 {
                        return Some((char_idx, line_idx));
                    }
                }
            }
        }
    } else {
        // Search backward
        for line_idx in (0..=cursor_y).rev() {
            let search_line = &content[line_idx];
            let end_x = if line_idx == cursor_y { cursor_x } else { search_line.len() };
            let line_chars: Vec<char> = search_line.chars().collect();
            
            for char_idx in (0..end_x.min(line_chars.len())).rev() {
                let ch = line_chars[char_idx];
                if ch == closing {
                    count += 1;
                } else if ch == opening {
                    count -= 1;
                    if count == 0 {
                        return Some((char_idx, line_idx));
                    }
                }
            }
        }
    }
    
    None
}
