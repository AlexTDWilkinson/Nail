use crate::checker::checker;
use crate::parser::parse;
use crate::parser::ASTNode;
use crate::transpiler::Transpiler;
// The key tables live in the library half of the crate rather than being
// declared a second time here, so there is one Action enum and not two that
// merely look alike.
use nail::keymap::{Action, Resolution, VimMode};
use crate::CodeError;
use crate::Editor;
use log::error;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::prelude::Position;
use std::backtrace::Backtrace;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

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

use crate::colorizer::ColorizeCache;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

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
    Complete(String),
    Failed(String),
}

/// One function's stats from a running instrumented program.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfiledFunction {
    pub name: String,
    pub calls: u64,
    pub total_nanos: u64,
    pub max_nanos: u64,
}

/// A parsed .nail_profile.json dump, written every second by a running
/// instrumented program. The source hash tells the IDE whether the timings
/// still describe the code on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileData {
    pub source_hash: String,
    pub wall_nanos: u64,
    pub functions: Vec<ProfiledFunction>,
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

    // The colored copy of the file, kept between frames. An unchanged file
    // costs one pass of string comparisons and an edited one costs the lines
    // it edited; only the visible window is cloned out of it for the
    // cursor and selection overlays.
    let mut colorize_cache = ColorizeCache::new();

    // Buffer fingerprint and function declaration lines for timing
    // annotations, keyed by content hash. Rebuilt only when the buffer
    // changes, so an unedited frame costs one hash comparison.
    let mut profile_line_cache: Option<(u64, String, HashMap<String, usize>)> = None;

    // Which tab the cursor was in and where, as of the last frame. The view
    // chases the cursor only when this changes, which is what lets the scroll
    // keys and the wheel move the page without it snapping straight back.
    let mut last_cursor = (usize::MAX, usize::MAX, usize::MAX);

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
            // Held mutably because drawing is also what works out where things
            // ended up on screen, and a click has to be able to ask later.
            let mut editor = match try_lock_with_timeout(&editor_arc, 50) {
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
            let tab_titles = editor.tab_titles();

            let current_tab = editor.get_current_tab();
            let file_title = if current_tab.modified { 
                format!("FILES [*] - Press Ctrl+S to save") 
            } else { 
                "FILES".to_string() 
            };
            
            let tabs = Tabs::new(tab_titles)
                .block(Block::default().borders(Borders::ALL).title(file_title))
                .select(editor.tab_index)
                .style(Style::default().fg(editor.theme.default).bg(editor.theme.background))
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

            // Where the text itself ends up: the content column minus the
            // border drawn around it. A mouse click arrives as a row and a
            // column of the terminal and this is what turns it into a line of
            // the file, so it is recorded before anything is painted.
            let content_area = match (editor.show_line_numbers, editor.show_minimap) {
                (true, _) => content_layout[1],
                (false, _) => content_layout[0],
            };
            let text_area = Rect {
                x: content_area.x + 1,
                y: content_area.y + 1,
                width: content_area.width.saturating_sub(2),
                height: content_area.height.saturating_sub(2),
            };
            let minimap_area = match (editor.show_minimap, editor.show_line_numbers) {
                (true, true) => content_layout[2],
                (true, false) => content_layout[1],
                (false, _) => Rect::default(),
            };
            editor.view = crate::ViewLayout { tabs: chunks[0], text: text_area, minimap: minimap_area };

            // Slide the view to wherever the cursor went, but only when it
            // went somewhere: the scroll keys and the wheel move the page
            // without moving the cursor, and pulling the page back every frame
            // would make them useless.
            let cursor_now = {
                let tab = editor.get_current_tab();
                (editor.tab_index, tab.cursor_x, tab.cursor_y)
            };
            if cursor_now != last_cursor {
                last_cursor = cursor_now;
                let (width, height) = (text_area.width as usize, text_area.height as usize);
                editor.get_current_tab_mut().follow_cursor(width, height);
            }

            // Render main content

            let visible_lines = text_area.height as usize;
            let first_column = editor.get_current_tab().h_scroll as usize;

            // Colorize the content, which is done for the whole file because a
            // string can run across several lines, but only for the lines that
            // changed since the last frame. A keystroke that leaves the rest of
            // the file alone costs one line of coloring, and the key thread is
            // waiting on this lock while it happens.
            let current_tab = editor.get_current_tab();
            colorize_cache.colorize(&current_tab.content, &editor.theme);

            // Then extract the visible portion and apply cursor and selection highlighting
            let first_line = current_tab.scroll_position as usize;
            let mut visible_content: Vec<Line> =
                (first_line..first_line + visible_lines).filter_map(|index| colorize_cache.line(index)).cloned().collect();
            
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
                let has_error_line = editor.code_errors.iter().any(|error| actual_line_idx + 1 == error.code_span.start_line);
                
                for span in line.spans.iter() {
                    let text = span.content.to_string();
                    let mut span_style = span.style;
                    
                    // Apply current line background highlighting
                    if is_current_line {
                        span_style = span_style.bg(editor.theme.current_line_bg);
                    }

                    // Apply error line background highlighting (overrides current line)
                    if has_error_line {
                        span_style = span_style.bg(editor.theme.error_line_bg);
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
                                    style = style.fg(editor.theme.danger);
                                },
                                _ => {}
                            }
                            
                            // Highlight trailing whitespace in red (with bounds check)
                            if actual_line_idx < current_tab.content.len() {
                                let line_content = &current_tab.content[actual_line_idx];
                                let trimmed_len = line_content.trim_end().len();
                                if char_pos >= trimmed_len && (ch == ' ' || ch == '\t') {
                                    style = style.bg(editor.theme.danger).fg(editor.theme.on_emphasis);
                                }
                            }
                        }
                        
                        // Underline the exact error spans so the message can live at
                        // the end of the line without a caret overlay covering code
                        if has_error_line {
                            for error in &editor.code_errors {
                                if error.code_span.start_line != actual_line_idx + 1 {
                                    continue;
                                }
                                let span_start = error.code_span.start_column.saturating_sub(1);
                                let span_end = if error.code_span.end_line == error.code_span.start_line {
                                    error.code_span.end_column.saturating_sub(1).max(span_start + 1)
                                } else {
                                    usize::MAX
                                };
                                if char_pos >= span_start && char_pos < span_end {
                                    style = style.fg(editor.theme.error).add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
                                    break;
                                }
                            }
                        }

                        // Check if this character is within a search result (highlight all matches dimly)
                        let mut is_current_match = false;
                        for (match_idx, &(line, start, end)) in editor.search_results.iter().enumerate() {
                            if actual_line_idx == line && char_pos >= start && char_pos < end {
                                if match_idx == editor.current_match_index {
                                    // Current match - bright highlight
                                    style = style.bg(editor.theme.search_match_bg).fg(editor.theme.search_match_fg);
                                    is_current_match = true;
                                } else {
                                    // Other matches - dim highlight
                                    style = style.bg(editor.theme.search_other_bg).fg(editor.theme.search_other_fg);
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
                                style = style.bg(editor.theme.selection_bg).fg(editor.theme.selection_fg);
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
                                    style = style.bg(editor.theme.bracket_match_bg).fg(editor.theme.on_emphasis).add_modifier(Modifier::BOLD);
                                }
                            }
                        }
                        
                        // Apply cursor highlighting so the cursor position stands out
                        if actual_line_idx == current_tab.cursor_y && char_pos == current_tab.cursor_x {
                            style = style.fg(editor.theme.cursor_fg);
                        }
                        
                        // Everything above decides how the character looks at
                        // its real column in the file. Only the columns the
                        // view has scrolled to are drawn.
                        if char_pos >= first_column {
                            new_spans.push(Span::styled(ch.to_string(), style));
                        }
                        char_pos += 1;
                    }
                }

                // Handle case where cursor is at the end of the line
                let cursor_y_visible = current_tab.cursor_y.saturating_sub(current_tab.scroll_position as usize);
                if visible_line_idx == cursor_y_visible && char_pos == current_tab.cursor_x && current_tab.cursor_x >= first_column {
                    new_spans.push(Span::styled(" ", Style::default().fg(editor.theme.cursor_fg)));
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

            if editor.show_minimap {
                render_minimap(f, &editor, &colorize_cache, minimap_area);
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

            let cursor_column = current_tab.cursor_x.saturating_sub(first_column);
            if cursor_y < text_area.height as usize && current_tab.cursor_x >= first_column && cursor_column < text_area.width as usize {
                let cursor_pos = Position {
                    x: text_area.x + cursor_column as u16,
                    y: text_area.y + cursor_y as u16,
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

            // End-of-line annotations: function timings from the last run,
            // then errors, and an error always wins its line
            if editor.profile_data.is_some() || !editor.profile_dumps.is_empty() {
                let mut hasher = DefaultHasher::new();
                editor.get_current_tab().content.hash(&mut hasher);
                let content_hash = hasher.finish();
                let cache_outdated = profile_line_cache.as_ref().map_or(true, |(cached_hash, _, _)| *cached_hash != content_hash);
                if cache_outdated {
                    let current_tab = editor.get_current_tab();
                    let source = current_tab.content.join("\n");
                    profile_line_cache = Some((content_hash, nail::prof::source_fingerprint(&source), function_declaration_lines(&current_tab.content)));
                }
            }
            let annotations = build_line_annotations(&editor, profile_line_cache.as_ref());
            if !annotations.is_empty() {
                display_line_annotations(f, &editor, content_area, &annotations);
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

            // A requested screen copy is answered here, after every overlay
            // and dialog has painted, so what lands on the clipboard is
            // exactly what the user is looking at.
            if editor.screen_copy_requested {
                let text = buffer_text(f.buffer_mut());
                editor.finish_screen_copy(&text);
            }
        });

        match result_draw {
            Ok(_) => {}
            Err(err) => log::error!("Error drawing terminal: {:?}", err),
        }
    }
}

/// A painted frame read back as plain text, which is what makes everything
/// the IDE displays copyable: overlays and popups live nowhere else. Styling
/// is dropped, right-hand padding is trimmed, and empty rows at the bottom
/// go, so what is pasted reads like a screenshot in text.
pub fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    let area = buffer.area;
    let mut lines: Vec<String> = Vec::new();
    for y in area.top()..area.bottom() {
        let mut line = String::new();
        for x in area.left()..area.right() {
            line.push_str(buffer[(x, y)].symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    return lines.join("\n");
}

/// How many characters of a line one braille dot column stands for. Fifteen
/// cells of two dot columns at four characters each cover the first 120
/// columns of the file, which is enough to give every line a recognisable
/// shape.
const MINIMAP_CHARS_PER_DOT: usize = 4;

/// The bit each dot of a braille cell occupies in its code point, indexed by
/// dot column and then dot row from the top. Braille grew its bottom two dots
/// after the original six, which is why the last row's bits are out of
/// sequence with the rest.
const BRAILLE_DOTS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// A color leaned a third of the way toward the theme background, used for
/// the minimap rows not on screen. The rows that are on screen keep their
/// full colors, which is what makes the lit band findable at a glance.
fn toward_background(color: Color, background: Color) -> Color {
    match (color, background) {
        (Color::Rgb(red, green, blue), Color::Rgb(back_red, back_green, back_blue)) => {
            let mix = |channel: u8, back: u8| -> u8 { ((channel as u16 * 2 + back as u16) / 3) as u8 };
            return Color::Rgb(mix(red, back_red), mix(green, back_green), mix(blue, back_blue));
        }
        _ => return color,
    }
}

/// The file in miniature: each braille cell condenses a slice of the file
/// into a two by four grid of dots, with a dot wherever those lines have
/// text, painted in the same colors the syntax highlighter gives that text,
/// so a wall of comments, a string block and a run of keywords each look
/// like themselves. Dots rather than solid blocks so the map reads as faint
/// small print instead of slabs of ink. The rows showing what is on screen
/// sit on the current-line grey at full strength while the rest lean toward
/// the background, so the band doubles as a scrollbar you can read.
fn render_minimap(f: &mut Frame, editor: &Editor, colors: &ColorizeCache, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let tab = editor.get_current_tab();
    let lines_per_row = crate::minimap_lines_per_row(tab.content.len(), area.height);
    let lines_per_dot_row = lines_per_row / 4;
    let width = area.width as usize;
    let dot_columns = width * 2;

    let view_top = tab.scroll_position as usize;
    let view_bottom = view_top + (editor.view.text.height as usize).max(1);

    let mut rows: Vec<Line> = Vec::with_capacity(area.height as usize);
    for row in 0..area.height as usize {
        let first_line = row * lines_per_row;

        // The dots one row of cells lights up, and a tally of the token
        // colors behind each cell, of which the commonest becomes the cell's
        // color. Each source line is walked once, so the whole map costs one
        // pass over the file per frame.
        let mut dots: Vec<u8> = vec![0; width];
        let mut tallies: Vec<Vec<(Color, u32)>> = vec![Vec::new(); width];
        for dot_row in 0..4 {
            let start = first_line + dot_row * lines_per_dot_row;
            for index in start..start + lines_per_dot_row {
                let Some(line) = colors.line(index) else { break };
                let mut position = 0;
                'line_done: for span in line.spans.iter() {
                    let color = span.style.fg.unwrap_or(editor.theme.default);
                    for character in span.content.chars() {
                        if position >= dot_columns * MINIMAP_CHARS_PER_DOT {
                            break 'line_done;
                        }
                        if !character.is_whitespace() {
                            let dot_column = position / MINIMAP_CHARS_PER_DOT;
                            let cell = dot_column / 2;
                            dots[cell] |= BRAILLE_DOTS[dot_column % 2][dot_row];
                            match tallies[cell].iter_mut().find(|(seen, _)| *seen == color) {
                                Some((_, count)) => *count += 1,
                                None => tallies[cell].push((color, 1)),
                            }
                        }
                        position += 1;
                    }
                }
            }
        }

        let on_screen = first_line < view_bottom && first_line + lines_per_row > view_top;
        let row_background = if on_screen { editor.theme.current_line_bg } else { editor.theme.background };
        let mut cells: Vec<Span> = Vec::with_capacity(width);
        for column in 0..width {
            let commonest = tallies[column].iter().max_by_key(|(_, count)| *count).map(|(color, _)| *color);
            let (glyph, foreground) = match commonest {
                None => (' ', row_background),
                Some(color) => {
                    let braille = char::from_u32(0x2800 + dots[column] as u32).expect("every braille code point is assigned");
                    (braille, if on_screen { color } else { toward_background(color, editor.theme.background) })
                }
            };
            cells.push(Span::styled(glyph.to_string(), Style::default().fg(foreground).bg(row_background)));
        }
        rows.push(Line::from(cells));
    }
    f.render_widget(Paragraph::new(rows), area);
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
    let modified_indicator = if current_tab.disk_changed_underneath {
        " [*] file changed on disk. F9 takes the disk copy, Ctrl+S keeps yours "
    } else if current_tab.modified {
        " [*] "
    } else {
        " "
    };
    
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
            let chars_in_first_line = current_tab.content[start_pos.1].chars().count().saturating_sub(start_pos.0);
            let chars_in_last_line = end_pos.0;
            let chars_in_middle_lines: usize = if lines > 2 {
                current_tab.content[(start_pos.1 + 1)..end_pos.1]
                    .iter()
                    .map(|line| line.chars().count() + 1) // +1 for newline
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
    // Only shown when off, because a mouse that works is what everyone
    // assumes, and a mouse that has been handed back to the terminal is the
    // thing worth saying out loud.
    if !editor.mouse_enabled { visual_features.push("no mouse"); }
    let features_info = if visual_features.is_empty() {
        String::new()
    } else {
        format!(" [{}] ", visual_features.join(","))
    };
    
    // Tab info
    let tab_info = format!(" Tab {}/{} ", editor.tab_index + 1, editor.tabs.len());
    
    // One hint, for the key that lists every other key by name
    let shortcuts = " Ctrl+P: commands | Ctrl+R: symbols | F8: next error | F4: mouse ";
    
    // Create spans for different parts
    let mut spans = vec![
        Span::styled(file_info, Style::default().fg(editor.theme.info).bg(editor.theme.ui_panel_bg)),
        Span::styled(modified_indicator, Style::default().fg(editor.theme.danger).bg(editor.theme.ui_panel_bg)),
        Span::styled(cursor_info, Style::default().fg(editor.theme.success).bg(editor.theme.ui_panel_bg)),
        Span::styled(line_count, Style::default().fg(editor.theme.accent).bg(editor.theme.ui_panel_bg)),
        Span::styled(size_info, Style::default().fg(editor.theme.primary).bg(editor.theme.ui_panel_bg)),
        Span::styled(tab_info, Style::default().fg(editor.theme.special).bg(editor.theme.ui_panel_bg)),
    ];
    
    // Add selection info if there's a selection
    if !selection_info.is_empty() {
        spans.push(Span::styled(selection_info, Style::default().fg(editor.theme.info_bright).bg(editor.theme.ui_panel_bg)));
    }
    
    // Add visual features info if any are enabled
    if !features_info.is_empty() {
        spans.push(Span::styled(features_info, Style::default().fg(editor.theme.success_bright).bg(editor.theme.ui_panel_bg)));
    }
    
    // Which bindings are in force, when they are not the ones a user would
    // assume. Pushed before the width is measured, so the right flush below
    // still counts it.
    if let Some(label) = editor.keymap.label(editor.vim_mode) {
        spans.push(Span::styled(label, Style::default().fg(editor.theme.badge_fg).bg(editor.theme.badge_bg).add_modifier(Modifier::BOLD)));
    }

    // Add padding to push shortcuts to the right
    let current_width: usize = spans.iter().map(|s| s.content.len()).sum();
    let shortcuts_width = shortcuts.len();
    let total_available = area.width as usize;
    
    if current_width + shortcuts_width < total_available {
        let padding_needed = total_available - current_width - shortcuts_width;
        spans.push(Span::styled(" ".repeat(padding_needed), Style::default().bg(editor.theme.ui_panel_bg)));
    }
    
    spans.push(Span::styled(shortcuts, Style::default().fg(editor.theme.ui_hint).bg(editor.theme.ui_panel_bg)));
    
    let status_line = Line::from(spans);
    let status_paragraph = Paragraph::new(vec![status_line])
        .style(Style::default().bg(editor.theme.ui_panel_bg));
    
    f.render_widget(status_paragraph, area);
}

/// Compiling is the only build step slow enough to be worth a progress
/// reading. Cargo says nothing until it is finished, so the reading is time
/// spent so far measured against the last build of the same kind. It stops at
/// 99% because a build that beats its estimate is still not done.
fn compiling_label(editor: &Editor) -> String {
    let elapsed = match editor.compile_started {
        Some(started) => started.elapsed(),
        None => return "Compiling".to_string(),
    };
    match editor.compile_estimate {
        Some(estimate) if estimate.as_secs_f64() > 0.0 => {
            let percent = (elapsed.as_secs_f64() / estimate.as_secs_f64() * 100.0).min(99.0);
            format!("Compiling {:.0}%", percent)
        }
        // No comparable build on record yet. This one counts up in seconds and
        // becomes the estimate the next one is measured against.
        _ => format!("Compiling {:.0}s", elapsed.as_secs_f64()),
    }
}

fn display_build_status(f: &mut Frame, editor: &Editor) {
    let status_text = match &editor.build_status {
        BuildStatus::Idle => "Ready".to_string(),
        BuildStatus::Parsing => "Starting".to_string(),
        BuildStatus::Transpiling => "Transpiling".to_string(),
        BuildStatus::Compiling => compiling_label(editor),
        BuildStatus::Complete(message) => message.clone(),
        BuildStatus::Failed(err) => err.clone(),
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

/// What an end-of-line overlay annotation means, which decides its color.
/// Errors keep the error color, timings render dim like a comment, and stale
/// timings dim further because they describe an older build. Red always
/// means error in this IDE, so timings never use it, and neither do the
/// bottom-row notices that report a copy or a load went fine.
enum LineAnnotationKind {
    Error,
    Timing,
    TimingStale,
    Notice,
}

/// One overlay rendered after the end of a line's code. Each line carries at
/// most one, and an error always wins the line over a timing annotation.
struct LineAnnotation {
    text: String,
    kind: LineAnnotationKind,
}

/// Assembles every end-of-line annotation keyed by 1-based line number, with
/// 0 meaning a status notice that renders on the bottom row. Timing entries
/// go in first so any error on the same line overwrites them.
fn build_line_annotations(editor: &Editor, profile_cache: Option<&(u64, String, HashMap<String, usize>)>) -> BTreeMap<usize, LineAnnotation> {
    let mut annotations: BTreeMap<usize, LineAnnotation> = BTreeMap::new();

    // The dump matching the open buffer's fingerprint wins even if another
    // program wrote the dump file more recently. Only when no dump ever
    // matched does the latest one show, dimmed as stale.
    let chosen = if editor.show_timings {
        profile_cache
            .and_then(|(_, fingerprint, _)| editor.profile_dumps.get(fingerprint))
            .or(editor.profile_data.as_ref())
    } else {
        None
    };
    if let (Some(profile), Some((_, fingerprint, decl_lines))) = (chosen, profile_cache) {
        let stale = *fingerprint != profile.source_hash;
        for function in &profile.functions {
            if function.calls == 0 {
                continue;
            }
            let Some(line_idx) = decl_lines.get(&function.name) else { continue };
            let kind = if stale { LineAnnotationKind::TimingStale } else { LineAnnotationKind::Timing };
            annotations.insert(line_idx + 1, LineAnnotation { text: format_timing_annotation(function, profile.wall_nanos, stale), kind });
        }
    }

    // Errors sharing a line are joined so they never overdraw each other
    let mut messages_by_line: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for error in &editor.code_errors {
        messages_by_line.entry(error.code_span.start_line).or_default().push(error.message.as_str());
    }
    for (start_line, messages) in messages_by_line {
        if start_line == 0 {
            // A receipt says a copy or a load went fine. No arrow, because
            // it points at no line, and no red, because red means error.
            annotations.insert(0, LineAnnotation { text: messages.join(" | "), kind: LineAnnotationKind::Notice });
        } else {
            annotations.insert(start_line, LineAnnotation { text: format!("◀ {}", messages.join(" | ")), kind: LineAnnotationKind::Error });
        }
    }

    annotations
}

/// The display text of every annotation on the given 1-based line range,
/// keyed by line and kept exactly as painted, arrow and all, because the
/// copies weave each one back onto the end of its own line the way the
/// screen shows it. Rebuilt from the same sources the draw thread reads.
pub fn line_annotation_texts(editor: &Editor, first_line: usize, last_line: usize) -> BTreeMap<usize, String> {
    let current_tab = editor.get_current_tab();
    let source = current_tab.content.join("\n");
    let cache = (0u64, nail::prof::source_fingerprint(&source), function_declaration_lines(&current_tab.content));
    let annotations = build_line_annotations(editor, Some(&cache));
    return annotations.range(first_line..=last_line).map(|(line, annotation)| (*line, annotation.text.clone())).collect();
}

fn display_line_annotations(f: &mut Frame, editor: &Editor, content_area: Rect, annotations: &BTreeMap<usize, LineAnnotation>) {
    let current_tab = editor.get_current_tab();
    let scroll = current_tab.scroll_position as usize;
    // Content rows sit inside the block border: row n renders at content_area.y + 1 + n
    let visible_rows = content_area.height.saturating_sub(2) as usize;
    if visible_rows == 0 || content_area.width <= 2 {
        return;
    }
    let right_edge = content_area.x + content_area.width - 1;

    for (start_line, annotation) in annotations {
        // Messages without a code span (status notices like "Loaded: ...") go to the
        // bottom row of the content area instead of masquerading as a line-1 error
        let (row_y, msg_x) = if *start_line == 0 {
            (content_area.y + visible_rows as u16, content_area.x + 1)
        } else {
            let line_0based = start_line - 1;
            if line_0based < scroll || line_0based >= scroll + visible_rows {
                continue; // Annotated line is scrolled out of view
            }
            // Place the message after the end of the line's code so it never
            // covers it, counting from wherever the view has scrolled across to
            let line_len = current_tab.content.get(line_0based).map(|l| l.chars().count()).unwrap_or(0) as u16;
            let line_end = line_len.saturating_sub(current_tab.h_scroll);
            (content_area.y + 1 + (line_0based - scroll) as u16, content_area.x + 1 + line_end + 2)
        };

        if msg_x >= right_edge {
            continue;
        }
        let avail = (right_edge - msg_x) as usize;
        let mut text = annotation.text.clone();
        if text.chars().count() > avail {
            text = text.chars().take(avail.saturating_sub(1)).collect();
            text.push('…');
        }

        let overlay_area = Rect::new(msg_x, row_y, text.chars().count() as u16, 1).intersection(f.area());
        if overlay_area.width == 0 || overlay_area.height == 0 {
            continue;
        }

        let style = match annotation.kind {
            LineAnnotationKind::Error => Style::default().fg(editor.theme.error).bg(editor.theme.background),
            LineAnnotationKind::Timing => Style::default().fg(editor.theme.comment).bg(editor.theme.background),
            LineAnnotationKind::TimingStale => Style::default().fg(editor.theme.comment).bg(editor.theme.background).add_modifier(Modifier::DIM),
            LineAnnotationKind::Notice => Style::default().fg(editor.theme.success).bg(editor.theme.background),
        };
        let paragraph = Paragraph::new(Line::from(Span::styled(text, style)));
        f.render_widget(Clear, overlay_area);
        f.render_widget(paragraph, overlay_area);
    }
}

/// Full stats for one function, rendered at the end of its declaration line.
/// A stale suffix replaces the max when the dump predates the buffer's code.
fn format_timing_annotation(function: &ProfiledFunction, wall_nanos: u64, stale: bool) -> String {
    let percent = if wall_nanos > 0 { function.total_nanos as f64 / wall_nanos as f64 * 100.0 } else { 0.0 };
    let avg_nanos = function.total_nanos / function.calls.max(1);
    let tail = if stale { "stale".to_string() } else { format!("{} max", format_nanos(function.max_nanos)) };
    format!("◀ {} total ({:.1}%)  {} avg × {}  {}", format_nanos(function.total_nanos), percent, format_nanos(avg_nanos), function.calls, tail)
}

/// Adaptive duration formatting, same units and precision as the timing
/// sheet a profiled program prints at exit.
fn format_nanos(nanos: u64) -> String {
    if nanos < 1_000 {
        format!("{}ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.1}µs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.1}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", nanos as f64 / 1_000_000_000.0)
    }
}

fn format_millis(elapsed: Duration) -> String {
    format!("{:.1}ms", elapsed.as_secs_f64() * 1000.0)
}

/// Maps each function name to the 0-based line of its `f name(...)`
/// declaration. A plain text scan so annotations survive parse errors, and
/// function names are globally unique in Nail so one line per name is right.
fn function_declaration_lines(content: &[String]) -> HashMap<String, usize> {
    let mut decl_lines = HashMap::new();
    for (idx, line) in content.iter().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("f ") else { continue };
        let rest = rest.trim_start();
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if name.is_empty() {
            continue;
        }
        let after_name: String = rest.chars().skip(name.chars().count()).collect();
        if after_name.trim_start().starts_with('(') {
            decl_lines.entry(name).or_insert(idx);
        }
    }
    decl_lines
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
        Span::styled("Function: ", Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(&selected.label, Style::default().fg(editor.theme.ui_text).add_modifier(Modifier::BOLD)),
    ]));
    
    lines.push(Line::from(""));
    
    // Signature
    lines.push(Line::from(vec![
        Span::styled("Signature: ", Style::default().fg(editor.theme.info)),
        Span::styled(&selected.detail, Style::default().fg(editor.theme.ui_text)),
    ]));
    
    lines.push(Line::from(""));
    
    // Description
    if !selected.description.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Description:", Style::default().fg(editor.theme.success).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(&selected.description, Style::default().fg(editor.theme.ui_text)),
        ]));
        lines.push(Line::from(""));
    }
    
    // Two forms of the same example, because there are two things a person
    // can be short of: the call, for someone who already has the inputs, and
    // the whole runnable program, for someone meeting the function today.
    // Both are shown a line at a time with wrapping off, since an example
    // that reflows is no longer the thing that would be pasted.
    if !selected.example.is_empty() {
        let call = crate::stdlib_registry::example_snippet(&selected.label, &selected.example);

        lines.push(Line::from(vec![
            Span::styled("Example ", Style::default().fg(editor.theme.special).add_modifier(Modifier::BOLD)),
            Span::styled("(TAB to insert)", Style::default().fg(editor.theme.accent)),
        ]));
        lines.push(Line::from(vec![Span::styled(call.to_string(), Style::default().fg(editor.theme.ui_text_muted))]));
        lines.push(Line::from(""));

        if selected.example.trim() != call {
            lines.push(Line::from(vec![
                Span::styled("Full example ", Style::default().fg(editor.theme.special).add_modifier(Modifier::BOLD)),
                Span::styled("(SHIFT + TAB to insert)", Style::default().fg(editor.theme.accent)),
            ]));
            for example_line in selected.example.lines() {
                lines.push(Line::from(vec![
                    Span::styled(example_line.to_string(), Style::default().fg(editor.theme.ui_text_muted)),
                ]));
            }
            lines.push(Line::from(""));
        }
    }

    // Help text
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("ESC", Style::default().fg(editor.theme.accent)),
        Span::styled(" back  ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("TAB", Style::default().fg(editor.theme.accent)),
        Span::styled(" to insert the example  ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("SHIFT + TAB", Style::default().fg(editor.theme.accent)),
        Span::styled(" to insert the full example", Style::default().fg(editor.theme.ui_hint)),
    ]));

    // Calculate popup size
    let width = lines.iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(40)
        .max(50)
        .min(100)
        .min(content_area.width.saturating_sub(2) as usize) as u16;

    // A line longer than the box takes more than one row, and the description
    // is usually two or three sentences. Counting lines rather than rows made
    // the box too short by exactly that difference, which cut off the footer
    // saying what TAB does.
    let inner_width = width.saturating_sub(2).max(1) as usize;
    let rendered_rows: usize = lines.iter().map(|line| line.width().div_ceil(inner_width).max(1)).sum();
    let height = (rendered_rows + 2).min(content_area.height.saturating_sub(2).max(3) as usize) as u16; // +2 for borders
    
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
            .border_style(Style::default().fg(editor.theme.accent))
            .title(" Documentation (F1 to toggle) ")
            .title_style(Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(editor.theme.background))
        .wrap(Wrap { trim: false });
    
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
    // Counted from the first column on screen, so the list still points at the
    // word when the view has scrolled sideways
    let popup_x = content_area.x + (word_start as u16).saturating_sub(current_tab.h_scroll) + 1;
    
    // Limit completions shown
    let max_items = 10;
    let items_to_show = editor.completions.len().min(max_items);
    // The ten drawn are the ten around the picked one, not the first ten in
    // the list. Arrowing down past the tenth used to leave nothing
    // highlighted and the same ten rows on screen, so the eleventh match was
    // reachable but invisible.
    let first_shown = editor.completion_index.saturating_sub(items_to_show.saturating_sub(1));

    // Create list items with highlighting for selected item
    let items: Vec<ListItem> = editor.completions
        .iter()
        .skip(first_shown)
        .take(items_to_show)
        .enumerate()
        .map(|(offset, item)| {
            let i = first_shown + offset;
            let icon = match item.kind {
                CompletionKind::Function => "ƒ ",
                CompletionKind::Variable => "v ",
                CompletionKind::Struct => "s ",
                CompletionKind::Enum => "e ",
                CompletionKind::Keyword => "k ",
            };
            
            let content = if i == editor.completion_index {
                Line::from(vec![
                    Span::styled(icon, Style::default().fg(editor.theme.accent)),
                    Span::styled(&item.label, Style::default().fg(editor.theme.on_emphasis).bg(editor.theme.item_selection_bg)),
                    Span::raw(" "),
                    Span::styled(&item.detail, Style::default().fg(editor.theme.ui_text_muted)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(icon, Style::default().fg(editor.theme.ui_hint)),
                    Span::styled(&item.label, Style::default().fg(editor.theme.default)),
                    Span::raw(" "),
                    Span::styled(&item.detail, Style::default().fg(editor.theme.ui_hint)),
                ])
            };
            ListItem::new(content)
        })
        .collect();
    
    // Calculate popup width based on longest item - make it wider to show full signatures
    let max_width = editor.completions
        .iter()
        .skip(first_shown)
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
    
    // Say what the keys do, longest version that fits: a title wider than the
    // popup is cut off mid-word, which teaches nothing.
    let title = if !has_docs {
        " Completions "
    } else {
        [
            " Completions (tab to complete, shift + tab to insert full example, F1 docs) ",
            " Completions (tab to complete, shift + tab for full example) ",
            " Completions (tab to complete) ",
        ]
        .into_iter()
        .find(|hint| hint.len() <= popup_area.width as usize)
        .unwrap_or(" Completions ")
    };
    
    // Create and render the list
    let completions_list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(editor.theme.operator))
            .title(title)
            .title_style(if has_docs {
                Style::default().fg(editor.theme.accent)
            } else {
                Style::default().fg(editor.theme.operator)
            }))
        .style(Style::default().bg(editor.theme.background));

    f.render_widget(completions_list, popup_area);
    let list_area = Rect::new(popup_area.x + 1, popup_area.y + 1, popup_area.width.saturating_sub(2), items_to_show as u16);
    display_list_scrollbar(f, editor.theme, popup_area, list_area, editor.completions.len(), first_shown);
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
        DialogMode::Settings => {
            display_settings_dialog(f, editor);
        }
        DialogMode::ConfirmQuit => {
            display_confirm_quit_dialog(f, editor);
        }
        DialogMode::CommandPalette => {
            display_palette_dialog(f, editor);
        }
        DialogMode::SymbolPicker => {
            display_symbol_dialog(f, editor);
        }
        DialogMode::None => {
            // No dialog to display
        }
    }
}

