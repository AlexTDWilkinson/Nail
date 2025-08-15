mod checker;
mod colorizer;
mod common;
mod formatter;
mod lexer;
mod parser;
mod statics_for_tests;
mod stdlib_registry;
// mod stdlib_types; // Merged into stdlib_registry
mod transpilier;
mod utils;
use crate::colorizer::ColorScheme;
use crate::colorizer::LIGHT_THEME;
use crate::utils::create_welcome_message;
use crate::utils::lex_and_parse_thread_logic;
use std::backtrace::Backtrace;
use std::panic;

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
    // Intellisense fields
    completions: Vec<CompletionItem>,
    completion_index: usize,
    show_completions: bool,
    show_detail_view: bool,  // Show detailed documentation for selected completion
    completion_prefix: String,
    // AST and scope for intellisense
    ast: Option<parser::ASTNode>,
    scope_symbols: Vec<String>, // Variable names in current scope
}

#[derive(Clone, Debug)]
struct CompletionItem {
    label: String,
    detail: String, // Function signature or variable type
    description: String, // Description of what the function does
    example: String, // Example usage
    kind: CompletionKind,
}

#[derive(Clone, Debug, PartialEq)]
enum CompletionKind {
    Function,
    Variable,
    Keyword,
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
            completions: Vec::new(),
            completion_index: 0,
            show_completions: false,
            show_detail_view: false,
            completion_prefix: String::new(),
            ast: None,
            scope_symbols: Vec::new(),
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
        // Move up by visible lines (approximate page size)
        let page_size = 20; // Approximate visible lines
        let old_scroll = self.scroll_position;
        self.scroll_position = self.scroll_position.saturating_sub(page_size);
        self.scroll_state = self.scroll_state.position(self.scroll_position as usize);
        
