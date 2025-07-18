mod checker;
mod colorizer;
mod common;
mod formatter;
mod lexer;
mod parser;
mod statics_for_tests;
mod stdlib_registry;
mod stdlib_types;
mod transpilier;
mod utils;
use crate::colorizer::ColorScheme;
use crate::utils::create_welcome_message;
use crate::utils::lex_and_parse_thread_logic;
use std::backtrace::Backtrace;
use std::panic;
use crate::colorizer::LIGHT_THEME;

use crate::utils::build_thread_logic;

use crate::colorizer::DARK_THEME;
use crate::utils::draw_thread_logic;
use crate::utils::key_thread_logic;
use crate::utils::resize_thread_logic;
use crate::utils::EditorMessage;
use env_logger::Builder;

use log::error;
use log::LevelFilter;

use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;

use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use crate::utils::lock;
use crate::utils::BuildStatus;

use crate::common::CodeSpan;
use ratatui::crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs},
    Frame, Terminal,
};


#[derive(Debug, Clone, PartialEq)]
pub struct CodeError {
    code_span: CodeSpan,
    message: String,
}

impl Default for CodeError {
    fn default() -> Self {
        CodeError { message: "UNKNOWN ERROR".to_string(), code_span: CodeSpan::default() }
    }
}

impl From<String> for CodeError {
    fn from(error: String) -> Self {
        CodeError { message: error, code_span: CodeSpan::default() }
    }
}

struct Editor {
    theme: &'static ColorScheme,
    content: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    build_status: BuildStatus,
    code_error: Option<CodeError>,
    tokens: Vec<lexer::Token>,
    scroll_state: ScrollbarState,
    scroll_position: u16,
    tab_index: usize,
    current_file: Option<String>,
    modified: bool,
}

impl Editor {
    fn new() -> Editor {
        Editor {
            theme: &DARK_THEME,
            content: create_welcome_message(),
            cursor_x: 0,
            cursor_y: 0,
            build_status: BuildStatus::Idle,
            code_error: None,
            tokens: Vec::new(),
            scroll_state: ScrollbarState::default(),
            scroll_position: 0,
            tab_index: 0,
            current_file: None,
            modified: false,
        }
    }

    fn delete_char(&mut self) {
        if self.cursor_x > 0 {
            self.content[self.cursor_y].remove(self.cursor_x - 1);
            self.cursor_x -= 1;
            self.modified = true;
        } else if self.cursor_y > 0 {
            let current_line = self.content.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.content[self.cursor_y].len();
            self.content[self.cursor_y].push_str(&current_line);
            self.modified = true;
        }
    }