/// Asks before Escape throws the session away. It is small and says only the
/// one thing it needs to, because it appears in front of someone who was in
/// the middle of pressing Escape for a different reason.
fn display_confirm_quit_dialog(f: &mut Frame, editor: &Editor) {
    let unsaved = editor.has_unsaved_work();

    let mut lines = vec![Line::from(vec![Span::styled(
        "Quit?",
        Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD),
    )])];
    lines.push(Line::from(""));
    if unsaved {
        lines.push(Line::from(vec![Span::styled(
            "You have changes that are not saved.",
            Style::default().fg(editor.theme.danger).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![
        Span::styled("ESC", Style::default().fg(editor.theme.danger).add_modifier(Modifier::BOLD)),
        Span::styled(" again to quit", Style::default().fg(editor.theme.ui_text)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("any other key", Style::default().fg(editor.theme.success)),
        Span::styled(" to stay", Style::default().fg(editor.theme.ui_text)),
    ]));

    let width = 42;
    let height = lines.len() + 2;
    let popup_x = (f.area().width.saturating_sub(width)) / 2;
    let popup_y = (f.area().height.saturating_sub(height as u16)) / 2;
    let dialog_area = Rect::new(popup_x, popup_y, width, height as u16);

    f.render_widget(Clear, dialog_area);
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if unsaved { editor.theme.danger } else { editor.theme.accent }))
                    .title(" Quit ")
                    .title_style(Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)),
            )
            .style(Style::default().bg(editor.theme.background)),
        dialog_area,
    );
}