        // Move cursor up by the same amount
        let scroll_diff = old_scroll - self.scroll_position;
        for _ in 0..scroll_diff {
            if self.cursor_y > 0 {
                self.cursor_y -= 1;
            } else {
                break;
            }
        }
        // Ensure cursor_x is within bounds of the new line
        if self.cursor_y < self.content.len() {
            let line_len = self.content[self.cursor_y].len();
            self.cursor_x = self.cursor_x.min(line_len);
        }
    }

    fn scroll_down(&mut self) {
        // Move down by visible lines (approximate page size)
        let page_size = 20; // Approximate visible lines
        let old_scroll = self.scroll_position;
        let max_scroll = self.content.len().saturating_sub(1) as u16;
        self.scroll_position = (self.scroll_position + page_size).min(max_scroll);
        self.scroll_state = self.scroll_state.position(self.scroll_position as usize);
        
        // Move cursor down by the same amount
        let scroll_diff = self.scroll_position - old_scroll;
        for _ in 0..scroll_diff {
            if self.cursor_y < self.content.len() - 1 {
                self.cursor_y += 1;
            } else {
                break;
            }
        }
        // Ensure cursor_x is within bounds of the new line
        if self.cursor_y < self.content.len() {
            let line_len = self.content[self.cursor_y].len();
            self.cursor_x = self.cursor_x.min(line_len);
        }
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
            let filename = "welcome.nail";
            let content = self.content.join("\n");
            fs::write(filename, content)?;
            self.current_file = Some(filename.to_string());
            self.modified = false;
            log::info!("Saved new file as: {}", filename);
        }
        Ok(())
    }

    fn load_file(&mut self, filename: &str) -> io::Result<()> {
        // Read the file
        let content = fs::read_to_string(filename)?;

        // Split into lines
        self.content = content.lines().map(|s| s.to_string()).collect();

        // If content is empty, add an empty line
        if self.content.is_empty() {
            self.content.push(String::new());
        }

        // Reset cursor and scroll position
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.scroll_position = 0;

        // Update current file and reset modified flag
        self.current_file = Some(filename.to_string());
        self.modified = false;

        // Clear any errors
        self.code_error = None;
        self.build_status = BuildStatus::Idle;

        log::info!("Loaded file: {}", filename);
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

    // Intellisense methods
    fn get_completion_context(&self) -> CompletionContext {
        if self.cursor_y >= self.content.len() {
            return CompletionContext::None;
        }
        
        let line = &self.content[self.cursor_y];
        if self.cursor_x > line.len() {
            return CompletionContext::None;
        }
        
        // Look for tokens around cursor position
        let cursor_line = self.cursor_y + 1; // Lines are 1-indexed in CodeSpan
        let cursor_col = self.cursor_x + 1;  // Columns are 1-indexed in CodeSpan
        
        // Check if we're inside a function call by looking for opening parenthesis
        let mut paren_depth = 0;
        let mut in_function_call = false;
        let mut function_name = String::new();
        
        for token in &self.tokens {
            // Check if token is before cursor
            if token.code_span.end_line < cursor_line || 
               (token.code_span.end_line == cursor_line && token.code_span.end_column <= cursor_col) {
                match &token.token_type {
                    lexer::TokenType::Identifier(name) => {
                        // Store potential function name
                        function_name = name.clone();
                    }
                    lexer::TokenType::ParenthesisOpen => {
                        paren_depth += 1;
                        in_function_call = true;
                    }
                    lexer::TokenType::ParenthesisClose => {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            in_function_call = false;
                            function_name.clear();
                        }
                    }
                    _ => {}
                }
            } else if token.code_span.start_line > cursor_line ||
                     (token.code_span.start_line == cursor_line && token.code_span.start_column > cursor_col) {
                break;
            }
        }
        
        if in_function_call && !function_name.is_empty() {
            return CompletionContext::FunctionCall(function_name);
        }
        
        // Check if we're typing an identifier
        let current_word = self.get_current_word();
        if !current_word.is_empty() {
            return CompletionContext::Identifier(current_word);
        }
        
        CompletionContext::None
    }
    
    fn get_absolute_cursor_position(&self) -> usize {
        let mut pos = 0;
        for i in 0..self.cursor_y {
            if i < self.content.len() {
                pos += self.content[i].len() + 1; // +1 for newline
            }
        }
        pos + self.cursor_x
    }
    
    fn get_current_word(&self) -> String {
        if self.cursor_y >= self.content.len() {
            return String::new();
        }
        
        let line = &self.content[self.cursor_y];
        if self.cursor_x > line.len() {
            return String::new();
        }
        
        // Find word boundaries
        let mut start = self.cursor_x;
        while start > 0 && line.chars().nth(start - 1).map_or(false, |c| c.is_alphanumeric() || c == '_') {
            start -= 1;
        }
        
        let mut end = self.cursor_x;
        while end < line.len() && line.chars().nth(end).map_or(false, |c| c.is_alphanumeric() || c == '_') {
            end += 1;
        }
        
        line[start..end].to_string()
    }
    
    fn update_completions(&mut self) {
        let context = self.get_completion_context();
        
        // Reset detail view when updating completions
        self.show_detail_view = false;
        
        match context {
            CompletionContext::None => {
                self.show_completions = false;
                self.completions.clear();
            }
            CompletionContext::Identifier(prefix) => {
                if prefix.len() < 2 {
                    self.show_completions = false;
                    self.completions.clear();
                    return;
                }
                
                // Get stdlib functions
                use crate::stdlib_registry::STDLIB_FUNCTIONS;
                let mut completions = Vec::new();
                
                for (name, func) in STDLIB_FUNCTIONS.iter() {
                    if name.starts_with(&prefix) {
                        // Build function signature
                        let params: Vec<String> = func.parameters.iter()
                            .map(|p| format!("{}:{}", p.name, format_type(&p.param_type)))
                            .collect();
                        
                        // For debugging - log the function info
                        log::debug!("Function {}: {} params, return type: {:?}", 
                            name, func.parameters.len(), func.return_type);
                        
                        let signature = if params.is_empty() {
                            format!("{}() -> {}", name, format_type(&func.return_type))
                        } else {
                            format!("{}({}) -> {}", name, params.join(", "), format_type(&func.return_type))
                        };
                        
                        completions.push(CompletionItem {
                            label: name.to_string(),
                            detail: signature,
                            description: func.description.to_string(),
                            example: func.example.to_string(),
                            kind: CompletionKind::Function,
                        });
                    }
                }
                
                // Add variables from scope
                for var_name in &self.scope_symbols {
                    if var_name.starts_with(&prefix) {
                        completions.push(CompletionItem {
                            label: var_name.clone(),
                            detail: String::new(), // Could add type info here
                            description: "Local variable".to_string(),
                            example: String::new(),
                            kind: CompletionKind::Variable,
                        });
                    }
                }
                
                completions.sort_by(|a, b| a.label.cmp(&b.label));
                
                self.completions = completions;
                self.completion_prefix = prefix;
                self.show_completions = !self.completions.is_empty();
                self.completion_index = 0;
            }
            CompletionContext::FunctionCall(func_name) => {
                // Show parameter hints for the function
                use crate::stdlib_registry::get_stdlib_function;
                
                if let Some(func) = get_stdlib_function(&func_name) {
                    let params: Vec<String> = func.parameters.iter()
                        .map(|p| format!("{}:{}", p.name, format_type(&p.param_type)))
                        .collect();
                    
                    let hint = CompletionItem {
                        label: format!("{}({})", func_name, params.join(", ")),
                        detail: format!("Returns: {}", format_type(&func.return_type)),
                        description: func.description.to_string(),
                        example: func.example.to_string(),
                        kind: CompletionKind::Function,
                    };
                    
                    self.completions = vec![hint];
                    self.show_completions = true;
                    self.completion_index = 0;
                } else {
                    self.show_completions = false;
                    self.completions.clear();
                }
            }
        }
    }
    
    fn accept_completion(&mut self) {
        if !self.show_completions || self.completions.is_empty() {
            return;
        }
        
        let completion = &self.completions[self.completion_index];
        
        // Only complete if it's an identifier completion
        if let CompletionContext::Identifier(_) = self.get_completion_context() {
            let line = &mut self.content[self.cursor_y];
            
            // Find the start of the current word
            let mut start = self.cursor_x;
            while start > 0 && line.chars().nth(start - 1).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                start -= 1;
            }
            
            // Find the end of the current word
            let mut end = self.cursor_x;
            while end < line.len() && line.chars().nth(end).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                end += 1;
            }
            
            // Replace the current word with the completion
            let before = line[..start].to_string();
            let after = line[end..].to_string();
            
            *line = format!("{}{}{}", before, completion.label, after);
            self.cursor_x = start + completion.label.len();
            
            self.modified = true;
        }
        
        self.show_completions = false;
        self.show_detail_view = false;
        self.completions.clear();
    }
    
    fn next_completion(&mut self) {
        if !self.completions.is_empty() {
            self.completion_index = (self.completion_index + 1) % self.completions.len();
        }
    }
    
    fn previous_completion(&mut self) {
        if !self.completions.is_empty() {
            self.completion_index = if self.completion_index == 0 {
                self.completions.len() - 1
            } else {
                self.completion_index - 1
            };
        }
    }
}