    fn insert_char(&mut self, c: char) {
        if self.cursor_y >= self.content.len() {
            self.content.push(String::new());
        }

        let line = &mut self.content[self.cursor_y];
        if self.cursor_x > line.len() {
            line.push_str(&" ".repeat(self.cursor_x - line.len()));
        }

        line.insert(self.cursor_x, c);
        self.cursor_x += 1;
        self.modified = true;
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.content[self.cursor_y].len();
        }
    }

    fn move_cursor_right(&mut self) {
        let current_line_len = self.content[self.cursor_y].len();
        if self.cursor_x < current_line_len {
            self.cursor_x += 1;
        } else if self.cursor_y < self.content.len() - 1 {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            let upper_line_len = self.content[self.cursor_y].len();
            self.cursor_x = self.cursor_x.min(upper_line_len);
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_y < self.content.len() - 1 {
            self.cursor_y += 1;
            let lower_line_len = self.content[self.cursor_y].len();
            self.cursor_x = self.cursor_x.min(lower_line_len);
        }
    }

    fn insert_newline(&mut self) {
        let remaining = self.content[self.cursor_y].split_off(self.cursor_x);
        self.cursor_y += 1;
        self.content.insert(self.cursor_y, remaining);
        self.cursor_x = 0;
        self.modified = true;
    }

    fn toggle_theme(&mut self) {
        self.theme = if *self.theme == *LIGHT_THEME { &*DARK_THEME } else { &*LIGHT_THEME };

        let _ = self.save_config();
    }

    fn set_theme(&mut self, theme: &str) {
        self.theme = match theme {
            "light" => &LIGHT_THEME,
            "dark" => &DARK_THEME,
            _ => &DARK_THEME,
        };
        let _ = self.save_config();
    }

    fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
        self.scroll_state = self.scroll_state.position(self.scroll_position as usize);
    }

    fn scroll_down(&mut self) {
        self.scroll_position = self.scroll_position.saturating_add(1);
        self.scroll_state = self.scroll_state.position(self.scroll_position as usize);
    }

    fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % 4; // Assuming 4 tabs
    }

    fn previous_tab(&mut self) {
        self.tab_index = (self.tab_index + 3) % 4; // Assuming 4 tabs
    }
    
    fn save_file(&mut self) -> io::Result<()> {
        // Format the code before saving
        log::info!("Formatting code before save...");
        self.format_code();
        
        if let Some(filename) = self.current_file.clone() {
            // Write to file
            let content = self.content.join("\n");
            fs::write(&filename, content)?;
            self.modified = false;
            log::info!("Saved file: {}", filename);
        } else {
            // For now, save as example.nail if no filename
            let filename = "example.nail";
            let content = self.content.join("\n");
            fs::write(filename, content)?;
            self.current_file = Some(filename.to_string());
            self.modified = false;
            log::info!("Saved new file as: {}", filename);
        }
        Ok(())
    }
    
    fn format_code(&mut self) {
        use crate::formatter::format_nail_code;
        
        // Format all lines with proper indentation
        let original_content = self.content.clone();
        self.content = format_nail_code(&original_content);
        
        // Log changes
        for (i, (orig, formatted)) in original_content.iter().zip(&self.content).enumerate() {
            if orig != formatted {
                log::debug!("Formatted line {}: '{}' -> '{}'", i, orig, formatted);
            }
        }
    }

    fn save_config(&self) -> io::Result<()> {
        let home_dir = env::current_dir().expect("Could not get the directory that is running Nail to save configuration");
        let config_path = PathBuf::from(home_dir).join(".nail");

        // Debugging print to check file path
        log::info!("Saving configuration to {:?}", config_path);

        let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).open(&config_path)?;

        let theme = format!(
            "theme={}",
            match self.theme {
                x if x == &*LIGHT_THEME => "light",
                _ => "dark",
            }
        );

        file.write_all(theme.as_bytes())?;
        Ok(())
    }

    fn load_config() -> String {
        let home_dir = env::current_dir().expect("Could not get the directory that is running Nail to save configuration");
        let config_path = PathBuf::from(home_dir).join(".nail");

        // Debugging print to check file path
        log::info!("Loading configuration from {:?}", config_path);

        if let Ok(config_data) = fs::read_to_string(&config_path) {
            for line in config_data.lines() {
                if line.starts_with("theme=") {
                    return line["theme=".len()..].to_string();
                }
            }
        }

        "dark".to_string()
    }
}

