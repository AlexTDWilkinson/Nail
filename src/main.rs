mod checker;
mod colorizer;
mod common;
mod embedded;
mod formatter;
mod lexer;
mod parser;
mod version_line;
mod statics_for_tests;
mod stdlib_registry;
// mod stdlib_types; // Merged into stdlib_registry
mod transpiler;
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

use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

use crate::utils::lock;
use crate::utils::profile_watcher_thread_logic;
use crate::utils::BuildStatus;
use crate::utils::ProfileData;

use crate::common::CodeSpan;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
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
    // The first column on screen. Lines wider than the window used to run off
    // the right edge and take the cursor with them, so the view now slides
    // sideways to keep the cursor in sight.
    h_scroll: u16,
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
            h_scroll: 0,
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

    /// Slides the view until the cursor is inside it, both down the file and
    /// across it. Only the drawing knows how big the window is, so this is
    /// called from there, and only when the cursor has actually moved: scroll
    /// keys and the wheel are allowed to leave the cursor behind, and snapping
    /// back every frame would make them do nothing at all.
    fn follow_cursor(&mut self, width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        let top = self.scroll_position as usize;
        if self.cursor_y < top {
            self.scroll_position = self.cursor_y as u16;
        } else if self.cursor_y >= top + height {
            self.scroll_position = (self.cursor_y - height + 1) as u16;
        }

        let left = self.h_scroll as usize;
        if self.cursor_x < left {
            self.h_scroll = self.cursor_x as u16;
        } else if self.cursor_x >= left + width {
            self.h_scroll = (self.cursor_x - width + 1) as u16;
        }
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
    keymap: nail::keymap::Keymap,
    // The half-typed chord an emacs user is in the middle of, which decides
    // what the next key means
    pending_prefix: Option<nail::keymap::Prefix>,
    mark_active: bool,
    settings_row: usize,
    // Tab system
    tabs: Vec<Tab>,
    tab_index: usize,
    // Global state
    build_status: BuildStatus,
    code_errors: Vec<CodeError>,
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
    // Command palette: which commands survive the filter, and which of those
    // is picked. The commands themselves live in the keymap, beside the keys
    // that also reach them.
    palette_filter: String,
    palette_matches: Vec<usize>,
    palette_index: usize,
    // Go to symbol, built from the parsed file when it parses and read off
    // the text when it does not
    symbol_filter: String,
    symbol_source: SymbolSource,
    symbol_entries: Vec<FileSymbol>,
    symbol_matches: Vec<usize>,
    symbol_index: usize,
    // Find/Replace system (shared across tabs)
    search_query: String,
    replace_text: String,
    search_results: Vec<(usize, usize, usize)>, // (line, start, end)
    current_match_index: usize,
    case_sensitive: bool,
    // A pattern can be asked to match whole words only, or to be read as a
    // regular expression. A regular expression that does not compile is not
    // an error the user has to dismiss, it is simply a search with no matches
    // yet, so the message goes in the dialog and typing continues.
    whole_word: bool,
    use_regex: bool,
    search_error: Option<String>,
    search_direction_forward: bool, // For F3/Shift+F3 navigation
    replace_field_active: bool, // true if replace field is active, false if find field is active
    // The file finder. `file_entries` is what it is showing, which is the
    // ranked answer to whatever has been typed, and `file_index` is every
    // file in the project it ranks them out of.
    file_entries: Vec<FileEntry>,
    file_index: Vec<String>,
    file_dialog_index: usize,
    file_dialog_input: String,
    // The directory the IDE was started in, which is also where its `.nail`
    // file lives. One rule for both means the project the finder searches and
    // the project whose settings are remembered are never two different
    // things.
    project_root: String,
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
    // Where the draw thread last put things. A mouse reports a row and a
    // column of the terminal, and only the drawing knows which line of which
    // file that lands on, so it leaves the answer here for the key thread.
    view: ViewLayout,
    // Whether the terminal is reporting mouse events to us at all. Capture is
    // worth turning off, because while we hold it the terminal's own click to
    // select and copy stops working, and that is sometimes the thing the user
    // actually wants.
    mouse_enabled: bool,
    // Set while a drag is in progress, so a release outside the text area
    // still ends the selection it started.
    mouse_dragging: bool,
    // Selections that expanding grew out of, newest last, so shrinking can
    // walk back exactly the way it came.
    expand_stack: Vec<((usize, usize), (usize, usize))>,
    // Latest per-function timings from a running instrumented program,
    // updated by the profile watcher thread and read by the draw thread
    profile_data: Option<ProfileData>,
    // Every dump seen this session, keyed by the source fingerprint it
    // carries. Two Nail programs sharing a working directory rewrite the
    // same dump file in turns, and this keeps the one that matches the
    // open buffer available whichever program wrote last.
    profile_dumps: std::collections::HashMap<String, ProfileData>,
    // When the cargo build now running started, and how long the last build
    // of the same kind took. Together they turn the one slow build step into
    // a percentage in the status line.
    compile_started: Option<std::time::Instant>,
    compile_estimate: Option<std::time::Duration>,
}

/// The parts of the screen a mouse click can land in, as the draw thread last
/// laid them out. `text` is the area inside the editor's border, so a click at
/// its top left is the first visible character rather than the frame around it.
#[derive(Clone, Copy, Debug, Default)]
struct ViewLayout {
    tabs: ratatui::layout::Rect,
    text: ratatui::layout::Rect,
}

/// A named thing and the line it is declared on. `file` is the project file
/// it lives in, relative to the project root, and is absent when the symbol
/// came from the open buffer: an unsaved file has no path to name, and the
/// picker over one file has no need of one.
#[derive(Clone, Debug)]
struct FileSymbol {
    label: String,
    line: usize,
    file: Option<String>,
}

/// Where the list in the picker came from, which is what typing into it does
/// next. Two of these narrow a list already in hand. The third asks the
/// project again on every keystroke, because no list of every line in every
/// file was ever built to narrow.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SymbolSource {
    OpenFile,
    Project,
    ProjectText,
}