#[derive(Debug, Clone)]
enum CompletionContext {
    None,
    Identifier(String),        // Typing an identifier, show matching functions/variables
    FunctionCall(String),       // Inside function call, show parameter hints
}

fn format_type(data_type: &lexer::NailDataTypeDescriptor) -> String {
    match data_type {
        lexer::NailDataTypeDescriptor::Int => "i".to_string(),
        lexer::NailDataTypeDescriptor::Float => "f".to_string(),
        lexer::NailDataTypeDescriptor::String => "s".to_string(),
        lexer::NailDataTypeDescriptor::Boolean => "b".to_string(),
        lexer::NailDataTypeDescriptor::Void => "void".to_string(),
        lexer::NailDataTypeDescriptor::Array(inner) => format!("[{}]", format_type(inner)),
        lexer::NailDataTypeDescriptor::HashMap(key, value) => format!("h<{}, {}>", format_type(key), format_type(value)),
        lexer::NailDataTypeDescriptor::Result(result_type) => format!("{}!e", format_type(result_type)),
        lexer::NailDataTypeDescriptor::Any => "any".to_string(),
        lexer::NailDataTypeDescriptor::Struct(name) => name.clone(),
        lexer::NailDataTypeDescriptor::Enum(name) => name.clone(),
        _ => "?".to_string(),
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
                }
                Err(e) => {
                    log::error!("Failed to get terminal size: {}", e);
                }
            }
            t
        }
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
