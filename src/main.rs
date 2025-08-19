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
use std::time::Instant;

use crate::utils::lock;
use crate::utils::BuildStatus;

use crate::common::CodeSpan;
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
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

#[derive(Debug, Clone, PartialEq)]
enum EditOperation {
    InsertChar { position: (usize, usize), char: char },
    DeleteChar { position: (usize, usize), char: char },
    InsertNewline { position: (usize, usize) },
    DeleteNewline { position: (usize, usize), merged_content: String },
    InsertText { position: (usize, usize), text: String },
    DeleteText { position: (usize, usize), text: String },
    ReplaceText { position: (usize, usize), old_text: String, new_text: String },
    BatchOperation { operations: Vec<EditOperation> },
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

#[derive(Clone)]
struct Tab {
    filename: Option<String>,
    content: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    scroll_position: u16,
    modified: bool,
    // Selection fields
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    selection_mode: bool,
    // Undo/Redo system per tab
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    last_char_insert_time: Option<Instant>,
    char_insert_group: Vec<EditOperation>,
    // AST and scope for intellisense per tab
    ast: Option<parser::ASTNode>,
    scope_symbols: Vec<SymbolInfo>,
    tokens: Vec<lexer::Token>,
}

impl Tab {
    fn new() -> Self {
        Tab {
            filename: None,
            content: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            scroll_position: 0,
            modified: false,
            selection_start: None,
            selection_end: None,
            selection_mode: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_char_insert_time: None,
            char_insert_group: Vec::new(),
            ast: None,
            scope_symbols: Vec::new(),
            tokens: Vec::new(),
        }
    }
    
    fn new_with_file(filename: String, content: Vec<String>) -> Self {
        let mut tab = Tab::new();
        tab.filename = Some(filename);
        tab.content = content;
        tab
    }
    
    fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }
    
    fn get_normalized_selection(&self) -> ((usize, usize), (usize, usize)) {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            // Normalize so start is before end
            if start.1 < end.1 || (start.1 == end.1 && start.0 <= end.0) {
                (start, end)
            } else {
                (end, start)
            }
        } else {
            ((0, 0), (0, 0))
        }
    }
    
    fn delete_selected_text(&mut self) {
        if !self.has_selection() {
            return;
        }
        
        let (start_pos, end_pos) = self.get_normalized_selection();
        
        if start_pos.1 == end_pos.1 {
            // Single line selection
            let line = &mut self.content[start_pos.1];
            let before = line[..start_pos.0].to_string();
            let after = line[end_pos.0..].to_string();
            *line = format!("{}{}", before, after);
        } else {
            // Multi-line selection
            let start_line = &self.content[start_pos.1];
            let end_line = &self.content[end_pos.1];
            
            let before = start_line[..start_pos.0].to_string();
            let after = end_line[end_pos.0..].to_string();
            let new_line = format!("{}{}", before, after);
            
            // Remove lines between start and end
            self.content.drain(start_pos.1 + 1..=end_pos.1);
            self.content[start_pos.1] = new_line;
        }
        
        // Clear selection and position cursor
        self.cursor_x = start_pos.0;
        self.cursor_y = start_pos.1;
        self.selection_start = None;
        self.selection_end = None;
        self.selection_mode = false;
        self.modified = true;
    }
    
    fn record_operation(&mut self, op: EditOperation) {
        // Group consecutive character insertions for better user experience
        if let EditOperation::InsertChar { .. } = &op {
            let now = Instant::now();
            let should_group = if let Some(last_time) = self.last_char_insert_time {
                now.duration_since(last_time).as_millis() < 500 // Group within 500ms
            } else {
                false
            };
            
            if should_group && !self.char_insert_group.is_empty() {
                self.char_insert_group.push(op);
                self.last_char_insert_time = Some(now);
                return;
            } else {
                // Flush any existing group first
                self.flush_char_group();
                self.char_insert_group.push(op);
                self.last_char_insert_time = Some(now);
                return;
            }
        } else {
            // Non-character operation, flush any pending group
            self.flush_char_group();
        }
        
        // Clear redo stack when new operation is performed
        self.redo_stack.clear();
        
        // Add to undo stack
        self.undo_stack.push(op);
        
        // Limit undo stack size
        let max_undo = 1000;
        if self.undo_stack.len() > max_undo {
            self.undo_stack.remove(0);
        }
    }
    
    fn flush_char_group(&mut self) {
        if !self.char_insert_group.is_empty() {
            let group = EditOperation::BatchOperation {
                operations: self.char_insert_group.clone(),
            };
            self.undo_stack.push(group);
            self.char_insert_group.clear();
        }
        self.last_char_insert_time = None;
    }
}

#[derive(Clone)]
struct FileEntry {
    name: String,
    path: String,
    is_directory: bool,
    is_recent: bool,
}

#[derive(Clone)]
struct StdLibFunction {
    name: String,
    signature: String,
    description: String,
    example: String,
    category: String,
}

struct Editor {
    debug_mode: bool,
    theme: &'static ColorScheme,
    // Tab system
    tabs: Vec<Tab>,
    tab_index: usize,
    // Global state
    build_status: BuildStatus,
    code_error: Option<CodeError>,
    scroll_state: ScrollbarState,
    max_undo_history: usize,
    // Intellisense fields (shared across tabs)
    completions: Vec<CompletionItem>,
    completion_index: usize,
    show_completions: bool,
    show_detail_view: bool,  // Show detailed documentation for selected completion
    completion_prefix: String,
    // Dialog system
    dialog_mode: DialogMode,
    goto_line_input: String,
    // Find/Replace system (shared across tabs)
    search_query: String,
    replace_text: String,
    search_results: Vec<(usize, usize, usize)>, // (line, start, end)
    current_match_index: usize,
    case_sensitive: bool,
    search_direction_forward: bool, // For F3/Shift+F3 navigation
    replace_field_active: bool, // true if replace field is active, false if find field is active
    // File dialog system
    file_entries: Vec<FileEntry>,
    file_dialog_index: usize,
    file_dialog_input: String,
    current_directory: String,
    recent_files: Vec<String>,
    // Standard library browser
    stdlib_functions: Vec<StdLibFunction>,
    stdlib_filter: String,
    stdlib_index: usize,
    stdlib_category_filter: Option<String>,
    // Visual enhancement settings
    show_line_numbers: bool,
    highlight_current_line: bool,
    highlight_matching_brackets: bool,
    show_whitespace: bool,
    show_indentation_guides: bool,
    show_minimap: bool,
    // Bracket matching state
    matching_bracket_pos: Option<(usize, usize)>,
}

#[derive(Clone, Debug)]
struct CompletionItem {
    label: String,
    detail: String, // Function signature or variable type
    description: String, // Description of what the function does
    example: String, // Example usage
    kind: CompletionKind,
}

#[derive(Clone, Debug)]
struct SymbolInfo {
    name: String,
    symbol_type: SymbolType,
    data_type: Option<String>, // Type information if available
}

#[derive(Clone, Debug)]
enum SymbolType {
    Variable,
    Struct { fields: Vec<(String, String)> }, // (field_name, field_type)
    Enum { variants: Vec<String> },
}

#[derive(Clone, Debug, PartialEq)]
enum CompletionKind {
    Function,
    Variable,
    Struct,
    Enum,
    Keyword,
}

#[derive(Clone, Debug, PartialEq)]
enum DialogMode {
    None,
    GoToLine,
    Find,
    Replace,
    OpenFile,
    StdLibBrowser,
}

impl Editor {
    // Helper function to convert character index to byte index for string operations
    fn char_to_byte_index(s: &str, char_index: usize) -> usize {
        s.char_indices()
            .nth(char_index)
            .map(|(byte_index, _)| byte_index)
            .unwrap_or(s.len())
    }
    
    // Helper function to get the byte length of the character at the given character index
    fn char_byte_len_at(s: &str, char_index: usize) -> usize {
        s.chars()
            .nth(char_index)
            .map(|c| c.len_utf8())
            .unwrap_or(0)
    }

    fn new() -> Editor {
        Self::new_with_debug(false)
    }
    
    fn new_with_debug(debug: bool) -> Editor {
        if debug {
            log::warn!("IDE starting in DEBUG MODE - extra logging enabled");
        }
        let mut welcome_tab = Tab::new();
        welcome_tab.content = create_welcome_message();
        
        Editor {
            debug_mode: debug,
            theme: &DARK_THEME,
            tabs: vec![welcome_tab],
            tab_index: 0,
            build_status: BuildStatus::Idle,
            code_error: None,
            scroll_state: ScrollbarState::default(),
            max_undo_history: 1000,
            completions: Vec::new(),
            completion_index: 0,
            show_completions: false,
            show_detail_view: false,
            completion_prefix: String::new(),
            dialog_mode: DialogMode::None,
            goto_line_input: String::new(),
            search_query: String::new(),
            replace_text: String::new(),
            search_results: Vec::new(),
            current_match_index: 0,
            case_sensitive: false,
            search_direction_forward: true,
            replace_field_active: false,
            file_entries: Vec::new(),
            file_dialog_index: 0,
            file_dialog_input: String::new(),
            current_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
            recent_files: Vec::new(),
            stdlib_functions: Vec::new(),
            stdlib_filter: String::new(),
            stdlib_index: 0,
            stdlib_category_filter: None,
            // Visual enhancement settings - enabled by default
            show_line_numbers: true,
            highlight_current_line: true,
            highlight_matching_brackets: true,
            show_whitespace: false,
            show_indentation_guides: false,
            show_minimap: false, // Disabled by default as it takes screen space
            // Bracket matching state
            matching_bracket_pos: None,
        }
    }

    // Tab management methods
    fn get_current_tab(&self) -> &Tab {
        &self.tabs[self.tab_index]
    }
    