/// What one of the fuzzy pickers did with a key.
enum PickerKey {
    Handled,
    Run(nail::keymap::Action),
    Ignored,
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
    Settings,
    ConfirmQuit,
    CommandPalette,
    SymbolPicker,
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
            theme: stored_theme(),
            keymap: stored_keymap().unwrap_or_else(nail::keymap::detect),
            pending_prefix: None,
            mark_active: false,
            settings_row: 0,
            tabs: vec![welcome_tab],
            tab_index: 0,
            build_status: BuildStatus::Idle,
            code_errors: Vec::new(),
            scroll_state: ScrollbarState::default(),
            max_undo_history: 1000,
            completions: Vec::new(),
            completion_index: 0,
            show_completions: false,
            show_detail_view: false,
            completion_prefix: String::new(),
            dialog_mode: DialogMode::None,
            goto_line_input: String::new(),
            palette_filter: String::new(),
            palette_matches: Vec::new(),
            palette_index: 0,
            symbol_filter: String::new(),
            symbol_source: SymbolSource::OpenFile,
            symbol_entries: Vec::new(),
            symbol_matches: Vec::new(),
            symbol_index: 0,
            search_query: String::new(),
            replace_text: String::new(),
            search_results: Vec::new(),
            current_match_index: 0,
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            search_error: None,
            search_direction_forward: true,
            replace_field_active: false,
            file_entries: Vec::new(),
            file_index: Vec::new(),
            file_dialog_index: 0,
            file_dialog_input: String::new(),
            project_root: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
            recent_files: Vec::new(),
            stdlib_functions: Vec::new(),
            stdlib_filter: String::new(),
            stdlib_index: 0,
            stdlib_category_filter: None,
            // Visual enhancement settings, as the settings screen last left
            // them, or the defaults for anyone who has never opened it
            show_line_numbers: stored_flag("line_numbers", true),
            highlight_current_line: stored_flag("current_line", true),
            highlight_matching_brackets: stored_flag("brackets", true),
            show_whitespace: stored_flag("whitespace", false),
            show_indentation_guides: stored_flag("indent_guides", false),
            show_minimap: stored_flag("minimap", false), // Disabled by default as it takes screen space
            // Bracket matching state
            matching_bracket_pos: None,
            view: ViewLayout::default(),
            // On, because a click landing where the user pointed is what
            // everyone expects, and F4 is there for the times it is not.
            mouse_enabled: true,
            mouse_dragging: false,
            expand_stack: Vec::new(),
            profile_data: None,
            profile_dumps: std::collections::HashMap::new(),
            compile_started: None,
            compile_estimate: None,
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
        // Is it already open? The same file reaches this by more than one
        // name: the launcher opens what was typed on the command line, the
        // finder opens a full path, and an import opens a path built from the
        // importing file's own. Comparing the names would open a second tab
        // on the same file and let the two copies drift apart, so what is
        // compared is where the names lead.
        let wanted = std::fs::canonicalize(&filename).unwrap_or_else(|_| std::path::PathBuf::from(&filename));
        for (i, tab) in self.tabs.iter().enumerate() {
            if let Some(tab_filename) = &tab.filename {
                let same = std::fs::canonicalize(tab_filename).map(|open| open == wanted).unwrap_or_else(|_| tab_filename == &filename);
                if same {
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
        self.save_session();
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
        self.save_session();
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
    
    /// Reading the tree costs a few milliseconds on a project and is done on
    /// every open rather than cached, so a file created a second ago by
    /// something else is already there to be found. The alternative is a
    /// cache and a way to tell it it is stale, which is a great deal of
    /// machinery to save a walk nobody can feel.
    fn open_file_dialog(&mut self) {
        self.dialog_mode = DialogMode::OpenFile;
        self.file_index = crate::utils::scan_project_files(std::path::Path::new(&self.project_root));
        self.file_dialog_input.clear();
        self.filter_file_entries();
    }

    /// Whether what has been typed is a path being spelled out rather than a
    /// name being searched for. Everything the finder knows about lives under
    /// the project, so a leading slash or tilde is the one unambiguous way to
    /// say the file wanted is somewhere else entirely.
    fn browsing_by_path(&self) -> bool {
        return self.file_dialog_input.starts_with('/') || self.file_dialog_input.starts_with('~');
    }

    /// How many rows the finder will build. Nothing below this is reachable
    /// by scrolling before the next keystroke changes the order anyway, and
    /// building twenty thousand of them per keypress is work thrown away.
    const FILE_MATCH_LIMIT: usize = 200;

    fn filter_file_entries(&mut self) {
        self.file_entries = if self.browsing_by_path() { self.entries_by_path() } else { self.entries_by_score() };
        self.file_dialog_index = 0;
    }

    /// The project's files, best answer to the query first. With nothing
    /// typed the order is the recently opened ones and then the rest by path,
    /// which makes the finder a recent-files list until it is asked to be
    /// something else.
    fn entries_by_score(&self) -> Vec<FileEntry> {
        let needle = self.file_dialog_input.to_lowercase();
        let open_paths: Vec<&str> = self.tabs.iter().filter_map(|tab| tab.filename.as_deref()).collect();
        let showing = self.get_current_tab().filename.clone();
        let mut scored: Vec<(i32, FileEntry)> = Vec::new();
        for relative in &self.file_index {
            let mut score = match crate::utils::path_score(&relative.to_lowercase(), &needle) {
                Some(score) => score,
                None => continue,
            };
            let full = std::path::Path::new(&self.project_root).join(relative).to_string_lossy().to_string();
            // A file opened recently is far more likely to be wanted than one
            // never opened at all, and the more recently the more so.
            let recent = self.recent_files.iter().position(|path| path == &full);
            if let Some(position) = recent {
                score += 60 - 5 * position.min(10) as i32;
            }
            if open_paths.contains(&full.as_str()) {
                score += 10;
            }
            // With nothing typed the finder is a list of somewhere else to be,
            // and the file already on screen is not somewhere else. Opening it
            // and pressing return therefore lands on the file before this one,
            // which is the move people make most. Once something is typed the
            // penalty lifts, because then the user is naming a file rather
            // than picking one, and they may well be naming this one.
            if needle.is_empty() && showing.as_deref() == Some(full.as_str()) {
                score -= 100;
            }
            scored.push((score, FileEntry { name: relative.clone(), path: full, is_directory: false, is_recent: recent.is_some() }));
        }
        // Stable, so that files scoring the same stay in the alphabetical
        // order the walk left them in rather than shuffling as the user types.
        scored.sort_by(|left, right| right.0.cmp(&left.0));
        scored.truncate(Self::FILE_MATCH_LIMIT);
        return scored.into_iter().map(|(_, entry)| entry).collect();
    }

    /// Directory listing for a path typed out by hand, so that a file outside
    /// the project is still reachable. The last piece of what has been typed
    /// filters the directory above it, which is what tab completion would do
    /// if this had tab completion.
    fn entries_by_path(&self) -> Vec<FileEntry> {
        let typed = match self.file_dialog_input.strip_prefix('~') {
            Some(rest) => format!("{}{}", std::env::var("HOME").unwrap_or_else(|_| "~".to_string()), rest),
            None => self.file_dialog_input.clone(),
        };
        let (directory, prefix) = match typed.rsplit_once('/') {
            Some((directory, prefix)) => (if directory.is_empty() { "/" } else { directory }, prefix.to_lowercase()),
            None => ("/", String::new()),
        };
        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };
        let mut found: Vec<FileEntry> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().starts_with(&prefix) {
                continue;
            }
            let is_directory = entry.path().is_dir();
            found.push(FileEntry {
                name: if is_directory { format!("{}/", name) } else { name },
                path: entry.path().to_string_lossy().to_string(),
                is_directory,
                is_recent: false,
            });
        }
        found.sort_by(|left, right| right.is_directory.cmp(&left.is_directory).then(left.name.cmp(&right.name)));
        found.truncate(Self::FILE_MATCH_LIMIT);
        return found;
    }

    fn handle_file_dialog_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.dialog_mode = DialogMode::None;
                return true;
            }
            KeyCode::Enter => {
                self.handle_file_dialog_enter();
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
                self.handle_file_dialog_input(c);
                return true;
            }
            KeyCode::Backspace => {
                self.handle_file_dialog_backspace();
                return true;
            }
            _ => return false,
        }
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
            let category = func.module.display_name();

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
    
    /// A directory is not something to open, so choosing one spells it into
    /// the query and lists what is inside. The typed text stays the one place
    /// the finder keeps where it is, which is why backspace walks back out
    /// again without needing to know it is doing so.
    fn handle_file_dialog_enter(&mut self) {
        let entry = match self.file_entries.get(self.file_dialog_index) {
            Some(entry) => entry.clone(),
            None => return,
        };
        if entry.is_directory {
            self.file_dialog_input = format!("{}/", entry.path);
            self.filter_file_entries();
            return;
        }
        let _ = self.open_file_in_tab(entry.path);
        self.dialog_mode = DialogMode::None;
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
    
    /// Removes as many characters as the given text has, starting at the
    /// cursor and working forward. Undo and redo both place the cursor where
    /// an edit began and then take back what it put there, so this deletes
    /// ahead of the cursor: deleting behind it would eat the text before the
    /// edit instead of the edit itself.
    fn delete_text_at_position(&mut self, text: &str) {
        let current_tab = self.get_current_tab_mut();
        let char_count = text.chars().count();
        for _ in 0..char_count {
            if current_tab.cursor_y >= current_tab.content.len() {
                return;
            }
            let line_width = current_tab.content[current_tab.cursor_y].chars().count();
            if current_tab.cursor_x < line_width {
                let byte_pos = Self::char_to_byte_index(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x);
                current_tab.content[current_tab.cursor_y].remove(byte_pos);
            } else if current_tab.cursor_y + 1 < current_tab.content.len() {
                let next_line = current_tab.content.remove(current_tab.cursor_y + 1);
                current_tab.content[current_tab.cursor_y].push_str(&next_line);
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
        
        // A bracket or a quote typed over a selection wraps it, because
        // replacing a selection with a single bracket is never what the person
        // holding the keyboard meant.
        if let Some(closing) = Self::surround_pair(c) {
            if self.has_selection() {
                self.surround_selection(c, closing);
                return;
            }
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        
        if extend_selection {
            self.anchor_selection();
        } else {
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
        self.search_error = None;

        if self.search_query.is_empty() {
            return;
        }

        let pattern = match self.search_pattern() {
            Ok(pattern) => pattern,
            Err(message) => {
                self.search_error = Some(message);
                return;
            }
        };

        let content = {
            let current_tab = self.get_current_tab();
            current_tab.content.clone()
        };

        for (line_idx, line) in content.iter().enumerate() {
            for found in pattern.find_iter(line) {
                // A pattern can match nothing at all, and highlighting a match
                // of no characters would be highlighting nothing at all.
                if found.start() == found.end() {
                    continue;
                }
                self.search_results.push((line_idx, found.start(), found.end()));
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
        // Merged into the project's .nail file rather than written over it,
        // because build timings live there too
        let theme = match self.theme {
            x if x == &*LIGHT_THEME => "light",
            _ => "dark",
        };
        crate::utils::write_config_values(&[("theme", theme.to_string())]);
        Ok(())
    }

    /// True when the user has never chosen, which is what makes the settings
    /// screen open by itself the first time.
    pub fn has_never_chosen_a_keymap(&self) -> bool {
        return stored_keymap().is_none();
    }

    /// Escape is one key away from every cancel a user makes all day, so it
    /// asks before it throws the session away.
    pub fn ask_before_quitting(&mut self) {
        self.dialog_mode = DialogMode::ConfirmQuit;
    }

    /// True when at least one tab has edits nobody has written to disk, which
    /// is what the confirmation is really warning about.
    pub fn has_unsaved_work(&self) -> bool {
        return self.tabs.iter().any(|tab| tab.modified);
    }

    pub fn open_settings(&mut self) {
        self.dialog_mode = DialogMode::Settings;
        self.settings_row = 0;
    }

    /// Leaving the screen is what commits it, so there is no separate save
    /// key to hunt for or forget.
    pub fn close_settings(&mut self) {
        self.dialog_mode = DialogMode::None;
        self.save_settings();
    }

    pub fn settings_row_count(&self) -> usize {
        return 8;
    }

    pub fn settings_next_row(&mut self) {
        self.settings_row = (self.settings_row + 1) % self.settings_row_count();
    }

    pub fn settings_previous_row(&mut self) {
        let count = self.settings_row_count();
        self.settings_row = (self.settings_row + count - 1) % count;
    }

    pub fn settings_cycle_value(&mut self, forward: bool) {
        match self.settings_row {
            // Vim is somewhere detection can leave the user, never somewhere
            // cycling can put them, because there is no vim key table yet.
            0 => {
                self.keymap = match (self.keymap, forward) {
                    (nail::keymap::Keymap::Cua, _) => nail::keymap::Keymap::Emacs,
                    (nail::keymap::Keymap::Emacs, _) => nail::keymap::Keymap::Cua,
                    (nail::keymap::Keymap::Vim, true) => nail::keymap::Keymap::Cua,
                    (nail::keymap::Keymap::Vim, false) => nail::keymap::Keymap::Emacs,
                };
            }
            1 => self.toggle_theme(),
            _ => {
                let value = match self.settings_row {
                    2 => &mut self.show_line_numbers,
                    3 => &mut self.highlight_current_line,
                    4 => &mut self.highlight_matching_brackets,
                    5 => &mut self.show_whitespace,
                    6 => &mut self.show_indentation_guides,
                    7 => &mut self.show_minimap,
                    _ => return,
                };
                *value = !*value;
                // A highlight that is off must not leave its last match lit
                if !self.highlight_matching_brackets {
                    self.matching_bracket_pos = None;
                }
            }
        }
    }

    pub fn settings_rows(&self) -> Vec<(String, String)> {
        let keys = match self.keymap {
            // Detection can land here, so the row says which bindings are
            // actually answering keys rather than only which one was detected.
            nail::keymap::Keymap::Vim => "vim (falls back to cua)".to_string(),
            other => keymap_name(other).to_string(),
        };
        return vec![
            ("Keys".to_string(), keys),
            ("Theme".to_string(), self.theme_name().to_string()),
            ("Line numbers".to_string(), flag_name(self.show_line_numbers)),
            ("Current line".to_string(), flag_name(self.highlight_current_line)),
            ("Brackets".to_string(), flag_name(self.highlight_matching_brackets)),
            ("Whitespace".to_string(), flag_name(self.show_whitespace)),
            ("Indent guides".to_string(), flag_name(self.show_indentation_guides)),
            ("Minimap".to_string(), flag_name(self.show_minimap)),
        ];
    }

    fn theme_name(&self) -> &'static str {
        return if *self.theme == *LIGHT_THEME { "light" } else { "dark" };
    }

    /// One call rather than eight, because each one rewrites the whole config
    /// file and eight of those would be eight chances to be interrupted
    /// halfway through a single screen's worth of changes.
    fn save_settings(&self) {
        crate::utils::write_config_values(&[
            ("keymap", keymap_name(self.keymap).to_string()),
            ("theme", self.theme_name().to_string()),
            ("line_numbers", flag_name(self.show_line_numbers)),
            ("current_line", flag_name(self.highlight_current_line)),
            ("brackets", flag_name(self.highlight_matching_brackets)),
            ("whitespace", flag_name(self.show_whitespace)),
            ("indent_guides", flag_name(self.show_indentation_guides)),
            ("minimap", flag_name(self.show_minimap)),
        ]);
    }

    /// Emacs sets a mark, moves, and calls everything in between the region.
    /// That is this editor's selection with the anchor dropped first, so the
    /// mark rides on the selection rather than beside it.
    pub fn set_mark(&mut self) {
        self.mark_active = true;
        self.start_selection();
    }

    pub fn clear_mark(&mut self) {
        self.mark_active = false;
        self.clear_selection();
    }

    /// From the middle of a line this takes the rest of it. From the end it
    /// takes the newline instead, which is how emacs empties a line and then
    /// closes the gap in two presses of the same key.
    pub fn kill_to_line_end(&mut self) {
        self.start_selection();
        self.move_to_line_end_with_selection(true);
        if self.has_selection() {
            self.delete_selected_text();
            return;
        }
        self.clear_selection();
        self.delete_forward();
    }

    fn save_file(&mut self) -> Result<(), String> {
        let current_tab = self.get_current_tab_mut();
        if current_tab.filename.is_none() {
            return Err("No filename set for current tab".to_string());
        }

        // Formatting is enforced on save: reformat the buffer, then keep the
        // cursor, selection, and scroll position sane against the new content.
        let mut formatted = formatter::format_nail_code(&current_tab.content);
        if formatted.is_empty() {
            formatted.push(String::new());
        }
        if formatted != current_tab.content {
            current_tab.content = formatted;
            if current_tab.cursor_y >= current_tab.content.len() {
                current_tab.cursor_y = current_tab.content.len() - 1;
            }
            let line_len = current_tab.content[current_tab.cursor_y].chars().count();
            if current_tab.cursor_x > line_len {
                current_tab.cursor_x = line_len;
            }
            // Selections refer to pre-format coordinates; drop them
            current_tab.selection_start = None;
            current_tab.selection_end = None;
            current_tab.selection_mode = false;
            let max_scroll = current_tab.content.len().saturating_sub(1) as u16;
            if current_tab.scroll_position > max_scroll {
                current_tab.scroll_position = max_scroll;
            }
        }

        // The IDE maintains the file's version line, so the user never types
        // it. A file that already has one is left exactly as it is, including
        // `nail latest`: re-stamping would silently migrate code the author
        // pinned on purpose. Only a file with no line gets one, and since the
        // compiler now requires the line, this is what keeps that requirement
        // from ever being something a person has to think about.
        //
        // A released IDE stamps its own version, which is a real release
        // anyone else can fetch. A development checkout stamps `latest`
        // instead, because its version was never published and pinning to it
        // would produce a file nobody else could open.
        let source = current_tab.content.join("\n");
        if crate::version_line::scan_header(source.as_bytes()).pin.is_none() {
            let pin = match nail::toolchain::BundledToolchain::detect() {
                Some(_) => env!("CARGO_PKG_VERSION").parse::<crate::version_line::Version>().map(crate::version_line::Pin::Exact).unwrap_or(crate::version_line::Pin::Latest),
                None => crate::version_line::Pin::Latest,
            };
            let stamped = crate::version_line::stamp(&source, &pin);
            current_tab.content = stamped.split('\n').map(String::from).collect();
            current_tab.cursor_y += 1;
        }

        let filename = current_tab.filename.clone().expect("filename checked above");
        let content = current_tab.content.join("\n");
        std::fs::write(&filename, content)
            .map_err(|e| format!("Failed to save file: {}", e))?;
        current_tab.modified = false;
        self.save_session();
        Ok(())
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
    // The sender stays bound so the watcher's channel lives until main exits
    let (_tx_profile, rx_profile) = channel::<EditorMessage>();

    // Set up terminal. Mouse reporting is asked for here and can be handed
    // back at any time with F4, because while we hold it the terminal's own
    // click to select stops working.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    
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
    match file_to_open {
        Some(filename) => {
            if let Err(e) = editor.open_file_path(&filename) {
                log::error!("Failed to open file '{}': {}", filename, e);
            } else {
                log::info!("Opened file: {}", filename);
            }
        }
        // Nobody named a file, so put back the ones that were open here last
        // time, each at the line its cursor was on.
        None => editor.restore_session(),
    }
    
    // Nobody has ever been asked which keys they want, so ask now rather than
    // guessing quietly. Detection has already filled in the answer it thinks
    // is right, and closing the screen accepts it.
    if editor.has_never_chosen_a_keymap() {
        editor.open_settings();
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

    // Launch the profile watcher thread, which keeps timing annotations
    // fresh while an instrumented program is running
    let editor_for_profile = Arc::clone(&shared_editor);
    thread::spawn(move || {
        profile_watcher_thread_logic(editor_for_profile, rx_profile);
    });

    // Main draw thread (this runs on the main thread)
    draw_thread_logic(terminal.clone(), editor_for_draw, rx_draw);
    
    // Clean up terminal on exit
    disable_raw_mode()?;
    execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen)?;

    Ok(())
}

#[derive(Debug, Clone)]
enum CompletionContext {
    None,
    Identifier(String),        // Typing an identifier, show matching functions/variables
    FunctionCall(String),       // Inside function call, show parameter hints
}

// Anything unreadable or unrecognized falls back to dark, so a hand-edited
// config can never leave the editor without colors
fn stored_theme() -> &'static ColorScheme {
    return match crate::utils::read_config_value("theme").as_deref() {
        Some("light") => &LIGHT_THEME,
        _ => &DARK_THEME,
    };
}

/// The keymap the user picked, or None if they have never been asked. A value
/// nobody recognizes counts as never asked, so a hand-edited config puts the
/// question back rather than quietly picking an answer.
fn stored_keymap() -> Option<nail::keymap::Keymap> {
    return match crate::utils::read_config_value("keymap").as_deref() {
        Some("cua") => Some(nail::keymap::Keymap::Cua),
        Some("vim") => Some(nail::keymap::Keymap::Vim),
        Some("emacs") => Some(nail::keymap::Keymap::Emacs),
        _ => None,
    };
}

/// How a keymap is spelled in the config file.
fn keymap_name(keymap: nail::keymap::Keymap) -> &'static str {
    return match keymap {
        nail::keymap::Keymap::Cua => "cua",
        nail::keymap::Keymap::Vim => "vim",
        nail::keymap::Keymap::Emacs => "emacs",
    };
}

/// How a yes or no setting is spelled, both in the config file and on the
/// settings screen, so the two can never disagree.
fn flag_name(on: bool) -> String {
    return if on { "on".to_string() } else { "off".to_string() };
}

/// Reads back what `flag_name` wrote. Anything else is treated as absent,
/// which lands on the default rather than on off.
fn stored_flag(key: &str, default: bool) -> bool {
    return match crate::utils::read_config_value(key).as_deref() {
        Some("on") => true,
        Some("off") => false,
        _ => default,
    };
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
    
    /// Drops the anchor where the cursor is now, if a selection is not already
    /// under way. Every shift+motion calls this before it moves, because the
    /// anchor belongs where the selection started rather than where the first
    /// press of the key landed: setting it afterwards is what used to make
    /// shift+down select one line short.
    fn anchor_selection(&mut self) {
        let current_tab = self.get_current_tab_mut();
        if current_tab.selection_start.is_none() {
            current_tab.selection_start = Some((current_tab.cursor_x, current_tab.cursor_y));
            current_tab.selection_end = Some((current_tab.cursor_x, current_tab.cursor_y));
            current_tab.selection_mode = true;
        }
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
        // Losing the selection loses the trail of expansions that built it,
        // because shrinking back to a selection the user has since moved away
        // from would be a jump to somewhere they have not been.
        self.expand_stack.clear();
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

/// The commands that were missing next to the ones this editor already had:
/// the mouse, a sideways view, the two fuzzy pickers, the word sized edits,
/// and remembering which files were open. They share a block because they
/// share a subject, which is the editor as a whole rather than the letters in
/// one buffer.
impl Editor {
    /// A position in the file as a single offset into its text, and back
    /// again. Scanning outward for a bracket or a word is a one dimensional
    /// problem, and solving it in one dimension is what keeps it short.
    fn offset_of(&self, position: (usize, usize)) -> usize {
        let content = &self.get_current_tab().content;
        let mut offset = 0;
        for line in content.iter().take(position.1) {
            offset += line.chars().count() + 1;
        }
        let width = content.get(position.1).map_or(0, |line| line.chars().count());
        return offset + position.0.min(width);
    }

    fn position_of(&self, offset: usize) -> (usize, usize) {
        let content = &self.get_current_tab().content;
        let mut remaining = offset;
        for (index, line) in content.iter().enumerate() {
            let width = line.chars().count();
            if remaining <= width {
                return (remaining, index);
            }
            remaining -= width + 1;
        }
        let last = content.len().saturating_sub(1);
        return (content.get(last).map_or(0, |line| line.chars().count()), last);
    }

    fn flat_text(&self) -> Vec<char> {
        return self.get_current_tab().content.join("\n").chars().collect();
    }

    /// The text between two positions, without disturbing what is selected.
    fn text_in_span(&mut self, start: (usize, usize), end: (usize, usize)) -> String {
        let previous = {
            let tab = self.get_current_tab();
            (tab.selection_start, tab.selection_end, tab.selection_mode)
        };
        {
            let tab = self.get_current_tab_mut();
            tab.selection_start = Some(start);
            tab.selection_end = Some(end);
            tab.selection_mode = true;
        }
        let text = self.get_selected_text();
        let tab = self.get_current_tab_mut();
        tab.selection_start = previous.0;
        tab.selection_end = previous.1;
        tab.selection_mode = previous.2;
        return text;
    }

    /// Swaps one span of the file for some other text as a single edit, so one
    /// undo takes back one command however many lines it touched.
    fn replace_span(&mut self, start: (usize, usize), end: (usize, usize), new_text: &str) {
        let old_text = self.text_in_span(start, end);
        if old_text == new_text {
            return;
        }
        {
            let tab = self.get_current_tab_mut();
            tab.selection_start = Some(start);
            tab.selection_end = Some(end);
            tab.selection_mode = true;
            // The tab's own delete keeps no undo entry, which is what this
            // wants: the one entry recorded below covers the whole swap.
            tab.delete_selected_text();
        }
        self.insert_text_at_cursor(new_text);
        let tab = self.get_current_tab_mut();
        tab.modified = true;
        tab.record_operation(EditOperation::ReplaceText { position: start, old_text, new_text: new_text.to_string() });
    }

    /// Deletes everything between two positions and records it as one edit.
    fn delete_span(&mut self, start: (usize, usize), end: (usize, usize)) {
        if start == end {
            return;
        }
        {
            let tab = self.get_current_tab_mut();
            tab.selection_start = Some(start);
            tab.selection_end = Some(end);
            tab.selection_mode = true;
        }
        self.delete_selected_text();
    }

    fn cursor_position(&self) -> (usize, usize) {
        let tab = self.get_current_tab();
        return (tab.cursor_x, tab.cursor_y);
    }

    pub fn delete_word_left(&mut self) {
        if self.has_selection() {
            self.delete_selected_text();
            return;
        }
        let target = self.find_prev_word_boundary();
        let cursor = self.cursor_position();
        self.delete_span(target, cursor);
    }

    pub fn delete_word_right(&mut self) {
        if self.has_selection() {
            self.delete_selected_text();
            return;
        }
        let cursor = self.cursor_position();
        let target = self.find_next_word_boundary();
        self.delete_span(cursor, target);
    }

    /// Joins the line below into this one, or every line of a selection into
    /// one, the way a paragraph gets un-wrapped.
    pub fn join_lines(&mut self) {
        let (first, last) = self.selected_line_range();
        let last = if last > first { last } else { first + 1 };
        let line_count = self.get_current_tab().content.len();
        if last >= line_count {
            return;
        }
        // Where the seam ends up, measured before the edit: the cursor belongs
        // at the join rather than at the far end of whatever was dragged up.
        let join_column = self.get_current_tab().content[first].trim_end().chars().count();
        let joined = {
            let tab = self.get_current_tab();
            let mut joined = tab.content[first].trim_end().to_string();
            for line in &tab.content[first + 1..=last] {
                let piece = line.trim();
                if piece.is_empty() {
                    continue;
                }
                if !joined.is_empty() {
                    joined.push(' ');
                }
                joined.push_str(piece);
            }
            joined
        };
        let end_column = self.get_current_tab().content[last].chars().count();
        self.replace_span((0, first), (end_column, last), &joined);
        self.clear_selection();
        let tab = self.get_current_tab_mut();
        tab.cursor_y = first;
        tab.cursor_x = join_column;
    }

    /// Sorts the selected lines. Without a selection there is no obvious
    /// answer to "which lines", so nothing happens rather than the whole file
    /// being rearranged by a stray key.
    fn sort_lines(&mut self) {
        if !self.has_selection() {
            return;
        }
        let (first, last) = self.selected_line_range();
        if last <= first {
            return;
        }
        let sorted = {
            let tab = self.get_current_tab();
            let mut lines: Vec<String> = tab.content[first..=last].to_vec();
            lines.sort();
            lines.join("\n")
        };
        let end_column = self.get_current_tab().content[last].chars().count();
        self.replace_span((0, first), (end_column, last), &sorted);
        self.clear_selection();
    }

    /// The lines a command that works on whole lines should work on: the
    /// selected ones, or the cursor's own when nothing is selected.
    fn selected_line_range(&self) -> (usize, usize) {
        let tab = self.get_current_tab();
        if tab.has_selection() {
            let (start, end) = tab.get_normalized_selection();
            // A selection that stops at the very start of a line has not
            // really reached that line, and joining or sorting it would
            // surprise whoever made the selection by dragging.
            let last = if end.0 == 0 && end.1 > start.1 { end.1 - 1 } else { end.1 };
            return (start.1, last);
        }
        return (tab.cursor_y, tab.cursor_y);
    }

    /// Typing a bracket or a quote with something selected wraps the
    /// selection instead of replacing it, which is the only thing anyone
    /// means by it.
    fn surround_pair(c: char) -> Option<char> {
        return match c {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            '`' => Some('`'),
            '\'' => Some('\''),
            '"' => Some('"'),
            _ => None,
        };
    }

    fn surround_selection(&mut self, opening: char, closing: char) {
        let (start, end) = {
            let tab = self.get_current_tab();
            tab.get_normalized_selection()
        };
        let inner = self.text_in_span(start, end);
        let wrapped = format!("{}{}{}", opening, inner, closing);
        self.replace_span(start, end, &wrapped);
        // Keep the selection around what was wrapped, so wrapping twice in a
        // row nests rather than starting over.
        let tab = self.get_current_tab_mut();
        let new_end = if start.1 == end.1 { (end.0 + 2, end.1) } else { (end.0 + 1, end.1) };
        tab.selection_start = Some(start);
        tab.selection_end = Some(new_end);
        tab.selection_mode = true;
        tab.cursor_x = new_end.0;
        tab.cursor_y = new_end.1;
    }

    /// Grows the selection by one step: the word, then what encloses it, then
    /// the line, then the file.
    fn expand_selection(&mut self) {
        let text = self.flat_text();
        let (start_position, end_position) = {
            let tab = self.get_current_tab();
            if tab.has_selection() {
                tab.get_normalized_selection()
            } else {
                ((tab.cursor_x, tab.cursor_y), (tab.cursor_x, tab.cursor_y))
            }
        };
        let start = self.offset_of(start_position);
        let end = self.offset_of(end_position);
        let wider = match wider_span(&text, start, end) {
            Some(wider) => wider,
            None => return,
        };
        self.expand_stack.push((start_position, end_position));
        let new_start = self.position_of(wider.0);
        let new_end = self.position_of(wider.1);
        let tab = self.get_current_tab_mut();
        tab.selection_start = Some(new_start);
        tab.selection_end = Some(new_end);
        tab.selection_mode = true;
        tab.cursor_x = new_end.0;
        tab.cursor_y = new_end.1;
    }

    /// Takes back one expansion. Anything else that changes the selection
    /// leaves the trail behind, so shrinking after that does nothing rather
    /// than jumping somewhere the user has not been.
    fn shrink_selection(&mut self) {
        let previous = match self.expand_stack.pop() {
            Some(previous) => previous,
            None => return,
        };
        let tab = self.get_current_tab_mut();
        tab.cursor_x = previous.1 .0;
        tab.cursor_y = previous.1 .1;
        if previous.0 == previous.1 {
            tab.selection_start = None;
            tab.selection_end = None;
            tab.selection_mode = false;
        } else {
            tab.selection_start = Some(previous.0);
            tab.selection_end = Some(previous.1);
            tab.selection_mode = true;
        }
    }

    /// The lines the checker complained about, in order and without repeats.
    /// A span of line zero is a message with no place in the file, such as
    /// which example was just loaded, and is not somewhere to jump to.
    fn error_lines(&self) -> Vec<usize> {
        let mut lines: Vec<usize> = self.code_errors.iter().map(|error| error.code_span.start_line).filter(|line| *line > 0).collect();
        lines.sort_unstable();
        lines.dedup();
        return lines;
    }

    fn go_to_error(&mut self, forward: bool) {
        let lines = self.error_lines();
        if lines.is_empty() {
            return;
        }
        let current = self.get_current_tab().cursor_y + 1;
        let target = if forward {
            lines.iter().copied().find(|line| *line > current).unwrap_or(lines[0])
        } else {
            lines.iter().rev().copied().find(|line| *line < current).unwrap_or_else(|| *lines.last().expect("checked non-empty above"))
        };
        self.go_to_line(target);
    }

    pub fn go_to_next_error(&mut self) {
        self.go_to_error(true);
    }

    pub fn go_to_previous_error(&mut self) {
        self.go_to_error(false);
    }

    /// Puts the cursor on a line counted from one. The view catches up on the
    /// next frame, which is the only place that knows how tall it is.
    fn go_to_line(&mut self, line_number: usize) {
        self.clear_selection();
        let tab = self.get_current_tab_mut();
        let last = tab.content.len().saturating_sub(1);
        tab.cursor_y = line_number.saturating_sub(1).min(last);
        tab.cursor_x = 0;
    }

    fn scroll_by(&mut self, delta: i32) {
        let tab = self.get_current_tab_mut();
        let furthest = tab.content.len().saturating_sub(1) as u16;
        tab.scroll_position = if delta < 0 { tab.scroll_position.saturating_sub(delta.unsigned_abs() as u16) } else { (tab.scroll_position + delta as u16).min(furthest) };
    }

    /// Slides the view without taking the cursor along, which is the point of
    /// scrolling by key: the cursor is allowed to go off screen and stay where
    /// it was. The drawing code only chases the cursor when it has moved.
    pub fn scroll_line_up(&mut self) {
        self.scroll_by(-1);
    }

    pub fn scroll_line_down(&mut self) {
        self.scroll_by(1);
    }

    /// Letters in order, not letters in a row: typing "dupl" or "dl" both
    /// reach "Duplicate line", which is the whole point of typing a command's
    /// name instead of hunting for its key.
    /// Whether the letters typed appear in order, which is all the command
    /// palette needs: its list is short and already in the order a person
    /// would want to read it, so there is nothing to rank. The answer comes
    /// from the scorer the other pickers rank with, so that one query never
    /// matches in one list and not in another.
    fn fuzzy_matches(haystack: &str, needle: &str) -> bool {
        return crate::utils::fuzzy_score(haystack, needle).is_some();
    }

    /// What a key press did to whichever picker is open. `Run` is the command
    /// a user picked by name, handed back rather than carried out here: some
    /// commands need the key loop's channels, and being chosen from a list
    /// does not make a command a different command.
    fn handle_picker_key(&mut self, key: KeyEvent) -> PickerKey {
        let is_palette = self.dialog_mode == DialogMode::CommandPalette;
        match key.code {
            KeyCode::Esc => {
                self.dialog_mode = DialogMode::None;
                return PickerKey::Handled;
            }
            KeyCode::Up => {
                if is_palette {
                    self.palette_move(false);
                } else {
                    self.symbol_move(false);
                }
                return PickerKey::Handled;
            }
            KeyCode::Down => {
                if is_palette {
                    self.palette_move(true);
                } else {
                    self.symbol_move(true);
                }
                return PickerKey::Handled;
            }
            KeyCode::Enter => {
                if !is_palette {
                    self.symbol_choose();
                    return PickerKey::Handled;
                }
                let chosen = self.palette_choice();
                self.dialog_mode = DialogMode::None;
                return match chosen {
                    Some(action) => PickerKey::Run(action),
                    None => PickerKey::Handled,
                };
            }
            KeyCode::Backspace => {
                if is_palette {
                    self.palette_backspace();
                } else {
                    self.symbol_backspace();
                }
                return PickerKey::Handled;
            }
            // Modified letters are still commands, so they go back to the key
            // tables rather than being typed into the filter.
            KeyCode::Char(c) if !key.modifiers.intersects(ratatui::crossterm::event::KeyModifiers::CONTROL | ratatui::crossterm::event::KeyModifiers::ALT) => {
                if is_palette {
                    self.palette_input(c);
                } else {
                    self.symbol_input(c);
                }
                return PickerKey::Handled;
            }
            _ => return PickerKey::Ignored,
        }
    }

    fn open_command_palette(&mut self) {
        self.dialog_mode = DialogMode::CommandPalette;
        self.palette_filter.clear();
        self.filter_palette();
    }

    fn filter_palette(&mut self) {
        let needle = self.palette_filter.to_lowercase();
        self.palette_matches = nail::keymap::COMMANDS
            .iter()
            .enumerate()
            .filter(|(_, command)| Self::fuzzy_matches(&command.name.to_lowercase(), &needle))
            .map(|(index, _)| index)
            .collect();
        self.palette_index = 0;
    }

    fn palette_input(&mut self, c: char) {
        self.palette_filter.push(c);
        self.filter_palette();
    }

    fn palette_backspace(&mut self) {
        self.palette_filter.pop();
        self.filter_palette();
    }

    fn palette_move(&mut self, forward: bool) {
        if self.palette_matches.is_empty() {
            return;
        }
        let count = self.palette_matches.len();
        self.palette_index = if forward { (self.palette_index + 1) % count } else { (self.palette_index + count - 1) % count };
    }

    /// The command the palette is sitting on, which the key thread runs the
    /// same way it runs a key: some commands need its channels, and a command
    /// is not a different thing for having been picked from a list.
    fn palette_choice(&self) -> Option<nail::keymap::Action> {
        let index = *self.palette_matches.get(self.palette_index)?;
        return Some(nail::keymap::COMMANDS[index].action);
    }

    /// The declarations in the open file, read off the text rather than out of
    /// the parse tree. A file being edited is a file that does not parse half
    /// the time, and a list of what is in it is least useful exactly then, so
    /// this takes the reading that always works.
    fn collect_file_symbols(&self) -> Vec<FileSymbol> {
        let mut symbols = Vec::new();
        for (index, line) in self.get_current_tab().content.iter().enumerate() {
            let label = declaration_label(line);
            if let Some(label) = label {
                symbols.push(FileSymbol { label, line: index + 1, file: None });
            }
        }
        return symbols;
    }

    /// Every declaration in every Nail file in the project, from the files on
    /// disk. A file open and edited is read from the buffer instead, so what
    /// the picker offers is what the user has in front of them rather than
    /// what was last saved.
    ///
    /// Reading the whole project is worth doing on the spot rather than
    /// keeping an index in step with an editor: declarations are one line
    /// each and finding them is a string comparison, so a project of a few
    /// hundred files costs a few milliseconds and is never out of date.
    fn collect_project_symbols(&self) -> Vec<FileSymbol> {
        let mut symbols = Vec::new();
        for relative in crate::utils::scan_project_files(std::path::Path::new(&self.project_root)) {
            if !relative.ends_with(".nail") {
                continue;
            }
            let lines = match self.project_lines(&relative) {
                Some(lines) => lines,
                None => continue,
            };
            for (index, line) in lines.iter().enumerate() {
                if let Some(label) = declaration_label(line) {
                    symbols.push(FileSymbol { label, line: index + 1, file: Some(relative.clone()) });
                }
            }
        }
        return symbols;
    }

    /// One project file's text, taken from the tab holding it when there is
    /// one. A file being edited is answered for by what is on screen, so that
    /// a symbol added a moment ago can be jumped to before it is saved.
    fn project_lines(&self, relative: &str) -> Option<Vec<String>> {
        let full = std::path::Path::new(&self.project_root).join(relative).to_string_lossy().to_string();
        if let Some(tab) = self.tabs.iter().find(|tab| tab.filename.as_deref() == Some(full.as_str())) {
            return Some(tab.content.clone());
        }
        return std::fs::read_to_string(&full).ok().map(|text| text.lines().map(|line| line.to_string()).collect());
    }

    /// Every line in the project that contains what was typed. Reading the
    /// whole project takes a few milliseconds, which is less than the time
    /// between two keystrokes, so there is no index to keep and nothing that
    /// can be out of date.
    ///
    /// Nothing is searched for until two letters are in, because one letter
    /// matches most lines of most files and a list of everything is not an
    /// answer. Matching ignores case, which is what a search box is assumed
    /// to do when nobody has said otherwise.
    fn search_project(&mut self) {
        self.symbol_entries.clear();
        self.symbol_matches.clear();
        self.symbol_index = 0;
        let needle = self.symbol_filter.to_lowercase();
        if needle.chars().count() < 2 {
            return;
        }
        for relative in crate::utils::scan_project_files(std::path::Path::new(&self.project_root)) {
            let lines = match self.project_lines(&relative) {
                Some(lines) => lines,
                None => continue,
            };
            for (index, line) in lines.iter().enumerate() {
                if !line.to_lowercase().contains(&needle) {
                    continue;
                }
                // Long lines are cut rather than wrapped, because a result
                // list is for choosing between places and not for reading the
                // code that is about to be opened anyway.
                let text: String = line.trim().chars().take(160).collect();
                self.symbol_entries.push(FileSymbol { label: text, line: index + 1, file: Some(relative.clone()) });
                if self.symbol_entries.len() >= Self::PROJECT_SEARCH_LIMIT {
                    self.symbol_matches = (0..self.symbol_entries.len()).collect();
                    return;
                }
            }
        }
        self.symbol_matches = (0..self.symbol_entries.len()).collect();
    }

    /// How many hits the search will collect before it stops looking. The
    /// title says when this is what happened, because a list that stops at a
    /// round number and does not say so reads as a complete answer.
    const PROJECT_SEARCH_LIMIT: usize = 200;

    fn open_symbol_picker(&mut self) {
        self.dialog_mode = DialogMode::SymbolPicker;
        self.symbol_source = SymbolSource::OpenFile;
        self.symbol_entries = self.collect_file_symbols();
        self.symbol_filter.clear();
        self.filter_symbols();
    }

    fn open_project_symbol_picker(&mut self) {
        self.dialog_mode = DialogMode::SymbolPicker;
        self.symbol_source = SymbolSource::Project;
        self.symbol_entries = self.collect_project_symbols();
        self.symbol_filter.clear();
        self.filter_symbols();
    }

    /// Opens the file the cursor's line imports. Paths in an import are
    /// written relative to the file doing the importing, which is how the
    /// compiler reads them, so they are resolved the same way here.
    ///
    /// A line that imports nothing does nothing, and a path that names no
    /// file does nothing either: the compiler already marks that line with an
    /// import error, and a second complaint about it would only be in the way.
    fn open_imported_file(&mut self) {
        let tab = self.get_current_tab();
        let line = match tab.content.get(tab.cursor_y) {
            Some(line) => line.clone(),
            None => return,
        };
        let path = match imported_path(&line) {
            Some(path) => path,
            None => return,
        };
        let beside = tab.filename.as_ref().and_then(|filename| std::path::Path::new(filename).parent().map(|parent| parent.to_path_buf()));
        let base = beside.unwrap_or_else(|| std::path::PathBuf::from(&self.project_root));
        let _ = self.open_file_in_tab(base.join(path).to_string_lossy().to_string());
    }

    /// The same picker again, over every line rather than every declaration.
    /// It opens empty because there is nothing to show until there is
    /// something to look for.
    fn open_project_search(&mut self) {
        self.dialog_mode = DialogMode::SymbolPicker;
        self.symbol_source = SymbolSource::ProjectText;
        self.symbol_filter.clear();
        self.search_project();
    }

    /// A query is matched against the file's name as well as the symbol's, so
    /// that `website new_post` narrows to one file's declarations without
    /// anybody having to invent a syntax for saying so.
    fn filter_symbols(&mut self) {
        let needle = self.symbol_filter.to_lowercase();
        let mut scored: Vec<(i32, usize)> = Vec::new();
        for (index, symbol) in self.symbol_entries.iter().enumerate() {
            // The name is what was asked for, so a match in it counts for
            // more than the same letters found in the path. A match in the
            // path still counts, because that is how one file's declarations
            // are picked out of the whole project's.
            let by_name = crate::utils::fuzzy_score(&symbol.label.to_lowercase(), &needle).map(|score| score + 40);
            let by_file = match &symbol.file {
                Some(file) => crate::utils::fuzzy_score(&format!("{} {}", symbol.label, file).to_lowercase(), &needle),
                None => None,
            };
            if let Some(score) = by_name.into_iter().chain(by_file).max() {
                scored.push((score, index));
            }
        }
        // Stable, so equal scores keep the order they were collected in,
        // which is file by file and top to bottom within each.
        scored.sort_by(|left, right| right.0.cmp(&left.0));
        self.symbol_matches = scored.into_iter().map(|(_, index)| index).collect();
        self.symbol_index = 0;
    }

    fn symbol_input(&mut self, c: char) {
        self.symbol_filter.push(c);
        self.symbol_query_changed();
    }

    fn symbol_backspace(&mut self) {
        self.symbol_filter.pop();
        self.symbol_query_changed();
    }

    fn symbol_query_changed(&mut self) {
        match self.symbol_source {
            SymbolSource::ProjectText => self.search_project(),
            SymbolSource::OpenFile | SymbolSource::Project => self.filter_symbols(),
        }
    }

    fn symbol_move(&mut self, forward: bool) {
        if self.symbol_matches.is_empty() {
            return;
        }
        let count = self.symbol_matches.len();
        self.symbol_index = if forward { (self.symbol_index + 1) % count } else { (self.symbol_index + count - 1) % count };
    }

    /// A symbol from another file is opened before it is jumped to. A symbol
    /// from this one is only jumped to, which is what keeps the picker usable
    /// on a file that has never been saved and has no path to open.
    fn symbol_choose(&mut self) {
        let chosen = match self.symbol_matches.get(self.symbol_index).and_then(|index| self.symbol_entries.get(*index)) {
            Some(symbol) => symbol.clone(),
            None => return,
        };
        if let Some(file) = chosen.file {
            let full = std::path::Path::new(&self.project_root).join(file).to_string_lossy().to_string();
            if self.open_file_in_tab(full).is_err() {
                return;
            }
        }
        self.go_to_line(chosen.line);
        self.dialog_mode = DialogMode::None;
    }

    fn toggle_whole_word(&mut self) {
        self.whole_word = !self.whole_word;
        self.find_all_matches();
    }

    fn toggle_regex(&mut self) {
        self.use_regex = !self.use_regex;
        self.find_all_matches();
    }

    /// The find box's three switches all come out as one regular expression:
    /// plain text is escaped so its punctuation means itself, whole word adds
    /// the boundaries, and ignoring case is a flag on the front.
    fn search_pattern(&self) -> Result<regex::Regex, String> {
        let body = if self.use_regex { self.search_query.clone() } else { regex::escape(&self.search_query) };
        let body = if self.whole_word { format!(r"\b(?:{})\b", body) } else { body };
        let source = if self.case_sensitive { body } else { format!("(?i){}", body) };
        return regex::Regex::new(&source).map_err(|error| error.to_string());
    }

    /// What the find box says under the pattern: how many matches, or why
    /// there are none.
    fn search_status_line(&self) -> String {
        if let Some(message) = &self.search_error {
            return format!("Not a pattern yet: {}", message);
        }
        return self.get_search_status();
    }

    /// Which titles the tab bar is showing. The draw thread paints them and a
    /// click has to work out which one it landed on, so both ask here.
    fn tab_titles(&self) -> Vec<String> {
        return self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let mut title = match &tab.filename {
                    Some(filename) => std::path::Path::new(filename).file_name().unwrap_or_default().to_string_lossy().to_string(),
                    None => format!("Untitled {}", index + 1),
                };
                if tab.modified {
                    title.push('*');
                }
                title
            })
            .collect();
    }

    /// Which tab a click at this column landed on. The arithmetic mirrors how
    /// the tab bar is drawn: a border, then each title with a space either
    /// side, with a divider between neighbours.
    fn tab_at_column(&self, column: u16) -> Option<usize> {
        let mut left = self.view.tabs.x + 1;
        for (index, title) in self.tab_titles().iter().enumerate() {
            let width = title.chars().count() as u16 + 2;
            if column >= left && column < left + width {
                return Some(index);
            }
            left += width + 1;
        }
        return None;
    }

    /// Which position in the file a click at this row and column points at,
    /// when it points into the text at all.
    fn text_position_at(&self, column: u16, row: u16) -> Option<(usize, usize)> {
        let text = self.view.text;
        if text.width == 0 || text.height == 0 || !point_in_rect(column, row, text) {
            return None;
        }
        let tab = self.get_current_tab();
        let line = ((row - text.y) as usize + tab.scroll_position as usize).min(tab.content.len().saturating_sub(1));
        let width = tab.content.get(line).map_or(0, |text| text.chars().count());
        let offset = ((column - text.x) as usize + tab.h_scroll as usize).min(width);
        return Some((offset, line));
    }

    fn mouse_press(&mut self, column: u16, row: u16) {
        if point_in_rect(column, row, self.view.tabs) {
            if let Some(index) = self.tab_at_column(column) {
                self.switch_to_tab(index);
            }
            return;
        }
        let position = match self.text_position_at(column, row) {
            Some(position) => position,
            None => return,
        };
        self.clear_selection();
        self.expand_stack.clear();
        {
            let tab = self.get_current_tab_mut();
            tab.cursor_x = position.0;
            tab.cursor_y = position.1;
        }
        self.mouse_dragging = true;
        self.start_selection();
        self.update_bracket_matching();
    }

    /// A drag that leaves the text area keeps selecting: the pointer is
    /// clamped back to the nearest edge rather than the drag being dropped,
    /// because someone dragging past the bottom means "keep going".
    fn mouse_drag(&mut self, column: u16, row: u16) {
        if !self.mouse_dragging {
            return;
        }
        let text = self.view.text;
        if text.width == 0 || text.height == 0 {
            return;
        }
        let column = column.clamp(text.x, text.x + text.width - 1);
        let row = row.clamp(text.y, text.y + text.height - 1);
        let position = match self.text_position_at(column, row) {
            Some(position) => position,
            None => return,
        };
        {
            let tab = self.get_current_tab_mut();
            tab.cursor_x = position.0;
            tab.cursor_y = position.1;
        }
        self.extend_selection();
    }

    fn mouse_release(&mut self) {
        self.mouse_dragging = false;
        // A click that never moved leaves an empty selection behind, which
        // would otherwise make the next typed character look like a replace.
        if !self.has_selection() {
            self.clear_selection();
        }
    }

    /// What the IDE opens with next time it is started here: the files that
    /// were open, where the cursor was in each, and which one was in front.
    /// A file that has moved away since is simply skipped on the way back in.
    fn save_session(&self) {
        let open_files: Vec<String> = self.tabs.iter().filter_map(|tab| tab.filename.as_ref().map(|filename| format!("{}:{}:{}", filename, tab.cursor_y + 1, tab.cursor_x))).collect();
        let active = self.tabs.get(self.tab_index).and_then(|tab| tab.filename.clone()).unwrap_or_default();
        crate::utils::write_config_values(&[("open_files", open_files.join(",")), ("active_file", active), ("recent_files", self.recent_files.join(","))]);
    }

    fn restore_session(&mut self) {
        if let Some(recent) = crate::utils::read_config_value("recent_files") {
            self.recent_files = recent.split(',').filter(|entry| !entry.is_empty()).map(String::from).collect();
        }
        let open_files = match crate::utils::read_config_value("open_files") {
            Some(open_files) => open_files,
            None => return,
        };
        let active = crate::utils::read_config_value("active_file").unwrap_or_default();
        let opened_before = self.tabs.len();
        for entry in open_files.split(',').filter(|entry| !entry.is_empty()) {
            // Split from the right, because a path may contain a colon and the
            // line and column never do.
            let mut pieces = entry.rsplitn(3, ':');
            let column = pieces.next().and_then(|piece| piece.parse::<usize>().ok());
            let line = pieces.next().and_then(|piece| piece.parse::<usize>().ok());
            let path = match pieces.next() {
                Some(path) => path,
                None => continue,
            };
            if !std::path::Path::new(path).exists() || self.open_file_in_tab(path.to_string()).is_err() {
                continue;
            }
            if let (Some(line), Some(column)) = (line, column) {
                let tab = self.get_current_tab_mut();
                tab.cursor_y = line.saturating_sub(1).min(tab.content.len().saturating_sub(1));
                let width = tab.content[tab.cursor_y].chars().count();
                tab.cursor_x = column.min(width);
            }
        }
        if self.tabs.len() > opened_before {
            self.drop_welcome_tab();
        }
        if let Some(index) = self.tabs.iter().position(|tab| tab.filename.as_deref() == Some(active.as_str())) {
            self.tab_index = index;
        }
    }

    /// The greeting is there for someone with nothing open. Once real files
    /// are back it is just a tab in the way.
    fn drop_welcome_tab(&mut self) {
        let welcome_is_untouched = self.tabs.first().map_or(false, |tab| tab.filename.is_none() && !tab.modified);
        if welcome_is_untouched && self.tabs.len() > 1 {
            self.tabs.remove(0);
            self.tab_index = self.tab_index.saturating_sub(1);
        }
    }
}

/// The declaration a line of Nail starts, if it starts one. Functions,
/// structs and enums announce themselves with a keyword, and a binding at the
/// left margin is the file's own data rather than a local inside something
/// else.
fn declaration_label(line: &str) -> Option<String> {
    let text = line.trim_start();
    let indented = text.len() != line.len();
    if let Some(rest) = text.strip_prefix("f ") {
        return Some(format!("f {}", name_until(rest, &['(', ':', ' '])));
    }
    if let Some(rest) = text.strip_prefix("struct ") {
        return Some(format!("struct {}", name_until(rest, &['{', ' '])));
    }
    if let Some(rest) = text.strip_prefix("enum ") {
        return Some(format!("enum {}", name_until(rest, &['{', ' '])));
    }
    if indented || !text.contains('=') {
        return None;
    }
    let name = name_until(text, &[':']);
    let is_binding = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') && text[name.len()..].starts_with(':');
    if is_binding {
        return Some(name.to_string());
    }
    return None;
}

fn name_until<'a>(text: &'a str, stops: &[char]) -> &'a str {
    let end = text.find(|c| stops.contains(&c)).unwrap_or(text.len());
    return text[..end].trim();
}

/// The file an import line brings in, as it is written on the line. Only
/// import lines answer, because a path is the one thing in a Nail file that
/// names another file, and reading any old string literal as one would send
/// people to files that do not exist.
///
/// The dangerous form is tried first: `import` is a prefix of
/// `import_dangerous`, so testing the shorter one first would match both and
/// then fail to find the bracket.
fn imported_path(line: &str) -> Option<String> {
    let text = line.trim_start();
    let rest = match text.strip_prefix("import_dangerous") {
        Some(rest) => rest,
        None => text.strip_prefix("import")?,
    };
    let rest = rest.trim_start().strip_prefix('(')?.trim_start().strip_prefix('`')?;
    let (path, _) = rest.split_once('`')?;
    if path.is_empty() {
        return None;
    }
    return Some(path.to_string());
}

/// The next selection outward from the one given: the word under a bare
/// cursor, then whatever brackets enclose it, then the whole line, then the
/// whole file. Returns nothing once the selection is the file, which is where
/// expanding runs out of room.
fn wider_span(text: &[char], start: usize, end: usize) -> Option<(usize, usize)> {
    if start == end {
        if let Some(word) = word_span(text, start) {
            if word != (start, end) {
                return Some(word);
            }
        }
    }
    if let Some((open, close)) = enclosing_bracket_span(text, start, end) {
        if (open + 1, close) != (start, end) {
            return Some((open + 1, close));
        }
        return Some((open, close + 1));
    }
    let line = line_span(text, start, end);
    if line != (start, end) {
        return Some(line);
    }
    if (start, end) != (0, text.len()) {
        return Some((0, text.len()));
    }
    return None;
}

fn word_span(text: &[char], at: usize) -> Option<(usize, usize)> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut start = at;
    while start > 0 && is_word(text[start - 1]) {
        start -= 1;
    }
    let mut end = at;
    while end < text.len() && is_word(text[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    return Some((start, end));
}

fn line_span(text: &[char], start: usize, end: usize) -> (usize, usize) {
    let mut line_start = start;
    while line_start > 0 && text[line_start - 1] != '\n' {
        line_start -= 1;
    }
    let mut line_end = end;
    while line_end < text.len() && text[line_end] != '\n' {
        line_end += 1;
    }
    return (line_start, line_end);
}

/// The innermost bracket pair that contains the whole span, found by counting
/// depth outward from each end.
fn enclosing_bracket_span(text: &[char], start: usize, end: usize) -> Option<(usize, usize)> {
    let opening = |c: char| matches!(c, '(' | '[' | '{');
    let closing = |c: char| matches!(c, ')' | ']' | '}');
    let partner = |c: char| match c {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => c,
    };

    let mut depth = 0;
    let mut open = None;
    for index in (0..start).rev() {
        let c = text[index];
        if closing(c) {
            depth += 1;
        } else if opening(c) {
            if depth == 0 {
                open = Some(index);
                break;
            }
            depth -= 1;
        }
    }
    let open = open?;
    let wanted = partner(text[open]);

    let mut depth = 0;
    for index in end..text.len() {
        let c = text[index];
        if c == text[open] {
            depth += 1;
        } else if c == wanted {
            if depth == 0 {
                return Some((open, index));
            }
            depth -= 1;
        }
    }
    return None;
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


/// The pieces of the editor that are plain functions over text, which is the
/// half of it that can be checked without a terminal in front of it.
#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<char> {
        return text.chars().collect();
    }

    /// An editor holding one unsaved buffer of the given lines. Nothing here
    /// touches the disk, so these never write a session file.
    fn editor_with(lines: &[&str]) -> Editor {
        let mut editor = Editor::new_with_debug(false);
        editor.tabs = vec![Tab::new_with_file("test.nail".to_string(), lines.iter().map(|line| line.to_string()).collect())];
        editor.tab_index = 0;
        return editor;
    }

    fn select(editor: &mut Editor, start: (usize, usize), end: (usize, usize)) {
        let tab = editor.get_current_tab_mut();
        tab.selection_start = Some(start);
        tab.selection_end = Some(end);
        tab.selection_mode = true;
    }

    #[test]
    fn sorting_reorders_the_selected_lines_and_leaves_the_rest() {
        let mut editor = editor_with(&["header", "zebra", "alpha", "middle"]);
        // Dragged from the start of "zebra" to the start of "middle", which
        // reaches two lines rather than three
        select(&mut editor, (0, 1), (0, 3));
        editor.sort_lines();
        assert_eq!(editor.get_current_tab().content, vec!["header", "alpha", "zebra", "middle"]);
    }

    /// Shift with a motion has to keep the place it started from. It used to
    /// drop its anchor after moving, which lost the first line of every
    /// selection made this way and left sorting and joining working on one
    /// line less than was highlighted.
    #[test]
    fn shift_with_a_motion_selects_from_where_it_started() {
        let mut editor = editor_with(&["first", "second", "third"]);
        editor.move_cursor_down_with_selection(true);
        assert_eq!(editor.get_current_tab().selection_start, Some((0, 0)));
        assert_eq!(editor.get_current_tab().selection_end, Some((0, 1)));
        assert_eq!(editor.get_selected_text(), "first\n");

        let mut editor = editor_with(&["alpha"]);
        editor.move_cursor_right_with_selection(true);
        assert_eq!(editor.get_selected_text(), "a");
    }

    #[test]
    fn sorting_without_a_selection_leaves_the_file_alone() {
        let mut editor = editor_with(&["zebra", "alpha"]);
        editor.sort_lines();
        assert_eq!(editor.get_current_tab().content, vec!["zebra", "alpha"]);
    }

    #[test]
    fn joining_pulls_the_next_line_up_and_undo_puts_it_back() {
        let mut editor = editor_with(&["first line", "    second line", "third"]);
        editor.join_lines();
        assert_eq!(editor.get_current_tab().content, vec!["first line second line", "third"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["first line", "    second line", "third"]);
        assert!(editor.redo());
        assert_eq!(editor.get_current_tab().content, vec!["first line second line", "third"]);
    }

    #[test]
    fn a_bracket_typed_over_a_selection_wraps_it() {
        let mut editor = editor_with(&["print(greeting);"]);
        select(&mut editor, (6, 0), (14, 0));
        editor.insert_char('`');
        assert_eq!(editor.get_current_tab().content, vec!["print(`greeting`);"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["print(greeting);"]);
    }

    #[test]
    fn deleting_a_word_takes_the_word_and_not_the_line() {
        let mut editor = editor_with(&["alpha beta gamma"]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_x = 10;
        }
        editor.delete_word_left();
        assert_eq!(editor.get_current_tab().content, vec!["alpha  gamma"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["alpha beta gamma"]);
    }

    /// The list of errors is walked in file order and wraps at the end, and a
    /// message with no line in the file is not somewhere to jump to.
    #[test]
    fn the_error_keys_walk_the_errors_in_order() {
        let mut editor = editor_with(&["one", "two", "three", "four", "five"]);
        editor.code_errors = vec![
            CodeError { message: "later".to_string(), code_span: CodeSpan { start_line: 4, start_column: 1, end_line: 4, end_column: 2 } },
            CodeError { message: "earlier".to_string(), code_span: CodeSpan { start_line: 2, start_column: 1, end_line: 2, end_column: 2 } },
            CodeError::from("a notice with no line".to_string()),
        ];
        editor.go_to_error(true);
        assert_eq!(editor.get_current_tab().cursor_y, 1);
        editor.go_to_error(true);
        assert_eq!(editor.get_current_tab().cursor_y, 3);
        // Past the last one, so back round to the first
        editor.go_to_error(true);
        assert_eq!(editor.get_current_tab().cursor_y, 1);
        editor.go_to_error(false);
        assert_eq!(editor.get_current_tab().cursor_y, 3);
    }

    /// The three switches on the find box, each of which changes what counts
    /// as a match rather than how the matches are shown.
    #[test]
    fn the_search_switches_change_what_matches() {
        let mut editor = editor_with(&["cat", "concat", "CAT", "c.t"]);
        editor.search_query = "cat".to_string();
        editor.find_all_matches();
        assert_eq!(editor.search_results.len(), 3, "case insensitive plain text finds every cat");

        editor.case_sensitive = true;
        editor.find_all_matches();
        assert_eq!(editor.search_results.len(), 2);

        editor.whole_word = true;
        editor.find_all_matches();
        assert_eq!(editor.search_results.len(), 1, "concat contains cat but is not the word");

        // Punctuation in a plain search means itself, so this finds the
        // literal line and not the three letter words
        editor.whole_word = false;
        editor.search_query = "c.t".to_string();
        editor.find_all_matches();
        assert_eq!(editor.search_results.len(), 1);

        editor.use_regex = true;
        editor.find_all_matches();
        assert_eq!(editor.search_results.len(), 3, "as a pattern the dot matches any letter");

        // A pattern that cannot compile is a message under the box, not an
        // error to dismiss, and finds nothing until it is finished
        editor.search_query = "c(t".to_string();
        editor.find_all_matches();
        assert!(editor.search_results.is_empty());
        assert!(editor.search_error.is_some());
        assert!(editor.search_status_line().starts_with("Not a pattern yet"));
    }

    #[test]
    fn the_palette_narrows_to_the_command_that_was_typed() {
        let mut editor = editor_with(&["anything"]);
        editor.open_command_palette();
        assert!(editor.palette_matches.len() > 20, "an empty filter offers everything");
        for letter in "sortl".chars() {
            editor.palette_input(letter);
        }
        assert_eq!(editor.palette_choice(), Some(nail::keymap::Action::SortLines));
    }

    #[test]
    fn the_symbol_picker_lists_what_the_file_declares() {
        let mut editor = editor_with(&["nail latest", "greeting:s = `hi`;", "f shout(word:s):s {", "    y word;", "}"]);
        editor.open_symbol_picker();
        let labels: Vec<&str> = editor.symbol_matches.iter().map(|index| editor.symbol_entries[*index].label.as_str()).collect();
        assert_eq!(labels, vec!["greeting", "f shout"]);
        editor.symbol_move(true);
        editor.symbol_choose();
        assert_eq!(editor.get_current_tab().cursor_y, 2);
        assert_eq!(editor.dialog_mode, DialogMode::None);
    }

    /// A click reports a row and a column of the terminal, and only the text
    /// area's own corner turns that into a place in the file.
    #[test]
    fn a_click_lands_on_the_line_it_points_at() {
        let mut editor = editor_with(&["first line", "second line", "third line"]);
        editor.view = ViewLayout { tabs: ratatui::layout::Rect::new(0, 0, 80, 3), text: ratatui::layout::Rect::new(5, 4, 40, 10) };
        editor.get_current_tab_mut().scroll_position = 1;

        assert_eq!(editor.text_position_at(8, 5), Some((3, 2)));
        // Past the end of a line is the end of that line, not a column that
        // has no character in it
        assert_eq!(editor.text_position_at(40, 4), Some((11, 1)));
        // Outside the text area entirely
        assert_eq!(editor.text_position_at(2, 1), None);
    }

    #[test]
    fn a_declaration_is_recognized_by_its_keyword() {
        assert_eq!(declaration_label("f handle_request(request:HTTP_Request):s {"), Some("f handle_request".to_string()));
        assert_eq!(declaration_label("struct Player {"), Some("struct Player".to_string()));
        assert_eq!(declaration_label("enum Suit {"), Some("enum Suit".to_string()));
    }

    /// A binding at the left margin is the file's own data. The same line
    /// indented is a local inside something else, and listing those would bury
    /// the declarations worth jumping to.
    #[test]
    fn only_a_binding_at_the_margin_counts_as_a_symbol() {
        assert_eq!(declaration_label("greeting:s = `Hello`;"), Some("greeting".to_string()));
        assert_eq!(declaration_label("    greeting:s = `Hello`;"), None);
        assert_eq!(declaration_label("print(greeting);"), None);
        assert_eq!(declaration_label(""), None);
        assert_eq!(declaration_label("// a comment about f things"), None);
    }

    #[test]
    fn an_import_line_says_which_file_it_brings_in() {
        assert_eq!(imported_path("import(`website/safe/text_helpers.nail`)"), Some("website/safe/text_helpers.nail".to_string()));
        assert_eq!(imported_path("  import_dangerous(`helper.nail`)"), Some("helper.nail".to_string()));
        assert_eq!(imported_path("import (  `spaced.nail` )"), Some("spaced.nail".to_string()));
    }

    /// Anything that is not an import names no file, including a line that
    /// only mentions one. Following those would land the user in a file the
    /// program never reads.
    #[test]
    fn a_line_that_imports_nothing_names_no_file() {
        assert_eq!(imported_path("print(`import(`)"), None);
        assert_eq!(imported_path("// import(`commented_out.nail`)"), None);
        assert_eq!(imported_path("imported:s = `not_an_import.nail`;"), None);
        assert_eq!(imported_path("import()"), None);
        assert_eq!(imported_path("import(``)"), None);
        assert_eq!(imported_path(""), None);
    }

    #[test]
    fn a_word_grows_out_of_a_bare_cursor() {
        let text = chars("print(greeting);");
        assert_eq!(word_span(&text, 8), Some((6, 14)));
        // A cursor just past a word still means that word
        assert_eq!(word_span(&text, 5), Some((0, 5)));
        // Nothing but punctuation either side, so there is no word to take
        assert_eq!(word_span(&chars("a  b"), 2), None);
    }

    #[test]
    fn expanding_walks_from_word_to_brackets_to_line_to_file() {
        let text = chars("print(greeting);\nnext();");
        let word = wider_span(&text, 8, 8).expect("a cursor in a word has a word to grow into");
        assert_eq!(word, (6, 14));
        // The call holds nothing but that word, so the next step out is the
        // brackets themselves rather than a repeat of what is already selected
        let call = wider_span(&text, word.0, word.1).expect("the word is inside a call");
        assert_eq!(call, (5, 15));
        let line = wider_span(&text, call.0, call.1).expect("then the whole line");
        assert_eq!(line, (0, 16));
        let file = wider_span(&text, line.0, line.1).expect("then the whole file");
        assert_eq!(file, (0, text.len()));
        assert_eq!(wider_span(&text, 0, text.len()), None);
    }

    #[test]
    fn brackets_are_matched_by_depth_rather_than_by_the_nearest_one() {
        let text = chars("outer(inner(x), y)");
        assert_eq!(enclosing_bracket_span(&text, 12, 13), Some((11, 13)));
        // From outside the inner call, the pair found is the outer one
        assert_eq!(enclosing_bracket_span(&text, 6, 14), Some((5, 17)));
    }

    #[test]
    fn a_filter_matches_letters_in_order_not_letters_in_a_row() {
        assert!(Editor::fuzzy_matches("duplicate line", "dupl"));
        assert!(Editor::fuzzy_matches("duplicate line", "dl"));
        assert!(Editor::fuzzy_matches("duplicate line", ""));
        assert!(!Editor::fuzzy_matches("duplicate line", "ld"));
        assert!(!Editor::fuzzy_matches("duplicate line", "duplicated"));
    }

    /// The window chases the cursor in both directions, and leaves it alone
    /// when it is already on screen.
    #[test]
    fn the_view_follows_the_cursor_both_ways() {
        let mut tab = Tab::new();
        tab.content = (0..100).map(|index| format!("line {} {}", index, "x".repeat(200))).collect();

        tab.cursor_y = 50;
        tab.cursor_x = 0;
        tab.follow_cursor(80, 20);
        assert_eq!(tab.scroll_position, 31);
        assert_eq!(tab.h_scroll, 0);

        tab.cursor_x = 150;
        tab.follow_cursor(80, 20);
        assert_eq!(tab.h_scroll, 71);

        // Already in view, so nothing moves
        tab.cursor_y = 40;
        tab.cursor_x = 100;
        tab.follow_cursor(80, 20);
        assert_eq!(tab.scroll_position, 31);
        assert_eq!(tab.h_scroll, 71);

        tab.cursor_y = 0;
        tab.cursor_x = 0;
        tab.follow_cursor(80, 20);
        assert_eq!(tab.scroll_position, 0);
        assert_eq!(tab.h_scroll, 0);
    }
}