fn main() -> Result<(), io::Error> {
    let log_file = File::create("nail.log").expect("Failed to create log file");
    Builder::new().target(env_logger::Target::Pipe(Box::new(log_file))).filter_level(LevelFilter::Debug).init();

    panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::capture();
        error!("Panic occurred: {:?}", panic_info);
        error!("Backtrace:\n{:?}", backtrace);
    }));

    let (tx_resize, rx_resize) = channel::<EditorMessage>();
    let (tx_draw, rx_draw) = channel::<EditorMessage>();
    let (tx_key, rx_key) = channel::<EditorMessage>();
    let (tx_main, rx_main) = channel::<EditorMessage>();
    let (tx_build, rx_build) = channel::<EditorMessage>();
    let (tx_code_error, rx_code_error) = channel::<EditorMessage>();

    log::info!("Initializing terminal...");
    
    if let Err(e) = enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {}", e);
        return Err(e.into());
    }
    log::info!("Raw mode enabled successfully");
    
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        eprintln!("Failed to setup terminal screen: {}", e);
        let _ = disable_raw_mode();
        return Err(e.into());
    }
    log::info!("Terminal screen setup successfully");
    
    let backend = CrosstermBackend::new(stdout);
    let terminal = match Terminal::new(backend) {
        Ok(t) => {
            // Get initial terminal size
            match t.size() {
                Ok(size) => {
                    log::info!("Initial terminal size: {:?}", size);
                },
                Err(e) => {
                    log::error!("Failed to get terminal size: {}", e);
                }
            }
            t
        },
        Err(e) => {
            eprintln!("Failed to create terminal: {}", e);
            let _ = disable_raw_mode();
            return Err(e.into());
        }
    };
    log::info!("Terminal created successfully");

    let mut editor = Editor::new();
    let theme = Editor::load_config();
    editor.set_theme(&theme);

    let editor_arc = Arc::new(Mutex::new(editor));
    let terminal_arc = Arc::new(Mutex::new(terminal));
    
    log::info!("Starting UI threads...");

    // Resize thread - listens for resize events and resizes the terminal
    let resize_terminal_arc = terminal_arc.clone();
    let resize_handle = thread::spawn(move || {
        resize_thread_logic(resize_terminal_arc, rx_resize);
    });

    // Draw thread - listens for draw events and draws the editor
    let draw_terminal_arc = terminal_arc.clone();
    let draw_editor_arc = editor_arc.clone();
    let draw_handle = thread::spawn(move || {
        draw_thread_logic(draw_terminal_arc, draw_editor_arc, rx_draw);
    });
    
    // Give threads a moment to start
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Clear terminal after threads are running
    lock(&terminal_arc).clear()?;

    // Key input thread - listens for key events and updates the editor
    let key_editor_arc = editor_arc.clone();
    let tx_build_binding = tx_build.clone();
    let key_handle = thread::spawn(move || {
        key_thread_logic(key_editor_arc, rx_key, tx_main, tx_build_binding);
    });

    // Build thread - listens for build events and runs the build command
    let build_editor_arc = editor_arc.clone();
    let tx_build_binding = tx_build.clone();
    let build_handle = thread::spawn(move || {
        build_thread_logic(build_editor_arc, rx_build, tx_build_binding);
    });

    // Syntax error thread - loops lexer for parser errors and sets them in the editor state
    let lex_and_parse_editor_arc = editor_arc.clone();
    let lex_and_parse_handle = thread::spawn(move || {
        lex_and_parse_thread_logic(lex_and_parse_editor_arc, rx_code_error);
    });

    // Main loop - wait for shutdown signal
    loop {
        match rx_main.recv() {
            Ok(EditorMessage::Shutdown) => {
                log::info!("Received shutdown signal");
                break;
            }
            Ok(message) => {
                log::info!("Main thread received message as is ignoring: {:?}", message);
            }
            Err(_) => {
                log::error!("Channel closed unexpectedly");
                break;
            }
        }
    }

    // Send shutdown signal to all threads
    let _ = tx_resize.send(EditorMessage::Shutdown);
    let _ = tx_draw.send(EditorMessage::Shutdown);
    let _ = tx_key.send(EditorMessage::Shutdown);
    let _ = tx_build.send(EditorMessage::Shutdown);
    let _ = tx_code_error.send(EditorMessage::Shutdown);

    // // Wait for all threads to finish
    let _ = resize_handle.join();
    let _ = draw_handle.join();
    let _ = key_handle.join();
    let _ = build_handle.join();
    let _ = lex_and_parse_handle.join();

    disable_raw_mode()?;
    execute!(lock(&terminal_arc).backend_mut(), LeaveAlternateScreen)?;
    lock(&terminal_arc).show_cursor()?;

    Ok(())
}