    fn get_current_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.tab_index]
    }
    
    fn new_tab(&mut self) {
        let new_tab = Tab::new();
        self.tabs.push(new_tab);
        self.tab_index = self.tabs.len() - 1;
        // Clear search results and update syntax highlighting is now handled automatically
    }
    
    fn open_file_in_tab(&mut self, filename: String) -> Result<(), String> {
        // Check if file is already open in a tab
        for (i, tab) in self.tabs.iter().enumerate() {
            if let Some(tab_filename) = &tab.filename {
                if tab_filename == &filename {
                    self.tab_index = i;
                    return Ok(());
                }
            }
        }
        
        // Read file content
        let content = match std::fs::read_to_string(&filename) {
            Ok(content) => {
                let lines: Vec<String> = if content.is_empty() {
                    vec![String::new()]
                } else {
                    content.lines().map(|s| s.to_string()).collect()
                };
                lines
            }
            Err(err) => return Err(format!("Failed to read file: {}", err)),
        };
        
        // Create new tab with file content
        let new_tab = Tab::new_with_file(filename.clone(), content);
        self.tabs.push(new_tab);
        self.tab_index = self.tabs.len() - 1;
        
        // Add to recent files
        if !self.recent_files.contains(&filename) {
            self.recent_files.insert(0, filename);
            self.recent_files.truncate(10); // Keep only 10 recent files
        }
        
        // Clear search results and update syntax highlighting is now handled automatically
        Ok(())
    }
    
    fn close_tab(&mut self, tab_index: usize) -> bool {
        if self.tabs.len() <= 1 {
            return false; // Always keep at least one tab
        }
        
        if tab_index >= self.tabs.len() {
            return false;
        }
        
        // Check if tab is modified and needs saving
        if self.tabs[tab_index].modified {
            // In a real implementation, you'd show a save dialog here
            // For now, we'll just close without saving
        }
        
        self.tabs.remove(tab_index);
        
        // Adjust current tab index
        if self.tab_index >= self.tabs.len() {
            self.tab_index = self.tabs.len() - 1;
        } else if self.tab_index > tab_index {
            self.tab_index -= 1;
        }
        
        // Clear search results and update syntax highlighting is now handled automatically
        true
    }
    
    fn switch_to_tab(&mut self, tab_index: usize) {
        if tab_index < self.tabs.len() {
            self.tab_index = tab_index;
            // Clear search results and update syntax highlighting handled automatically
        }
    }
    
    fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tab_index = (self.tab_index + 1) % self.tabs.len();
            // Clear search results and update syntax highlighting handled automatically
        }
    }
    
    fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tab_index = if self.tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.tab_index - 1
            };
            // Clear search results and update syntax highlighting handled automatically
        }
    }

    // File dialog methods
    fn open_file_path(&mut self, path: &str) -> io::Result<()> {
        let content = fs::read_to_string(path)?;
        
        // Create a new tab for the file
        let mut new_tab = Tab::new();
        new_tab.content = content.lines().map(|l| l.to_string()).collect();
        new_tab.filename = Some(path.to_string());
        new_tab.modified = false;
        
        // Add the tab and switch to it
        self.tabs.push(new_tab);
        self.tab_index = self.tabs.len() - 1;
        
        Ok(())
    }
    
    fn open_file_dialog(&mut self) {
        self.dialog_mode = DialogMode::OpenFile;
        self.refresh_file_entries();
        self.file_dialog_index = 0;
        self.file_dialog_input.clear();
    }
    
    fn refresh_file_entries(&mut self) {
        self.file_entries.clear();
        
        // Add recent files first
        for (i, file) in self.recent_files.iter().enumerate() {
            if i >= 5 { break; } // Show only first 5 recent files
            self.file_entries.push(FileEntry {
                name: format!("📄 {}", std::path::Path::new(file).file_name().unwrap_or_default().to_string_lossy()),
                path: file.clone(),
                is_directory: false,
                is_recent: true,
            });
        }
        
        // Add parent directory entry if not at root
        if self.current_directory != "/" {
            self.file_entries.push(FileEntry {
                name: "📁 ..".to_string(),
                path: std::path::Path::new(&self.current_directory)
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("/"))
                    .to_string_lossy()
                    .to_string(),
                is_directory: true,
                is_recent: false,
            });
        }
        
        // Read current directory
        if let Ok(entries) = std::fs::read_dir(&self.current_directory) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }
                
                if path.is_dir() {
                    dirs.push(FileEntry {
                        name: format!("📁 {}", name),
                        path: path.to_string_lossy().to_string(),
                        is_directory: true,
                        is_recent: false,
                    });
                } else if name.ends_with(".nail") || name.ends_with(".rs") || name.ends_with(".txt") {
                    files.push(FileEntry {
                        name: format!("📄 {}", name),
                        path: path.to_string_lossy().to_string(),
                        is_directory: false,
                        is_recent: false,
                    });
                }
            }
            
            // Sort directories and files separately
            dirs.sort_by(|a, b| a.name.cmp(&b.name));
            files.sort_by(|a, b| a.name.cmp(&b.name));
            
            // Add directories first, then files
            self.file_entries.extend(dirs);
            self.file_entries.extend(files);
        }
    }
    
    fn handle_file_dialog_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.dialog_mode = DialogMode::None;
                return true;
            }
            KeyCode::Enter => {
                if !self.file_entries.is_empty() && self.file_dialog_index < self.file_entries.len() {
                    let entry = &self.file_entries[self.file_dialog_index].clone();
                    if entry.is_directory {
                        self.current_directory = entry.path.clone();
                        self.refresh_file_entries();
                        self.file_dialog_index = 0;
                    } else {
                        let _ = self.open_file_in_tab(entry.path.clone());
                        self.dialog_mode = DialogMode::None;
                    }
                }
                return true;
            }
            KeyCode::Up => {
                if self.file_dialog_index > 0 {
                    self.file_dialog_index -= 1;
                }
                return true;
            }
            KeyCode::Down => {
                if self.file_dialog_index < self.file_entries.len().saturating_sub(1) {
                    self.file_dialog_index += 1;
                }
                return true;
            }
            KeyCode::Char(c) => {
                self.file_dialog_input.push(c);
                // Filter entries based on input
                self.filter_file_entries();
                return true;
            }
            KeyCode::Backspace => {
                self.file_dialog_input.pop();
                self.filter_file_entries();
                return true;
            }
            _ => return false,
        }
    }
    
    fn filter_file_entries(&mut self) {
        if self.file_dialog_input.is_empty() {
            self.refresh_file_entries();
        } else {
            let input_lower = self.file_dialog_input.to_lowercase();
            self.file_entries.retain(|entry| {
                entry.name.to_lowercase().contains(&input_lower) ||
                entry.path.to_lowercase().contains(&input_lower)
            });
        }
        self.file_dialog_index = 0;
    }

    // Standard library browser methods
    fn open_stdlib_browser(&mut self) {
        self.dialog_mode = DialogMode::StdLibBrowser;
        self.refresh_stdlib_functions();
        self.stdlib_index = 0;
        self.stdlib_filter.clear();
    }
    
    fn refresh_stdlib_functions(&mut self) {
        use crate::stdlib_registry::STDLIB_FUNCTIONS;
        
        self.stdlib_functions.clear();
        
        for (name, func) in STDLIB_FUNCTIONS.iter() {
            let category = match func.module {
                crate::stdlib_registry::StdlibModule::Http => "HTTP",
                crate::stdlib_registry::StdlibModule::Fs => "File System",
                crate::stdlib_registry::StdlibModule::IO => "Input/Output",
                crate::stdlib_registry::StdlibModule::Math => "Math",
                crate::stdlib_registry::StdlibModule::String => "String",
                crate::stdlib_registry::StdlibModule::Array => "Array",
                crate::stdlib_registry::StdlibModule::Error => "Error",
                crate::stdlib_registry::StdlibModule::Print => "Print",
                crate::stdlib_registry::StdlibModule::Time => "Time",
                crate::stdlib_registry::StdlibModule::Regex => "Regex",
                crate::stdlib_registry::StdlibModule::Json => "JSON",
                crate::stdlib_registry::StdlibModule::Env => "Environment",
                crate::stdlib_registry::StdlibModule::Process => "Process",
                crate::stdlib_registry::StdlibModule::Crypto => "Crypto",
                crate::stdlib_registry::StdlibModule::Database => "Database",
                crate::stdlib_registry::StdlibModule::Compress => "Compression",
                _ => "Other",
            };
            
            // Build function signature
            let mut signature = format!("{}(", name);
            for (i, param) in func.parameters.iter().enumerate() {
                if i > 0 { signature.push_str(", "); }
                signature.push_str(&format!("{}: {}", param.name, format_type(&param.param_type)));
            }
            signature.push_str(&format!(") -> {}", format_type(&func.return_type)));
            
            self.stdlib_functions.push(StdLibFunction {
                name: name.to_string(),
                signature,
                description: func.description.to_string(),
                example: func.example.to_string(),
                category: category.to_string(),
            });
        }
        
        // Sort by category, then by name
        self.stdlib_functions.sort_by(|a, b| {
            a.category.cmp(&b.category).then(a.name.cmp(&b.name))
        });
    }
    
    fn handle_stdlib_browser_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.dialog_mode = DialogMode::None;
                return true;
            }
            KeyCode::Enter => {
                if !self.stdlib_functions.is_empty() && self.stdlib_index < self.stdlib_functions.len() {
                    let func = &self.stdlib_functions[self.stdlib_index].clone();
                    self.insert_stdlib_function(&func.name);
                    self.dialog_mode = DialogMode::None;
                }
                return true;
            }
            KeyCode::Up => {
                if self.stdlib_index > 0 {
                    self.stdlib_index -= 1;
                }
                return true;
            }
            KeyCode::Down => {
                if self.stdlib_index < self.stdlib_functions.len().saturating_sub(1) {
                    self.stdlib_index += 1;
                }
                return true;
            }
            KeyCode::Char(c) => {
                self.stdlib_filter.push(c);
                self.filter_stdlib_functions();
                return true;
            }
            KeyCode::Backspace => {
                self.stdlib_filter.pop();
                self.filter_stdlib_functions();
                return true;
            }
            _ => return false,
        }
    }
    
    fn filter_stdlib_functions(&mut self) {
        if self.stdlib_filter.is_empty() {
            self.refresh_stdlib_functions();
        } else {
            let filter_lower = self.stdlib_filter.to_lowercase();
            self.stdlib_functions.retain(|func| {
                func.name.to_lowercase().contains(&filter_lower) ||
                func.description.to_lowercase().contains(&filter_lower) ||
                func.category.to_lowercase().contains(&filter_lower)
            });
        }
        self.stdlib_index = 0;
    }
    
    // Wrapper methods for simplified stdlib dialog handling from utils.rs
    fn handle_stdlib_dialog_input(&mut self, c: char) {
        self.stdlib_filter.push(c);
        self.filter_stdlib_functions();
    }
    
    fn handle_stdlib_dialog_backspace(&mut self) {
        self.stdlib_filter.pop();
        self.filter_stdlib_functions();
    }
    
    fn handle_stdlib_dialog_enter(&mut self) {
        if !self.stdlib_functions.is_empty() && self.stdlib_index < self.stdlib_functions.len() {
            let func = &self.stdlib_functions[self.stdlib_index].clone();
            self.insert_stdlib_function(&func.name);
            self.dialog_mode = DialogMode::None;
        }
    }
    
    // Wrapper methods for simplified file dialog handling from utils.rs
    fn handle_file_dialog_input(&mut self, c: char) {
        self.file_dialog_input.push(c);
        self.filter_file_entries();
    }
    
    fn handle_file_dialog_backspace(&mut self) {
        self.file_dialog_input.pop();
        self.filter_file_entries();
    }
    
    fn handle_file_dialog_enter(&mut self) {
        if !self.file_entries.is_empty() && self.file_dialog_index < self.file_entries.len() {
            let entry = &self.file_entries[self.file_dialog_index].clone();
            if entry.is_directory {
                self.current_directory = entry.path.clone();
                self.refresh_file_entries();
                self.file_dialog_index = 0;
            } else {
                let _ = self.open_file_in_tab(entry.path.clone());
                self.dialog_mode = DialogMode::None;
            }
        }
    }
    
    fn insert_stdlib_function(&mut self, func_name: &str) {
        let current_tab = self.get_current_tab_mut();
        let insert_text = format!("{}()", func_name);
        
        // Insert at cursor position
        if current_tab.cursor_y >= current_tab.content.len() {
            current_tab.content.push(String::new());
            current_tab.cursor_y = current_tab.content.len() - 1;
        }
        
        let line = &mut current_tab.content[current_tab.cursor_y];
        if current_tab.cursor_x > line.chars().count() {
            current_tab.cursor_x = line.chars().count();
        }
        
        let byte_pos = Self::char_to_byte_index(line, current_tab.cursor_x);
        line.insert_str(byte_pos, &insert_text);
        current_tab.cursor_x += func_name.len() + 1; // Position cursor between parentheses
        current_tab.modified = true;
    }

    // Undo/Redo management methods
    fn record_operation(&mut self, op: EditOperation) {
        let current_tab = self.get_current_tab_mut();
        current_tab.record_operation(op);
    }
    
    fn flush_char_group(&mut self) {
        let current_tab = self.get_current_tab_mut();
        current_tab.flush_char_group();
    }
    
    fn undo(&mut self) -> bool {
        // Flush any pending character group first
        self.flush_char_group();
        
        // Pop operation from undo stack
        let operation = {
            let current_tab = self.get_current_tab_mut();
            current_tab.undo_stack.pop()
        };
        
        if let Some(operation) = operation {
            // Apply reverse operation
            self.apply_operation(&operation, true);
            
            // Move operation to redo stack
            let max_undo = self.max_undo_history;
            let current_tab = self.get_current_tab_mut();
            current_tab.redo_stack.push(operation);
            
            // Limit redo history size
            if current_tab.redo_stack.len() > max_undo {
                current_tab.redo_stack.remove(0);
            }
            
            current_tab.modified = true;
            true
        } else {
            false
        }
    }
    
    fn redo(&mut self) -> bool {
        // Pop operation from redo stack
        let operation = {
            let current_tab = self.get_current_tab_mut();
            current_tab.redo_stack.pop()
        };
        
        if let Some(operation) = operation {
            // Apply forward operation
            self.apply_operation(&operation, false);
            
            // Move operation back to undo stack
            let current_tab = self.get_current_tab_mut();
            current_tab.undo_stack.push(operation);
            
            current_tab.modified = true;
            true
        } else {
            false
        }
    }
    
    fn apply_operation(&mut self, operation: &EditOperation, reverse: bool) {
        let current_tab = self.get_current_tab_mut();
        match operation {
            EditOperation::InsertChar { position, char } => {
                if reverse {
                    // Undo: remove the character
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y < current_tab.content.len() {
                        let line = &current_tab.content[current_tab.cursor_y];
                        if current_tab.cursor_x < line.chars().count() {
                            let byte_pos = Self::char_to_byte_index(line, current_tab.cursor_x);
                            current_tab.content[current_tab.cursor_y].remove(byte_pos);
                        }
                    }
                } else {
                    // Redo: insert the character
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y < current_tab.content.len() {
                        let byte_pos = Self::char_to_byte_index(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x);
                        current_tab.content[current_tab.cursor_y].insert(byte_pos, *char);
                        current_tab.cursor_x += 1;
                    }
                }
            }
            EditOperation::DeleteChar { position, char } => {
                if reverse {
                    // Undo: insert the character back
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y < current_tab.content.len() {
                        let byte_pos = Self::char_to_byte_index(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x);
                        current_tab.content[current_tab.cursor_y].insert(byte_pos, *char);
                        current_tab.cursor_x += 1;
                    }
                } else {
                    // Redo: remove the character
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y < current_tab.content.len() {
                        let line = &current_tab.content[current_tab.cursor_y];
                        if current_tab.cursor_x < line.chars().count() {
                            let byte_pos = Self::char_to_byte_index(line, current_tab.cursor_x);
                            current_tab.content[current_tab.cursor_y].remove(byte_pos);
                        }
                    }
                }
            }
            EditOperation::InsertNewline { position } => {
                if reverse {
                    // Undo: merge lines back
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y + 1 < current_tab.content.len() {
                        let next_line = current_tab.content.remove(current_tab.cursor_y + 1);
                        current_tab.content[current_tab.cursor_y].push_str(&next_line);
                    }
                } else {
                    // Redo: split line
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y < current_tab.content.len() {
                        let remaining = current_tab.content[current_tab.cursor_y].split_off(current_tab.cursor_x);
                        current_tab.content.insert(current_tab.cursor_y + 1, remaining);
                        current_tab.cursor_y += 1;
                        current_tab.cursor_x = 0;
                    }
                }
            }
            EditOperation::DeleteNewline { position, merged_content } => {
                if reverse {
                    // Undo: split the line back
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y < current_tab.content.len() {
                        let remaining = current_tab.content[current_tab.cursor_y].split_off(current_tab.cursor_x);
                        current_tab.content.insert(current_tab.cursor_y + 1, remaining);
                        current_tab.cursor_y += 1;
                        current_tab.cursor_x = 0;
                    }
                } else {
                    // Redo: merge lines
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y + 1 < current_tab.content.len() {
                        let next_line = current_tab.content.remove(current_tab.cursor_y + 1);
                        current_tab.content[current_tab.cursor_y].push_str(&next_line);
                    }
                }
            }
            EditOperation::InsertText { position, text } => {
                if reverse {
                    // Undo: remove the text
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    self.delete_text_at_position(text);
                } else {
                    // Redo: insert the text
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    self.insert_text_at_cursor(text);
                }
            }
            EditOperation::DeleteText { position, text } => {
                if reverse {
                    // Undo: insert the text back
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    self.insert_text_at_cursor(text);
                } else {
                    // Redo: remove the text
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    self.delete_text_at_position(text);
                }
            }
            EditOperation::ReplaceText { position, old_text, new_text } => {
                if reverse {
                    // Undo: replace new_text with old_text
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    self.delete_text_at_position(new_text);
                    self.insert_text_at_cursor(old_text);
                } else {
                    // Redo: replace old_text with new_text
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    self.delete_text_at_position(old_text);
                    self.insert_text_at_cursor(new_text);
                }
            }
            EditOperation::BatchOperation { operations } => {
                if reverse {
                    // Undo: apply operations in reverse order
                    for op in operations.iter().rev() {
                        self.apply_operation(op, true);
                    }
                } else {
                    // Redo: apply operations in forward order
                    for op in operations {
                        self.apply_operation(op, false);
                    }
                }
            }
        }
    }
    
    fn insert_text_at_cursor(&mut self, text: &str) {
        let current_tab = self.get_current_tab_mut();
        for c in text.chars() {
            if c == '\n' {
                let remaining = current_tab.content[current_tab.cursor_y].split_off(current_tab.cursor_x);
                current_tab.content.insert(current_tab.cursor_y + 1, remaining);
                current_tab.cursor_y += 1;
                current_tab.cursor_x = 0;
            } else {
                if current_tab.cursor_y >= current_tab.content.len() {
                    current_tab.content.push(String::new());
                }
                let line = &mut current_tab.content[current_tab.cursor_y];
                let line_char_count = line.chars().count();
                if current_tab.cursor_x > line_char_count {
                    line.push_str(&" ".repeat(current_tab.cursor_x - line_char_count));
                }
                let byte_pos = Self::char_to_byte_index(line, current_tab.cursor_x);
                line.insert(byte_pos, c);
                current_tab.cursor_x += 1;
            }
        }
    }
    
    fn delete_text_at_position(&mut self, text: &str) {
        let current_tab = self.get_current_tab_mut();
        let char_count = text.chars().count();
        for _ in 0..char_count {
            if current_tab.cursor_x > 0 {
                let byte_pos = Self::char_to_byte_index(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x - 1);
                current_tab.content[current_tab.cursor_y].remove(byte_pos);
                current_tab.cursor_x -= 1;
            } else if current_tab.cursor_y > 0 {
                let current_line = current_tab.content.remove(current_tab.cursor_y);
                current_tab.cursor_y -= 1;
                current_tab.cursor_x = current_tab.content[current_tab.cursor_y].len();
                current_tab.content[current_tab.cursor_y].push_str(&current_line);
            }
        }
    }

    fn delete_char(&mut self) {
        // If there's a selection, delete it instead of single character
        if self.has_selection() {
            self.delete_selected_text();
            return;
        }
        
        let current_tab = self.get_current_tab_mut();
        if current_tab.cursor_x > 0 {
            // Safely get the character to delete
            if let Some(deleted_char) = current_tab.content[current_tab.cursor_y].chars().nth(current_tab.cursor_x.saturating_sub(1)) {
                let operation = EditOperation::DeleteChar {
                    position: (current_tab.cursor_x - 1, current_tab.cursor_y),
                    char: deleted_char,
                };
                
                let byte_pos = Self::char_to_byte_index(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x - 1);
                current_tab.content[current_tab.cursor_y].remove(byte_pos);
                current_tab.cursor_x -= 1;
                current_tab.modified = true;
                
                current_tab.record_operation(operation);
            }
        } else if current_tab.cursor_y > 0 {
            let current_line = current_tab.content.remove(current_tab.cursor_y);
            let old_cursor_x = current_tab.cursor_x;
            let old_cursor_y = current_tab.cursor_y;
            
            current_tab.cursor_y -= 1;
            current_tab.cursor_x = current_tab.content[current_tab.cursor_y].chars().count();
            current_tab.content[current_tab.cursor_y].push_str(&current_line);
            current_tab.modified = true;
            
            let operation = EditOperation::DeleteNewline {
                position: (current_tab.cursor_x, current_tab.cursor_y),
                merged_content: current_line.clone(),
            };
            current_tab.record_operation(operation);
        }
    }

    fn insert_char(&mut self, c: char) {
        let debug_mode = self.debug_mode;
        if debug_mode {
            log::info!("insert_char called with '{}' at cursor ({}, {})", c, 
                self.get_current_tab().cursor_x, self.get_current_tab().cursor_y);
        }
        
        // If there's a selection, delete it first
        {
            let current_tab = self.get_current_tab_mut();
            if current_tab.has_selection() {
                if debug_mode {
                    log::info!("Deleting selection before inserting char");
                }
                current_tab.delete_selected_text();
            }
        }
        
        // Handle smart dedent for closing braces
        if c == '}' {
            let should_dedent = {
                let current_tab = self.get_current_tab();
                self.should_smart_dedent(current_tab)
            };
            if should_dedent {
                // Get the current tab index and handle smart dedent directly
                let current_tab_index = self.tab_index;
                if let Some(tab) = self.tabs.get_mut(current_tab_index) {
                    Self::smart_dedent_tab(tab);
                }
                return; // Smart dedent handles the insertion
            }
        }
        
        // Get auto-closing character before getting mutable reference
        let closing_char = {
            let current_tab = self.get_current_tab();
            let line_ref = &current_tab.content[current_tab.cursor_y];
            let cursor_x = current_tab.cursor_x;
            self.get_auto_closing_char(c, line_ref, cursor_x)
        };
        
        let current_tab = self.get_current_tab_mut();
        if current_tab.cursor_y >= current_tab.content.len() {
            current_tab.content.push(String::new());
        }

        let line = &mut current_tab.content[current_tab.cursor_y];
        let line_char_count = line.chars().count();
        if current_tab.cursor_x > line_char_count {
            line.push_str(&" ".repeat(current_tab.cursor_x - line_char_count));
        }
        
        let operation = EditOperation::InsertChar {
            position: (current_tab.cursor_x, current_tab.cursor_y),
            char: c,
        };

        let line = &mut current_tab.content[current_tab.cursor_y];
        let byte_pos = Self::char_to_byte_index(line, current_tab.cursor_x);
        line.insert(byte_pos, c);
        current_tab.cursor_x += 1;
        
        // Insert closing character if needed
        if let Some(close_char) = closing_char {
            let byte_pos = Self::char_to_byte_index(line, current_tab.cursor_x);
            line.insert(byte_pos, close_char);
        }
        
        current_tab.modified = true;
        current_tab.record_operation(operation);
    }

    fn delete_forward(&mut self) {
        // Delete key should delete selected text or character after cursor
        let current_tab = self.get_current_tab_mut();
        if current_tab.has_selection() {
            current_tab.delete_selected_text();
            return;
        }
        
        let cursor_x = current_tab.cursor_x;
        let cursor_y = current_tab.cursor_y;
        
        if cursor_y >= current_tab.content.len() {
            return;
        }
        
        let line_len = current_tab.content[cursor_y].len();
        
        if cursor_x < line_len {
            // Delete character after cursor - safely
            if let Some(deleted_char) = current_tab.content[cursor_y].chars().nth(cursor_x) {
                let operation = EditOperation::DeleteChar {
                    position: (cursor_x, cursor_y),
                    char: deleted_char,
                };
                
                let byte_pos = Self::char_to_byte_index(&current_tab.content[cursor_y], cursor_x);
                current_tab.content[cursor_y].remove(byte_pos);
                current_tab.modified = true;
                current_tab.record_operation(operation);
            }
        } else if cursor_y < current_tab.content.len() - 1 {
            // At end of line, merge with next line
            let next_line = current_tab.content.remove(cursor_y + 1);
            let operation = EditOperation::DeleteNewline {
                position: (cursor_x, cursor_y),
                merged_content: next_line.clone(),
            };
            
            current_tab.content[cursor_y].push_str(&next_line);
            current_tab.modified = true;
            current_tab.record_operation(operation);
        }
    }

    fn update_bracket_matching(&mut self) {
        if !self.highlight_matching_brackets {
            self.matching_bracket_pos = None;
            return;
        }

        let current_tab = self.get_current_tab();
        let matching_pos = utils::find_matching_bracket(&current_tab.content, current_tab.cursor_y, current_tab.cursor_x);
        self.matching_bracket_pos = matching_pos;
    }

    fn move_cursor_left(&mut self) {
        self.move_cursor_left_with_selection(false);
    }
    
    fn move_cursor_left_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        if current_tab.cursor_x > 0 {
            current_tab.cursor_x -= 1;
        } else if current_tab.cursor_y > 0 {
            current_tab.cursor_y -= 1;
            current_tab.cursor_x = current_tab.content[current_tab.cursor_y].chars().count();
        }
        
        if extend_selection {
            self.extend_selection();
        }
        
        // Update bracket matching after cursor movement
        self.update_bracket_matching();
    }

    fn move_cursor_right(&mut self) {
        self.move_cursor_right_with_selection(false);
    }
    
    fn move_cursor_right_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        let current_line_len = current_tab.content[current_tab.cursor_y].chars().count();
        if current_tab.cursor_x < current_line_len {
            current_tab.cursor_x += 1;
        } else if current_tab.cursor_y < current_tab.content.len() - 1 {
            current_tab.cursor_y += 1;
            current_tab.cursor_x = 0;
        }
        
        if extend_selection {
            self.extend_selection();
        }
        
        // Update bracket matching after cursor movement
        self.update_bracket_matching();
    }

    fn move_cursor_up(&mut self) {
        self.move_cursor_up_with_selection(false);
    }
    
    fn move_cursor_up_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        if current_tab.cursor_y > 0 {
            current_tab.cursor_y -= 1;
            let upper_line_len = current_tab.content[current_tab.cursor_y].len();
            current_tab.cursor_x = current_tab.cursor_x.min(upper_line_len);
        }
        
        if extend_selection {
            self.extend_selection();
        }
        
        // Update bracket matching after cursor movement
        self.update_bracket_matching();
    }

    fn move_cursor_down(&mut self) {
        self.move_cursor_down_with_selection(false);
    }
    
    fn move_cursor_down_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        if current_tab.cursor_y < current_tab.content.len() - 1 {
            current_tab.cursor_y += 1;
            let lower_line_len = current_tab.content[current_tab.cursor_y].len();
            current_tab.cursor_x = current_tab.cursor_x.min(lower_line_len);
        }
        
        if extend_selection {
            self.extend_selection();
        }
        
        // Update bracket matching after cursor movement
        self.update_bracket_matching();
    }

    // Home/End navigation methods
    fn move_to_line_start(&mut self) {
        self.move_to_line_start_with_selection(false);
    }
    
    fn move_to_line_start_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        current_tab.cursor_x = 0;
        
        if extend_selection {
            self.extend_selection();
        }
    }
    
    fn move_to_line_end(&mut self) {
        self.move_to_line_end_with_selection(false);
    }
    
    fn move_to_line_end_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        if current_tab.cursor_y < current_tab.content.len() {
            current_tab.cursor_x = current_tab.content[current_tab.cursor_y].len();
        }
        
        if extend_selection {
            self.extend_selection();
        }
    }
    
    fn move_to_file_start(&mut self) {
        self.move_to_file_start_with_selection(false);
    }
    
    fn move_to_file_start_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        current_tab.cursor_x = 0;
        current_tab.cursor_y = 0;
        current_tab.scroll_position = 0;
        
        if extend_selection {
            self.extend_selection();
        }
    }
    
    fn move_to_file_end(&mut self) {
        self.move_to_file_end_with_selection(false);
    }
    
    fn move_to_file_end_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        if !current_tab.content.is_empty() {
            current_tab.cursor_y = current_tab.content.len() - 1;
            current_tab.cursor_x = current_tab.content[current_tab.cursor_y].len();
        } else {
            current_tab.cursor_x = 0;
            current_tab.cursor_y = 0;
        }
        
        if extend_selection {
            self.extend_selection();
        }
    }

    // Word boundary helper methods
    fn is_word_char(&self, c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    
    fn find_prev_word_boundary(&self) -> (usize, usize) {
        let current_tab = self.get_current_tab();
        if current_tab.cursor_y >= current_tab.content.len() {
            return (0, 0);
        }
        
        let line = &current_tab.content[current_tab.cursor_y];
        let mut x = current_tab.cursor_x;
        let mut y = current_tab.cursor_y;
        
        // If at start of line, go to end of previous line
        if x == 0 {
            if y > 0 {
                y -= 1;
                if y < current_tab.content.len() {
                    x = current_tab.content[y].len();
                }
            }
            return (x, y);
        }
        
        // Move back one character to start
        x -= 1;
        
        // If current char is non-word, skip non-word chars
        if x < line.len() && !self.is_word_char(line.chars().nth(x).unwrap_or(' ')) {
            while x > 0 && !self.is_word_char(line.chars().nth(x - 1).unwrap_or(' ')) {
                x -= 1;
            }
        } else {
            // Skip word chars to find start of current word
            while x > 0 && self.is_word_char(line.chars().nth(x - 1).unwrap_or(' ')) {
                x -= 1;
            }
        }
        
        (x, y)
    }
    
    fn find_next_word_boundary(&self) -> (usize, usize) {
        let current_tab = self.get_current_tab();
        if current_tab.cursor_y >= current_tab.content.len() {
            return (0, 0);
        }
        
        let line = &current_tab.content[current_tab.cursor_y];
        let mut x = current_tab.cursor_x;
        let mut y = current_tab.cursor_y;
        
        // If at end of line, go to start of next line
        if x >= line.len() {
            if y < current_tab.content.len() - 1 {
                y += 1;
                x = 0;
            }
            return (x, y);
        }
        
        // If current char is word char, skip to end of word
        if self.is_word_char(line.chars().nth(x).unwrap_or(' ')) {
            while x < line.len() && self.is_word_char(line.chars().nth(x).unwrap_or(' ')) {
                x += 1;
            }
        }
        
        // Skip non-word chars to next word start
        while x < line.len() && !self.is_word_char(line.chars().nth(x).unwrap_or(' ')) {
            x += 1;
        }
        
        // If we reached end of line, go to next line
        if x >= line.len() && y < current_tab.content.len() - 1 {
            y += 1;
            x = 0;
        }
        
        (x, y)
    }
    
    // Word-wise navigation methods
    fn move_cursor_left_word(&mut self) {
        self.move_cursor_left_word_with_selection(false);
    }
    
    fn move_cursor_left_word_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let (new_x, new_y) = self.find_prev_word_boundary();
        let current_tab = self.get_current_tab_mut();
        current_tab.cursor_x = new_x;
        current_tab.cursor_y = new_y;
        
        if extend_selection {
            self.extend_selection();
        }
    }
    
    fn move_cursor_right_word(&mut self) {
        self.move_cursor_right_word_with_selection(false);
    }
    
    fn move_cursor_right_word_with_selection(&mut self, extend_selection: bool) {
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if !extend_selection {
            self.clear_selection();
        }
        
        let (new_x, new_y) = self.find_next_word_boundary();
        let current_tab = self.get_current_tab_mut();
        current_tab.cursor_x = new_x;
        current_tab.cursor_y = new_y;
        
        if extend_selection {
            self.extend_selection();
        }
    }

    // Go to Line dialog methods
    fn show_goto_line_dialog(&mut self) {
        self.dialog_mode = DialogMode::GoToLine;
        self.goto_line_input.clear();
    }
    
    fn close_dialog(&mut self) {
        // Clear search highlights when closing find/replace dialogs
        if matches!(self.dialog_mode, DialogMode::Find | DialogMode::Replace) {
            self.search_results.clear();
            self.clear_selection();
        }
        
        self.dialog_mode = DialogMode::None;
        self.goto_line_input.clear();
    }
    
    fn handle_goto_line_input(&mut self, c: char) {
        if c.is_ascii_digit() && self.goto_line_input.len() < 10 {
            self.goto_line_input.push(c);
        }
    }
    
    fn handle_goto_line_backspace(&mut self) {
        self.goto_line_input.pop();
    }
    
    fn execute_goto_line(&mut self) {
        if let Ok(line_number) = self.goto_line_input.parse::<usize>() {
            let current_tab = self.get_current_tab();
            if line_number > 0 && line_number <= current_tab.content.len() {
                // Convert to 0-based indexing
                let target_line = line_number - 1;
                let current_tab = self.get_current_tab_mut();
                current_tab.cursor_y = target_line;
                current_tab.cursor_x = 0;
                
                // Ensure the target line is visible by updating scroll position
                let visible_lines = 20; // Approximate visible lines - could be calculated dynamically
                if target_line < current_tab.scroll_position as usize {
                    current_tab.scroll_position = target_line as u16;
                } else if target_line >= (current_tab.scroll_position as usize + visible_lines) {
                    current_tab.scroll_position = (target_line.saturating_sub(visible_lines / 2)) as u16;
                }
                
                // Clear any selection when jumping to line
                self.clear_selection();
            }
        }
        self.close_dialog();
    }
    
    fn get_current_line_number(&self) -> usize {
        let current_tab = self.get_current_tab();
        current_tab.cursor_y + 1 // Convert to 1-based indexing for display
    }
    
    fn get_total_lines(&self) -> usize {
        let current_tab = self.get_current_tab();
        current_tab.content.len()
    }

    // Find/Replace methods
    fn show_find_dialog(&mut self) {
        self.dialog_mode = DialogMode::Find;
        // Keep existing search query for easier repeated searches
        self.find_all_matches();
    }
    
    fn show_replace_dialog(&mut self) {
        self.dialog_mode = DialogMode::Replace;
        self.replace_field_active = false; // Start with find field active
        // Keep existing search query and replace text
        self.find_all_matches();
    }
    
    fn handle_find_input(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        self.search_query.push(c);
        self.find_all_matches();
    }
    
    fn handle_replace_input(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        self.replace_text.push(c);
    }
    
    fn handle_find_backspace(&mut self) {
        self.search_query.pop();
        self.find_all_matches();
    }
    
    fn handle_replace_backspace(&mut self) {
        self.replace_text.pop();
    }
    
    fn handle_replace_dialog_input(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        
        if self.replace_field_active {
            self.replace_text.push(c);
        } else {
            self.search_query.push(c);
            self.find_all_matches();
        }
    }
    
    fn handle_replace_dialog_backspace(&mut self) {
        if self.replace_field_active {
            self.replace_text.pop();
        } else {
            self.search_query.pop();
            self.find_all_matches();
        }
    }
    
    fn switch_replace_field(&mut self) {
        self.replace_field_active = !self.replace_field_active;
    }
    
    fn toggle_case_sensitivity(&mut self) {
        self.case_sensitive = !self.case_sensitive;
        self.find_all_matches();
    }
    
    fn find_all_matches(&mut self) {
        self.search_results.clear();
        self.current_match_index = 0;
        
        if self.search_query.is_empty() {
            return;
        }
        
        let query = if self.case_sensitive {
            self.search_query.clone()
        } else {
            self.search_query.to_lowercase()
        };
        
        let content = {
            let current_tab = self.get_current_tab();
            current_tab.content.clone()
        };
        
        for (line_idx, line) in content.iter().enumerate() {
            let search_line = if self.case_sensitive {
                line.clone()
            } else {
                line.to_lowercase()
            };
            
            let mut start_pos = 0;
            while let Some(pos) = search_line[start_pos..].find(&query) {
                let actual_pos = start_pos + pos;
                self.search_results.push((line_idx, actual_pos, actual_pos + query.len()));
                start_pos = actual_pos + 1; // Find overlapping matches
            }
        }
        
        // If we have results, navigate to the first one that's at or after current cursor
        if !self.search_results.is_empty() {
            self.find_next_from_cursor();
        }
    }
    
    fn find_next_from_cursor(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        
        // Find the first match at or after the current cursor position
        let current_tab = self.get_current_tab();
        let cursor_line = current_tab.cursor_y;
        let cursor_col = current_tab.cursor_x;
        
        for (i, &(line, start, _end)) in self.search_results.iter().enumerate() {
            if line > cursor_line || (line == cursor_line && start >= cursor_col) {
                self.current_match_index = i;
                self.highlight_current_match();
                return;
            }
        }
        
        // If no match found after cursor, wrap to first match
        if !self.search_results.is_empty() {
            self.current_match_index = 0;
            self.highlight_current_match();
        }
    }
    
    fn find_next(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        
        if self.search_direction_forward {
            self.current_match_index = (self.current_match_index + 1) % self.search_results.len();
        } else {
            self.current_match_index = if self.current_match_index == 0 {
                self.search_results.len() - 1
            } else {
                self.current_match_index - 1
            };
        }
        
        self.highlight_current_match();
    }
    
    fn find_previous(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        
        self.current_match_index = if self.current_match_index == 0 {
            self.search_results.len() - 1
        } else {
            self.current_match_index - 1
        };
        
        self.highlight_current_match();
    }
    
    fn highlight_current_match(&mut self) {
        if self.search_results.is_empty() || self.current_match_index >= self.search_results.len() {
            return;
        }
        
        let (line, start, end) = self.search_results[self.current_match_index];
        
        // Move cursor to the match
        let current_tab = self.get_current_tab_mut();
        current_tab.cursor_y = line;
        current_tab.cursor_x = start;
        
        // Select the match text
        current_tab.selection_start = Some((start, line));
        current_tab.selection_end = Some((end, line));
        current_tab.selection_mode = true;
        
        // Ensure the match is visible by adjusting scroll
        let visible_lines = 20; // Approximate visible lines
        if line < current_tab.scroll_position as usize {
            current_tab.scroll_position = line as u16;
        } else if line >= (current_tab.scroll_position as usize + visible_lines) {
            current_tab.scroll_position = (line.saturating_sub(visible_lines / 2)) as u16;
        }
    }
    
    fn replace_current(&mut self) {
        if self.search_results.is_empty() || self.current_match_index >= self.search_results.len() {
            return;
        }
        
        let (line, start, end) = self.search_results[self.current_match_index];
        
        // Perform the replacement
        let replace_text = self.replace_text.clone();
        let current_tab = self.get_current_tab();
        let old_text = current_tab.content[line][start..end].to_string();
        let operation = EditOperation::ReplaceText {
            position: (start, line),
            old_text: old_text,
            new_text: replace_text.clone(),
        };
        
        // Replace the text
        let current_tab = self.get_current_tab_mut();
        current_tab.content[line].replace_range(start..end, &replace_text);
        current_tab.modified = true;
        current_tab.record_operation(operation);
        
        // Update search results to account for the replacement
        let length_diff = replace_text.len() as isize - (end - start) as isize;
        
        // Remove the current match from results
        self.search_results.remove(self.current_match_index);
        
        // Adjust positions of subsequent matches on the same line
        for result in &mut self.search_results {
            if result.0 == line && result.1 > start {
                result.1 = (result.1 as isize + length_diff).max(0) as usize;
                result.2 = (result.2 as isize + length_diff).max(0) as usize;
            }
        }
        
        // Adjust current match index
        if self.current_match_index >= self.search_results.len() && !self.search_results.is_empty() {
            self.current_match_index = self.search_results.len() - 1;
        }
        
        // Move to next match if available
        if !self.search_results.is_empty() && self.current_match_index < self.search_results.len() {
            self.highlight_current_match();
        } else {
            self.clear_selection();
        }
    }
    
    fn replace_all(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        
        let mut operations = Vec::new();
        let replaced_count = self.search_results.len();
        
        // Process replacements from end to beginning to maintain position accuracy
        let mut sorted_results = self.search_results.clone();
        sorted_results.sort_by(|a, b| {
            if a.0 == b.0 {
                b.1.cmp(&a.1) // Reverse order for same line
            } else {
                b.0.cmp(&a.0) // Reverse order for lines
            }
        });
        
        let replace_text = self.replace_text.clone();
        let sorted_results_for_replace = sorted_results.clone();
        
        // Get content and build operations first
        {
            let current_tab = self.get_current_tab();
            for (line, start, end) in sorted_results {
                let old_text = current_tab.content[line][start..end].to_string();
                let operation = EditOperation::ReplaceText {
                    position: (start, line),
                    old_text: old_text,
                    new_text: replace_text.clone(),
                };
                operations.push(operation);
            }
        }
        
        // Perform the replacements
        let current_tab = self.get_current_tab_mut();
        for (line, start, end) in sorted_results_for_replace {
            current_tab.content[line].replace_range(start..end, &replace_text);
        }
        
        // Record all operations as a batch
        if !operations.is_empty() {
            let batch_operation = EditOperation::BatchOperation { operations };
            current_tab.record_operation(batch_operation);
            current_tab.modified = true;
        }
        
        // Clear search results and selection after replace all
        self.search_results.clear();
        self.current_match_index = 0;
        self.clear_selection();
        
        log::info!("Replaced {} occurrences", replaced_count);
    }
    
    fn get_search_status(&self) -> String {
        if self.search_results.is_empty() {
            if self.search_query.is_empty() {
                String::new()
            } else {
                "No matches".to_string()
            }
        } else {
            format!("{} of {} matches", self.current_match_index + 1, self.search_results.len())
        }
    }

    fn insert_newline(&mut self) {
        // If there's a selection, delete it first
        {
            let current_tab = self.get_current_tab_mut();
            if current_tab.has_selection() {
                current_tab.delete_selected_text();
            }
        }
        
        // Calculate auto-indentation
        let (current_line, cursor_x) = {
            let current_tab = self.get_current_tab();
            let line = current_tab.content[current_tab.cursor_y].clone();
            let x = current_tab.cursor_x;
            (line, x)
        };
        let indent = self.calculate_auto_indent(&current_line, cursor_x);
        
        let current_tab = self.get_current_tab_mut();
        
        let operation = EditOperation::InsertNewline {
            position: (current_tab.cursor_x, current_tab.cursor_y),
        };
        
        let remaining = current_tab.content[current_tab.cursor_y].split_off(current_tab.cursor_x);
        current_tab.cursor_y += 1;
        current_tab.content.insert(current_tab.cursor_y, format!("{}{}", indent, remaining));
        current_tab.cursor_x = indent.len();
        current_tab.modified = true;
        
        current_tab.record_operation(operation);
    }

    fn calculate_auto_indent(&self, current_line: &str, cursor_x: usize) -> String {
        // Extract indentation from current line
        let mut base_indent = String::new();
        for ch in current_line.chars() {
            if ch == ' ' || ch == '\t' {
                base_indent.push(ch);
            } else {
                break;
            }
        }
        
        // Check if we need to increase indentation after opening braces
        let line_before_cursor = &current_line[..cursor_x.min(current_line.len())];
        let should_increase_indent = line_before_cursor.trim_end().ends_with('{') ||
                                   line_before_cursor.trim_end().ends_with('[') ||
                                   line_before_cursor.trim_end().ends_with('(');
        
        if should_increase_indent {
            format!("{}    ", base_indent) // Add 4 spaces for indentation
        } else {
            base_indent
        }
    }

    fn get_auto_closing_char(&self, c: char, line: &str, cursor_x: usize) -> Option<char> {
        // Check if the next character is already the closing character (skip auto-close)
        let next_char = line.chars().nth(cursor_x);
        
        match c {
            '{' => {
                // Don't auto-close if the next character is already '}'
                if next_char == Some('}') {
                    None
                } else {
                    Some('}')
                }
            }
            '[' => {
                if next_char == Some(']') {
                    None
                } else {
                    Some(']')
                }
            }
            '(' => {
                if next_char == Some(')') {
                    None
                } else {
                    Some(')')
                }
            }
            '"' => {
                if next_char == Some('"') {
                    None
                } else {
                    Some('"')
                }
            }
            '\'' => {
                if next_char == Some('\'') {
                    None
                } else {
                    Some('\'')
                }
            }
            _ => None,
        }
    }

    fn should_smart_dedent(&self, tab: &Tab) -> bool {
        if tab.cursor_y >= tab.content.len() {
            return false;
        }
        
        let line = &tab.content[tab.cursor_y];
        let before_cursor = &line[..tab.cursor_x.min(line.len())];
        
        // Only dedent if the line contains only whitespace before the cursor
        before_cursor.trim().is_empty()
    }

    fn smart_dedent(&mut self, tab: &mut Tab) {
        if tab.cursor_y >= tab.content.len() {
            return;
        }
        
        let line = &mut tab.content[tab.cursor_y];
        let before_cursor = line[..tab.cursor_x.min(line.len())].to_string();
        
        // Remove one level of indentation (4 spaces)
        let dedented = if before_cursor.len() >= 4 && before_cursor.ends_with("    ") {
            format!("{}{}{}", 
                &before_cursor[..before_cursor.len() - 4], 
                '}', 
                &line[tab.cursor_x.min(line.len())..])
        } else {
            format!("{}{}{}", 
                before_cursor, 
                '}', 
                &line[tab.cursor_x.min(line.len())..])
        };
        
        let operation = EditOperation::ReplaceText {
            position: (0, tab.cursor_y),
            old_text: line.clone(),
            new_text: dedented.clone(),
        };
        
        *line = dedented;
        tab.cursor_x = tab.cursor_x.saturating_sub(4).max(before_cursor.len() - 4 + 1);
        tab.modified = true;
        tab.record_operation(operation);
    }

    fn smart_dedent_tab(tab: &mut Tab) {
        if tab.cursor_y >= tab.content.len() {
            return;
        }
        
        let line = &mut tab.content[tab.cursor_y];
        let before_cursor = line[..tab.cursor_x.min(line.len())].to_string();
        
        // Remove one level of indentation (4 spaces)
        let dedented = if before_cursor.len() >= 4 && before_cursor.ends_with("    ") {
            format!("{}{}{}", 
                &before_cursor[..before_cursor.len() - 4], 
                '}', 
                &line[tab.cursor_x.min(line.len())..])
        } else {
            format!("{}{}{}", 
                before_cursor, 
                '}', 
                &line[tab.cursor_x.min(line.len())..])
        };
        
        let operation = EditOperation::ReplaceText {
            position: (0, tab.cursor_y),
            old_text: line.clone(),
            new_text: dedented.clone(),
        };
        
        *line = dedented;
        tab.cursor_x = tab.cursor_x.saturating_sub(4).max(before_cursor.len() - 4 + 1);
        tab.modified = true;
        tab.record_operation(operation);
    }

    fn indent_selection(&mut self) {
        let current_tab = self.get_current_tab_mut();
        
        if !current_tab.has_selection() {
            // No selection - just indent current line
            self.indent_line();
            return;
        }
        
        let (start_pos, end_pos) = current_tab.get_normalized_selection();
        let start_line = start_pos.1;
        let end_line = end_pos.1;
        
        // Collect operations for undo
        let mut operations = Vec::new();
        
        for line_num in start_line..=end_line {
            if line_num < current_tab.content.len() {
                let old_line = current_tab.content[line_num].clone();
                let new_line = format!("    {}", old_line); // Add 4 spaces
                
                operations.push(EditOperation::ReplaceText {
                    position: (0, line_num),
                    old_text: old_line,
                    new_text: new_line.clone(),
                });
                
                current_tab.content[line_num] = new_line;
            }
        }
        
        // Update selection to maintain relative positions
        if let (Some(start), Some(end)) = (current_tab.selection_start, current_tab.selection_end) {
            current_tab.selection_start = Some((start.0 + 4, start.1));
            current_tab.selection_end = Some((end.0 + 4, end.1));
        }
        
        // Update cursor position
        current_tab.cursor_x += 4;
        current_tab.modified = true;
        
        // Record batch operation
        if !operations.is_empty() {
            current_tab.record_operation(EditOperation::BatchOperation { operations });
        }
    }

    fn dedent_selection(&mut self) {
        let current_tab = self.get_current_tab_mut();
        
        if !current_tab.has_selection() {
            // No selection - just dedent current line
            self.dedent_line();
            return;
        }
        
        let (start_pos, end_pos) = current_tab.get_normalized_selection();
        let start_line = start_pos.1;
        let end_line = end_pos.1;
        
        // Collect operations for undo
        let mut operations = Vec::new();
        
        for line_num in start_line..=end_line {
            if line_num < current_tab.content.len() {
                let old_line = current_tab.content[line_num].clone();
                let new_line = if old_line.starts_with("    ") {
                    old_line[4..].to_string() // Remove 4 spaces
                } else if old_line.starts_with('\t') {
                    old_line[1..].to_string() // Remove 1 tab
                } else {
                    old_line.clone() // No indentation to remove
                };
                
                if new_line != old_line {
                    operations.push(EditOperation::ReplaceText {
                        position: (0, line_num),
                        old_text: old_line,
                        new_text: new_line.clone(),
                    });
                    
                    current_tab.content[line_num] = new_line;
                }
            }
        }
        
        // Update selection to maintain relative positions  
        if let (Some(start), Some(end)) = (current_tab.selection_start, current_tab.selection_end) {
            let new_start_x = start.0.saturating_sub(4);
            let new_end_x = end.0.saturating_sub(4);
            current_tab.selection_start = Some((new_start_x, start.1));
            current_tab.selection_end = Some((new_end_x, end.1));
        }
        
        // Update cursor position
        current_tab.cursor_x = current_tab.cursor_x.saturating_sub(4);
        current_tab.modified = true;
        
        // Record batch operation
        if !operations.is_empty() {
            current_tab.record_operation(EditOperation::BatchOperation { operations });
        }
    }

    fn indent_line(&mut self) {
        let tab = self.get_current_tab_mut();
        if tab.cursor_y >= tab.content.len() {
            return;
        }
        
        let old_line = tab.content[tab.cursor_y].clone();
        let new_line = format!("    {}", old_line);
        
        let operation = EditOperation::ReplaceText {
            position: (0, tab.cursor_y),
            old_text: old_line,
            new_text: new_line.clone(),
        };
        
        tab.content[tab.cursor_y] = new_line;
        tab.cursor_x += 4;
        tab.modified = true;
        tab.record_operation(operation);
    }

    fn dedent_line(&mut self) {
        let tab = self.get_current_tab_mut();
        if tab.cursor_y >= tab.content.len() {
            return;
        }
        
        let old_line = tab.content[tab.cursor_y].clone();
        let new_line = if old_line.starts_with("    ") {
            old_line[4..].to_string()
        } else if old_line.starts_with('\t') {
            old_line[1..].to_string()
        } else {
            return; // No indentation to remove
        };
        
        let operation = EditOperation::ReplaceText {
            position: (0, tab.cursor_y),
            old_text: old_line,
            new_text: new_line.clone(),
        };
        
        tab.content[tab.cursor_y] = new_line;
        tab.cursor_x = tab.cursor_x.saturating_sub(4);
        tab.modified = true;
        tab.record_operation(operation);
    }

    fn toggle_comment(&mut self) {
        let has_selection = {
            let current_tab = self.get_current_tab();
            current_tab.has_selection()
        };
        
        if !has_selection {
            // No selection - toggle comment on current line
            self.toggle_line_comment();
        } else {
            // Has selection - toggle comment on all selected lines
            self.toggle_selection_comment();
        }
    }

    fn toggle_line_comment(&mut self) {
        let tab = self.get_current_tab_mut();
        if tab.cursor_y >= tab.content.len() {
            return;
        }
        
        let line = &tab.content[tab.cursor_y];
        let trimmed = line.trim_start();
        
        let (new_line, cursor_offset) = if trimmed.starts_with("//") {
            // Remove comment
            let indent = line.len() - trimmed.len();
            let after_comment = if trimmed.len() > 2 && trimmed.chars().nth(2) == Some(' ') {
                &trimmed[3..] // Remove "// "
            } else {
                &trimmed[2..] // Remove "//"
            };
            (format!("{}{}", " ".repeat(indent), after_comment), -3i32) // -3 for "// "
        } else {
            // Add comment
            let indent = line.len() - trimmed.len();
            (format!("{}// {}", " ".repeat(indent), trimmed), 3i32) // +3 for "// "
        };
        
        let operation = EditOperation::ReplaceText {
            position: (0, tab.cursor_y),
            old_text: line.clone(),
            new_text: new_line.clone(),
        };
        
        tab.content[tab.cursor_y] = new_line;
        
        // Adjust cursor position
        if cursor_offset > 0 {
            tab.cursor_x += cursor_offset as usize;
        } else {
            tab.cursor_x = tab.cursor_x.saturating_sub((-cursor_offset) as usize);
        }
        
        tab.modified = true;
        tab.record_operation(operation);
    }

    fn toggle_selection_comment(&mut self) {
        let tab = self.get_current_tab_mut();
        let (start_pos, end_pos) = tab.get_normalized_selection();
        let start_line = start_pos.1;
        let end_line = end_pos.1;
        
        // Check if all lines are commented
        let all_commented = (start_line..=end_line)
            .filter(|&line_num| line_num < tab.content.len())
            .all(|line_num| {
                let line = &tab.content[line_num];
                line.trim_start().starts_with("//")
            });
        
        let mut operations = Vec::new();
        let mut cursor_offset = 0i32;
        
        for line_num in start_line..=end_line {
            if line_num < tab.content.len() {
                let line = &tab.content[line_num];
                let trimmed = line.trim_start();
                
                let new_line = if all_commented && trimmed.starts_with("//") {
                    // Remove comment
                    let indent = line.len() - trimmed.len();
                    let after_comment = if trimmed.len() > 2 && trimmed.chars().nth(2) == Some(' ') {
                        &trimmed[3..] // Remove "// "
                    } else {
                        &trimmed[2..] // Remove "//"
                    };
                    if line_num == tab.cursor_y {
                        cursor_offset = -3;
                    }
                    format!("{}{}", " ".repeat(indent), after_comment)
                } else if !all_commented {
                    // Add comment
                    let indent = line.len() - trimmed.len();
                    if line_num == tab.cursor_y {
                        cursor_offset = 3;
                    }
                    format!("{}// {}", " ".repeat(indent), trimmed)
                } else {
                    line.clone()
                };
                
                if new_line != *line {
                    operations.push(EditOperation::ReplaceText {
                        position: (0, line_num),
                        old_text: line.clone(),
                        new_text: new_line.clone(),
                    });
                    
                    tab.content[line_num] = new_line;
                }
            }
        }
        
        // Adjust cursor position
        if cursor_offset > 0 {
            tab.cursor_x += cursor_offset as usize;
        } else {
            tab.cursor_x = tab.cursor_x.saturating_sub((-cursor_offset) as usize);
        }
        
        // Update selection to maintain relative positions
        if let (Some(start), Some(end)) = (tab.selection_start, tab.selection_end) {
            let offset = if cursor_offset > 0 { cursor_offset as usize } else { 0 };
            let sub_offset = if cursor_offset < 0 { (-cursor_offset) as usize } else { 0 };
            
            tab.selection_start = Some((start.0 + offset - sub_offset, start.1));
            tab.selection_end = Some((end.0 + offset - sub_offset, end.1));
        }
        
        tab.modified = true;
        
        // Record batch operation
        if !operations.is_empty() {
            tab.record_operation(EditOperation::BatchOperation { operations });
        }
    }

    fn duplicate_line(&mut self) {
        let has_selection = {
            let current_tab = self.get_current_tab();
            current_tab.has_selection()
        };
        
        if !has_selection {
            // No selection - duplicate current line
            self.duplicate_current_line();
        } else {
            // Has selection - duplicate selected lines
            self.duplicate_selected_lines();
        }
    }

    fn duplicate_current_line(&mut self) {
        let tab = self.get_current_tab_mut();
        if tab.cursor_y >= tab.content.len() {
            return;
        }
        
        let line_to_duplicate = tab.content[tab.cursor_y].clone();
        
        let operation = EditOperation::InsertText {
            position: (0, tab.cursor_y + 1),
            text: format!("{}\n", line_to_duplicate),
        };
        
        tab.content.insert(tab.cursor_y + 1, line_to_duplicate);
        tab.cursor_y += 1;
        tab.modified = true;
        tab.record_operation(operation);
    }

    fn duplicate_selected_lines(&mut self) {
        let tab = self.get_current_tab_mut();
        let (start_pos, end_pos) = tab.get_normalized_selection();
        let start_line = start_pos.1;
        let end_line = end_pos.1;
        
        // Collect lines to duplicate
        let lines_to_duplicate: Vec<String> = (start_line..=end_line)
            .filter(|&line_num| line_num < tab.content.len())
            .map(|line_num| tab.content[line_num].clone())
            .collect();
        
        // Insert duplicated lines after the selection
        for (i, line) in lines_to_duplicate.iter().enumerate() {
            tab.content.insert(end_line + 1 + i, line.clone());
        }
        
        let operation = EditOperation::InsertText {
            position: (0, end_line + 1),
            text: lines_to_duplicate.join("\n") + "\n",
        };
        
        // Move cursor to the duplicated selection
        tab.cursor_y = end_line + lines_to_duplicate.len();
        tab.modified = true;
        tab.record_operation(operation);
    }

    fn move_line_up(&mut self) {
        let has_selection = {
            let current_tab = self.get_current_tab();
            current_tab.has_selection()
        };
        
        if !has_selection {
            self.move_current_line_up();
        } else {
            self.move_selected_lines_up();
        }
    }

    fn move_line_down(&mut self) {
        let has_selection = {
            let current_tab = self.get_current_tab();
            current_tab.has_selection()
        };
        
        if !has_selection {
            self.move_current_line_down();
        } else {
            self.move_selected_lines_down();
        }
    }

    fn move_current_line_up(&mut self) {
        let tab = self.get_current_tab_mut();
        if tab.cursor_y == 0 || tab.cursor_y >= tab.content.len() {
            return;
        }
        
        // Swap current line with previous line
        tab.content.swap(tab.cursor_y - 1, tab.cursor_y);
        tab.cursor_y -= 1;
        tab.modified = true;
        
        // Record the operation
        let operation = EditOperation::BatchOperation {
            operations: vec![
                EditOperation::ReplaceText {
                    position: (0, tab.cursor_y),
                    old_text: tab.content[tab.cursor_y + 1].clone(),
                    new_text: tab.content[tab.cursor_y].clone(),
                },
                EditOperation::ReplaceText {
                    position: (0, tab.cursor_y + 1),
                    old_text: tab.content[tab.cursor_y].clone(),
                    new_text: tab.content[tab.cursor_y + 1].clone(),
                },
            ],
        };
        tab.record_operation(operation);
    }

    fn move_current_line_down(&mut self) {
        let tab = self.get_current_tab_mut();
        if tab.cursor_y + 1 >= tab.content.len() {
            return;
        }
        
        // Swap current line with next line
        tab.content.swap(tab.cursor_y, tab.cursor_y + 1);
        tab.cursor_y += 1;
        tab.modified = true;
        
        // Record the operation
        let operation = EditOperation::BatchOperation {
            operations: vec![
                EditOperation::ReplaceText {
                    position: (0, tab.cursor_y - 1),
                    old_text: tab.content[tab.cursor_y].clone(),
                    new_text: tab.content[tab.cursor_y - 1].clone(),
                },
                EditOperation::ReplaceText {
                    position: (0, tab.cursor_y),
                    old_text: tab.content[tab.cursor_y - 1].clone(),
                    new_text: tab.content[tab.cursor_y].clone(),
                },
            ],
        };
        tab.record_operation(operation);
    }

    fn move_selected_lines_up(&mut self) {
        let tab = self.get_current_tab_mut();
        let (start_pos, end_pos) = tab.get_normalized_selection();
        let start_line = start_pos.1;
        let end_line = end_pos.1;
        
        if start_line == 0 {
            return; // Can't move up from top
        }
        
        // Move the line above the selection down to after the selection
        let line_above = tab.content.remove(start_line - 1);
        tab.content.insert(end_line, line_above);
        
        // Update cursor and selection positions
        tab.cursor_y = tab.cursor_y.saturating_sub(1);
        if let (Some(start), Some(end)) = (tab.selection_start, tab.selection_end) {
            tab.selection_start = Some((start.0, start.1.saturating_sub(1)));
            tab.selection_end = Some((end.0, end.1.saturating_sub(1)));
        }
        
        tab.modified = true;
        
        // For simplicity, record as a batch operation
        let operation = EditOperation::BatchOperation {
            operations: vec![], // Could be more detailed
        };
        tab.record_operation(operation);
    }

    fn move_selected_lines_down(&mut self) {
        let tab = self.get_current_tab_mut();
        let (start_pos, end_pos) = tab.get_normalized_selection();
        let start_line = start_pos.1;
        let end_line = end_pos.1;
        
        if end_line + 1 >= tab.content.len() {
            return; // Can't move down from bottom
        }
        
        // Move the line below the selection up to before the selection
        let line_below = tab.content.remove(end_line + 1);
        tab.content.insert(start_line, line_below);
        
        // Update cursor and selection positions
        tab.cursor_y += 1;
        if let (Some(start), Some(end)) = (tab.selection_start, tab.selection_end) {
            tab.selection_start = Some((start.0, start.1 + 1));
            tab.selection_end = Some((end.0, end.1 + 1));
        }
        
        tab.modified = true;
        
        // For simplicity, record as a batch operation
        let operation = EditOperation::BatchOperation {
            operations: vec![], // Could be more detailed
        };
        tab.record_operation(operation);
    }

    fn delete_line(&mut self) {
        let has_selection = {
            let current_tab = self.get_current_tab();
            current_tab.has_selection()
        };
        
        if !has_selection {
            self.delete_current_line();
        } else {
            self.delete_selected_lines();
        }
    }

    fn delete_current_line(&mut self) {
        let tab = self.get_current_tab_mut();
        if tab.content.len() <= 1 {
            // Don't delete the last line, just clear it
            let old_line = tab.content[0].clone();
            tab.content[0] = String::new();
            tab.cursor_x = 0;
            
            let operation = EditOperation::ReplaceText {
                position: (0, 0),
                old_text: old_line,
                new_text: String::new(),
            };
            tab.record_operation(operation);
        } else {
            let deleted_line = tab.content.remove(tab.cursor_y);
            
            // Adjust cursor position
            if tab.cursor_y >= tab.content.len() {
                tab.cursor_y = tab.content.len().saturating_sub(1);
            }
            tab.cursor_x = 0;
            
            let operation = EditOperation::DeleteText {
                position: (0, tab.cursor_y),
                text: deleted_line + "\n",
            };
            tab.record_operation(operation);
        }
        
        tab.modified = true;
    }

    fn delete_selected_lines(&mut self) {
        let tab = self.get_current_tab_mut();
        let (start_pos, end_pos) = tab.get_normalized_selection();
        let start_line = start_pos.1;
        let end_line = end_pos.1;
        
        // Collect deleted lines for undo
        let deleted_lines: Vec<String> = (start_line..=end_line)
            .filter(|&line_num| line_num < tab.content.len())
            .map(|line_num| tab.content[line_num].clone())
            .collect();
        
        // Remove lines from end to start to maintain indices
        for line_num in (start_line..=end_line).rev() {
            if line_num < tab.content.len() {
                tab.content.remove(line_num);
            }
        }
        
        // Ensure at least one line remains
        if tab.content.is_empty() {
            tab.content.push(String::new());
        }
        
        // Adjust cursor position
        tab.cursor_y = start_line.min(tab.content.len().saturating_sub(1));
        tab.cursor_x = 0;
        
        // Clear selection
        tab.selection_start = None;
        tab.selection_end = None;
        tab.selection_mode = false;
        
        let operation = EditOperation::DeleteText {
            position: (0, start_line),
            text: deleted_lines.join("\n") + "\n",
        };
        tab.record_operation(operation);
        tab.modified = true;
    }

    fn smart_home(&mut self) {
        let current_tab = self.get_current_tab_mut();
        
        if current_tab.cursor_y >= current_tab.content.len() {
            return;
        }
        
        let line = &current_tab.content[current_tab.cursor_y];
        
        // Find first non-whitespace character
        let first_non_whitespace = line.chars()
            .position(|c| !c.is_whitespace())
            .unwrap_or(0);
        
        // Toggle between beginning of line and first non-whitespace
        if current_tab.cursor_x == 0 {
            // At beginning - move to first non-whitespace
            current_tab.cursor_x = first_non_whitespace;
        } else if current_tab.cursor_x == first_non_whitespace {
            // At first non-whitespace - move to beginning
            current_tab.cursor_x = 0;
        } else {
            // Somewhere else - move to first non-whitespace
            current_tab.cursor_x = first_non_whitespace;
        }
    }

    fn find_matching_bracket(&self, tab: &Tab) -> Option<(usize, usize)> {
        if tab.cursor_y >= tab.content.len() {
            return None;
        }
        
        let line = &tab.content[tab.cursor_y];
        if tab.cursor_x >= line.len() {
            return None;
        }
        
        let current_char = line.chars().nth(tab.cursor_x)?;
        
        match current_char {
            '(' => self.find_closing_bracket(tab, '(', ')'),
            '[' => self.find_closing_bracket(tab, '[', ']'),
            '{' => self.find_closing_bracket(tab, '{', '}'),
            ')' => self.find_opening_bracket(tab, ')', '('),
            ']' => self.find_opening_bracket(tab, ']', '['),
            '}' => self.find_opening_bracket(tab, '}', '{'),
            _ => None,
        }
    }

    fn find_closing_bracket(&self, tab: &Tab, open: char, close: char) -> Option<(usize, usize)> {
        let mut depth = 1;
        let mut y = tab.cursor_y;
        let mut x = tab.cursor_x + 1;
        
        while y < tab.content.len() {
            let line = &tab.content[y];
            
            while x < line.len() {
                match line.chars().nth(x)? {
                    c if c == open => depth += 1,
                    c if c == close => {
                        depth -= 1;
                        if depth == 0 {
                            return Some((x, y));
                        }
                    }
                    _ => {}
                }
                x += 1;
            }
            
            y += 1;
            x = 0;
        }
        
        None
    }

    fn find_opening_bracket(&self, tab: &Tab, close: char, open: char) -> Option<(usize, usize)> {
        let mut depth = 1;
        let mut y = tab.cursor_y;
        let mut x = tab.cursor_x;
        
        loop {
            let line = &tab.content[y];
            
            while x > 0 {
                x -= 1;
                match line.chars().nth(x)? {
                    c if c == close => depth += 1,
                    c if c == open => {
                        depth -= 1;
                        if depth == 0 {
                            return Some((x, y));
                        }
                    }
                    _ => {}
                }
            }
            
            if y == 0 {
                break;
            }
            y -= 1;
            x = tab.content[y].len();
        }
        
        None
    }

    fn jump_to_matching_bracket(&mut self) {
        let current_tab = self.get_current_tab();
        
        if let Some((x, y)) = self.find_matching_bracket(current_tab) {
            let current_tab = self.get_current_tab_mut();
            current_tab.cursor_x = x;
            current_tab.cursor_y = y;
        }
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
        
        let new_scroll_pos = {
            let current_tab = self.get_current_tab_mut();
            let old_scroll = current_tab.scroll_position;
            current_tab.scroll_position = current_tab.scroll_position.saturating_sub(page_size);
            
            // Move cursor up by the same amount
            let scroll_diff = old_scroll - current_tab.scroll_position;
            for _ in 0..scroll_diff {
                if current_tab.cursor_y > 0 {
                    current_tab.cursor_y -= 1;
                } else {
                    break;
                }
            }
            // Ensure cursor_x is within bounds of the new line
            if current_tab.cursor_y < current_tab.content.len() {
                let line_len = current_tab.content[current_tab.cursor_y].len();
                current_tab.cursor_x = current_tab.cursor_x.min(line_len);
            }
            current_tab.scroll_position
        };
        
        self.scroll_state = self.scroll_state.position(new_scroll_pos as usize);
    }

    fn scroll_down(&mut self) {
        // Move down by visible lines (approximate page size)
        let page_size = 20; // Approximate visible lines
        
        let new_scroll_pos = {
            let current_tab = self.get_current_tab_mut();
            let old_scroll = current_tab.scroll_position;
            let max_scroll = current_tab.content.len().saturating_sub(1) as u16;
            current_tab.scroll_position = (current_tab.scroll_position + page_size).min(max_scroll);
            
            // Move cursor down by the same amount
            let scroll_diff = current_tab.scroll_position - old_scroll;
            for _ in 0..scroll_diff {
                if current_tab.cursor_y < current_tab.content.len() - 1 {
                    current_tab.cursor_y += 1;
                } else {
                    break;
                }
            }
            // Ensure cursor_x is within bounds of the new line
            if current_tab.cursor_y < current_tab.content.len() {
                let line_len = current_tab.content[current_tab.cursor_y].len();
                current_tab.cursor_x = current_tab.cursor_x.min(line_len);
            }
            current_tab.scroll_position
        };
        
        self.scroll_state = self.scroll_state.position(new_scroll_pos as usize);
    }

    fn save_config(&self) -> std::io::Result<()> {
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
    
    fn save_file(&mut self) -> Result<(), String> {
        let current_tab = self.get_current_tab_mut();
        if let Some(filename) = &current_tab.filename {
            let content = current_tab.content.join("\n");
            std::fs::write(filename, content)
                .map_err(|e| format!("Failed to save file: {}", e))?;
            current_tab.modified = false;
            Ok(())
        } else {
            Err("No filename set for current tab".to_string())
        }
    }
    
    fn load_file(&mut self, filename: &str) -> Result<(), String> {
        self.open_file_in_tab(filename.to_string())
    }
    
    fn previous_tab(&mut self) {
        self.prev_tab();
    }
}