/// The settings screen. It is the first thing a new user sees, because the
/// editor opens it once when nobody has ever chosen a keymap, so it has to
/// explain itself without anywhere to put a manual.
fn display_settings_dialog(f: &mut Frame, editor: &Editor) {
    let rows = editor.settings_rows();

    let mut lines = vec![Line::from(vec![Span::styled(
        "Settings",
        Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD),
    )])];
    lines.push(Line::from(""));

    let label_width = rows.iter().map(|(label, _)| label.len()).max().unwrap_or(0);
    for (index, (label, value)) in rows.iter().enumerate() {
        let selected = index == editor.settings_row;
        let marker = if selected { "> " } else { "  " };
        let value_style = if selected {
            Style::default().fg(editor.theme.badge_fg).bg(editor.theme.badge_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(editor.theme.ui_text)
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(editor.theme.accent)),
            Span::styled(format!("{:width$}  ", label, width = label_width), Style::default().fg(editor.theme.ui_text_muted)),
            Span::styled(format!(" {} ", value), value_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("UP", Style::default().fg(editor.theme.success)),
        Span::styled(" and ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("DOWN", Style::default().fg(editor.theme.success)),
        Span::styled(" to choose, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("LEFT", Style::default().fg(editor.theme.success)),
        Span::styled(" and ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("RIGHT", Style::default().fg(editor.theme.success)),
        Span::styled(" to change", Style::default().fg(editor.theme.ui_hint)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("ESC", Style::default().fg(editor.theme.danger)),
        Span::styled(" saves and closes", Style::default().fg(editor.theme.ui_hint)),
    ]));

    let width = 52;
    let height = lines.len() + 2;
    let popup_x = (f.area().width.saturating_sub(width)) / 2;
    let popup_y = (f.area().height.saturating_sub(height as u16)) / 2;
    let dialog_area = Rect::new(popup_x, popup_y, width, height as u16);

    f.render_widget(Clear, dialog_area);

    let dialog_paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(editor.theme.accent))
                .title(" Settings (F2) ")
                .title_style(Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)),
        )
        .style(Style::default().bg(editor.theme.background));

    f.render_widget(dialog_paragraph, dialog_area);
}

fn display_goto_line_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::Wrap;
    
    let current_line = editor.get_current_line_number();
    let total_lines = editor.get_total_lines();
    
    // Build dialog content
    let mut lines = vec![];
    
    // Title
    lines.push(Line::from(vec![
        Span::styled("Go to Line", Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    
    // Input field
    lines.push(Line::from(vec![
        Span::styled("Line number: ", Style::default().fg(editor.theme.ui_text)),
        Span::styled(&editor.goto_line_input, Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)),
        Span::styled("_", Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)), // Cursor
    ]));
    lines.push(Line::from(""));
    
    // Info
    lines.push(Line::from(vec![
        Span::styled(format!("Current: {} / {}", current_line, total_lines), Style::default().fg(editor.theme.ui_text_muted)),
    ]));
    lines.push(Line::from(""));
    
    // Help text
    lines.push(Line::from(vec![
        Span::styled("Press ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("ENTER", Style::default().fg(editor.theme.success)),
        Span::styled(" to go, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("ESC", Style::default().fg(editor.theme.danger)),
        Span::styled(" to cancel", Style::default().fg(editor.theme.ui_hint)),
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
            .border_style(Style::default().fg(editor.theme.accent))
            .title(" Go to Line (Ctrl+G) ")
            .title_style(Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(editor.theme.background))
        .wrap(Wrap { trim: true });
    
    f.render_widget(dialog_paragraph, dialog_area);
}

/// The three switches both search dialogs carry, drawn so that which ones are
/// on can be read at a glance rather than worked out from the results.
fn search_switches(editor: &Editor) -> Vec<Span<'static>> {
    let switches = [("case", editor.case_sensitive), ("word", editor.whole_word), ("regex", editor.use_regex)];
    let mut spans = Vec::new();
    for (label, is_on) in switches {
        let style = if is_on { Style::default().fg(editor.theme.toggle_on_fg).bg(editor.theme.toggle_on_bg) } else { Style::default().fg(editor.theme.ui_hint) };
        spans.push(Span::styled(format!(" {} ", label), style));
        spans.push(Span::raw(" "));
    }
    return spans;
}

/// The mark on the edge of a list that says it runs past its window.
///
/// A list that scrolls with nothing to show for it reads as the whole answer,
/// and the rows below the fold never get looked for. The bar is drawn down
/// the box's right border rather than in a column of its own, so the rows
/// keep every character they had, and its track is the same line the border
/// was already drawing. It appears only when there is more than fits.
///
/// `rows` is how many there are in total, `offset` is the first one on
/// screen, and `list_area` is where the rows themselves were drawn: the bar
/// runs level with them and stops where they stop.
fn display_list_scrollbar(f: &mut Frame, theme: &crate::colorizer::ColorScheme, box_area: Rect, list_area: Rect, rows: usize, offset: usize) {
    let visible = list_area.height as usize;
    if visible == 0 || rows <= visible {
        return;
    }

    let bar_area = Rect::new(box_area.x + box_area.width.saturating_sub(1), list_area.y, 1, list_area.height);
    let bar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .symbols(ratatui::symbols::scrollbar::VERTICAL)
        .begin_symbol(None)
        .end_symbol(None)
        .track_style(Style::default().fg(theme.scroll_track))
        .thumb_style(Style::default().fg(theme.scroll_thumb));
    let mut state = ScrollbarState::new(rows).position(offset).viewport_content_length(visible);

    f.render_stateful_widget(bar, bar_area, &mut state);
}

/// The fuzzy list behind both the command palette and go to symbol: what has
/// been typed on top, the matches under it, one of them picked. They share a
/// drawing because they are the same thing pointed at different contents, and
/// learning one should be learning both.
fn display_picker(f: &mut Frame, editor: &Editor, title: &str, filter: &str, rows: &[(String, String)], selected: usize) {
    use ratatui::widgets::{Clear, List, ListItem, ListState};

    let area = f.area();
    let width = std::cmp::min(70, area.width.saturating_sub(4));
    let height = std::cmp::min(18, area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(0)]).split(Rect::new(dialog_area.x + 1, dialog_area.y + 1, dialog_area.width.saturating_sub(2), dialog_area.height.saturating_sub(2)));

    let block = Block::default().borders(Borders::ALL).title(title.to_string()).title_style(Style::default().fg(editor.theme.success).add_modifier(Modifier::BOLD)).style(Style::default().bg(editor.theme.background));
    f.render_widget(block, dialog_area);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(editor.theme.success)),
        Span::styled(filter.to_string(), Style::default().fg(editor.theme.ui_text)),
        Span::styled("_", Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)),
    ]));
    f.render_widget(prompt, chunks[0]);

    // Hints sit at the right edge, so the names on the left line up and can
    // be read as a column. The hint keeps its room and the label is cut to
    // what is left: a search result whose line of code ran to the edge would
    // otherwise push out the one thing saying which file it is in.
    let inner_width = chunks[1].width as usize;
    let items: Vec<ListItem> = rows
        .iter()
        .map(|(label, hint)| {
            let room = inner_width.saturating_sub(hint.chars().count() + 1);
            let label = if label.chars().count() > room { format!("{}…", label.chars().take(room.saturating_sub(1)).collect::<String>()) } else { label.clone() };
            let padding = inner_width.saturating_sub(label.chars().count() + hint.chars().count());
            ListItem::new(Line::from(vec![
                Span::styled(label, Style::default().fg(editor.theme.ui_text)),
                Span::raw(" ".repeat(padding)),
                Span::styled(hint.clone(), Style::default().fg(editor.theme.ui_hint)),
            ]))
        })
        .collect();

    let list = List::new(items).highlight_style(Style::default().fg(editor.theme.menu_selection_fg).bg(editor.theme.menu_selection_bg));
    let mut list_state = ListState::default();
    if !rows.is_empty() {
        list_state.select(Some(selected.min(rows.len() - 1)));
    }
    f.render_stateful_widget(list, chunks[1], &mut list_state);
    // Asked after the list has drawn, because the list is the one that
    // decided how far down it had to scroll to keep the picked row in sight.
    display_list_scrollbar(f, editor.theme, dialog_area, chunks[1], rows.len(), list_state.offset());
}

fn display_palette_dialog(f: &mut Frame, editor: &Editor) {
    let rows: Vec<(String, String)> = editor
        .palette_matches
        .iter()
        .filter_map(|index| nail::keymap::COMMANDS.get(*index))
        // Which keys a command answers to depends on which keymap is in force,
        // and a hint naming the wrong one is worse than no hint at all.
        .map(|command| (command.name.to_string(), command.keys.for_keymap(editor.keymap).to_string()))
        .collect();
    // The palette names its own key too, and that key is not the same one in
    // every keymap either.
    let title = match editor.keymap {
        nail::keymap::Keymap::Cua => " Commands (Ctrl+P) ",
        nail::keymap::Keymap::Vim => " Commands (:) ",
        nail::keymap::Keymap::Emacs => " Commands (Alt+X) ",
    };
    display_picker(f, editor, title, &editor.palette_filter, &rows, editor.palette_index);
}

fn display_symbol_dialog(f: &mut Frame, editor: &Editor) {
    let rows: Vec<(String, String)> = editor
        .symbol_matches
        .iter()
        .filter_map(|index| editor.symbol_entries.get(*index))
        .map(|symbol| {
            let where_from = match &symbol.file {
                Some(file) => format!("{}:{}", file, symbol.line),
                None => format!("line {}", symbol.line),
            };
            (symbol.label.clone(), where_from)
        })
        .collect();
    // One picker serves three lists, and the title is what says which, since
    // their rows look alike. A search that stopped at its limit says so:
    // a list that ends at a round number and keeps quiet reads as the whole
    // answer when it is not.
    let title = match editor.symbol_source {
        crate::SymbolSource::OpenFile => " Go to symbol (Ctrl+R) ".to_string(),
        crate::SymbolSource::Project => " Go to symbol in project (Ctrl+T) ".to_string(),
        crate::SymbolSource::ProjectText if rows.len() >= crate::Editor::PROJECT_SEARCH_LIMIT => {
            format!(" Search the project (Ctrl+E) - first {} ", rows.len())
        }
        crate::SymbolSource::ProjectText => format!(" Search the project (Ctrl+E) - {} ", rows.len()),
    };
    display_picker(f, editor, &title, &editor.symbol_filter, &rows, editor.symbol_index);
}

