use crate::checker::checker;
use crate::parser::parse;
use crate::parser::ASTNode;
use crate::transpilier::Transpiler;
use crate::CodeError;
use crate::Editor;
use log::error;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
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
    style::{Color, Style},
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
            let file_title = if editor.modified {
                format!("FILES [*] - Press Ctrl+S to save")
            } else {
                "FILES".to_string()
            };
            let tabs = Tabs::new(titles)
                .block(Block::default().borders(Borders::ALL).title(file_title))
                .select(editor.tab_index)
                .style(Style::default().fg(editor.theme.default))
                .highlight_style(Style::default().fg(editor.theme.operator));
            f.render_widget(tabs, chunks[0]);

            // Create a horizontal layout for the main content area
            let content_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Min(0), Constraint::Length(1)].as_ref()).split(chunks[1]);

            // Render main content

            let visible_lines = if chunks[1].height > 2 {
                chunks[1].height as usize - 2
            } else {
                0
            };

            let visible_content: Vec<Line> = editor
                .content
                .iter()
                .skip(editor.scroll_position as usize)
                .take(visible_lines)
                .map(|line| Line::from(vec![Span::styled(line.clone(), Style::default().fg(editor.theme.default).bg(editor.theme.background))]))
                .collect();

            let visible_content: Vec<Line> = colorize_code(visible_content, &editor.theme);

            let paragraph = Paragraph::new(visible_content).block(Block::default().borders(Borders::ALL).title("NAIL")).style(Style::default().bg(editor.theme.background).fg(editor.theme.default));

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
            log::info!("Cursor position calculation - editor.cursor_y: {}, editor.cursor_x: {}, scroll: {}, cursor_y: {}", 
                editor.cursor_y, editor.cursor_x, editor.scroll_position, cursor_y);
            
            if cursor_y < content_layout[0].height.saturating_sub(2) as usize {
                let cursor_pos = Position { 
                    x: content_layout[0].x + editor.cursor_x as u16 + 1, 
                    y: content_layout[0].y + cursor_y as u16 + 1 
                };
                log::info!("Setting cursor position: {:?}, content_layout[0]: {:?}", cursor_pos, content_layout[0]);
                f.set_cursor_position(cursor_pos);
            }

            // Always display status
            display_build_status(f, &editor);

            // Check and draw errors
            if let Some(error) = &editor.code_error {
                display_error(f, error, &editor, content_layout[0]);
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
    
    let build_status = Line::from(vec![Span::styled(
        status_text,
        Style::default().fg(editor.theme.default),
    )]);

    let build_status_width = build_status.width() as u16;

    let paragraph = Paragraph::new(build_status).style(Style::default().fg(editor.theme.default).bg(editor.theme.background)).alignment(Alignment::Right);

    let status_width = build_status_width;
    let status_height = 1;
    let status_area = Rect::new(f.area().width.saturating_sub(status_width), 0, status_width, status_height);
    
    log::info!("Build status area: {:?}, frame area: {:?}", status_area, f.area());
    
    // Check if status area is within frame bounds
    if status_area.x + status_area.width > f.area().width || 
       status_area.y + status_area.height > f.area().height {
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

    log::info!("Displaying error - line: {}, column: {}, message: {}", 
        error.code_span.start_line, error_column, error_message);

    // Only display the error if it's within the visible area

    let error_message = Line::from(vec![Span::styled(error_message, Style::default().fg(editor.theme.error).bg(editor.theme.background))]);

    let paragraph = Paragraph::new(error_message.clone()).style(Style::default().fg(editor.theme.error).bg(editor.theme.background)).alignment(Alignment::Left);

    let error_area = Rect::new(
        content_area.x + error_column as u16,
        content_area.y + error_line as u16 + 1, // +1 for the border
        error_message.width() as u16,
        1,
    );
    
    log::info!("Error area: {:?}, content_area: {:?}, frame area: {:?}", 
        error_area, content_area, f.area());
    
    // Check if error area is within frame bounds
    if error_area.x + error_area.width > f.area().width || 
       error_area.y + error_area.height > f.area().height {
        log::warn!("Error area exceeds frame bounds, skipping render");
        return;
    }
    
    f.render_widget(Clear, error_area);
    f.render_widget(paragraph, error_area);
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
                            KeyCode::Char(c) => editor.insert_char(c),
                            KeyCode::Up => editor.move_cursor_up(),
                            KeyCode::Down => editor.move_cursor_down(),
                            KeyCode::PageDown => editor.scroll_down(),
                            KeyCode::PageUp => editor.scroll_up(),
                            KeyCode::Tab => editor.next_tab(),
                            KeyCode::BackTab => editor.previous_tab(),
                            KeyCode::Backspace => editor.delete_char(),
                            KeyCode::Enter => editor.insert_newline(),
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
                Ok(ast) => {
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
                .arg("run")
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

        let mut ast = match parse(tokens) {
            Ok(ast) => ast,
            Err(e) => {
                let mut editor = lock(&editor_arc);
                editor.code_error = Some(CodeError { message: format!("^ {}", e.message), code_span: e.code_span });
                ASTNode::default()
            }
        };

        let _ = match checker(&mut ast) {
            Ok(_) => {}
            Err(errors) => {
                let mut editor = lock(&editor_arc);
                editor.code_error = Some(CodeError { message: format!("^ {}", errors[0].message), code_span: errors[0].code_span.clone() });
            }
        };

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
    Nail = { path = ".." }

    # Binary target for the project
    [[bin]]
    name = "nail_transpilation"
    path = "src/main.rs"
    "#
    .to_string()
}

static WELCOME_MESSAGE: &str = r#"// Welcome to NAIL - Simple, Safe, Parallel Programming!
// Press F7 to compile & run, F6 to toggle theme, Ctrl+C to exit
// Use backticks for strings: `like this`

// === STRUCTS - Custom Data Types ===
struct Player {
    player_name:s,
    health:i,
    level:i
}

player:Player = Player {
    player_name: `Hero`,
    health: 100,
    level: 1
};

// === ENUMS - Choice Types ===
enum Status {
    Active,
    Paused,
    Stopped
}

current:Status = Status::Active;

// === ERROR HANDLING - Safe by Default ===
fn divide(num:i, den:i):i!e {
    if {
        den == 0 => { r e(`Cannot divide by zero!`); },
        else => { r num / den; }
    }
}

// safe() function handles errors from i!e types
fn safe(result:i!e, error_handler:s):i {
    // This would be implemented in the transpiler
    // For now, just show the function signature
    r 42; // Placeholder
}

// Handle errors gracefully with safe()
result:i = safe(divide(10, 2), `default_error_message`);
result_msg:a:s = [`10 / 2 = `, to_string(result)];
print(string_concat(result_msg));

// === BASIC TYPES ===
name:s = `Alice`;
age:i = 25;
score:f = 95.7;

// === FUNCTIONS ===
fn greet(person:s):s {
    parts:a:s = [`Hello, `, person, `!`];
    r string_concat(parts);
}

print(greet(name));

// === PARALLEL PROCESSING - Nail's Superpower! ===
parallel {
    task1:s = to_string(42);
    task2:i = time_now();
    print(`Running in parallel!`);
    fast_calc:i = 100 * 50;
}

// === ARRAYS ===
numbers:a:i = [10, 20, 30, 40, 50];
names:a:s = [`Alice`, `Bob`, `Charlie`];

// === FUNCTIONAL OPERATIONS (No loops in Nail!) ===
// Generate a range
nums:a:i = range(1, 5);  // [1, 2, 3, 4, 5]

// Helper functions for functional operations
fn double_func(n:i):i { r n * 2; }
fn is_even_func(n:i):b { 
    r n % 2 == 0; 
}
fn add_func(acc:i, n:i):i { r acc + n; }
fn square_func(n:i):i { r n * n; }

// Map - transform each element
doubled:a:i = map_int(nums, double_func);

// Filter - keep only matching elements  
evens:a:i = filter_int(nums, is_even_func);

// Reduce - combine all elements
sum:i = reduce_int(nums, 0, add_func);
sum_msg:a:s = [`Sum 1-5: `, to_string(sum)];
print(string_concat(sum_msg));

// Chain operations - sum of squares
sum_squares:i = reduce_int(
    map_int(nums, square_func),
    0,
    add_func
);
squares_msg:a:s = [`Sum of squares: `, to_string(sum_squares)];
print(string_concat(squares_msg));

// === CONTROL FLOW ===
if {
    current == Status::Active => {
        print(`System is active`);
    },
    else => {
        print(`System inactive`);
    }
}

// More Functions
current_time:i = time_now();
square_root:f = math_sqrt(16.0);

// Print results
print(`Welcome to Nail programming!`);
array_length:i = array_len(numbers);
print(to_string(array_length));
print(to_string(square_root));

// Comments work everywhere!
final_message:s = `Nail makes parallel programming easy!`; // Inline comment

// Ready to code? Clear this and write your own Nail program!
// Try experimenting with structs, enums, and parallel blocks!"#;

// static WELCOME_MESSAGE: &str = r#"
// fn print(message:s):s {
//     R{ println!("{}", message); }
// }"#;

pub fn create_welcome_message() -> Vec<String> {
    WELCOME_MESSAGE.lines().map(String::from).collect()
}