// Helper function to check if a point is inside a rectangle
fn point_in_rect(x: u16, y: u16, rect: ratatui::layout::Rect) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

fn main() -> Result<(), io::Error> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    // Check for help flag
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Nail IDE - A simple text editor for the Nail language");
        println!();
        println!("Usage: {} [OPTIONS] [FILE]", args[0]);
        println!();
        println!("Options:");
        println!("  --ide [FILE]    Run the IDE (default mode)");
        println!("  --debug         Enable debug logging");
        println!("  --help, -h      Show this help message");
        println!();
        println!("Examples:");
        println!("  {}              Start IDE with welcome screen", args[0]);
        println!("  {} file.nail    Open file.nail in the IDE", args[0]);
        println!("  {} --debug      Start IDE with debug logging", args[0]);
        return Ok(());
    }
    
    // Check for debug flag
    let debug_mode = args.iter().any(|arg| arg == "--debug") || env::var("NAIL_DEBUG").is_ok();
    
    // Set up logging
    let log_file = File::create("nail.log").expect("Failed to create log file");
    let log_level = if debug_mode {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };
    Builder::new().target(env_logger::Target::Pipe(Box::new(log_file))).filter_level(log_level).init();
    
    if debug_mode {
        log::warn!("Debug mode enabled via command line flag");
    }

    panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::capture();
        error!("Panic occurred: {:?}", panic_info);
        error!("Backtrace:\n{:?}", backtrace);
    }));

    let (tx_resize, rx_resize) = channel::<EditorMessage>();
    let (tx_key, rx_key) = channel::<EditorMessage>();
    let (tx_draw, rx_draw) = channel::<EditorMessage>();
    let (tx_build, rx_build) = channel::<EditorMessage>();
    let (tx_lex, rx_lex) = channel::<EditorMessage>();

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    // Create terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    let terminal = Arc::new(Mutex::new(terminal));

    // Check if a file was specified to open
    let file_to_open = args.iter()
        .filter(|arg| !arg.starts_with("--") && *arg != &args[0])
        .next()
        .cloned();
    
    let mut editor = Editor::new_with_debug(debug_mode);
    
    // Open the file if specified
    if let Some(filename) = file_to_open {
        if let Err(e) = editor.open_file_path(&filename) {
            log::error!("Failed to open file '{}': {}", filename, e);
        } else {
            log::info!("Opened file: {}", filename);
        }
    }
    
    let shared_editor = Arc::new(Mutex::new(editor));

    // Thread communication setup
    let editor_for_resize = Arc::clone(&shared_editor);
    let editor_for_key = Arc::clone(&shared_editor);
    let editor_for_draw = Arc::clone(&shared_editor);
    let editor_for_build = Arc::clone(&shared_editor);
    let editor_for_lex = Arc::clone(&shared_editor);

    // Launch the lexer and parser thread
    thread::spawn(move || {
        lex_and_parse_thread_logic(editor_for_lex, rx_lex);
    });

    // Launch the build thread
    let tx_draw_for_build = tx_draw.clone();
    thread::spawn(move || {
        build_thread_logic(editor_for_build, rx_build, tx_draw_for_build);
    });

    // Launch the key handling thread
    let tx_draw_for_key = tx_draw.clone();
    let tx_build_for_key = tx_build.clone();
    thread::spawn(move || {
        key_thread_logic(editor_for_key, rx_key, tx_draw_for_key, tx_build_for_key);
    });

    // Launch the resize thread
    let terminal_for_resize = terminal.clone();
    thread::spawn(move || {
        resize_thread_logic(terminal_for_resize, rx_resize);
    });

    // Main draw thread (this runs on the main thread)
    draw_thread_logic(terminal.clone(), editor_for_draw, rx_draw);
    
    // Clean up terminal on exit
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    
    Ok(())
}