fn display_find_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::{Wrap, Clear};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Style, Modifier};
    use ratatui::layout::Rect;
    use ratatui::widgets::{Block, Borders, Paragraph};
    
    let search_status = editor.search_status_line();

    // Build dialog content
    let mut lines = vec![];

    // Title
    lines.push(Line::from(vec![
        Span::styled("Find", Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    // Search input field
    lines.push(Line::from(vec![
        Span::styled("Find: ", Style::default().fg(editor.theme.ui_text)),
        Span::styled(&editor.search_query, Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)),
        Span::styled("_", Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)), // Cursor
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(search_switches(editor)));

    // Search results
    if !search_status.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(search_status, Style::default().fg(editor.theme.success)),
        ]));
    }
    lines.push(Line::from(""));

    // Help text
    lines.push(Line::from(vec![
        Span::styled("ENTER", Style::default().fg(editor.theme.success)),
        Span::styled(": next, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("F3", Style::default().fg(editor.theme.success)),
        Span::styled(": next, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("Shift+F3", Style::default().fg(editor.theme.success)),
        Span::styled(": prev", Style::default().fg(editor.theme.ui_hint)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Alt+C", Style::default().fg(editor.theme.success)),
        Span::styled(": case, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("Alt+W", Style::default().fg(editor.theme.success)),
        Span::styled(": word, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("Alt+R", Style::default().fg(editor.theme.success)),
        Span::styled(": regex, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("ESC", Style::default().fg(editor.theme.danger)),
        Span::styled(": close", Style::default().fg(editor.theme.ui_hint)),
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
            .border_style(Style::default().fg(editor.theme.accent))
            .title(" Find (Ctrl+F) ")
            .title_style(Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(editor.theme.background))
        .wrap(Wrap { trim: true });
    
    f.render_widget(dialog_paragraph, dialog_area);
}

fn display_replace_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::{Wrap, Clear};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Style, Modifier};
    use ratatui::layout::Rect;
    use ratatui::widgets::{Block, Borders, Paragraph};
    
    let search_status = editor.search_status_line();
    
    // Build dialog content
    let mut lines = vec![];
    
    // Title
    lines.push(Line::from(vec![
        Span::styled("Find and Replace", Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    
    // Find input field
    if editor.replace_field_active {
        // Find field inactive
        lines.push(Line::from(vec![
            Span::styled("Find: ", Style::default().fg(editor.theme.ui_text)),
            Span::styled(&editor.search_query, Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_inactive_bg)),
        ]));
    } else {
        // Find field active
        lines.push(Line::from(vec![
            Span::styled("Find: ", Style::default().fg(editor.theme.ui_text)),
            Span::styled(&editor.search_query, Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)),
            Span::styled("_", Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)), // Cursor
        ]));
    }
    
    // Replace input field
    if editor.replace_field_active {
        // Replace field active
        lines.push(Line::from(vec![
            Span::styled("Replace: ", Style::default().fg(editor.theme.ui_text)),
            Span::styled(&editor.replace_text, Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)),
            Span::styled("_", Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_bg)), // Cursor
        ]));
    } else {
        // Replace field inactive
        lines.push(Line::from(vec![
            Span::styled("Replace: ", Style::default().fg(editor.theme.ui_text)),
            Span::styled(&editor.replace_text, Style::default().fg(editor.theme.ui_text).bg(editor.theme.input_inactive_bg)),
        ]));
    }
    lines.push(Line::from(""));

    lines.push(Line::from(search_switches(editor)));

    // Search results
    if !search_status.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(search_status, Style::default().fg(editor.theme.success)),
        ]));
    }
    lines.push(Line::from(""));

    // Help text
    lines.push(Line::from(vec![
        Span::styled("ENTER", Style::default().fg(editor.theme.success)),
        Span::styled(": replace current, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("Alt+ENTER", Style::default().fg(editor.theme.success)),
        Span::styled(": replace all", Style::default().fg(editor.theme.ui_hint)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("TAB", Style::default().fg(editor.theme.success)),
        Span::styled(": switch field, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("F3", Style::default().fg(editor.theme.success)),
        Span::styled(": next, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("Shift+F3", Style::default().fg(editor.theme.success)),
        Span::styled(": prev", Style::default().fg(editor.theme.ui_hint)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+I", Style::default().fg(editor.theme.success)),
        Span::styled(": toggle case, ", Style::default().fg(editor.theme.ui_hint)),
        Span::styled("ESC", Style::default().fg(editor.theme.danger)),
        Span::styled(": close", Style::default().fg(editor.theme.ui_hint)),
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
            .border_style(Style::default().fg(editor.theme.accent))
            .title(" Find and Replace (Ctrl+H) ")
            .title_style(Style::default().fg(editor.theme.accent).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(editor.theme.background))
        .wrap(Wrap { trim: true });
    
    f.render_widget(dialog_paragraph, dialog_area);
}

/// How long a completion list waits before it is built. Building one reads the
/// standard library and the symbols in scope, and doing that between one
/// keystroke and the next is what a fast typist felt as lag. The clock starts
/// at the first key that asks for a list and is not put back by the keys after
/// it, so a burst of typing costs one list rather than one per letter, and the
/// list is never more than this far behind the word it belongs to.
const COMPLETION_DELAY: Duration = Duration::from_millis(120);

/// How long to wait for a key when nothing is owed. Short enough that shutdown
/// is prompt, long enough that an idle editor is asleep.
const IDLE_POLL: Duration = Duration::from_millis(100);

pub fn key_thread_logic(editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>, tx: Sender<EditorMessage>, tx_build: Sender<EditorMessage>) {
    log::info!("Key thread started");

    // Set up panic handler for this thread
    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("KEY THREAD PANICKED: {:?}", panic_info);
        eprintln!("KEY THREAD PANICKED: {:?}", panic_info);
    }));

    // When the oldest completion list still owed was asked for.
    let mut asked_for_a_list: Option<Instant> = None;

    loop {
        // Check for messages
        match rx.try_recv() {
            Ok(EditorMessage::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Shutting down key thread");
                break;
            }
            _ => {}
        }

        // Build the list that was asked for, once the typing has had its
        // moment. Anything that reads the list sooner than this builds it
        // itself, so waiting can only ever cost a redraw.
        if let Some(asked) = asked_for_a_list {
            if asked.elapsed() >= COMPLETION_DELAY {
                if let Some(mut editor) = try_lock_with_timeout(&editor_arc, 100) {
                    editor.flush_completion_request();
                    asked_for_a_list = None;
                }
            }
        }

        // Check for key input. A list that is owed shortens the wait, so it
        // arrives on time rather than at the end of the next idle poll.
        let wait = match asked_for_a_list {
            Some(asked) => COMPLETION_DELAY.saturating_sub(asked.elapsed()).max(Duration::from_millis(1)),
            None => IDLE_POLL,
        };
        match event::poll(wait) {
            Ok(true) => {
                log::debug!("Event available from poll");
                match event::read() {
                    Ok(Event::Key(key)) => {
                        log::debug!("==> KEY EVENT: {:?}", key);
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
                            // The two fuzzy pickers take the keys that mean
                            // something to a list of choices, and hand back a
                            // command when one is chosen by name.
                            crate::DialogMode::CommandPalette | crate::DialogMode::SymbolPicker => {
                                match editor.handle_picker_key(key) {
                                    crate::PickerKey::Handled => continue,
                                    crate::PickerKey::Run(action) => {
                                        if run_action(&mut editor, action, &tx, &tx_build) {
                                            break;
                                        }
                                        continue;
                                    }
                                    crate::PickerKey::Ignored => {}
                                }
                            },
                            crate::DialogMode::ConfirmQuit => {
                                if key.code == KeyCode::Esc {
                                    let _ = tx.send(EditorMessage::Shutdown);
                                    break;
                                }
                                // Any other key means the Escape was meant for
                                // something else, so the editor stays open.
                                editor.dialog_mode = crate::DialogMode::None;
                                continue;
                            },
                            // The settings screen answers every key itself.
                            // Nothing typed into it should reach the buffer,
                            // and no binding should fire behind it.
                            crate::DialogMode::Settings => {
                                match key.code {
                                    KeyCode::Up => editor.settings_previous_row(),
                                    KeyCode::Down => editor.settings_next_row(),
                                    KeyCode::Left => editor.settings_cycle_value(false),
                                    KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => editor.settings_cycle_value(true),
                                    KeyCode::Esc => editor.close_settings(),
                                    _ => {}
                                }
                                continue;
                            },
                            _ => {}
                        }
                        
                        // Bindings resolve to a named action first, so which
                        // key does a thing and what that thing is stay
                        // separable. A key the table declines is text input,
                        // and falls through to the editor's own routing,
                        // which is where an open dialog or completion list
                        // gets to claim it.
                        // A prefix key is spent by the press after it, so
                        // the waiting one is taken and cleared before this key
                        // resolves rather than after.
                        let pending = editor.pending_prefix.take();
                        let vim_mode = vim_mode_for_dialog(&editor);
                        // Vim leaves the matches lit after the search box has
                        // closed, so an edit under them has to put them out:
                        // highlighting stays where it was put, and the text
                        // under it does not. What was searched for is kept, so
                        // `n` finds them again.
                        let before_the_key = editor.edit_marker();
                        match nail::keymap::resolve(editor.keymap, vim_mode, pending, key) {
                            Resolution::Pending(prefix) => {
                                editor.pending_prefix = Some(prefix);
                            }
                            Resolution::Run(action) => {
                                if run_action(&mut editor, action, &tx, &tx_build) {
                                    break;
                                }
                            }
                            // A key the bindings read and had no command for.
                            // Vim's normal mode is where these come from, and
                            // the point of them is that they are not text.
                            Resolution::Swallowed => {}
                            // A cancelled chord must not leave its second key
                            // in the buffer, so an unbound key only counts as
                            // text when nothing was waiting on it.
                            Resolution::Unbound if pending.is_some() => {}
                            // A chord no table claimed is still not a letter.
                            // Without this, Ctrl+U in a keymap that does not
                            // bind it types a u, and so does Alt+Q.
                            //
                            // Control and alt together are left alone, because
                            // that is how a Windows keyboard spells AltGr and
                            // the character it produces is text.
                            Resolution::Unbound
                                if matches!(key.code, KeyCode::Char(_))
                                    && key.modifiers.intersects(KeyModifiers::CONTROL.union(KeyModifiers::ALT))
                                    && !key.modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) => {}
                            Resolution::Unbound => match key.code {
                                KeyCode::Char(c) => {
                                    log::debug!("Received KeyCode::Char('{}'), dialog_mode: {:?}", c, editor.dialog_mode);
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
                                        crate::DialogMode::Settings | crate::DialogMode::ConfirmQuit | crate::DialogMode::CommandPalette | crate::DialogMode::SymbolPicker => {},
                                        crate::DialogMode::None => {
                                            log::debug!("Calling insert_char('{}') in normal mode", c);
                                            editor.insert_char(c);
                                            // Only update completions for single character input
                                            // to avoid overwhelming the system during paste operations
                                            if !key.modifiers.contains(KeyModifiers::SHIFT) {
                                                editor.request_completions();
                                            }
                                        }
                                    }
                                },
                                KeyCode::Tab => {
                                    // Whether this key takes a completion or
                                    // indents depends on there being a list,
                                    // so any list still owed is built now
                                    // rather than waited for.
                                    editor.flush_completion_request();
                                    if editor.dialog_mode == crate::DialogMode::Replace {
                                        // In replace mode, Tab switches between find and replace fields
                                        editor.switch_replace_field();
                                    } else if editor.show_completions {
                                        // Some terminals send shift+tab as Tab
                                        // with the modifier set rather than as
                                        // BackTab, so both spellings have to
                                        // mean the full example here.
                                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                                            editor.accept_completion_full();
                                        } else {
                                            editor.accept_completion();
                                        }
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
                                    editor.flush_completion_request();
                                    // An open completion list owns this key
                                    // exactly as it owns tab, whether or not
                                    // the documentation is showing: shift asks
                                    // for the full example. Letting it fall
                                    // through to dedent left the list on
                                    // screen with nothing pasted.
                                    if editor.show_completions {
                                        editor.accept_completion_full();
                                    } else {
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
                                        crate::DialogMode::Settings | crate::DialogMode::ConfirmQuit | crate::DialogMode::CommandPalette | crate::DialogMode::SymbolPicker => {},
                                        crate::DialogMode::None => {
                                            editor.delete_char();
                                            editor.request_completion_refresh();
                                        }
                                    }
                                },
                                KeyCode::Delete => {
                                    editor.delete_forward();
                                    editor.request_completion_refresh();
                                },
                                KeyCode::Enter => {
                                    match editor.dialog_mode {
                                        crate::DialogMode::GoToLine => {
                                            editor.execute_goto_line();
                                        },
                                        crate::DialogMode::Find => {
                                            // A vim search ends at Enter: the
                                            // box closes, the cursor stays on
                                            // the match the typing already
                                            // found, and `n` carries on from
                                            // there. The other keymaps keep
                                            // the box open, because there
                                            // Enter is how the matches are
                                            // walked.
                                            if editor.keymap == nail::keymap::Keymap::Vim {
                                                editor.close_dialog();
                                            } else {
                                                editor.find_next();
                                            }
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
                                        crate::DialogMode::Settings | crate::DialogMode::ConfirmQuit | crate::DialogMode::CommandPalette | crate::DialogMode::SymbolPicker => {},
                                        crate::DialogMode::None => {
                                            // Enter either takes what the list
                                            // is offering or breaks the line,
                                            // so a list still owed decides
                                            // which, and is built first.
                                            editor.flush_completion_request();
                                            if editor.show_completions {
                                                editor.accept_completion();
                                            } else {
                                                editor.insert_newline();
                                            }
                                        }
                                    }
                                },
                                KeyCode::Esc => {
                                    // A list that was asked for and never
                                    // shown is one of the things this key
                                    // dismisses.
                                    editor.cancel_completion_request();
                                    if editor.dialog_mode != crate::DialogMode::None {
                                        // Close any open dialog
                                        editor.close_dialog();
                                    } else if editor.keymap == nail::keymap::Keymap::Vim && editor.vim_mode != VimMode::Normal {
                                        // Under vim, getting back to normal
                                        // mode is what Escape is for, and one
                                        // press has to do it rather than
                                        // spending the first on whatever else
                                        // is open.
                                        enter_normal_mode(&mut editor);
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
                                    } else if editor.keymap == nail::keymap::Keymap::Vim {
                                        // Already in normal mode with nothing
                                        // to dismiss. A vim user presses
                                        // Escape to be sure of the mode, not
                                        // to be asked whether they meant to
                                        // leave, so quitting is left to the
                                        // name in the palette that `:` opens.
                                    } else {
                                        // Nothing left to dismiss, so this
                                        // Escape was aimed at the editor
                                        // itself. It still asks first.
                                        editor.ask_before_quitting();
                                    }
                                },
                                _ => {}
                            },
                        }

                        // Whatever that key turned out to be, if it changed the
                        // text then the highlighting no longer points at what
                        // it was pointing at.
                        if editor.edit_marker() != before_the_key && !editor.search_results.is_empty() && editor.dialog_mode == crate::DialogMode::None {
                            editor.clear_search_highlight();
                        }

                        // Start the clock at the first key that asks for a
                        // list, and leave it running while the rest arrive:
                        // the list is then built once, within
                        // `COMPLETION_DELAY` of being asked for, however fast
                        // the typing is.
                        asked_for_a_list = match editor.completion_request.is_some() {
                            true => asked_for_a_list.or_else(|| Some(Instant::now())),
                            false => None,
                        };
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
                    // A click puts the cursor where it points, a drag selects,
                    // and the wheel scrolls without taking the cursor along.
                    // While a dialog is open the mouse is ignored, because
                    // clicking the text behind a dialog is not a thing anyone
                    // means to do.
                    Ok(Event::Mouse(mouse)) => {
                        use ratatui::crossterm::event::{MouseButton, MouseEventKind};

                        // A terminal reports every twitch of the pointer, and
                        // most of those mean nothing here. Deciding that before
                        // reaching for the editor keeps the common case from
                        // queueing behind whatever else holds the lock.
                        let interesting = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) | MouseEventKind::ScrollUp | MouseEventKind::ScrollDown);
                        if !interesting {
                            continue;
                        }
                        let mut editor = match try_lock_with_timeout(&editor_arc, 200) {
                            Some(editor) => editor,
                            None => {
                                log::warn!("Key thread: editor lock timeout, skipping mouse event");
                                continue;
                            }
                        };
                        if editor.dialog_mode != crate::DialogMode::None {
                            continue;
                        }
                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => editor.mouse_press(mouse.column, mouse.row),
                            MouseEventKind::Drag(MouseButton::Left) => editor.mouse_drag(mouse.column, mouse.row),
                            MouseEventKind::Up(MouseButton::Left) => editor.mouse_release(),
                            MouseEventKind::ScrollUp => editor.scroll_by(-3),
                            MouseEventKind::ScrollDown => editor.scroll_by(3),
                            _ => {}
                        }
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

/// Carries out any action at all, including the four that need the key loop's
/// own channels. Returns whether the editor should shut down, which is the one
/// thing an action can ask for that the caller has to do itself.
///
/// Both the keyboard and the command palette come through here, because a
/// command picked from a list by name has to do exactly what its key does,
/// including the ones that quit or start a build.
fn run_action(editor: &mut Editor, action: Action, tx: &Sender<EditorMessage>, tx_build: &Sender<EditorMessage>) -> bool {
    match action {
        Action::Quit => {
            // Which files were open is worth keeping, and this is the last
            // moment there is to write it down.
            editor.save_session();
            let _ = tx.send(EditorMessage::Shutdown);
            return true;
        }
        // Vim's `ZZ`, which is the other way out: the one that writes the file
        // on the way rather than leaving the question open.
        Action::SaveAndQuit => {
            if let Err(e) = editor.save_file() {
                // Leaving on a failed write would throw the work away without
                // saying so, so the editor stays open and says so instead.
                editor.build_status = BuildStatus::Failed(format!("Save failed: {}", e));
                log::error!("Failed to save file: {}", e);
                return false;
            }
            editor.save_session();
            let _ = tx.send(EditorMessage::Shutdown);
            return true;
        }
        Action::Save => {
            log::info!("Save requested");
            match editor.save_file() {
                Ok(_) => {
                    editor.build_status = BuildStatus::Complete("Saved!".to_string());
                    log::info!("File saved successfully");
                }
                Err(e) => {
                    editor.build_status = BuildStatus::Failed(format!("Save failed: {}", e));
                    log::error!("Failed to save file: {}", e);
                }
            }
        }
        Action::CycleExampleFiles => {
            log::info!("Cycling through example files");
            // The examples directory is found by walking up from the open
            // file, then from where the IDE was started, then from the
            // checkout the IDE was compiled from. Launch directory stops
            // mattering: F5 works wherever the IDE was started.
            let examples_dir = {
                let mut anchors: Vec<PathBuf> = Vec::new();
                if let Some(name) = &editor.get_current_tab().filename {
                    if let Ok(canonical) = fs::canonicalize(name) {
                        anchors.push(canonical);
                    }
                }
                if let Ok(cwd) = std::env::current_dir() {
                    anchors.push(cwd);
                }
                anchors.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
                anchors.iter().flat_map(|anchor| anchor.ancestors()).map(|dir| dir.join("examples")).find(|candidate| candidate.is_dir())
            };
            let examples_dir = match examples_dir {
                Some(dir) => dir,
                None => {
                    editor.build_status = BuildStatus::Failed("Examples directory not found".to_string());
                    return false;
                }
            };
            // Relative to the working directory when it is inside it, so tab
            // titles stay short in the everyday checkout case.
            let display_dir = std::env::current_dir().ok().and_then(|cwd| examples_dir.strip_prefix(&cwd).map(Path::to_path_buf).ok()).unwrap_or_else(|| examples_dir.clone());
            let mut example_files = match fs::read_dir(&examples_dir) {
                Ok(dir) => dir
                    .filter_map(Result::ok)
                    .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "nail"))
                    .filter_map(|entry| entry.path().file_name().map(|name| display_dir.join(name).to_string_lossy().to_string()))
                    .collect::<Vec<String>>(),
                Err(e) => {
                    log::warn!("Failed to read examples directory: {}", e);
                    editor.build_status = BuildStatus::Failed("Examples directory not found".to_string());
                    return false;
                }
            };
            // read_dir order is arbitrary, which would make F5 jump around.
            example_files.sort();

            if example_files.is_empty() {
                editor.build_status = BuildStatus::Failed("No example files found".to_string());
                return false;
            }

            // Find current file index
            let current_tab = editor.get_current_tab();
            let current_index = match &current_tab.filename {
                Some(current) => {
                    let current_canonical = fs::canonicalize(current).ok();
                    example_files.iter().position(|file| file == current || (current_canonical.is_some() && fs::canonicalize(file).ok() == current_canonical)).unwrap_or(0)
                }
                None => 0,
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
                        editor.code_errors = vec![format!("Loaded: {}", next_file).into()];
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
        Action::Build => {
            match editor.build_status {
                BuildStatus::Idle | BuildStatus::Failed(_) | BuildStatus::Complete(_) => {
                    let _ = tx_build.send(EditorMessage::BuildStart);
                }
                _ => {
                    // Don't allow new builds while one is in progress
                }
            }
        }
        other => apply_action(editor, other),
    }
    return false;
}

/// Carries out an action against the editor. Four of them are missing on
/// purpose: quitting, saving, cycling the examples and starting a build all
/// need the key loop's own channels or its ability to break out, so
/// `run_action` above keeps those and hands everything else here. Listing them
/// rather than catching the rest with a wildcard means a new action cannot be
/// added without someone deciding which side of that line it falls on.
fn apply_action(editor: &mut Editor, action: Action) {
    match action {
        Action::Quit | Action::SaveAndQuit | Action::Save | Action::CycleExampleFiles | Action::Build => {}
        // The user saying the disk's copy wins: the buffer becomes whatever
        // the file says now, and the edits that were in it become one undo
        // step away rather than gone. The watcher reloads a clean buffer by
        // itself, so this key exists for the dirty one it refuses to touch.
        Action::ReloadFromDisk => {
            let filename = editor.get_current_tab().filename.clone();
            match filename {
                None => editor.build_status = BuildStatus::Failed("No file behind this tab to reload".to_string()),
                Some(filename) => match fs::read_to_string(&filename) {
                    Ok(text) => {
                        let mtime = fs::metadata(&filename).and_then(|meta| meta.modified()).ok();
                        editor.get_current_tab_mut().take_disk_copy(text.lines().map(String::from).collect(), mtime);
                        editor.build_status = BuildStatus::Complete("Took the disk copy. Undo brings your edits back".to_string());
                    }
                    Err(e) => editor.build_status = BuildStatus::Failed(format!("Reload failed: {}", e)),
                },
            }
        }
        Action::ToggleTheme => editor.toggle_theme(),
        Action::ToggleLineNumbers => {
            editor.show_line_numbers = !editor.show_line_numbers;
            log::info!("Line numbers toggled: {}", editor.show_line_numbers);
        }
        Action::ToggleCurrentLineHighlight => {
            editor.highlight_current_line = !editor.highlight_current_line;
            log::info!("Current line highlighting toggled: {}", editor.highlight_current_line);
        }
        Action::ToggleBracketMatching => {
            editor.highlight_matching_brackets = !editor.highlight_matching_brackets;
            if !editor.highlight_matching_brackets {
                editor.matching_bracket_pos = None;
            }
            log::info!("Bracket matching toggled: {}", editor.highlight_matching_brackets);
        }
        Action::ToggleWhitespace => {
            editor.show_whitespace = !editor.show_whitespace;
            log::info!("Whitespace visualization toggled: {}", editor.show_whitespace);
        }
        Action::ToggleIndentationGuides => {
            editor.show_indentation_guides = !editor.show_indentation_guides;
            log::info!("Indentation guides toggled: {}", editor.show_indentation_guides);
        }
        Action::ToggleMinimap => {
            editor.show_minimap = !editor.show_minimap;
            log::info!("Minimap toggled: {}", editor.show_minimap);
        }
        Action::SelectAll => editor.select_all(),
        Action::Cut => {
            if let Err(e) = editor.cut_selection() {
                log::error!("Failed to cut to clipboard: {}", e);
            }
        }
        Action::Paste => {
            if let Err(e) = editor.paste_from_clipboard() {
                log::error!("Failed to paste from clipboard: {}", e);
            }
        }
        Action::Undo => {
            if editor.undo() {
                log::info!("Undo operation performed");
            } else {
                log::info!("Nothing to undo");
            }
        }
        Action::Redo => {
            if editor.redo() {
                log::info!("Redo operation performed");
            } else {
                log::info!("Nothing to redo");
            }
        }
        Action::GoToLineDialog => editor.show_goto_line_dialog(),
        Action::FindDialog => editor.show_find_dialog(),
        Action::ReplaceDialog => editor.show_replace_dialog(),
        Action::ToggleCaseSensitivity => {
            if matches!(editor.dialog_mode, crate::DialogMode::Find | crate::DialogMode::Replace) {
                editor.toggle_case_sensitivity();
            }
        }
        Action::FindNext => editor.find_again(true),
        Action::FindPrevious => editor.find_again(false),
        Action::OpenFileDialog => editor.open_file_dialog(),
        Action::CloseTab => {
            let tab_index = editor.tab_index;
            editor.close_tab(tab_index);
        }
        Action::NewTab => editor.new_tab(),
        Action::StdLibBrowser => editor.open_stdlib_browser(),
        Action::NextTab => editor.next_tab(),
        Action::PreviousTab => editor.prev_tab(),
        Action::SwitchToTab(index) => editor.switch_to_tab(index),
        Action::ToggleComment => editor.toggle_comment(),
        Action::DuplicateLine => editor.duplicate_line(),
        Action::MoveLineUp => editor.move_line_up(),
        Action::MoveLineDown => editor.move_line_down(),
        Action::DeleteLine => editor.delete_line(),
        Action::JumpToMatchingBracket => editor.jump_to_matching_bracket(),
        Action::ToggleCompletionDetail => {
            editor.flush_completion_request();
            if editor.show_completions && !editor.completions.is_empty() {
                editor.show_detail_view = !editor.show_detail_view;
            }
        }
        // A page of scrolling takes the cursor with it, so a selection being
        // dragged along has to hear about the move the same way a motion tells
        // it.
        Action::ScrollUp => {
            editor.scroll_up();
            if editor.mark_active {
                editor.extend_selection();
            }
        }
        Action::ScrollDown => {
            editor.scroll_down();
            if editor.mark_active {
                editor.extend_selection();
            }
        }
        // An open completion list owns the up and down keys, because picking
        // from it is what the user is in the middle of doing. A list that was
        // asked for a moment ago counts as open, so these keys build it rather
        // than moving the cursor out from under it.
        Action::CursorUp { extend } => {
            editor.flush_completion_request();
            if editor.show_completions {
                editor.previous_completion();
            } else {
                editor.move_cursor_up_with_selection(extend || editor.mark_active);
            }
        }
        Action::CursorDown { extend } => {
            editor.flush_completion_request();
            if editor.show_completions {
                editor.next_completion();
            } else {
                editor.move_cursor_down_with_selection(extend || editor.mark_active);
            }
        }
        Action::CursorLeft { extend } => editor.move_cursor_left_with_selection(extend || editor.mark_active),
        Action::CursorRight { extend } => editor.move_cursor_right_with_selection(extend || editor.mark_active),
        Action::CursorWordLeft { extend } => editor.move_cursor_left_word_with_selection(extend || editor.mark_active),
        Action::CursorWordRight { extend } => editor.move_cursor_right_word_with_selection(extend || editor.mark_active),
        Action::SmartHome => editor.smart_home(),
        Action::LineStart { extend } => editor.move_to_line_start_with_selection(extend || editor.mark_active),
        Action::LineEnd { extend } => editor.move_to_line_end_with_selection(extend || editor.mark_active),
        Action::FileStart { extend } => editor.move_to_file_start_with_selection(extend || editor.mark_active),
        Action::FileEnd { extend } => editor.move_to_file_end_with_selection(extend || editor.mark_active),
        Action::Copy => {
            if let Err(e) = editor.copy_selection() {
                log::error!("Failed to copy to clipboard: {}", e);
            }
        }
        Action::SetMark => editor.set_mark(),
        Action::ClearMark => editor.clear_mark(),
        Action::KillToLineEnd => editor.kill_to_line_end(),
        Action::DeleteForward => {
            editor.delete_forward();
            editor.request_completion_refresh();
        }
        Action::OpenSettings => editor.open_settings(),
        // Editing a word at a time is only ever meant for the buffer, so a
        // dialog with the keyboard keeps it.
        Action::DeleteWordLeft => {
            if editor.dialog_mode == crate::DialogMode::None {
                editor.delete_word_left();
                editor.request_completion_refresh();
            }
        }
        Action::DeleteWordRight => {
            if editor.dialog_mode == crate::DialogMode::None {
                editor.delete_word_right();
                editor.request_completion_refresh();
            }
        }
        Action::NextError => editor.go_to_error(true),
        Action::PreviousError => editor.go_to_error(false),
        Action::CopyErrors => editor.copy_errors(),
        Action::CopyScreen => editor.request_screen_copy(),
        Action::CopySelectionWithAnnotations => editor.copy_selection_with_annotations(),
        Action::CopyFileText => editor.copy_file_text(),
        Action::CopyFileWithAnnotations => editor.copy_file_with_annotations(),
        Action::CommandPalette => editor.open_command_palette(),
        Action::SymbolPicker => editor.open_symbol_picker(),
        Action::ProjectSymbolPicker => editor.open_project_symbol_picker(),
        Action::ProjectSearch => editor.open_project_search(),
        Action::OpenImportedFile => editor.open_imported_file(),
        Action::ExpandSelection => editor.expand_selection(),
        Action::ShrinkSelection => editor.shrink_selection(),
        Action::JoinLines => editor.join_lines(),
        Action::SortLines => editor.sort_lines(),
        // The search switches belong to the search boxes. Outside them the key
        // would change what the next search means with nothing on screen to
        // say so, which is how a search comes back empty for no visible reason.
        Action::ToggleWholeWord => {
            if matches!(editor.dialog_mode, crate::DialogMode::Find | crate::DialogMode::Replace) {
                editor.toggle_whole_word();
            }
        }
        Action::ToggleRegex => {
            if matches!(editor.dialog_mode, crate::DialogMode::Find | crate::DialogMode::Replace) {
                editor.toggle_regex();
            }
        }
        Action::ToggleMouse => set_mouse_capture(editor, !editor.mouse_enabled),
        Action::ScrollLineUp => editor.scroll_by(-1),
        Action::ScrollLineDown => editor.scroll_by(1),
        // The vim edits. The mode changes are here rather than in the keymap
        // because a mode is something the editor is in, and the table's job
        // ends at saying which key asked for it.
        Action::EnterInsertMode => enter_insert_mode(editor),
        Action::EnterNormalMode => enter_normal_mode(editor),
        // Visual mode is the mark with a name: dropping an anchor is what
        // makes every motion afterwards extend the selection, which is exactly
        // what visual mode is. Pressing the key again is how vim turns it off.
        Action::EnterVisualMode => {
            if editor.vim_mode == VimMode::Visual {
                enter_normal_mode(editor);
            } else {
                editor.vim_mode = VimMode::Visual;
                editor.set_mark();
            }
        }
        Action::EnterVisualLineMode => {
            if editor.vim_mode == VimMode::VisualLine {
                enter_normal_mode(editor);
            } else {
                editor.vim_mode = VimMode::VisualLine;
                editor.set_mark();
            }
        }
        Action::InsertAfterCursor => {
            editor.move_cursor_right_with_selection(false);
            enter_insert_mode(editor);
        }
        Action::InsertAtLineStart => {
            editor.smart_home();
            enter_insert_mode(editor);
        }
        Action::InsertAtLineEnd => {
            editor.move_to_line_end_with_selection(false);
            enter_insert_mode(editor);
        }
        Action::OpenLineBelow => {
            editor.open_line_below();
            enter_insert_mode(editor);
        }
        Action::OpenLineAbove => {
            editor.open_line_above();
            enter_insert_mode(editor);
        }
        Action::DeleteCharAtCursor => editor.delete_char_at_cursor(),
        Action::DeleteBackward => editor.delete_char(),
        Action::DeleteToLineStart => editor.kill_to_line_start(),
        Action::ChangeToLineEnd => {
            editor.kill_to_line_end();
            enter_insert_mode(editor);
        }
        Action::ChangeLine => {
            editor.change_line();
            enter_insert_mode(editor);
        }
        Action::ChangeWord => {
            editor.delete_word_right();
            enter_insert_mode(editor);
        }
        Action::SubstituteChar => {
            editor.delete_char_at_cursor();
            enter_insert_mode(editor);
        }
        Action::YankLine => editor.yank_line(),
        Action::YankWord => editor.yank_word(),
        Action::YankToLineEnd => editor.yank_to_line_end(),
        Action::PasteAfter => editor.paste_around_cursor(true),
        Action::PasteBefore => editor.paste_around_cursor(false),
        // The visual mode operators. Each one is done with the selection by
        // the time it finishes, which is why each ends back in normal mode.
        //
        // Visual line mode takes a different route through three of them: a
        // selection that covers whole lines still stops at the last character
        // rather than after the last newline, so cutting it charwise would
        // leave the emptied lines behind.
        Action::CutSelection => {
            if editor.vim_mode == VimMode::VisualLine {
                editor.yank_selection_as_lines();
                editor.delete_line();
            } else if let Err(e) = editor.cut_selection() {
                log::error!("Failed to cut to clipboard: {}", e);
            }
            enter_normal_mode(editor);
        }
        Action::YankSelection => {
            if editor.vim_mode == VimMode::VisualLine {
                editor.yank_selection_as_lines();
            } else if let Err(e) = editor.copy_selection() {
                log::error!("Failed to copy to clipboard: {}", e);
            }
            enter_normal_mode(editor);
        }
        Action::ChangeSelection => {
            if let Err(e) = editor.cut_selection() {
                log::error!("Failed to cut to clipboard: {}", e);
            }
            enter_insert_mode(editor);
        }
        Action::PasteOverSelection => {
            if editor.vim_mode == VimMode::VisualLine {
                editor.delete_line();
                editor.paste_around_cursor(false);
            } else if let Err(e) = editor.paste_from_clipboard() {
                log::error!("Failed to paste from clipboard: {}", e);
            }
            enter_normal_mode(editor);
        }
        Action::Indent => editor.indent_selection(),
        Action::Dedent => editor.dedent_selection(),
        Action::ClearSearchHighlight => editor.clear_search_highlight(),
        // The status line is where the editor already answers Ctrl+S, so it is
        // where a key that cannot be answered says why.
        Action::Unsupported(message) => editor.build_status = BuildStatus::Complete(message.to_string()),
    }

    // Visual line mode is the only thing that has to be put right after every
    // key rather than by the key itself: a motion moves one end of the
    // selection, and whole lines are what that end has to land on.
    if editor.vim_mode == VimMode::VisualLine {
        editor.snap_selection_to_lines();
    }
}

/// Both mode changes clear the same two things. The mark is what makes motions
/// extend the selection, so leaving visual mode without dropping it would
/// leave every later motion selecting.
fn enter_insert_mode(editor: &mut Editor) {
    editor.vim_mode = VimMode::Insert;
    editor.pending_prefix = None;
    editor.clear_mark();
}

/// A completion list is something only insert mode can pick from, so it is
/// dismissed on the way out rather than left over normal mode where every key
/// that could choose from it means something else.
fn enter_normal_mode(editor: &mut Editor) {
    editor.show_completions = false;
    editor.show_detail_view = false;
    editor.completions.clear();
    editor.vim_mode = VimMode::Normal;
    editor.pending_prefix = None;
    editor.clear_mark();
}

/// Which vim mode the keys are read in, which is not always the mode the
/// editor is in. A dialog that takes typing is insert mode by another name:
/// normal mode swallows letters, so the find box would never see one.
fn vim_mode_for_dialog(editor: &Editor) -> VimMode {
    return match editor.dialog_mode {
        crate::DialogMode::GoToLine | crate::DialogMode::Find | crate::DialogMode::Replace => VimMode::Insert,
        _ => editor.vim_mode,
    };
}

/// Asks the terminal to start or stop reporting the mouse. Handing it back is
/// how the user gets their terminal's own click to select and copy working
/// again, which is worth more than our clicks whenever they want to take text
/// out of the window.
fn set_mouse_capture(editor: &mut Editor, wanted: bool) {
    use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};

    let result = if wanted { execute!(io::stdout(), EnableMouseCapture) } else { execute!(io::stdout(), DisableMouseCapture) };
    match result {
        Ok(()) => {
            editor.mouse_enabled = wanted;
            editor.mouse_dragging = false;
            editor.build_status = BuildStatus::Complete(if wanted { "Mouse on".to_string() } else { "Mouse off, terminal selection back".to_string() });
        }
        Err(e) => log::error!("Could not change mouse reporting: {}", e),
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
            let content = current_tab.content.join("\n");
            let filename = current_tab.filename.clone();
            drop(editor);

            let stages_started = std::time::Instant::now();
            let lex_started = std::time::Instant::now();
            let program = lexer::lex_program(&content, filename.as_deref().map(Path::new));
            let tokens = program.tokens;
            let source_map = program.source_map;
            let lex_elapsed = lex_started.elapsed();

            let parse_started = std::time::Instant::now();
            let mut ast = match parse(tokens) {
                Ok(ast) => {
                    log::info!("AST (parsed): {:#?}", ast);
                    ast
                }
                Err(e) => {
                    let mut editor = editor_arc.lock().unwrap();
                    editor.build_status = BuildStatus::Failed(prefix_with_file(&e.message, &e.code_span, &source_map));
                    log::error!("Parsing failed: {:?}", e);
                    continue;
                }
            };
            let parse_elapsed = parse_started.elapsed();

            let check_started = std::time::Instant::now();
            let ast = match checker(&mut ast) {
                Ok(_) => {
                    log::info!("AST (type checked): {:#?}", ast);
                    ast
                }
                Err(errors) => {
                    let combined_message =
                        errors.iter().map(|error| prefix_with_file(&error.message, &error.code_span, &source_map)).collect::<Vec<_>>().join("; ");
                    let mut editor = editor_arc.lock().unwrap();
                    editor.build_status = BuildStatus::Failed(combined_message);
                    log::error!("Checker failed: {:?}", errors);
                    continue;
                }
            };
            let check_elapsed = check_started.elapsed();

            // Step 2: Transpile to Rust, instrumented so the running program
            // dumps per-function timings the IDE can render live
            let mut editor = editor_arc.lock().unwrap();
            editor.build_status = BuildStatus::Transpiling;
            drop(editor); // Release the lock
            let mut transpiler = Transpiler::new();
            transpiler.profile = true;
            transpiler.profile_source_hash = Some(nail::prof::source_fingerprint(&content));
            let transpile_started = std::time::Instant::now();
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
            let transpile_elapsed = transpile_started.elapsed();

            let compiler_timings = format!(
                "compiler timings: lex {}, parse {}, check {}, transpile {}, total {}",
                format_millis(lex_elapsed),
                format_millis(parse_elapsed),
                format_millis(check_elapsed),
                format_millis(transpile_elapsed),
                format_millis(stages_started.elapsed())
            );
            log::info!("{}", compiler_timings);

            // Step 3: Write Rust code into the persistent build project, the
            // same .nail-build directory beside the source that `nail run`
            // and `nail build` use. The directory is kept between builds so
            // cargo can reuse target/ instead of recompiling every dependency
            // from scratch. Files are only rewritten when their content
            // changed, preserving cargo's mtime-based fingerprints where
            // possible.
            let build_dir = match &filename {
                Some(name) => Path::new(name).parent().unwrap_or(Path::new(".")).join(".nail-build"),
                None => PathBuf::from(".nail-build"),
            };
            let build_src_dir = build_dir.join("src");
            if let Err(e) = fs::create_dir_all(&build_src_dir) {
                let mut editor = editor_arc.lock().unwrap();
                editor.build_status = BuildStatus::Failed(format!("Failed to create src directory: {}", e));
                log::error!("Failed to create src directory: {}", e);
                continue;
            }

            // Installed bundles build with their own pinned toolchain and
            // nail crate; development checkouts use the system cargo and the
            // checkout this IDE was compiled from, wherever the IDE was
            // started. The build directory sits beside the source rather than
            // beside the crate, so a relative path would resolve from the
            // wrong place.
            let bundle = nail::toolchain::BundledToolchain::detect();
            let nail_crate_path = match &bundle {
                Some(bundle) => bundle.nail_crate_path().display().to_string(),
                None => env!("CARGO_MANIFEST_DIR").to_string(),
            };
            let transpilation_toml = transpiler.generate_cargo_toml("nail_transpilation", &nail_crate_path);
            let transpilation_toml_path = build_dir.join("Cargo.toml");
            let toml_unchanged = fs::read_to_string(&transpilation_toml_path).map(|existing| existing == transpilation_toml).unwrap_or(false);
            if !toml_unchanged {
                if let Err(e) = fs::write(&transpilation_toml_path, &transpilation_toml) {
                    let mut editor = editor_arc.lock().unwrap();
                    editor.build_status = BuildStatus::Failed(format!("Failed to write Cargo.toml file: {}", e));
                    log::error!("Failed to write Cargo.toml file: {}", e);
                    continue;
                }
            }

            let temp_file_path = build_src_dir.join("main.rs");
            let main_rs_unchanged = fs::read_to_string(&temp_file_path).map(|existing| existing == rust_code).unwrap_or(false);
            if !main_rs_unchanged {
                if let Err(e) = fs::write(&temp_file_path, &rust_code) {
                    let mut editor = editor_arc.lock().unwrap();
                    editor.build_status = BuildStatus::Failed(format!("Failed to write Rust code to file: {}", e));
                    log::error!("Failed to write Rust code to file: {}", e);

                    continue;
                }
            }

            // Step 4: Compile the Rust code. This is the one step slow enough
            // to report progress on, measured against the last build of the
            // same kind: a changed Cargo.toml rebuilds dependencies and takes
            // minutes, a changed main.rs alone takes seconds.
            let deps_changed = !toml_unchanged;
            let compile_started = std::time::Instant::now();
            let mut editor = editor_arc.lock().unwrap();
            editor.build_status = BuildStatus::Compiling;
            editor.compile_started = Some(compile_started);
            editor.compile_estimate = read_build_estimate(deps_changed);
            drop(editor); // Release the lock
            let mut cargo = match &bundle {
                Some(bundle) => bundle.cargo_command(),
                None => Command::new("cargo"),
            };
            let output = cargo.arg("build").arg("--release").current_dir(&build_dir).output();
            let compile_elapsed = compile_started.elapsed();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        // Only a finished build is a fair estimate. A failed
                        // one stops at the first error, far short of the work
                        // the next successful build has to do.
                        record_build_time(deps_changed, compile_elapsed);
                        log::debug!("Compiler stdout: {}", String::from_utf8_lossy(&output.stdout));

                        let binary_path = match &bundle {
                            Some(bundle) => bundle.built_binary_path("nail_transpilation"),
                            None => build_dir.join("target/release/nail_transpilation"),
                        };
                        // Beside the source and named after it, the same
                        // place `nail build` puts it.
                        let destination_path = match &filename {
                            Some(name) => Path::new(name).with_extension(""),
                            None => PathBuf::from("build"),
                        };
                        // Copy (not move) so cargo's target/ stays intact and the
                        // next build can skip even the final link when unchanged.
                        // Unlink first so copying succeeds even if the binary is running.
                        let _ = fs::remove_file(&destination_path);
                        if let Err(e) = fs::copy(&binary_path, &destination_path) {
                            log::error!("Failed to copy binary: {}", e);
                            let mut editor = editor_arc.lock().unwrap();
                            editor.build_status = BuildStatus::Failed(format!("Failed to copy binary: {}", e));
                        } else {
                            let mut editor = editor_arc.lock().unwrap();
                            editor.build_status = BuildStatus::Complete(format!("Saved! {}", compiler_timings));
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
            editor.compile_started = None;
            editor.build_status = BuildStatus::Idle;
        }

        thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Everything the IDE remembers about a project lives in one `.nail` file in
/// the directory it was started from: plain `key=value` lines, one per line.
/// It is a convenience, never a requirement, so a missing or unreadable file
/// only costs the defaults. The one thing kept out of it is the profiler dump,
/// which a separate process rewrites every second.
const PROJECT_CONFIG_FILE: &str = ".nail";

fn project_config_path() -> Option<PathBuf> {
    Some(std::env::current_dir().ok()?.join(PROJECT_CONFIG_FILE))
}

fn config_value_in(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix('=').map(|value| value.trim().to_string()))
}

/// Rewrites the given keys and leaves every other line the file already had
/// alone, so two parts of the IDE storing different things never erase each
/// other. Existing keys keep their position, new ones go at the end.
fn config_text_with(existing: &str, pairs: &[(&str, String)]) -> String {
    let mut lines: Vec<String> = existing.lines().map(|line| line.to_string()).collect();
    for (key, value) in pairs {
        let replacement = format!("{}={}", key, value);
        match lines.iter_mut().find(|line| line.starts_with(&format!("{}=", key))) {
            Some(line) => *line = replacement,
            None => lines.push(replacement),
        }
    }
    return format!("{}\n", lines.join("\n"));
}

pub fn read_config_value(key: &str) -> Option<String> {
    let text = fs::read_to_string(project_config_path()?).ok()?;
    config_value_in(&text, key)
}

pub fn write_config_values(pairs: &[(&str, String)]) {
    let path = match project_config_path() {
        Some(path) => path,
        None => return,
    };
    let existing = fs::read_to_string(&path).unwrap_or_default();
    if let Err(e) = fs::write(&path, config_text_with(&existing, pairs)) {
        log::warn!("Could not write {}: {}", PROJECT_CONFIG_FILE, e);
    }
}

/// A build that rebuilt dependencies and one that only recompiled the program
/// are minutes apart, so each kind remembers its own duration and is only ever
/// compared against its own kind.
fn build_time_key(deps_changed: bool) -> &'static str {
    if deps_changed {
        "build_deps_nanos"
    } else {
        "build_code_nanos"
    }
}

fn read_build_estimate(deps_changed: bool) -> Option<std::time::Duration> {
    let nanos: u64 = read_config_value(build_time_key(deps_changed))?.parse().ok()?;
    Some(std::time::Duration::from_nanos(nanos))
}

fn record_build_time(deps_changed: bool, took: std::time::Duration) {
    write_config_values(&[(build_time_key(deps_changed), (took.as_nanos() as u64).to_string())]);
}

/// A profiled program writes .nail_profile.json via tmp and rename, so a
/// read never sees a partial file. Parse failure still returns None instead
/// of panicking because the file is outside the IDE's control.
fn parse_profile_dump(text: &str) -> Option<ProfileData> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let source_hash = value.get("source_hash")?.as_str()?.to_string();
    let wall_nanos = value.get("wall_nanos")?.as_u64()?;
    let functions = value
        .get("functions")?
        .as_array()?
        .iter()
        .filter_map(|entry| {
            Some(ProfiledFunction {
                name: entry.get("name")?.as_str()?.to_string(),
                calls: entry.get("calls")?.as_u64()?,
                total_nanos: entry.get("total_nanos")?.as_u64()?,
                max_nanos: entry.get("max_nanos")?.as_u64()?,
            })
        })
        .collect();
    Some(ProfileData { source_hash, wall_nanos, functions })
}

/// Polls .nail_profile.json about once a second, both beside the file on
/// screen (where a program started with `nail run` writes it) and in the
/// IDE's own working directory. A stat of the mtime is the only steady cost,
/// a file is re-read and re-parsed on change only. Missing or unreadable
/// files just mean no annotations.
pub fn profile_watcher_thread_logic(editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>) {
    const PROFILE_DUMP_NAME: &str = ".nail_profile.json";
    let mut last_mtimes: std::collections::HashMap<PathBuf, std::time::SystemTime> = std::collections::HashMap::new();
    loop {
        match rx.try_recv() {
            Ok(EditorMessage::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Shutting down profile watcher thread");
                break;
            }
            _ => {}
        }

        let mut candidates = vec![PathBuf::from(PROFILE_DUMP_NAME)];
        {
            let editor = lock(&editor_arc);
            if let Some(name) = &editor.get_current_tab().filename {
                if let Some(parent) = Path::new(name).parent().filter(|parent| !parent.as_os_str().is_empty()) {
                    candidates.push(parent.join(PROFILE_DUMP_NAME));
                }
            }
        }

        let mut any_present = false;
        for candidate in candidates {
            match fs::metadata(&candidate).and_then(|meta| meta.modified()) {
                Ok(mtime) => {
                    any_present = true;
                    if last_mtimes.get(&candidate) != Some(&mtime) {
                        last_mtimes.insert(candidate.clone(), mtime);
                        if let Some(data) = fs::read_to_string(&candidate).ok().and_then(|text| parse_profile_dump(&text)) {
                            let mut editor = lock(&editor_arc);
                            editor.profile_dumps.insert(data.source_hash.clone(), data.clone());
                            editor.profile_data = Some(data);
                        }
                    }
                }
                Err(_) => {
                    last_mtimes.remove(&candidate);
                }
            }
        }
        if !any_present && !last_mtimes.is_empty() {
            last_mtimes.clear();
        }
        if !any_present {
            // Dumps deleted or unreadable, drop annotations rather than
            // keep showing timings that no longer exist on disk
            let mut editor = lock(&editor_arc);
            if editor.profile_data.is_some() {
                editor.profile_data = None;
            }
        }

        thread::sleep(Duration::from_millis(1000));
    }
}

/// Watches the files behind the open tabs and pulls in changes made by
/// anything that is not this editor: a formatter, a git checkout, an AI agent
/// working on the same file. A buffer with no unsaved edits is reloaded in
/// place, so the file on screen visibly follows along as something else
/// rewrites it. A buffer with unsaved edits is never clobbered: it is flagged
/// instead, and the status bar says the disk moved until the next save
/// settles whose copy wins.
pub fn file_watcher_thread_logic(editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>) {
    loop {
        match rx.try_recv() {
            Ok(EditorMessage::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Shutting down file watcher thread");
                break;
            }
            _ => {}
        }

        // What is open, snapshotted under the lock. The stats and reads
        // happen unlocked, so a slow disk never stalls a keystroke.
        let watched: Vec<(usize, String, Option<std::time::SystemTime>)> = {
            let editor = lock(&editor_arc);
            editor.tabs.iter().enumerate().filter_map(|(index, tab)| tab.filename.clone().map(|filename| (index, filename, tab.disk_mtime))).collect()
        };

        for (index, filename, known_mtime) in watched {
            let mtime = match fs::metadata(&filename).and_then(|meta| meta.modified()) {
                Ok(mtime) => mtime,
                // Deleted, or briefly missing in the middle of a rewrite.
                // Keep the buffer: it is the only copy left, and a save puts
                // the file back.
                Err(_) => continue,
            };
            if known_mtime == Some(mtime) {
                continue;
            }
            let lines: Vec<String> = match fs::read_to_string(&filename) {
                Ok(text) => text.lines().map(String::from).collect(),
                Err(_) => continue,
            };

            let mut editor = lock(&editor_arc);
            // Everything is re-checked under the lock, because tabs may have
            // closed and keystrokes may have landed since the snapshot.
            let tab = match editor.tabs.get_mut(index) {
                Some(tab) if tab.filename.as_deref() == Some(filename.as_str()) => tab,
                _ => continue,
            };
            if tab.disk_mtime == Some(mtime) {
                continue;
            }
            if tab.modified {
                tab.disk_changed_underneath = true;
                tab.disk_mtime = Some(mtime);
                log::info!("{} changed on disk under unsaved edits, keeping the buffer", filename);
                continue;
            }
            tab.reload_from_disk(lines, Some(mtime));
            log::info!("Reloaded {} after it changed on disk", filename);
        }

        thread::sleep(Duration::from_millis(250));
    }
}

/// The message an error from an imported file shows in a buffer that cannot
/// scroll to it: the file and its own line, in front of the original message.
fn prefix_with_file(message: &str, span: &crate::common::CodeSpan, map: &crate::common::SourceMap) -> String {
    match map.resolve(span.start_line) {
        Some((file, real_line)) if file.base != 0 => format!("in {}:{}: {}", file.path, real_line, message),
        _ => message.to_string(),
    }
}

/// Fold a program error onto the open buffer. An error in the buffer itself
/// keeps its span. An error inside an imported file gets its message prefixed
/// with that file and line, and its span moved to the import statement that
/// brought the file in, which is the nearest line the buffer can point at.
fn localize_span(message: String, code_span: crate::common::CodeSpan, map: &crate::common::SourceMap) -> CodeError {
    match map.resolve(code_span.start_line) {
        Some((file, real_line)) if file.base != 0 => {
            let anchor = map.anchor_in_entry(code_span.start_line);
            CodeError {
                message: format!("in {}:{}: {}", file.path, real_line, message),
                code_span: crate::common::CodeSpan { start_line: anchor, start_column: 1, end_line: anchor, end_column: 1 },
            }
        }
        _ => CodeError { message, code_span },
    }
}

pub fn lex_and_parse_thread_logic(editor_arc: Arc<Mutex<Editor>>, rx: Receiver<EditorMessage>) {
    // Tracks the (tab index, content hash) last run through the pipeline so
    // unchanged content is not re-lexed/re-parsed/re-checked every 250ms.
    let mut last_processed: Option<(usize, u64)> = None;
    loop {
        // Check for shutdown message
        match rx.try_recv() {
            Ok(EditorMessage::Shutdown) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                log::info!("Shutting down syntax error thread");
                break;
            }
            _ => {}
        }

        // Lock the editor and hash the lines in place. The join into one
        // String only happens when the hash says something changed, so an
        // idle editor costs a scan and no allocation, four times a second.
        // The filename comes along so imports resolve relative to the file,
        // not the process's directory.
        let changed = {
            let editor = lock(&editor_arc);
            let tab = editor.get_current_tab();
            let mut hasher = DefaultHasher::new();
            for line in &tab.content {
                line.hash(&mut hasher);
            }
            let content_hash = hasher.finish();
            if last_processed == Some((editor.tab_index, content_hash)) {
                None
            } else {
                last_processed = Some((editor.tab_index, content_hash));
                Some((editor.tab_index, tab.content.join("\n"), tab.filename.clone()))
            }
        };
        let (tab_index, content, filename) = match changed {
            Some(taken) => taken,
            None => {
                thread::sleep(Duration::from_millis(250));
                continue;
            }
        };

        // Run the lexer on the content, keeping the source map so an error
        // in an imported file can say which file and line it is really on
        let program = lexer::lex_program(&content, filename.as_deref().map(Path::new));
        let tokens = program.tokens;
        let source_map = program.source_map;

        // Copied before the lock is taken rather than under it: a long file is
        // ten thousand tokens, and the key thread is queued behind this.
        let tokens_for_the_tab = tokens.clone();
        {
            let mut editor = lock(&editor_arc);
            let current_tab = editor.get_current_tab_mut();
            current_tab.tokens = tokens_for_the_tab;
        }

        // Check for error tokens (collect_lexer_errors also finds errors nested
        // inside FunctionSignature tokens, so every one gets its own line)
        let mut lexing_errors: Vec<CodeError> =
            lexer::collect_lexer_errors(&tokens).into_iter().map(|error| localize_span(error.message, error.code_span, &source_map)).collect();

        // A file with no version line is not a Nail file: the compiler refuses
        // it outright. The editor used to say nothing and quietly write one in
        // on save, which fixed the file while hiding the rule, so the first
        // time anyone met it was on someone else's machine. It is an error
        // here now, the same one the compiler gives, and saving still writes
        // the line so the fix stays one keystroke away.
        if nail::version_line::scan_header(content.as_bytes()).pin.is_none() {
            lexing_errors.insert(
                0,
                CodeError {
                    message: "No version line. Line one says which Nail wrote this file, so it keeps compiling the same way forever. Add `nail latest`, or save and one is written for you.".to_string(),
                    code_span: crate::common::CodeSpan { start_line: 1, start_column: 1, end_line: 1, end_column: 1 },
                },
            );
        }

        {
            let mut editor = lock(&editor_arc);
            editor.code_errors = lexing_errors.clone();
        }

        if !lexing_errors.is_empty() {
            log::info!("Lexer errors detected: {:?}", lexing_errors);
            // Sleep for a while to avoid excessive CPU usage, no need to parse if there are lexer errors
            thread::sleep(Duration::from_millis(250));
            continue;
        }

        // if the above is successful, get the parser errors and do the same thing

        let (mut ast, parse_succeeded) = match parse(tokens) {
            Ok(ast) => (ast, true),
            Err(e) => {
                let mut editor = lock(&editor_arc);
                editor.code_errors = vec![localize_span(e.message.clone(), e.code_span, &source_map)];
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
                    editor.code_errors.clear();
                }
                Err(errors) => {
                    // Every checker error keeps its own span so it renders on
                    // its own line; a help suggestion rides along in the message
                    let code_errors: Vec<CodeError> = errors
                        .into_iter()
                        .map(|error| {
                            let message = match &error.help {
                                Some(help) => format!("{} — help: {}", error.message, help),
                                None => error.message,
                            };
                            localize_span(message, error.code_span, &source_map)
                        })
                        .collect();
                    let mut editor = lock(&editor_arc);
                    editor.code_errors = code_errors;
                }
            };
        }

        // Sleep to avoid excessive CPU usage
        thread::sleep(Duration::from_millis(250));
    }
}

static WELCOME_MESSAGE: &str = include_str!("../examples/hello_world.nail");

pub fn create_welcome_message() -> Vec<String> {
    WELCOME_MESSAGE.lines().map(String::from).collect()
}

/// Directories the finder never walks into. Both hold build output measured
/// in tens of thousands of files, none of which anybody opens by hand, and a
/// finder that offers them is worse than one that does not exist. Anything
/// starting with a dot is skipped as well, which covers `.git` and the rest.
const SKIPPED_DIRECTORIES: &[&str] = &["target", "node_modules"];

/// What the finder is willing to list. A Nail project is `.nail` files plus
/// the handful of others a person actually edits beside them, and a binary
/// offered by name is only ever a mistake about to happen.
const LISTED_EXTENSIONS: &[&str] = &["nail", "rs", "txt", "md", "toml", "json", "sh", "html", "css", "js"];

/// A ceiling on how much of a directory tree the finder will hold. It exists
/// so that starting the IDE in a home directory by accident costs a moment
/// rather than the session. The walk is breadth first, so when the ceiling is
/// reached what survives is the part of the tree nearest the project root,
/// which is the part worth having.
const PROJECT_FILE_LIMIT: usize = 20000;

/// Every file under `root` the finder is willing to open, as paths relative
/// to `root`, sorted. Symbolic links to directories are listed but not walked
/// into: a link pointing at an ancestor is a loop, and no project needs the
/// finder to chase one to find out.
pub fn scan_project_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(root.to_path_buf());
    while let Some(directory) = queue.pop_front() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let is_directory = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            if is_directory {
                if !SKIPPED_DIRECTORIES.contains(&name.as_str()) {
                    queue.push_back(entry.path());
                }
                continue;
            }
            let listed = Path::new(&name).extension().map(|extension| LISTED_EXTENSIONS.contains(&extension.to_string_lossy().as_ref())).unwrap_or(false);
            if !listed {
                continue;
            }
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap_or(&path);
            found.push(relative.to_string_lossy().to_string());
            if found.len() >= PROJECT_FILE_LIMIT {
                found.sort();
                return found;
            }
        }
    }
    found.sort();
    return found;
}

/// How well `haystack` answers `needle`, or `None` when it does not contain
/// the needle's letters in order at all. Both are expected in lower case, so
/// that the caller lowers each candidate once rather than once per query.
///
/// The shape being rewarded is the one people actually type: the first
/// letters of the words, and runs of letters they remember exactly. A letter
/// scores on its own, more when it continues the previous match, and more
/// again when it starts a word, and every letter skipped over on the way
/// costs a little. Matching is greedy rather than exhaustive, which can miss
/// the very best alignment in a repetitive path, but it is linear and it is
/// predictable, and both matter more here than the last point of score.
pub fn fuzzy_score(haystack: &str, needle: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }
    let letters: Vec<char> = haystack.chars().collect();
    let mut score = 0;
    let mut searched_from = 0;
    let mut previous: Option<usize> = None;
    for wanted in needle.chars() {
        let at = letters[searched_from..].iter().position(|letter| *letter == wanted)? + searched_from;
        score += 1;
        if previous == Some(at.wrapping_sub(1)) {
            score += 8;
        }
        if at == 0 || matches!(letters[at - 1], '/' | '_' | '-' | '.' | ' ') {
            score += 6;
        }
        let skipped = at - previous.map(|index| index + 1).unwrap_or(0);
        score -= (skipped as i32).min(10);
        previous = Some(at);
        searched_from = at + 1;
    }
    return Some(score);
}

/// How well a path answers a query. The file name is tried on its own and
/// heavily favoured, because a query is nearly always a name rather than a
/// route to one, and a directory that happens to spell the same letters
/// should not bury the file actually being asked for. Longer paths lose a
/// little, so that when two files match equally the nearer one is offered
/// first.
pub fn path_score(relative_path: &str, needle: &str) -> Option<i32> {
    // An empty query is not a ranking. Every file ties, and the order they
    // are already in stands, which is the alphabetical one the walk left.
    if needle.is_empty() {
        return Some(0);
    }
    let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
    let by_name = fuzzy_score(file_name, needle).map(|score| score + 40);
    let by_path = fuzzy_score(relative_path, needle);
    let best = by_name.into_iter().chain(by_path).max()?;
    return Some(best - relative_path.chars().count() as i32 / 8);
}

/// The finder: a query line and the files that answer it, best first. What is
/// typed is always shown, empty or not, because a picker whose prompt appears
/// only once there is something in it gives the user nothing to type at.
fn display_file_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::{Clear, List, ListItem, ListState};

    let area = f.area();
    let width = std::cmp::min(80, area.width.saturating_sub(4));
    let height = std::cmp::min(20, area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let title = if editor.browsing_by_path() {
        " Open file - by path ".to_string()
    } else {
        format!(" Open file - {} of {} ", editor.file_entries.len(), editor.file_index.len())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(editor.theme.success).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(editor.theme.ui_text).bg(editor.theme.ui_panel_bg));
    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let query_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let query = Paragraph::new(format!("> {}", editor.file_dialog_input)).style(Style::default().fg(editor.theme.accent).bg(editor.theme.ui_panel_bg));
    f.render_widget(query, query_area);
    if inner.height < 2 {
        return;
    }

    let items: Vec<ListItem> = editor
        .file_entries
        .iter()
        .map(|entry| {
            let style = if entry.is_directory {
                Style::default().fg(editor.theme.primary)
            } else if entry.is_recent {
                Style::default().fg(editor.theme.accent)
            } else {
                Style::default().fg(editor.theme.ui_text)
            };
            ListItem::new(entry.name.clone()).style(style)
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().fg(editor.theme.ui_text).bg(editor.theme.ui_panel_bg))
        .highlight_style(Style::default().fg(editor.theme.menu_selection_fg).bg(editor.theme.menu_selection_bg));

    let mut list_state = ListState::default();
    list_state.select(Some(editor.file_dialog_index));

    let list_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1);
    f.render_stateful_widget(list, list_area, &mut list_state);
    display_list_scrollbar(f, editor.theme, dialog_area, list_area, editor.file_entries.len(), list_state.offset());
}