#[derive(Debug, Clone)]
enum CompletionContext {
    None,
    Identifier(String),        // Typing an identifier, show matching functions/variables
    FunctionCall(String),       // Inside function call, show parameter hints
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

impl Editor {
    // Intellisense methods
    fn get_completion_context(&self) -> CompletionContext {
        let current_tab = self.get_current_tab();
        if current_tab.cursor_y >= current_tab.content.len() {
            return CompletionContext::None;
        }
        
        let line = &current_tab.content[current_tab.cursor_y];
        if current_tab.cursor_x > line.len() {
            return CompletionContext::None;
        }
        
        // Look for tokens around cursor position
        let cursor_line = current_tab.cursor_y + 1; // Lines are 1-indexed in CodeSpan
        let cursor_col = current_tab.cursor_x + 1;  // Columns are 1-indexed in CodeSpan
        
        // Check if we're inside a function call by looking for opening parenthesis
        let mut paren_depth = 0;
        let mut in_function_call = false;
        let mut function_name = String::new();
        
        for token in &current_tab.tokens {
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
        let current_tab = self.get_current_tab();
        let mut pos = 0;
        for i in 0..current_tab.cursor_y {
            if i < current_tab.content.len() {
                pos += current_tab.content[i].len() + 1; // +1 for newline
            }
        }
        pos + current_tab.cursor_x
    }
    
    fn get_current_word(&self) -> String {
        let current_tab = self.get_current_tab();
        if current_tab.cursor_y >= current_tab.content.len() {
            return String::new();
        }
        
        let line = &current_tab.content[current_tab.cursor_y];
        if current_tab.cursor_x > line.len() {
            return String::new();
        }
        
        // Find word boundaries
        let mut start = current_tab.cursor_x;
        while start > 0 && line.chars().nth(start - 1).map_or(false, |c| c.is_alphanumeric() || c == '_') {
            start -= 1;
        }
        
        let mut end = current_tab.cursor_x;
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
                    // Use ASCII case-insensitive comparison for better performance
                    if name.len() >= prefix.len() && name[..prefix.len()].eq_ignore_ascii_case(&prefix) {
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
                
                // Add symbols from scope (variables, structs, enums)
                let current_tab = self.get_current_tab();
                for symbol in &current_tab.scope_symbols {
                    // Use ASCII case-insensitive comparison for better performance
                    if symbol.name.len() >= prefix.len() && symbol.name[..prefix.len()].eq_ignore_ascii_case(&prefix) {
                        let (kind, description) = match &symbol.symbol_type {
                            SymbolType::Variable => (CompletionKind::Variable, "Local variable".to_string()),
                            SymbolType::Struct { .. } => (CompletionKind::Struct, "Struct type".to_string()),
                            SymbolType::Enum { .. } => (CompletionKind::Enum, "Enum type".to_string()),
                        };
                        
                        completions.push(CompletionItem {
                            label: symbol.name.clone(),
                            detail: symbol.data_type.clone().unwrap_or_else(String::new),
                            description,
                            example: String::new(),
                            kind,
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
            // Generate insertion text based on completion kind (before any mutable borrows)
            let insertion_text = self.generate_insertion_text(&completion);
            
            let current_tab = self.get_current_tab_mut();
            let line = &mut current_tab.content[current_tab.cursor_y];
            
            // Find the start of the current word
            let mut start = current_tab.cursor_x;
            while start > 0 && line.chars().nth(start - 1).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                start -= 1;
            }
            
            // Find the end of the current word
            let mut end = current_tab.cursor_x;
            while end < line.len() && line.chars().nth(end).map_or(false, |c| c.is_alphanumeric() || c == '_') {
                end += 1;
            }
            
            // Handle multi-line insertions
            let insertion_lines: Vec<&str> = insertion_text.split('\n').collect();
            
            if insertion_lines.len() == 1 {
                // Single line insertion - simple replacement
                let before = line[..start].to_string();
                let after = line[end..].to_string();
                *line = format!("{}{}{}", before, insertion_text, after);
                current_tab.cursor_x = start + insertion_text.len();
            } else {
                // Multi-line insertion
                let before = line[..start].to_string();
                let after = line[end..].to_string();
                
                // Replace current line with first line
                *line = format!("{}{}", before, insertion_lines[0]);
                
                // Insert additional lines
                for (i, insertion_line) in insertion_lines[1..].iter().enumerate() {
                    let new_line = if i == insertion_lines.len() - 2 {
                        // Last line - add the remaining content from original line
                        format!("{}{}", insertion_line, after)
                    } else {
                        insertion_line.to_string()
                    };
                    current_tab.content.insert(current_tab.cursor_y + 1 + i, new_line);
                }
                
                // Position cursor at end of last inserted line
                current_tab.cursor_y += insertion_lines.len() - 1;
                let last_line_addition = if insertion_lines.len() > 1 {
                    insertion_lines[insertion_lines.len() - 1]
                } else {
                    ""
                };
                current_tab.cursor_x = last_line_addition.len() + if insertion_lines.len() > 1 { after.len() } else { 0 };
            }
            
            current_tab.modified = true;
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
    
    fn generate_field_placeholder(&self, field_type: &str, field_name: &str) -> String {
        // Parse the field type string and generate appropriate placeholder using Nail syntax
        match field_type {
            "s" => {
                match field_name {
                    "name" | "title" | "label" => "`name`".to_string(),
                    "email" => "`user@example.com`".to_string(),
                    "url" | "link" => "`https://example.com`".to_string(),
                    "path" => "`/path/to/file`".to_string(),
                    _ => "`value`".to_string(),
                }
            }
            "i" => {
                match field_name {
                    "age" => "0".to_string(),
                    "count" | "size" | "length" => "0".to_string(),
                    "port" => "8080".to_string(),
                    "id" => "1".to_string(),
                    _ => "0".to_string(),
                }
            }
            "f" => "0.0".to_string(),
            "b" => {
                match field_name {
                    name if name.contains("enable") || name.contains("active") => "true".to_string(),
                    name if name.contains("disable") || name.contains("hidden") => "false".to_string(),
                    _ => "false".to_string(),
                }
            }
            t if t.starts_with("[") => "[]".to_string(),
            t if t.starts_with("h<") => "hashmap_new()".to_string(),
            _ => "`value`".to_string(),
        }
    }

    fn generate_parameter_placeholder(&self, param_type: &lexer::NailDataTypeDescriptor, param_name: &str) -> String {
        match param_type {
            lexer::NailDataTypeDescriptor::String => {
                // Use backticks for string literals in Nail
                match param_name {
                    "url" => "`https://api.example.com`".to_string(),
                    "method" => "`GET`".to_string(),
                    "path" => "`/path/to/file`".to_string(),
                    "content" | "data" | "body" => "`data`".to_string(),
                    "key" => "`key`".to_string(),
                    "name" => "`name`".to_string(),
                    "host" => "`localhost`".to_string(),
                    _ => "`value`".to_string(),
                }
            }
            lexer::NailDataTypeDescriptor::Int => {
                match param_name {
                    "port" => "8080".to_string(),
                    "timeout" => "5000".to_string(),
                    "max_" if param_name.starts_with("max_") => "100".to_string(),
                    "min_" if param_name.starts_with("min_") => "0".to_string(),
                    name if name.contains("count") || name.contains("size") || name.contains("limit") => "10".to_string(),
                    name if name.contains("id") => "1".to_string(),
                    _ => "0".to_string(),
                }
            }
            lexer::NailDataTypeDescriptor::Float => "0.0".to_string(),
            lexer::NailDataTypeDescriptor::Boolean => {
                match param_name {
                    name if name.contains("enable") || name.contains("active") => "true".to_string(),
                    name if name.contains("disable") || name.contains("hidden") => "false".to_string(),
                    _ => "false".to_string(),
                }
            }
            lexer::NailDataTypeDescriptor::Array(_) => "[]".to_string(),
            lexer::NailDataTypeDescriptor::HashMap(_, _) => "hashmap_new()".to_string(),
            lexer::NailDataTypeDescriptor::Result(_) => {
                // For result types, provide meaningful defaults
                match param_name {
                    "url" => "`https://api.example.com`".to_string(),
                    "method" => "`GET`".to_string(),
                    "path" => "`/path/to/file`".to_string(),
                    "content" | "data" | "body" => "`data`".to_string(),
                    "key" => "`key`".to_string(),
                    "name" => "`name`".to_string(),
                    _ => "`value`".to_string(),
                }
            }
            lexer::NailDataTypeDescriptor::Any => {
                // Provide contextual defaults based on parameter name
                match param_name {
                    "url" => "`https://api.example.com`".to_string(),
                    "method" => "`GET`".to_string(),
                    "headers" => "hashmap_new()".to_string(),
                    "body" | "data" | "content" => "`data`".to_string(),
                    "path" => "`/path/to/file`".to_string(),
                    "port" => "8080".to_string(),
                    "host" => "`localhost`".to_string(),
                    "timeout" => "5000".to_string(),
                    "max_" if param_name.starts_with("max_") => "100".to_string(),
                    "min_" if param_name.starts_with("min_") => "0".to_string(),
                    name if name.contains("count") || name.contains("size") || name.contains("limit") => "10".to_string(),
                    name if name.contains("name") || name.contains("key") || name.contains("id") => format!("`{}`", name),
                    name if name.contains("enable") || name.contains("disable") || name.ends_with("ed") => "true".to_string(),
                    _ => "`value`".to_string(),
                }
            }
            lexer::NailDataTypeDescriptor::Struct(name) => format!("{} {{}}", name),
            lexer::NailDataTypeDescriptor::Enum(name) => format!("{}::", name),
            _ => format!("`{}`", param_name), // Fallback with backticks
        }
    }

    fn generate_insertion_text(&self, completion: &CompletionItem) -> String {
        match completion.kind {
            CompletionKind::Function => {
                // Get function info from stdlib registry
                use crate::stdlib_registry::get_stdlib_function;
                if let Some(func) = get_stdlib_function(&completion.label) {
                    if func.parameters.is_empty() {
                        format!("{}()", completion.label)
                    } else {
                        // Generate variable declarations and function call
                        let mut lines = Vec::new();
                        let mut param_names = Vec::new();
                        
                        for param in &func.parameters {
                            let type_str = format_type(&param.param_type);
                            let value_placeholder = self.generate_parameter_placeholder(&param.param_type, &param.name);
                            // Ensure no extra spaces in type formatting
                            let clean_type_str = type_str.replace(" ", "");
                            lines.push(format!("{}:{} = {};", param.name, clean_type_str, value_placeholder));
                            param_names.push(param.name.clone());
                        }
                        
                        // For functions that return values, assign to a variable
                        use crate::stdlib_registry::get_stdlib_function;
                        if let Some(func) = get_stdlib_function(&completion.label) {
                            match &func.return_type {
                                lexer::NailDataTypeDescriptor::Void => {
                                    // Void functions just call
                                    lines.push(format!("{}({});", completion.label, param_names.join(", ")));
                                }
                                lexer::NailDataTypeDescriptor::Result(inner_type) => {
                                    // Result types need danger() wrapper and inner type for assignment
                                    let inner_type_str = format_type(inner_type);
                                    lines.push(format!("result: {} = danger({}({}));", inner_type_str, completion.label, param_names.join(", ")));
                                }
                                _ => {
                                    // Other functions that return values need assignment
                                    let return_type_str = format_type(&func.return_type);
                                    lines.push(format!("result: {} = {}({});", return_type_str, completion.label, param_names.join(", ")));
                                }
                            }
                        } else {
                            // Fallback for unknown functions
                            lines.push(format!("{}({});", completion.label, param_names.join(", ")));
                        }
                        lines.join("\n")
                    }
                } else {
                    // Fallback for unknown functions
                    format!("{}()", completion.label)
                }
            }
            CompletionKind::Struct => {
                // Find struct info from scope symbols
                let current_tab = self.get_current_tab();
                if let Some(symbol) = current_tab.scope_symbols.iter().find(|s| s.name == completion.label) {
                    if let SymbolType::Struct { fields } = &symbol.symbol_type {
                        if fields.is_empty() {
                            format!("{} {{}};", completion.label)
                        } else {
                            let field_placeholders: Vec<String> = fields.iter()
                                .map(|(name, field_type)| {
                                    let placeholder_value = self.generate_field_placeholder(field_type, name);
                                    format!("{} = {}", name, placeholder_value)
                                })
                                .collect();
                            format!("{} {{ {} }};", completion.label, field_placeholders.join(", "))
                        }
                    } else {
                        format!("{};", completion.label)
                    }
                } else {
                    // Fallback - just the struct name with semicolon
                    format!("{};", completion.label)
                }
            }
            CompletionKind::Enum => {
                // For enums, we just insert the enum name - user will need to add variant
                completion.label.clone()
            }
            CompletionKind::Variable => {
                // Variables just insert themselves
                completion.label.clone()
            }
            CompletionKind::Keyword => {
                // Keywords just insert themselves
                completion.label.clone()
            }
        }
    }
    
    // Selection management methods
    fn start_selection(&mut self) {
        let current_tab = self.get_current_tab_mut();
        current_tab.selection_start = Some((current_tab.cursor_x, current_tab.cursor_y));
        current_tab.selection_end = Some((current_tab.cursor_x, current_tab.cursor_y));
        current_tab.selection_mode = true;
    }
    
    fn extend_selection(&mut self) {
        let current_tab = self.get_current_tab_mut();
        if current_tab.selection_start.is_none() {
            current_tab.selection_start = Some((current_tab.cursor_x, current_tab.cursor_y));
            current_tab.selection_end = Some((current_tab.cursor_x, current_tab.cursor_y));
            current_tab.selection_mode = true;
        } else {
            current_tab.selection_end = Some((current_tab.cursor_x, current_tab.cursor_y));
        }
    }
    
    fn clear_selection(&mut self) {
        let current_tab = self.get_current_tab_mut();
        current_tab.selection_start = None;
        current_tab.selection_end = None;
        current_tab.selection_mode = false;
    }
    
    fn has_selection(&self) -> bool {
        let current_tab = self.get_current_tab();
        current_tab.selection_start.is_some() && current_tab.selection_end.is_some() &&
        current_tab.selection_start != current_tab.selection_end
    }
    
    fn get_selected_text(&self) -> String {
        if !self.has_selection() {
            return String::new();
        }
        
        let current_tab = self.get_current_tab();
        // Safely get selection bounds
        let (start, end) = match (current_tab.selection_start, current_tab.selection_end) {
            (Some(s), Some(e)) => (s, e),
            _ => return String::new(),
        };
        
        // Normalize selection order (start should be before end)
        let (start_pos, end_pos) = self.normalize_selection(start, end);
        
        if start_pos.1 == end_pos.1 {
            // Single line selection
            if start_pos.1 < current_tab.content.len() {
                let line = &current_tab.content[start_pos.1];
                let start_x = start_pos.0.min(line.len());
                let end_x = end_pos.0.min(line.len());
                return line[start_x..end_x].to_string();
            }
        } else {
            // Multi-line selection
            let mut result = String::new();
            
            for line_idx in start_pos.1..=end_pos.1 {
                if line_idx >= current_tab.content.len() {
                    break;
                }
                
                let line = &current_tab.content[line_idx];
                
                if line_idx == start_pos.1 {
                    // First line - from start_x to end of line
                    let start_x = start_pos.0.min(line.len());
                    result.push_str(&line[start_x..]);
                } else if line_idx == end_pos.1 {
                    // Last line - from beginning to end_x
                    let end_x = end_pos.0.min(line.len());
                    result.push_str(&line[..end_x]);
                } else {
                    // Middle lines - entire line
                    result.push_str(line);
                }
                
                // Add newline except for the last line
                if line_idx < end_pos.1 {
                    result.push('\n');
                }
            }
            
            return result;
        }
        
        String::new()
    }
    
    fn normalize_selection(&self, start: (usize, usize), end: (usize, usize)) -> ((usize, usize), (usize, usize)) {
        // Return (start_pos, end_pos) where start_pos is before end_pos
        if start.1 < end.1 || (start.1 == end.1 && start.0 <= end.0) {
            (start, end)
        } else {
            (end, start)
        }
    }
    
    fn delete_selected_text(&mut self) {
        if !self.has_selection() {
            return;
        }
        
        let selected_text = self.get_selected_text();
        let (start, end) = {
            let current_tab = self.get_current_tab();
            // Safely get selection bounds
            match (current_tab.selection_start, current_tab.selection_end) {
                (Some(s), Some(e)) => (s, e),
                _ => return,
            }
        };
        let (start_pos, end_pos) = self.normalize_selection(start, end);
        let current_tab = self.get_current_tab_mut();
        
        let operation = EditOperation::DeleteText {
            position: start_pos,
            text: selected_text,
        };
        
        if start_pos.1 == end_pos.1 {
            // Single line deletion
            if start_pos.1 < current_tab.content.len() {
                let line = &mut current_tab.content[start_pos.1];
                let start_x = start_pos.0.min(line.len());
                let end_x = end_pos.0.min(line.len());
                line.drain(start_x..end_x);
                current_tab.cursor_x = start_x;
                current_tab.cursor_y = start_pos.1;
            }
        } else {
            // Multi-line deletion
            if start_pos.1 < current_tab.content.len() && end_pos.1 < current_tab.content.len() {
                // Get the remaining parts of first and last lines
                let first_line_start = current_tab.content[start_pos.1][..start_pos.0.min(current_tab.content[start_pos.1].len())].to_string();
                let last_line_end = if end_pos.0 <= current_tab.content[end_pos.1].len() {
                    current_tab.content[end_pos.1][end_pos.0..].to_string()
                } else {
                    String::new()
                };
                
                // Remove all lines in between (and including the end line)
                for _ in start_pos.1 + 1..=end_pos.1 {
                    if start_pos.1 + 1 < current_tab.content.len() {
                        current_tab.content.remove(start_pos.1 + 1);
                    }
                }
                
                // Merge first line start with last line end
                current_tab.content[start_pos.1] = first_line_start + &last_line_end;
                current_tab.cursor_x = start_pos.0;
                current_tab.cursor_y = start_pos.1;
            }
        }
        
        // Clear selection directly on current_tab to avoid double mutable borrow
        current_tab.selection_start = None;
        current_tab.selection_end = None;
        current_tab.selection_mode = false;
        current_tab.modified = true;
        current_tab.record_operation(operation);
    }
    
    fn select_all(&mut self) {
        let current_tab = self.get_current_tab_mut();
        if current_tab.content.is_empty() {
            return;
        }
        
        current_tab.selection_start = Some((0, 0));
        let last_line_idx = current_tab.content.len() - 1;
        let last_line_len = current_tab.content[last_line_idx].len();
        current_tab.selection_end = Some((last_line_len, last_line_idx));
        current_tab.selection_mode = true;
    }
    
    fn copy_selection(&self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_text = self.get_selected_text();
        if !selected_text.is_empty() {
            use arboard::Clipboard;
            let mut clipboard = Clipboard::new()?;
            clipboard.set_text(selected_text)?;
        }
        Ok(())
    }
    
    fn cut_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.has_selection() {
            self.copy_selection()?;
            self.delete_selected_text();
        }
        Ok(())
    }
    
    fn paste_from_clipboard(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new()?;
        let text = clipboard.get_text()?;
        
        self.paste_text(&text);
        
        Ok(())
    }
    
    fn paste_text(&mut self, text: &str) {
        // If there's a selection, delete it first
        let current_tab = self.get_current_tab_mut();
        if current_tab.has_selection() {
            current_tab.delete_selected_text();
        }
        
        if !text.is_empty() {
            let operation = EditOperation::InsertText {
                position: (current_tab.cursor_x, current_tab.cursor_y),
                text: text.to_string(),
            };
            
            // Insert the text character by character
            for c in text.chars() {
                if c == '\n' {
                    let remaining = current_tab.content[current_tab.cursor_y].split_off(current_tab.cursor_x);
                    current_tab.cursor_y += 1;
                    current_tab.content.insert(current_tab.cursor_y, remaining);
                    current_tab.cursor_x = 0;
                } else if c == '\t' {
                    // Insert 4 spaces for tab
                    for _ in 0..4 {
                        let line = &mut current_tab.content[current_tab.cursor_y];
                        let line_char_count = line.chars().count();
                        if current_tab.cursor_x > line_char_count {
                            line.push_str(&" ".repeat(current_tab.cursor_x - line_char_count));
                        }
                        let byte_pos = Self::char_to_byte_index(line, current_tab.cursor_x);
                        line.insert(byte_pos, ' ');
                        current_tab.cursor_x += 1;
                    }
                } else if !c.is_control() {
                    let line = &mut current_tab.content[current_tab.cursor_y];
                    let line_char_count = line.chars().count();
                    if current_tab.cursor_x > line_char_count {
                        line.push_str(&" ".repeat(current_tab.cursor_x - line_char_count));
                    }
                    let byte_pos = Self::char_to_byte_index(line, current_tab.cursor_x);
                line.insert(byte_pos, c);
                    current_tab.cursor_x += 1;
                }
            }
            
            current_tab.modified = true;
            current_tab.record_operation(operation);
        }
    }
    
    fn extract_symbols_from_ast(&self, ast: &parser::ASTNode) -> Vec<SymbolInfo> {
        let mut symbols = Vec::new();
        
        match ast {
            parser::ASTNode::Program { statements, .. } => {
                for statement in statements {
                    symbols.extend(self.extract_symbols_from_ast(statement));
                }
            }
            parser::ASTNode::StructDeclaration { name, fields, .. } => {
                let struct_fields: Vec<(String, String)> = fields.iter()
                    .filter_map(|field| {
                        if let parser::ASTNode::StructDeclarationField { name: field_name, data_type, .. } = field {
                            Some((field_name.clone(), format_type(data_type)))
                        } else {
                            None
                        }
                    })
                    .collect();
                
                symbols.push(SymbolInfo {
                    name: name.clone(),
                    symbol_type: SymbolType::Struct { fields: struct_fields },
                    data_type: Some(format!("struct {}", name)),
                });
            }
            parser::ASTNode::EnumDeclaration { name, variants, .. } => {
                let enum_variants: Vec<String> = variants.iter()
                    .filter_map(|variant| {
                        if let parser::ASTNode::EnumVariant { variant: variant_name, .. } = variant {
                            Some(variant_name.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                
                symbols.push(SymbolInfo {
                    name: name.clone(),
                    symbol_type: SymbolType::Enum { variants: enum_variants },
                    data_type: Some(format!("enum {}", name)),
                });
            }
            parser::ASTNode::ConstDeclaration { name, data_type, .. } => {
                symbols.push(SymbolInfo {
                    name: name.clone(),
                    symbol_type: SymbolType::Variable,
                    data_type: Some(format_type(data_type)),
                });
            }
            // Recursively process nested nodes
            parser::ASTNode::FunctionDeclaration { body, .. } => {
                symbols.extend(self.extract_symbols_from_ast(body));
            }
            parser::ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                for (condition, body) in condition_branches {
                    symbols.extend(self.extract_symbols_from_ast(condition));
                    symbols.extend(self.extract_symbols_from_ast(body));
                }
                if let Some(else_body) = else_branch {
                    symbols.extend(self.extract_symbols_from_ast(else_body));
                }
            }
            parser::ASTNode::ForLoop { body, .. } => {
                symbols.extend(self.extract_symbols_from_ast(body));
            }
            parser::ASTNode::WhileLoop { body, .. } => {
                symbols.extend(self.extract_symbols_from_ast(body));
            }
            // Add more node types as needed
            _ => {
                // For other node types, we don't extract symbols but could add more cases
            }
        }
        
        symbols
    }
}

fn format_type(data_type: &lexer::NailDataTypeDescriptor) -> String {
    match data_type {
        lexer::NailDataTypeDescriptor::Int => "i".to_string(),
        lexer::NailDataTypeDescriptor::Float => "f".to_string(),
        lexer::NailDataTypeDescriptor::String => "s".to_string(),
        lexer::NailDataTypeDescriptor::Boolean => "b".to_string(),
        lexer::NailDataTypeDescriptor::Void => "void".to_string(),
        lexer::NailDataTypeDescriptor::Array(inner) => format!("[{}]", format_type(inner)),
        lexer::NailDataTypeDescriptor::HashMap(key, value) => format!("h<{},{}>", format_type(key), format_type(value)),
        lexer::NailDataTypeDescriptor::Result(result_type) => format_type(result_type),
        lexer::NailDataTypeDescriptor::Any => "any".to_string(),
        lexer::NailDataTypeDescriptor::Struct(name) => name.clone(),
        lexer::NailDataTypeDescriptor::Enum(name) => name.clone(),
        _ => "?".to_string(),
    }
}