fn display_stdlib_dialog(f: &mut Frame, editor: &Editor) {
    use ratatui::widgets::{Clear, List, ListItem, ListState};
    
    // Create the dialog area
    let area = f.area();
    let width = std::cmp::min(100, area.width.saturating_sub(4));
    let height = std::cmp::min(32, area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);
    
    // Clear the area
    f.render_widget(Clear, dialog_area);
    
    // Split dialog into list and detail areas. The detail half holds a whole
    // worked example now, not two lines about a call, so it gets the room.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(14)])
        .split(dialog_area);
    
    // Create function list items
    let items: Vec<ListItem> = editor.stdlib_functions.iter().map(|func| {
        let display = format!("[{}] {}", func.category, func.name);
        ListItem::new(display).style(Style::default().fg(editor.theme.ui_text))
    }).collect();
    
    // Create the list widget
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Standard Library Functions - {} ", editor.stdlib_functions.len()))
        .title_style(Style::default().fg(editor.theme.success).add_modifier(Modifier::BOLD));
    let list_area = list_block.inner(chunks[0]);
    let list = List::new(items)
        .block(list_block)
        .style(Style::default().fg(editor.theme.ui_text).bg(editor.theme.ui_panel_bg))
        .highlight_style(Style::default().fg(editor.theme.menu_selection_fg).bg(editor.theme.menu_selection_bg));

    // Create list state
    let mut list_state = ListState::default();
    list_state.select(Some(editor.stdlib_index));

    f.render_stateful_widget(list, chunks[0], &mut list_state);
    display_list_scrollbar(f, editor.theme, chunks[0], list_area, editor.stdlib_functions.len(), list_state.offset());

    // Show function details, ending with the example, because Enter is about
    // to put that example in the file and nobody should have to guess what
    // arrives.
    if !editor.stdlib_functions.is_empty() && editor.stdlib_index < editor.stdlib_functions.len() {
        let func = &editor.stdlib_functions[editor.stdlib_index];
        let mut details = vec![
            Line::from(vec![
                Span::styled("Signature: ", Style::default().fg(editor.theme.accent)),
                Span::styled(&func.signature, Style::default().fg(editor.theme.ui_text)),
            ]),
            Line::from(vec![
                Span::styled("Description: ", Style::default().fg(editor.theme.accent)),
                Span::styled(&func.description, Style::default().fg(editor.theme.ui_text)),
            ]),
        ];
        if !func.example.is_empty() {
            details.push(Line::from(vec![Span::styled(
                "Enter puts this example in your file:",
                Style::default().fg(editor.theme.accent),
            )]));
            for example_line in func.example.lines() {
                details.push(Line::from(vec![Span::styled(example_line.to_string(), Style::default().fg(editor.theme.ui_text_muted))]));
            }
        }

        let detail_paragraph = Paragraph::new(details)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" Function Details ")
                .title_style(Style::default().fg(editor.theme.primary).add_modifier(Modifier::BOLD))
            )
            .style(Style::default().fg(editor.theme.ui_text).bg(editor.theme.ui_panel_bg))
            .wrap(ratatui::widgets::Wrap { trim: false });

        f.render_widget(detail_paragraph, chunks[1]);
    }
    
    // Show search input if any
    if !editor.stdlib_filter.is_empty() {
        let input_area = Rect::new(dialog_area.x + 1, dialog_area.y + dialog_area.height - 2, dialog_area.width - 2, 1);
        let input_paragraph = Paragraph::new(format!("Filter: {}", editor.stdlib_filter))
            .style(Style::default().fg(editor.theme.accent).bg(editor.theme.ui_panel_bg));
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
        let has_error = editor.code_errors.iter().any(|error| line_number == error.code_span.start_line);

        let (style, line_text) = if has_error {
            // Keep the line number visible, just paint it error-red
            let style = Style::default().fg(editor.theme.danger).add_modifier(Modifier::BOLD);
            let text = format!("{:>width$}", line_number, width = (gutter_area.width - 1) as usize);
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
    let chars: Vec<char> = line.chars().collect();
    // The bound is the number of characters in the line. Checked against its
    // byte count instead, a cursor at the end of a line holding anything but
    // ASCII passed the check and then indexed off the end of this vector.
    if cursor_x >= chars.len() {
        return None;
    }

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
            let line_chars: Vec<char> = search_line.chars().collect();
            let end_x = if line_idx == cursor_y { cursor_x } else { line_chars.len() };
            
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule every copy command serves: the painted frame reads back as
    /// plain text, with styling gone, right-hand padding trimmed, and the
    /// blank rows under the last painted one dropped.
    #[test]
    fn a_painted_frame_reads_back_as_trimmed_text() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 4));
        buffer.set_string(0, 0, "hello", ratatui::style::Style::default());
        buffer.set_string(2, 1, "world", ratatui::style::Style::default());
        assert_eq!(buffer_text(&buffer), "hello\n  world");
    }

    /// The whole debugging flow at once: a file naming an import that does
    /// not exist is lexed, the error lands on the import's own line, and
    /// copying a selection over that line carries the error text with it.
    #[test]
    fn a_bad_import_line_copies_together_with_its_error() {
        let content = "nail latest\nimport(`no_such_folder/nope.nail`)";
        let program = lexer::lex_program(content, None);
        let errors: Vec<CodeError> = lexer::collect_lexer_errors(&program.tokens).into_iter().map(|error| localize_span(error.message, error.code_span, &program.source_map)).collect();
        assert!(!errors.is_empty(), "an import that cannot be read is an error");

        let mut editor = crate::Editor::new();
        editor.tabs = vec![crate::Tab::new_with_file("main.nail".to_string(), content.lines().map(str::to_string).collect())];
        editor.tab_index = 0;
        editor.code_errors = errors;
        let tab = editor.get_current_tab_mut();
        tab.selection_start = Some((0, 1));
        tab.selection_end = Some((10, 1));
        tab.selection_mode = true;

        let (text, count) = editor.selection_with_annotations_text();
        assert_eq!(count, 1, "the one error on the selected line came along");
        assert!(text.contains("Cannot resolve path"), "the copy says what the popup says: {}", text);
        assert!(text.starts_with("import(`no"), "the copy starts with the selected code: {}", text);
    }

    /// Turning the timing display off also keeps timings out of every copy,
    /// because what is copied is exactly what is displayed.
    #[test]
    fn timings_stay_out_of_annotations_when_their_display_is_off() {
        let mut editor = crate::Editor::new();
        editor.profile_data = Some(ProfileData {
            source_hash: "abc".to_string(),
            wall_nanos: 1_000_000,
            functions: vec![ProfiledFunction { name: "double".to_string(), calls: 1, total_nanos: 400_000, max_nanos: 400_000 }],
        });
        let mut decl_lines = HashMap::new();
        decl_lines.insert("double".to_string(), 0usize);
        let cache = (0u64, "abc".to_string(), decl_lines);
        editor.show_timings = true;
        assert!(build_line_annotations(&editor, Some(&cache)).contains_key(&1), "a timing for a function on line one");
        editor.show_timings = false;
        assert!(build_line_annotations(&editor, Some(&cache)).is_empty(), "no timing survives the display being off");
    }

    /// Draws a scrollbar on its own and hands back the column it drew in, so
    /// a test can say what the user would see on the right edge of a list.
    fn scrollbar_column(box_width: u16, list_height: u16, rows: usize, offset: usize) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal = Terminal::new(TestBackend::new(box_width, list_height + 2)).expect("a test terminal");
        let box_area = Rect::new(0, 0, box_width, list_height + 2);
        let list_area = Rect::new(1, 1, box_width.saturating_sub(2), list_height);
        terminal
            .draw(|f| display_list_scrollbar(f, &crate::colorizer::DARK_THEME, box_area, list_area, rows, offset))
            .expect("the frame draws");

        let buffer = terminal.backend().buffer().clone();
        let mut column = String::new();
        for y in list_area.y..list_area.y + list_area.height {
            column.push_str(buffer[(box_width - 1, y)].symbol());
        }
        return column;
    }

    /// Draws a whole dialog into a fixed size terminal and hands back its
    /// lines, so a test can read what a user would have seen.
    fn dialog_lines(width: u16, height: u16, draw: impl FnOnce(&mut Frame, &crate::Editor)) -> Vec<String> {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let editor = crate::Editor::new();
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
        terminal.draw(|f| draw(f, &editor)).expect("the frame draws");

        let buffer = terminal.backend().buffer().clone();
        let mut lines = Vec::new();
        for y in 0..height {
            let mut line = String::new();
            for x in 0..width {
                line.push_str(buffer[(x, y)].symbol());
            }
            lines.push(line);
        }
        return lines;
    }

    /// The palette is the list the complaint was about: it holds far more
    /// commands than a window shows, and without a bar on its edge the ones
    /// below the fold may as well not exist.
    #[test]
    fn the_command_palette_shows_that_it_has_more_rows_than_it_drew() {
        let rows: Vec<(String, String)> = (0..60).map(|number| (format!("Command {number}"), format!("Ctrl+{number}"))).collect();
        let lines = dialog_lines(80, 40, |f, editor| display_picker(f, editor, " Commands ", "", &rows, 0));

        let thumbs = lines.iter().filter(|line| line.contains('█')).count();
        assert!(thumbs > 0, "the palette draws a scrollbar thumb:\n{}", lines.join("\n"));

        let short: Vec<(String, String)> = rows.iter().take(3).cloned().collect();
        let fits = dialog_lines(80, 40, |f, editor| display_picker(f, editor, " Commands ", "", &short, 0));
        assert!(fits.iter().all(|line| !line.contains('█')), "three rows need no scrollbar:\n{}", fits.join("\n"));
    }

    /// The standard library browser is the longest list in the editor by a
    /// wide margin, and the window shows a couple of dozen of it. The bar and
    /// the count in the title are the two things saying so.
    #[test]
    fn the_standard_library_browser_says_how_long_its_list_is() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut editor = crate::Editor::new();
        editor.open_stdlib_browser();
        assert!(editor.stdlib_functions.len() > 40, "the registry has more functions than one window holds");

        let mut terminal = Terminal::new(TestBackend::new(110, 40)).expect("a test terminal");
        terminal.draw(|f| display_stdlib_dialog(f, &editor)).expect("the frame draws");

        let buffer = terminal.backend().buffer().clone();
        let mut drawn = String::new();
        for y in 0..40 {
            for x in 0..110 {
                drawn.push_str(buffer[(x, y)].symbol());
            }
            drawn.push('\n');
        }

        assert!(drawn.contains('█'), "the browser draws a scrollbar thumb:\n{drawn}");
        assert!(drawn.contains(&editor.stdlib_functions.len().to_string()), "the title carries the count:\n{drawn}");
    }

    /// A list that fits says nothing. Drawing a full-height thumb over every
    /// short list would make the mark meaningless in the case it is for.
    #[test]
    fn a_list_that_fits_gets_no_scrollbar() {
        assert_eq!(scrollbar_column(20, 5, 5, 0), "     ");
        assert_eq!(scrollbar_column(20, 5, 3, 0), "     ");
    }

    /// The whole point: five rows on screen out of forty has to look like
    /// five rows out of forty, or the other thirty five are never looked for.
    #[test]
    fn a_list_that_runs_past_its_window_shows_where_it_is() {
        let at_top = scrollbar_column(20, 5, 40, 0);
        assert!(at_top.starts_with('█'), "the thumb sits at the top: {at_top}");
        assert!(at_top.contains('│'), "the track is visible below it: {at_top}");

        let at_bottom = scrollbar_column(20, 5, 40, 35);
        assert!(at_bottom.ends_with('█'), "the thumb sits at the bottom: {at_bottom}");
        assert!(at_bottom.starts_with('│'), "the track is visible above it: {at_bottom}");

        let in_the_middle = scrollbar_column(20, 5, 40, 18);
        assert!(in_the_middle.starts_with('│') && in_the_middle.ends_with('│'), "track on both sides: {in_the_middle}");
        assert!(in_the_middle.contains('█'), "the thumb is somewhere between: {in_the_middle}");
    }

    #[test]
    fn config_read_ignores_keys_that_only_share_a_prefix() {
        let text = "theme=light\nbuild_code_nanos=42\n";
        assert_eq!(config_value_in(text, "theme"), Some("light".to_string()));
        assert_eq!(config_value_in(text, "build"), None);
        assert_eq!(config_value_in(text, "build_code_nanos"), Some("42".to_string()));
        assert_eq!(config_value_in(text, "missing"), None);
    }

    #[test]
    fn config_write_keeps_lines_it_does_not_own() {
        let existing = "theme=light\nbuild_deps_nanos=900\n";
        let written = config_text_with(existing, &[("build_code_nanos", "42".to_string())]);
        assert_eq!(config_value_in(&written, "theme"), Some("light".to_string()));
        assert_eq!(config_value_in(&written, "build_deps_nanos"), Some("900".to_string()));
        assert_eq!(config_value_in(&written, "build_code_nanos"), Some("42".to_string()));
    }

    #[test]
    fn config_write_replaces_in_place_rather_than_appending_a_duplicate() {
        let written = config_text_with("theme=light\nbuild_code_nanos=42\n", &[("theme", "dark".to_string())]);
        assert_eq!(written, "theme=dark\nbuild_code_nanos=42\n");
    }

    #[test]
    fn config_write_starts_a_file_that_does_not_exist_yet() {
        assert_eq!(config_text_with("", &[("theme", "dark".to_string())]), "theme=dark\n");
    }

    /// The old config writer left no trailing newline, so a file saved by an
    /// earlier version must still read and grow correctly.
    fn legacy_file_without_trailing_newline() -> &'static str {
        "theme=light"
    }

    #[test]
    fn config_write_upgrades_a_file_from_the_old_writer() {
        let written = config_text_with(legacy_file_without_trailing_newline(), &[("build_code_nanos", "42".to_string())]);
        assert_eq!(written, "theme=light\nbuild_code_nanos=42\n");
    }

    #[test]
    fn a_query_that_is_not_in_the_name_at_all_does_not_match() {
        assert_eq!(fuzzy_score("website.nail", "zzz"), None);
    }

    #[test]
    fn an_empty_query_matches_everything_equally() {
        assert_eq!(fuzzy_score("website.nail", ""), Some(0));
        assert_eq!(path_score("examples/website.nail", ""), Some(0));
        assert_eq!(path_score("a.nail", ""), path_score("examples/deeply/nested/thing.nail", ""));
    }

    #[test]
    fn typing_the_start_of_a_name_beats_the_same_letters_scattered() {
        let exact = path_score("tests/parser.nail", "parser").unwrap();
        let scattered = path_score("tests/pretty_arrays_error.nail", "parser").unwrap();
        assert!(exact > scattered, "exact {} should beat scattered {}", exact, scattered);
    }

    #[test]
    fn the_file_name_counts_for_more_than_the_directories_above_it() {
        let in_name = path_score("src/website.nail", "website").unwrap();
        let in_directory = path_score("website/other_thing.nail", "website").unwrap();
        assert!(in_name > in_directory, "name {} should beat directory {}", in_name, in_directory);
    }

    #[test]
    fn the_nearer_of_two_equal_matches_is_offered_first() {
        let near = path_score("main.nail", "main").unwrap();
        let far = path_score("examples/deeply/nested/main.nail", "main").unwrap();
        assert!(near > far, "near {} should beat far {}", near, far);
    }

    #[test]
    fn initials_reach_a_path_nobody_wants_to_type_out() {
        assert!(path_score("examples/website/page_sections.nail", "ewps").is_some());
    }

    /// Presses a key the way the key loop does, minus the parts that need a
    /// terminal: the bindings resolve it, a prefix waits for the key after it,
    /// and an action runs. Text input and the Escape cascade are the loop's
    /// own and are not reachable from here.
    fn press(editor: &mut Editor, code: ratatui::crossterm::event::KeyCode, modifiers: KeyModifiers) {
        use ratatui::crossterm::event::KeyEvent;
        let pending = editor.pending_prefix.take();
        let key = KeyEvent::new(code, modifiers);
        match nail::keymap::resolve(editor.keymap, vim_mode_for_dialog(editor), pending, key) {
            Resolution::Pending(prefix) => editor.pending_prefix = Some(prefix),
            Resolution::Run(action) => apply_action(editor, action),
            Resolution::Swallowed | Resolution::Unbound => {}
        }
    }

    fn typing(editor: &mut Editor, keys: &str) {
        for letter in keys.chars() {
            let modifiers = if letter.is_uppercase() { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
            press(editor, ratatui::crossterm::event::KeyCode::Char(letter), modifiers);
        }
    }

    fn vim_editor(lines: &[&str]) -> Editor {
        let mut editor = Editor::new_with_debug(false);
        editor.tabs = vec![crate::Tab::new_with_file("test.nail".to_string(), lines.iter().map(|line| line.to_string()).collect())];
        editor.tab_index = 0;
        editor.keymap = nail::keymap::Keymap::Vim;
        editor.vim_mode = VimMode::Normal;
        return editor;
    }

    /// An operator and the motion after it are two key presses that make one
    /// edit, which is the only thing in the editor that works that way.
    #[test]
    fn an_operator_and_its_motion_make_one_edit() {
        let mut editor = vim_editor(&["one", "two", "three"]);
        typing(&mut editor, "j");
        typing(&mut editor, "dd");
        assert_eq!(editor.get_current_tab().content, vec!["one", "three"]);
        assert_eq!(editor.vim_mode, VimMode::Normal);
        // The prefix is spent by the key after it, so the second d of a dd is
        // not still waiting once the line is gone.
        assert!(editor.pending_prefix.is_none());
    }

    #[test]
    fn opening_a_line_leaves_the_editor_in_insert_mode() {
        let mut editor = vim_editor(&["    one"]);
        typing(&mut editor, "o");
        assert_eq!(editor.vim_mode, VimMode::Insert);
        assert_eq!(editor.get_current_tab().content, vec!["    one", "    "]);
        // The letters that were commands a moment ago are letters again, so
        // the one that deletes a line no longer deletes anything.
        typing(&mut editor, "dd");
        assert_eq!(editor.get_current_tab().content, vec!["    one", "    "]);
    }

    /// Visual mode is the mark under another name, which is what makes every
    /// motion after it extend the selection.
    #[test]
    fn visual_mode_grows_the_selection_as_the_cursor_moves() {
        let mut editor = vim_editor(&["one", "two", "three"]);
        typing(&mut editor, "vj");
        assert_eq!(editor.vim_mode, VimMode::Visual);
        assert!(editor.has_selection());
        assert_eq!(editor.get_current_tab().selection_start, Some((0, 0)));
        assert_eq!(editor.get_current_tab().selection_end, Some((0, 1)));

        // Pressing it again is how vim turns it off, and the selection goes
        // with the mode.
        typing(&mut editor, "v");
        assert_eq!(editor.vim_mode, VimMode::Normal);
        assert!(!editor.has_selection());
    }

    #[test]
    fn a_line_selection_covers_whole_lines_from_the_first_key() {
        let mut editor = vim_editor(&["one", "two", "three"]);
        typing(&mut editor, "V");
        assert_eq!(editor.get_current_tab().selection_start, Some((0, 0)));
        assert_eq!(editor.get_current_tab().selection_end, Some((3, 0)));
        typing(&mut editor, "j");
        assert_eq!(editor.get_current_tab().selection_end, Some((3, 1)));
    }

    /// The matches used to be dropped when the find box closed, which left
    /// both `n` and F3 doing nothing at the exact moment a user reaches for
    /// them. The phrase outlives the box now, in every keymap.
    #[test]
    fn a_search_outlives_the_box_it_was_typed_into() {
        let mut editor = vim_editor(&["one two", "one three", "one four"]);
        editor.search_query = "one".to_string();
        editor.dialog_mode = crate::DialogMode::Find;
        editor.find_all_matches();
        assert_eq!(editor.cursor_position(), (0, 0));

        editor.close_dialog();
        // Vim leaves the matches lit, because `n` is one key away.
        assert!(!editor.search_results.is_empty());
        typing(&mut editor, "n");
        assert_eq!(editor.cursor_position(), (0, 1));
        typing(&mut editor, "N");
        assert_eq!(editor.cursor_position(), (0, 0));
    }

    #[test]
    fn finding_again_works_after_the_matches_have_been_dropped() {
        let mut editor = vim_editor(&["one two", "one three"]);
        editor.keymap = nail::keymap::Keymap::Cua;
        editor.search_query = "one".to_string();
        editor.dialog_mode = crate::DialogMode::Find;
        editor.find_all_matches();

        editor.close_dialog();
        // Every keymap but vim treats closing the box as the end of the
        // search, so the highlighting goes.
        assert!(editor.search_results.is_empty());
        apply_action(&mut editor, Action::FindNext);
        assert_eq!(editor.cursor_position(), (0, 1));
    }

    /// Putting the highlighting out is not the same as forgetting what was
    /// searched for, which is what makes `n` still work afterwards.
    #[test]
    fn clearing_the_highlight_keeps_the_phrase() {
        let mut editor = vim_editor(&["one two", "one three"]);
        editor.search_query = "one".to_string();
        editor.find_all_matches();
        apply_action(&mut editor, Action::ClearSearchHighlight);
        assert!(editor.search_results.is_empty());
        assert_eq!(editor.search_query, "one");
        typing(&mut editor, "n");
        assert_eq!(editor.cursor_position(), (0, 1));
    }

    /// Highlighting stays where it was put and the text under it does not, so
    /// the key loop puts the highlighting out after any key that changed the
    /// text. This is the reading it decides that on: it has to move for an
    /// edit and stay put for a motion, or the marks go out while a user is
    /// walking between them.
    #[test]
    fn the_edit_marker_moves_for_an_edit_and_not_for_a_motion() {
        let mut editor = vim_editor(&["one two", "one three"]);
        let untouched = editor.edit_marker();

        typing(&mut editor, "jw");
        assert_eq!(editor.edit_marker(), untouched, "a motion is not an edit");
        typing(&mut editor, "0");
        assert_eq!(editor.edit_marker(), untouched, "nor is going to the line start");

        typing(&mut editor, "x");
        assert_ne!(editor.edit_marker(), untouched, "deleting a character is");
        assert_eq!(editor.get_current_tab().content[1], "ne three");

        let after_deleting = editor.edit_marker();
        typing(&mut editor, "dd");
        assert_ne!(editor.edit_marker(), after_deleting, "so is deleting a line");
    }

    /// A key that cannot be honoured says why, because silence reads as a
    /// dropped keystroke and gets pressed again.
    #[test]
    fn a_key_with_nothing_behind_it_says_so() {
        let mut editor = vim_editor(&["one"]);
        press(&mut editor, ratatui::crossterm::event::KeyCode::Char('v'), KeyModifiers::CONTROL);
        assert!(matches!(&editor.build_status, BuildStatus::Complete(message) if message.contains("Blockwise")));
        assert_eq!(editor.get_current_tab().content, vec!["one"]);
    }

    #[test]
    fn control_bracket_puts_the_mode_and_the_selection_back() {
        let mut editor = vim_editor(&["one", "two"]);
        typing(&mut editor, "vj");
        press(&mut editor, ratatui::crossterm::event::KeyCode::Char('['), KeyModifiers::CONTROL);
        assert_eq!(editor.vim_mode, VimMode::Normal);
        assert!(!editor.has_selection());
        assert!(!editor.mark_active);
    }

    /// The minimap draws each cell as braille dots in the syntax color of
    /// the lines it stands for, which is what makes a comment block, a
    /// string run and a stretch of code tell apart at a glance. The two
    /// regressions this pins against are a single flat color, which says
    /// nothing about where in the file you are, and solid block glyphs,
    /// which read as slabs of ink rather than small print.
    #[test]
    fn the_minimap_wears_the_syntax_colors_of_the_lines_it_condenses() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut editor = crate::Editor::new();
        // Eight lines over a two-row map put one line on each braille
        // dot-row: the first cell row is all comment, the second all code.
        let mut content = vec!["// a comment long enough to fill a cell".to_string(); 4];
        content.extend(vec!["not_a_comment_just_plain_code".to_string(); 4]);
        editor.get_current_tab_mut().content = content;
        editor.view = crate::ViewLayout { tabs: Rect::default(), text: Rect::new(0, 0, 40, 10), minimap: Rect::new(40, 0, 15, 2) };

        let mut colors = ColorizeCache::new();
        colors.colorize(&editor.get_current_tab().content, &editor.theme);

        let mut terminal = Terminal::new(TestBackend::new(15, 2)).expect("a test terminal");
        terminal.draw(|f| render_minimap(f, &editor, &colors, Rect::new(0, 0, 15, 2))).expect("the frame draws");
        let buffer = terminal.backend().buffer().clone();

        // Four comment lines fill all eight dots of the first cell, and the
        // cell wears the comment color.
        let comment_cell = &buffer[(0u16, 0u16)];
        assert_eq!(comment_cell.symbol(), "⣿", "a full slice lights every dot");
        assert_eq!(comment_cell.fg, editor.theme.comment, "the comment rows wear the comment color");

        // The code rows below wear some other token color.
        let code_cell = &buffer[(0u16, 1u16)];
        assert_ne!(code_cell.fg, editor.theme.comment, "the code rows wear a different color");

        // Past the end of every line the cell is empty, and its background
        // is the current-line grey because these lines are the ones on
        // screen.
        let empty = &buffer[(14u16, 0u16)];
        assert_eq!(empty.symbol(), " ");
        assert_eq!(empty.bg, editor.theme.current_line_bg, "the on-screen band sits on the current-line grey");
    }

    #[test]
    fn the_walk_skips_build_output_and_hidden_directories() {
        let root = std::env::temp_dir().join("nail_finder_walk_test");
        let _ = fs::remove_dir_all(&root);
        for directory in ["src", "target", ".git", "src/deep"] {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        for file in ["src/main.nail", "src/deep/buried.nail", "target/generated.rs", ".git/config.toml", "src/.hidden.nail", "src/program.bin"] {
            fs::write(root.join(file), "").unwrap();
        }
        let found = scan_project_files(&root);
        assert_eq!(found, vec!["src/deep/buried.nail".to_string(), "src/main.nail".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }
}
