mod checker;
mod colorizer;
mod common;
mod docs;
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
use crate::utils::create_welcome_message;
use rayon::prelude::*;
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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

use crate::utils::lock;
use crate::utils::file_watcher_thread_logic;
use crate::utils::profile_watcher_thread_logic;
use crate::utils::BuildStatus;
use crate::utils::ProfileData;

use crate::common::CodeSpan;
use ratatui::crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen, LeaveAlternateScreen},
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
    // When the file on disk last changed, as of the last load or save. The
    // watcher thread compares this against the file to notice someone else
    // (a formatter, another editor, an AI agent) writing it while it is open.
    disk_mtime: Option<std::time::SystemTime>,
    // The file changed on disk while this buffer holds unsaved edits. The
    // buffer is kept, this is shown, and saving overwrites the disk.
    disk_changed_underneath: bool,
}

/// How many columns a line has. A column is a character, because that is what
/// the cursor steps over and what a selection counts. A line's byte length is
/// a different number the moment anyone types an accent, and using it as a
/// column puts the cursor past the end of the line it is on.
fn columns_in(line: &str) -> usize {
    return line.chars().count();
}

/// The byte a column begins at, or the end of the line for a column past it.
/// Every slice taken at a cursor or a selection edge goes through here: a line
/// indexed by a column directly is right only until the line stops being all
/// ASCII, and then it panics in the middle of a character.
fn byte_of_column(line: &str, column: usize) -> usize {
    return line.char_indices().nth(column).map(|(byte, _)| byte).unwrap_or(line.len());
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
            disk_mtime: None,
            disk_changed_underneath: false,
        }
    }

    fn new_with_file(filename: String, content: Vec<String>) -> Self {
        let mut tab = Tab::new();
        tab.filename = Some(filename);
        tab.content = content;
        tab.record_disk_mtime();
        tab
    }

    /// Remembers when the file on disk last changed, taken at every load and
    /// save, which are the moments the buffer and the disk agree. The watcher
    /// thread reloads the buffer when the disk moves past this.
    fn record_disk_mtime(&mut self) {
        self.disk_mtime = self.filename.as_ref().and_then(|filename| std::fs::metadata(filename).and_then(|meta| meta.modified()).ok());
        self.disk_changed_underneath = false;
    }

    /// Replaces the buffer with what the file says now, either because
    /// something other than this editor wrote it or because the user asked
    /// for the disk's copy. Selections point at lines that no longer exist,
    /// so they are dropped rather than left to land somewhere else. The
    /// cursor and scroll stay put, clamped back inside the file, so watching
    /// a file being rewritten does not fling the view around. The undo
    /// history is the caller's question, because the two callers need
    /// opposite answers.
    fn swap_in_disk_content(&mut self, lines: Vec<String>, mtime: Option<std::time::SystemTime>) {
        self.content = if lines.is_empty() { vec![String::new()] } else { lines };
        self.disk_mtime = mtime;
        self.disk_changed_underneath = false;
        self.modified = false;
        self.selection_start = None;
        self.selection_end = None;
        self.selection_mode = false;
        self.settle_cursor();
        let width = columns_in(&self.content[self.cursor_y]);
        if self.cursor_x > width {
            self.cursor_x = width;
        }
        let furthest = self.content.len().saturating_sub(1) as u16;
        if self.scroll_position > furthest {
            self.scroll_position = furthest;
        }
    }

    /// The watcher's reload, taken only when the buffer has nothing unsaved:
    /// the history pointed into text that is gone and held nothing of the
    /// user's worth keeping, so it goes too.
    fn reload_from_disk(&mut self, lines: Vec<String>, mtime: Option<std::time::SystemTime>) {
        self.swap_in_disk_content(lines, mtime);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.char_insert_group.clear();
        self.last_char_insert_time = None;
    }

    /// The user's reload, for a buffer whose edits they said to throw away:
    /// the whole swap is recorded as one edit, so undo is what un-throws
    /// them. A key that can be taken back this way needs no confirmation
    /// dialog in front of it.
    fn take_disk_copy(&mut self, lines: Vec<String>, mtime: Option<std::time::SystemTime>) {
        let old_text = self.content.join("\n");
        self.swap_in_disk_content(lines, mtime);
        let new_text = self.content.join("\n");
        if old_text != new_text {
            self.record_operation(EditOperation::ReplaceText { position: (0, 0), old_text, new_text });
        }
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
    
    /// Removes the selection and writes nothing to the undo stack, so the
    /// caller owes it an entry. Every command that replaces a selection
    /// records the removal and what went in its place as one edit, and this is
    /// the half of that which touches the text.
    fn delete_selected_text_unrecorded(&mut self) {
        if !self.has_selection() {
            return;
        }

        let (start_pos, end_pos) = self.get_normalized_selection();
        
        if start_pos.1 == end_pos.1 {
            // Single line selection
            let line = &mut self.content[start_pos.1];
            let before = line[..byte_of_column(line, start_pos.0)].to_string();
            let after = line[byte_of_column(line, end_pos.0)..].to_string();
            *line = format!("{}{}", before, after);
        } else {
            // Multi-line selection
            let start_line = &self.content[start_pos.1];
            let end_line = &self.content[end_pos.1];

            let before = start_line[..byte_of_column(start_line, start_pos.0)].to_string();
            let after = end_line[byte_of_column(end_line, end_pos.0)..].to_string();
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
    
    /// Puts the cursor back inside the file. Nothing is supposed to leave it
    /// outside, but an edit begins by reading the line the cursor is on, and a
    /// cursor that had left would take the editor down rather than miss.
    fn settle_cursor(&mut self) {
        if self.content.is_empty() {
            self.content.push(String::new());
        }
        if self.cursor_y >= self.content.len() {
            self.cursor_y = self.content.len() - 1;
        }
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
    // The half-typed chord an emacs user is in the middle of, or the operator
    // a vim user has started, which decides what the next key means
    pending_prefix: Option<nail::keymap::Prefix>,
    // Which vim mode is on. It means nothing under the other two keymaps, and
    // starts at normal because that is where vim starts.
    vim_mode: nail::keymap::VimMode,
    mark_active: bool,
    settings_row: usize,
    // Tab system
    tabs: Vec<Tab>,
    tab_index: usize,
    // Global state
    //
    // One clipboard for the life of the editor, made the first time it is
    // used. On X11 the copying program serves the paste itself, so a
    // clipboard that is dropped right after writing takes the copied text
    // with it, and arboard prints a warning straight over the screen.
    clipboard: Option<arboard::Clipboard>,
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
    /// A list the keyboard has asked for and that has not been built yet.
    /// See `request_completions`.
    completion_request: Option<CompletionRequest>,
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
    // The timing annotations can be turned off, and that is also how they
    // are kept out of a screen copy: what is displayed is what is copied.
    show_timings: bool,
    // Asked for by the keyboard, answered by the draw thread, because only
    // the finished frame knows what the screen actually says.
    screen_copy_requested: bool,
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
    // Set while a drag that began on the minimap is in progress, so the drag
    // keeps scrubbing the view instead of turning into a text selection.
    minimap_dragging: bool,
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
    // Counted up by every thread that changes what the screen should show,
    // and read by the draw thread, which paints only when it moves. The
    // number itself means nothing, only that it moved.
    repaint: u64,
}

/// The parts of the screen a mouse click can land in, as the draw thread last
/// laid them out. `text` is the area inside the editor's border, so a click at
/// its top left is the first visible character rather than the frame around it.
/// `minimap` is the strip to the right of the text, and is a zero-sized rect
/// whenever the minimap is switched off, which no click can land in.
#[derive(Clone, Copy, Debug, Default)]
struct ViewLayout {
    tabs: ratatui::layout::Rect,
    text: ratatui::layout::Rect,
    minimap: ratatui::layout::Rect,
}

/// How many source lines one minimap row stands for. A row is one braille
/// cell of four dot-rows, so each dot-row covers an equal share of the file,
/// and a file short enough to fit gets the finest scale of one line per
/// dot-row. The draw thread and the mouse both go through this so the picture
/// and the click can never disagree about where a line is.
pub fn minimap_lines_per_row(total_lines: usize, rows: u16) -> usize {
    let dot_rows = (rows as usize * 4).max(1);
    let lines_per_dot_row = (total_lines.div_ceil(dot_rows)).max(1);
    return lines_per_dot_row * 4;
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

/// Which of the two forms of a documented example to put in the file. The
/// names are the ones the documentation panel shows: the example is the
/// single line that calls the function, the full example is the whole
/// runnable program around it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ExampleForm {
    Example,
    FullExample,
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
        return byte_of_column(s, char_index);
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
            vim_mode: nail::keymap::VimMode::Normal,
            mark_active: false,
            settings_row: 0,
            tabs: vec![welcome_tab],
            tab_index: 0,
            clipboard: None,
            build_status: BuildStatus::Idle,
            code_errors: Vec::new(),
            scroll_state: ScrollbarState::default(),
            max_undo_history: 1000,
            completions: Vec::new(),
            completion_index: 0,
            show_completions: false,
            completion_request: None,
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
            show_timings: stored_flag("timings", true),
            screen_copy_requested: false,
            // Bracket matching state
            matching_bracket_pos: None,
            view: ViewLayout::default(),
            // On, because a click landing where the user pointed is what
            // everyone expects, and F4 is there for the times it is not.
            mouse_enabled: true,
            mouse_dragging: false,
            minimap_dragging: false,
            expand_stack: Vec::new(),
            profile_data: None,
            profile_dumps: std::collections::HashMap::new(),
            compile_started: None,
            compile_estimate: None,
            repaint: 0,
        }
    }

    /// Tells the draw thread the screen is stale. Called by whichever thread
    /// changed something visible, while it still holds the editor lock, and
    /// harmless to call when nothing actually changed: the cost is one frame.
    pub fn request_repaint(&mut self) {
        self.repaint = self.repaint.wrapping_add(1);
    }

    /// The current repaint count, compared by the draw thread against the
    /// value it last painted.
    pub fn repaint_count(&self) -> u64 {
        return self.repaint;
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
        new_tab.record_disk_mtime();
        
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
        // Scored in parallel: this runs on every keystroke over every file in
        // the project, and each file's score is independent of the others.
        // The collect keeps the walk's order, so the stable sort below still
        // means what it says.
        let mut scored: Vec<(i32, FileEntry)> = self
            .file_index
            .par_iter()
            .filter_map(|relative| {
                let mut score = crate::utils::path_score(&relative.to_lowercase(), &needle)?;
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
                Some((score, FileEntry { name: relative.clone(), path: full, is_directory: false, is_recent: recent.is_some() }))
            })
            .collect();
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

            let signature = func.nail_signature(name);
            
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
    
    /// Puts a piece of Nail into the file exactly as written, however many
    /// lines it runs to, and leaves the cursor at the end of it. Whatever the
    /// cursor was sitting in front of stays in front of it.
    fn insert_lines_at_cursor(&mut self, text: &str) {
        let tab = self.get_current_tab_mut();
        if tab.cursor_y >= tab.content.len() {
            tab.content.push(String::new());
            tab.cursor_y = tab.content.len() - 1;
        }
        let width = tab.content[tab.cursor_y].chars().count();
        if tab.cursor_x > width {
            tab.cursor_x = width;
        }

        let split = Self::char_to_byte_index(&tab.content[tab.cursor_y], tab.cursor_x);
        let tail = tab.content[tab.cursor_y][split..].to_string();
        tab.content[tab.cursor_y].truncate(split);

        let mut pieces = text.split('\n');
        let first = pieces.next().unwrap_or("");
        tab.content[tab.cursor_y].push_str(first);
        let rest: Vec<String> = pieces.map(|piece| piece.to_string()).collect();
        for (offset, extra) in rest.iter().enumerate() {
            tab.content.insert(tab.cursor_y + 1 + offset, extra.clone());
        }

        tab.cursor_y += rest.len();
        tab.cursor_x = tab.content[tab.cursor_y].chars().count();
        let landing = tab.cursor_y;
        tab.content[landing].push_str(&tail);
        tab.modified = true;
    }

    /// The library browser hands over the function's worked example, because
    /// a name and a pair of parentheses is the part you already knew.
    ///
    /// An example is a whole program rather than a word, so it starts on a
    /// line of its own. Landing it against whatever the cursor was sitting
    /// after would join two statements into one line.
    fn insert_stdlib_function(&mut self, func_name: &str) {
        let example = crate::stdlib_registry::get_stdlib_function(func_name)
            .map(|function| function.example.to_string())
            .filter(|example| !example.is_empty())
            .unwrap_or_else(|| format!("{}()", func_name));

        let needs_its_own_line = {
            let tab = self.get_current_tab();
            tab.content
                .get(tab.cursor_y)
                .map(|line| line.chars().take(tab.cursor_x).any(|character| !character.is_whitespace()))
                .unwrap_or(false)
        };
        let text = if needs_its_own_line { format!("\n{}", example) } else { example };
        self.insert_lines_at_cursor(&text);
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

    /// Takes the selection out of the file and hands back the edit that would
    /// put it back. Typing, pasting or pressing Enter over a selection all
    /// begin by clearing it, and the text they cleared belongs on the undo
    /// stack rather than gone: without this, one keystroke over a selected
    /// block is unrecoverable however many times Ctrl+Z is pressed.
    fn take_selection_for_replacement(&mut self) -> Option<EditOperation> {
        if !self.has_selection() {
            return None;
        }
        let text = self.get_selected_text();
        let (start, _) = {
            let current_tab = self.get_current_tab();
            current_tab.get_normalized_selection()
        };
        self.get_current_tab_mut().delete_selected_text_unrecorded();
        return Some(EditOperation::DeleteText { position: start, text });
    }

    /// Records an edit that replaced a selection as one entry, so a single
    /// undo takes back both the removal and what was put in its place.
    fn record_replacing_selection(&mut self, removed: Option<EditOperation>, operation: EditOperation) {
        let current_tab = self.get_current_tab_mut();
        match removed {
            Some(removal) => current_tab.record_operation(EditOperation::BatchOperation { operations: vec![removal, operation] }),
            None => current_tab.record_operation(operation),
        }
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
            EditOperation::DeleteNewline { position, merged_content } => {
                if reverse {
                    // Undo: split the line back
                    current_tab.cursor_x = position.0;
                    current_tab.cursor_y = position.1;
                    if current_tab.cursor_y < current_tab.content.len() {
                        let split = byte_of_column(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x);
                        current_tab.content[current_tab.cursor_y].truncate(split);
                        current_tab.content.insert(current_tab.cursor_y + 1, merged_content.clone());
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
                let split = byte_of_column(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x);
                let remaining = current_tab.content[current_tab.cursor_y].split_off(split);
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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

        // A selection typed over is the first half of this edit rather than
        // something that happened before it, so what it held is kept and
        // recorded alongside the character below.
        if debug_mode && self.has_selection() {
            log::info!("Deleting selection before inserting char");
        }
        let removed = self.take_selection_for_replacement();

        // Handle smart dedent for closing braces
        if c == '}' {
            let should_dedent = {
                let current_tab = self.get_current_tab();
                self.should_smart_dedent(current_tab)
            };
            if should_dedent {
                // Smart dedent records the line it rewrote, so a selection
                // taken on the way here needs an entry of its own.
                if let Some(removal) = removed {
                    self.get_current_tab_mut().record_operation(removal);
                }
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
        self.record_replacing_selection(removed, operation);
    }

    fn delete_forward(&mut self) {
        self.get_current_tab_mut().settle_cursor();
        // Delete key should delete selected text or character after cursor
        if self.has_selection() {
            self.delete_selected_text();
            return;
        }

        let current_tab = self.get_current_tab_mut();

        let cursor_x = current_tab.cursor_x;
        let cursor_y = current_tab.cursor_y;
        
        if cursor_y >= current_tab.content.len() {
            return;
        }
        
        let line_len = columns_in(&current_tab.content[cursor_y]);
        
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
            // The join happens at the end of the line, which is where undo has
            // to split it again. A cursor sitting past the end of a short line
            // would otherwise record a column the line does not have, and undo
            // padded the gap with spaces on the way to it.
            let operation = EditOperation::DeleteNewline {
                position: (line_len, cursor_y),
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
            let upper_line_len = columns_in(&current_tab.content[current_tab.cursor_y]);
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
        self.get_current_tab_mut().settle_cursor();
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
            let lower_line_len = columns_in(&current_tab.content[current_tab.cursor_y]);
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
        // Flush any pending character group when cursor moves
        self.flush_char_group();
        
        if extend_selection {
            self.anchor_selection();
        } else {
            self.clear_selection();
        }
        
        let current_tab = self.get_current_tab_mut();
        if current_tab.cursor_y < current_tab.content.len() {
            current_tab.cursor_x = columns_in(&current_tab.content[current_tab.cursor_y]);
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
            current_tab.cursor_x = columns_in(&current_tab.content[current_tab.cursor_y]);
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
                    x = columns_in(&current_tab.content[y]);
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
        if matches!(self.dialog_mode, DialogMode::Find | DialogMode::Replace) {
            // Vim leaves the matches lit after a search, because `n` is a key
            // away and the point of the highlighting is to show where it will
            // land. Ctrl+L is what puts them out. Every other keymap treats
            // closing the box as the end of the search.
            if self.keymap != nail::keymap::Keymap::Vim {
                self.search_results.clear();
            }
            self.clear_selection();
        }

        self.dialog_mode = DialogMode::None;
        self.goto_line_input.clear();
    }

    /// Vim's `:nohlsearch`, on the Ctrl+L neovim puts it on. The phrase itself
    /// is kept, so `n` can still find the next one after the lights go out.
    pub fn clear_search_highlight(&mut self) {
        self.search_results.clear();
        self.current_match_index = 0;
    }

    /// A number that changes whenever the text does, and never otherwise. It
    /// is only ever compared with an earlier reading of itself, which is why
    /// counting the recorded edits is enough and what they were does not
    /// matter.
    pub fn edit_marker(&self) -> (usize, usize, usize) {
        let tab = self.get_current_tab();
        return (tab.undo_stack.len(), tab.char_insert_group.len(), tab.content.len());
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
                // A match arrives measured in bytes and every other part of
                // the editor counts columns, so it is converted once, here.
                // Kept in bytes, a match after an accented character
                // highlighted the wrong letters and put the cursor in the
                // wrong place.
                let start = columns_in(&line[..found.start()]);
                let end = start + columns_in(found.as_str());
                self.search_results.push((line_idx, start, end));
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
    
    /// Searching again for whatever was searched for last. The matches are
    /// dropped when the find dialog closes, so a repeat search that comes
    /// later has to find them again before it can step through them. Without
    /// this, F3 and vim's `n` both do nothing the moment the dialog is shut,
    /// which is exactly when a user reaches for them.
    fn find_again(&mut self, forward: bool) {
        self.search_direction_forward = forward;
        if !self.search_results.is_empty() {
            if forward {
                self.find_next();
            } else {
                self.find_previous();
            }
            return;
        }
        if self.search_query.is_empty() {
            return;
        }
        // Where the cursor was before the search moved it, which is the last
        // match it landed on.
        let cursor = self.cursor_position();
        self.find_all_matches();
        if self.search_results.is_empty() {
            return;
        }
        // Finding again lands on the match at or after the cursor, and after a
        // previous search that is the match the cursor is already sitting on.
        // Stepping over it is what makes this a search for the next one.
        let (line, start, _) = self.search_results[self.current_match_index];
        if !forward || (start, line) == cursor {
            if forward {
                self.find_next();
            } else {
                self.find_previous();
            }
        }
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
        let span = byte_of_column(&current_tab.content[line], start)..byte_of_column(&current_tab.content[line], end);
        let old_text = current_tab.content[line][span.clone()].to_string();
        let operation = EditOperation::ReplaceText {
            position: (start, line),
            old_text: old_text,
            new_text: replace_text.clone(),
        };

        // Replace the text
        let current_tab = self.get_current_tab_mut();
        current_tab.content[line].replace_range(span, &replace_text);
        current_tab.modified = true;
        current_tab.record_operation(operation);

        // Update search results to account for the replacement
        let length_diff = columns_in(&replace_text) as isize - (end - start) as isize;
        
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
                let span = byte_of_column(&current_tab.content[line], start)..byte_of_column(&current_tab.content[line], end);
                let old_text = current_tab.content[line][span].to_string();
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
            let span = byte_of_column(&current_tab.content[line], start)..byte_of_column(&current_tab.content[line], end);
            current_tab.content[line].replace_range(span, &replace_text);
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
        self.get_current_tab_mut().settle_cursor();
        // A selection is what this line break replaces, so it is recorded with
        // the break rather than dropped on the way to it.
        let removed = self.take_selection_for_replacement();

        // Calculate auto-indentation
        let (current_line, cursor_x) = {
            let current_tab = self.get_current_tab();
            let line = current_tab.content[current_tab.cursor_y].clone();
            let x = current_tab.cursor_x;
            (line, x)
        };
        let indent = self.calculate_auto_indent(&current_line, cursor_x);
        
        let current_tab = self.get_current_tab_mut();

        // The break carries the new line's indentation with it, so the entry
        // says so. Recorded as a bare newline, undo merged the two lines back
        // together and left the indentation sitting in the middle of the
        // line: pressing Enter and changing your mind added four spaces.
        let operation = EditOperation::InsertText {
            position: (current_tab.cursor_x, current_tab.cursor_y),
            text: format!("\n{}", indent),
        };

        let split = byte_of_column(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x);
        let remaining = current_tab.content[current_tab.cursor_y].split_off(split);
        current_tab.cursor_y += 1;
        current_tab.content.insert(current_tab.cursor_y, format!("{}{}", indent, remaining));
        current_tab.cursor_x = columns_in(&indent);
        current_tab.modified = true;

        self.record_replacing_selection(removed, operation);
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
        let line_before_cursor = &current_line[..byte_of_column(current_line, cursor_x)];
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
        let before_cursor = &line[..byte_of_column(line, tab.cursor_x)];
        
        // Only dedent if the line contains only whitespace before the cursor
        before_cursor.trim().is_empty()
    }

    fn smart_dedent_tab(tab: &mut Tab) {
        if tab.cursor_y >= tab.content.len() {
            return;
        }

        let line = &mut tab.content[tab.cursor_y];
        let split = byte_of_column(line, tab.cursor_x);
        let before_cursor = line[..split].to_string();

        // Remove one level of indentation (4 spaces). A line with less than
        // that in front of the cursor has no level to remove, so the brace
        // simply goes in where it was typed.
        let kept_indent = match before_cursor.ends_with("    ") {
            true => &before_cursor[..before_cursor.len() - 4],
            false => &before_cursor[..],
        };
        let dedented = format!("{}{}{}", kept_indent, '}', &line[split..]);

        let operation = EditOperation::ReplaceText {
            position: (0, tab.cursor_y),
            old_text: line.clone(),
            new_text: dedented.clone(),
        };

        *line = dedented;
        // The cursor follows the brace, which sits at the end of whatever
        // indentation was kept. Working this out by subtracting four from
        // where the cursor was used to underflow on a line indented less
        // deeply than that, and take the editor down with it.
        tab.cursor_x = kept_indent.chars().count() + 1;
        tab.modified = true;
        tab.record_operation(operation);
    }

    fn indent_selection(&mut self) {
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
        self.get_current_tab_mut().settle_cursor();
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
        if tab.cursor_x >= columns_in(line) {
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
            x = columns_in(&tab.content[y]);
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
        self.cycle_theme(true);
    }

    fn cycle_theme(&mut self, forward: bool) {
        self.theme = crate::colorizer::neighbor_theme(self.theme, forward);
        let _ = self.save_config();
    }

    fn set_theme(&mut self, theme: &str) {
        self.theme = crate::colorizer::theme_by_name(theme).unwrap_or(&DARK_THEME);
        let _ = self.save_config();
    }

    fn scroll_up(&mut self) {
        let page_size = self.page_size();

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
                let line_len = columns_in(&current_tab.content[current_tab.cursor_y]);
                current_tab.cursor_x = current_tab.cursor_x.min(line_len);
            }
            current_tab.scroll_position
        };
        
        self.scroll_state = self.scroll_state.position(new_scroll_pos as usize);
    }

    fn scroll_down(&mut self) {
        let page_size = self.page_size();

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
                let line_len = columns_in(&current_tab.content[current_tab.cursor_y]);
                current_tab.cursor_x = current_tab.cursor_x.min(line_len);
            }
            current_tab.scroll_position
        };
        
        self.scroll_state = self.scroll_state.position(new_scroll_pos as usize);
    }

    /// How far one press of Page Up or Page Down moves: the height of the
    /// text area as the draw thread last measured it. Before the first frame
    /// has measured anything the old guess of twenty lines stands in.
    fn page_size(&self) -> u16 {
        let height = self.view.text.height;
        return if height == 0 { 20 } else { height };
    }

    fn save_config(&self) -> std::io::Result<()> {
        // Merged into the project's .nail file rather than written over it,
        // because build timings live there too
        crate::utils::write_config_values(&[("theme", self.theme_name().to_string())]);
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
        return 9;
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
            // Leaving a keymap has to leave its half-finished state behind
            // too, or the next keymap starts inside a chord nobody typed.
            0 => {
                self.keymap = match (self.keymap, forward) {
                    (nail::keymap::Keymap::Cua, true) => nail::keymap::Keymap::Vim,
                    (nail::keymap::Keymap::Vim, true) => nail::keymap::Keymap::Emacs,
                    (nail::keymap::Keymap::Emacs, true) => nail::keymap::Keymap::Cua,
                    (nail::keymap::Keymap::Cua, false) => nail::keymap::Keymap::Emacs,
                    (nail::keymap::Keymap::Emacs, false) => nail::keymap::Keymap::Vim,
                    (nail::keymap::Keymap::Vim, false) => nail::keymap::Keymap::Cua,
                };
                self.vim_mode = nail::keymap::VimMode::Normal;
                self.pending_prefix = None;
                self.clear_mark();
            }
            1 => self.cycle_theme(forward),
            _ => {
                let value = match self.settings_row {
                    2 => &mut self.show_line_numbers,
                    3 => &mut self.highlight_current_line,
                    4 => &mut self.highlight_matching_brackets,
                    5 => &mut self.show_whitespace,
                    6 => &mut self.show_indentation_guides,
                    7 => &mut self.show_minimap,
                    8 => &mut self.show_timings,
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
        // Naming a keymap is not enough to pick one, so each says what it
        // means. Vim and emacs are known by their keys, while the default is
        // known by the editors that use it.
        let keys = match self.keymap {
            nail::keymap::Keymap::Cua => "normal (VS Code, Atom)".to_string(),
            nail::keymap::Keymap::Emacs => "emacs (ctrl+a, ctrl+k)".to_string(),
            nail::keymap::Keymap::Vim => "vim (hjkl, modal)".to_string(),
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
            ("Function timings".to_string(), flag_name(self.show_timings)),
        ];
    }

    fn theme_name(&self) -> &'static str {
        return crate::colorizer::theme_name_of(self.theme);
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
            ("timings", flag_name(self.show_timings)),
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
        self.get_current_tab_mut().settle_cursor();
        self.start_selection();
        self.move_to_line_end_with_selection(true);
        if self.has_selection() {
            self.delete_selected_text();
            return;
        }
        self.clear_selection();
        self.delete_forward();
    }

    /// The mirror of the one above, for vim's Ctrl+U in insert mode. Nothing
    /// is deleted at the start of a line, rather than the line before being
    /// pulled up, because a key that clears a line should not also join two.
    pub fn kill_to_line_start(&mut self) {
        self.get_current_tab_mut().settle_cursor();
        self.start_selection();
        self.move_to_line_start_with_selection(true);
        if self.has_selection() {
            self.delete_selected_text();
            return;
        }
        self.clear_selection();
    }

    /// Starts a line under this one and puts the cursor on it, indented to
    /// match, which is vim's `o`. The indentation comes from `insert_newline`
    /// rather than from here, so a line opened under a `{` opens indented.
    pub fn open_line_below(&mut self) {
        self.get_current_tab_mut().settle_cursor();
        self.move_to_line_end();
        self.insert_newline();
    }

    /// Vim's `O`, written as opening a line under the one above rather than
    /// above the one the cursor is on. That is the same line in the end, and
    /// this way it inherits the indentation of the line it follows instead of
    /// arriving at column zero.
    pub fn open_line_above(&mut self) {
        self.get_current_tab_mut().settle_cursor();
        // The first line of the file has nothing above it to follow, so the
        // blank line goes in directly. Splitting the line at column zero
        // instead would hand its indentation to both halves.
        if self.get_current_tab().cursor_y == 0 {
            self.flush_char_group();
            let tab = self.get_current_tab_mut();
            tab.content.insert(0, String::new());
            tab.cursor_x = 0;
            tab.record_operation(EditOperation::InsertText { position: (0, 0), text: "\n".to_string() });
            tab.modified = true;
            return;
        }
        self.move_cursor_up();
        self.move_to_line_end();
        self.insert_newline();
    }

    /// Vim's `x`. Deleting forward from the end of a line pulls the next one
    /// up, which is what the Delete key is for and never what `x` means, so
    /// the end of a line is where this stops.
    pub fn delete_char_at_cursor(&mut self) {
        self.get_current_tab_mut().settle_cursor();
        let tab = self.get_current_tab();
        let line_length = match tab.content.get(tab.cursor_y) {
            Some(line) => line.chars().count(),
            None => return,
        };
        if tab.cursor_x >= line_length {
            return;
        }
        self.delete_forward();
    }

    /// Empties the line and leaves the cursor where the typing goes, which is
    /// vim's `cc`. The indentation stays, because a line being rewritten is
    /// nearly always being rewritten at the same depth.
    pub fn change_line(&mut self) {
        self.get_current_tab_mut().settle_cursor();
        let tab = self.get_current_tab_mut();
        let old_line = match tab.content.get(tab.cursor_y) {
            Some(line) => line.clone(),
            None => return,
        };
        let indent: String = old_line.chars().take_while(|c| c.is_whitespace()).collect();
        tab.cursor_x = indent.chars().count();
        if old_line == indent {
            return;
        }
        let position = (0, tab.cursor_y);
        tab.content[tab.cursor_y] = indent.clone();
        tab.record_operation(EditOperation::ReplaceText { position, old_text: old_line, new_text: indent });
        tab.modified = true;
    }

    /// A whole line is yanked with its newline still attached, because that is
    /// what tells the paste it is a line rather than a fragment: it lands on a
    /// line of its own instead of in the middle of the one the cursor is on.
    pub fn yank_line(&mut self) {
        let tab = self.get_current_tab();
        let line = match tab.content.get(tab.cursor_y) {
            Some(line) => line.clone(),
            None => return,
        };
        self.set_clipboard(&(line + "\n"));
    }

    /// A line selection is yanked with a newline on the end, the same way a
    /// whole line is, so that pasting it lays the lines back down rather than
    /// dropping them into the middle of another one.
    pub fn yank_selection_as_lines(&mut self) {
        let text = self.get_selected_text();
        if text.is_empty() {
            return;
        }
        self.set_clipboard(&(text + "\n"));
    }

    /// Neovim's `Y`: the rest of the line from the cursor, not the whole line.
    pub fn yank_to_line_end(&mut self) {
        self.yank_over(|editor| editor.move_to_line_end_with_selection(true));
    }

    /// Vim's `yw`. The cursor is put back where it started, because yanking is
    /// the one operator that leaves the file and the cursor alone.
    pub fn yank_word(&mut self) {
        self.yank_over(|editor| editor.move_cursor_right_word_with_selection(true));
    }

    /// Copies whatever the given motion covers and then undoes the motion, so
    /// that a yank leaves the file, the cursor and the selection as it found
    /// them. Every yank that takes a motion is this with a different one.
    fn yank_over(&mut self, motion: impl Fn(&mut Editor)) {
        let start = self.cursor_position();
        self.start_selection();
        motion(self);
        let text = self.get_selected_text();
        self.clear_selection();
        let tab = self.get_current_tab_mut();
        tab.cursor_x = start.0;
        tab.cursor_y = start.1;
        if !text.is_empty() {
            self.set_clipboard(&text);
        }
    }

    /// The one clipboard, made on first use and kept until the editor exits.
    /// A failure to make one is not cached, so a display that comes back is
    /// found on the next copy.
    fn clipboard(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        self.clipboard.as_mut()
    }

    fn set_clipboard(&mut self, text: &str) {
        // A machine with no clipboard is a machine where yanking quietly does
        // nothing, which is the same answer copying already gives there.
        if let Some(clipboard) = self.clipboard() {
            let _ = clipboard.set_text(text.to_string());
        }
    }

    fn get_clipboard(&mut self) -> Option<String> {
        return self.clipboard().and_then(|clipboard| clipboard.get_text().ok());
    }

    /// Vim's `p` and `P`. Whether the paste lands on a line of its own or
    /// inside the current one is decided by the trailing newline, which is the
    /// same rule vim uses to tell a yanked line from a yanked word.
    pub fn paste_around_cursor(&mut self, after: bool) {
        let text = match self.get_clipboard() {
            Some(text) if !text.is_empty() => text,
            _ => return,
        };

        if !text.ends_with('\n') {
            if after {
                let tab = self.get_current_tab();
                let line_length = tab.content.get(tab.cursor_y).map(|line| line.chars().count()).unwrap_or(0);
                if tab.cursor_x < line_length {
                    self.move_cursor_right();
                }
            }
            self.paste_text(&text);
            return;
        }

        let (line, last_line) = {
            let tab = self.get_current_tab();
            (tab.cursor_y, tab.content.len().saturating_sub(1))
        };
        // Pasting after the last line has no line to push down, so the block
        // is added to the end instead of being inserted before something.
        if after && line >= last_line {
            self.move_to_line_end();
            let body = text.trim_end_matches('\n').to_string();
            self.paste_text(&format!("\n{}", body));
        } else {
            let target = if after { line + 1 } else { line };
            {
                let tab = self.get_current_tab_mut();
                tab.cursor_y = target;
                tab.cursor_x = 0;
            }
            self.paste_text(&text);
        }
        // The cursor belongs on the first line of what was pasted, not after
        // the last of it, which is where inserting the text leaves it.
        let tab = self.get_current_tab_mut();
        tab.cursor_y = (line + if after { 1 } else { 0 }).min(tab.content.len().saturating_sub(1));
        tab.cursor_x = 0;
    }

    /// Grows the selection out to whole lines, which is the whole difference
    /// between visual line mode and visual mode. It runs after every key while
    /// that mode is on, because a motion moves one end of the selection and
    /// this is what puts that end back on a line boundary.
    pub fn snap_selection_to_lines(&mut self) {
        let tab = self.get_current_tab_mut();
        let (start, end) = match (tab.selection_start, tab.selection_end) {
            (Some(start), Some(end)) => (start, end),
            _ => return,
        };
        let width = |content: &[String], line: usize| content.get(line).map(|line| columns_in(line)).unwrap_or(0);
        // The anchor is whichever end the selection started from, so which end
        // gets the start of a line and which gets the end of one depends on
        // which way the selection is facing.
        if start.1 <= end.1 {
            tab.selection_start = Some((0, start.1));
            tab.selection_end = Some((width(&tab.content, end.1), end.1));
        } else {
            tab.selection_start = Some((width(&tab.content, start.1), start.1));
            tab.selection_end = Some((0, end.1));
        }
        tab.selection_mode = true;
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
        // The disk now says what the buffer says, so the watcher must not
        // read this save back as someone else's change.
        current_tab.record_disk_mtime();
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

/// Lays each annotation at the end of its own line, two spaces in from the
/// code, which is where the IDE paints it. `first_line` is the 1-based file
/// line the first row of `code` sits on, which is how a copied selection
/// from the middle of a file still lines up with its annotations.
fn weave_annotations(code: &str, first_line: usize, annotations: &std::collections::BTreeMap<usize, String>) -> String {
    return code
        .split('\n')
        .enumerate()
        .map(|(offset, line)| match annotations.get(&(first_line + offset)) {
            Some(annotation) => format!("{}  {}", line, annotation),
            None => line.to_string(),
        })
        .collect::<Vec<String>>()
        .join("\n");
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
        // Hand the terminal back before anything else. A crash used to leave
        // the shell in raw mode on the alternate screen, and now that we also
        // ask for the keyboard protocol it would leave the shell reading
        // escape sequences for every chord the user pressed afterwards.
        restore_terminal();
        let backtrace = Backtrace::capture();
        error!("Panic occurred: {:?}", panic_info);
        error!("Backtrace:\n{:?}", backtrace);
    }));

    let (tx_resize, rx_resize) = channel::<EditorMessage>();
    let (tx_key, rx_key) = channel::<EditorMessage>();
    let (tx_draw, rx_draw) = channel::<EditorMessage>();
    let (tx_build, rx_build) = channel::<EditorMessage>();
    let (tx_lex, rx_lex) = channel::<EditorMessage>();
    // The senders stay bound so the watchers' channels live until main exits
    let (_tx_profile, rx_profile) = channel::<EditorMessage>();
    let (_tx_file_watch, rx_file_watch) = channel::<EditorMessage>();

    // Set up terminal. Mouse reporting is asked for here and can be handed
    // back at any time with F4, because while we hold it the terminal's own
    // click to select stops working.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Ask the terminal for the newer keyboard protocol, which is the only way
    // a Ctrl+Shift chord ever arrives as one. Without it Ctrl+Shift+P is
    // indistinguishable from Ctrl+P on the wire, and half the toggles in the
    // keymap are keys nobody can press. The question is one round trip and it
    // has to be asked before the key thread starts reading events, because
    // the answer comes back as one. A terminal that does not know the
    // protocol answers the device attributes query instead, which is a no,
    // and everything keeps working on its plain chords.
    let enhanced_keys = matches!(supports_keyboard_enhancement(), Ok(true));
    if enhanced_keys {
        execute!(stdout, PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES))?;
        log::info!("Terminal speaks the progressive keyboard protocol, so Ctrl+Shift chords are readable");
    } else {
        log::info!("Terminal has no progressive keyboard protocol, so Ctrl+Shift chords arrive as plain Ctrl");
    }
    ENHANCED_KEYS.store(enhanced_keys, Ordering::Relaxed);

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

    // Launch the lexer and parser thread. Both this and the build thread
    // below run the compiler, and reading a program is recursive at every
    // stage, so they are given the compiler's stack rather than the two
    // megabytes a thread gets by default. Without it a deeply nested
    // expression takes the whole editor down with it.
    thread::Builder::new()
        .name("nail-lex".to_string())
        .stack_size(nail::common::COMPILER_STACK_BYTES)
        .spawn(move || {
            lex_and_parse_thread_logic(editor_for_lex, rx_lex);
        })
        .expect("the lexer thread starts");

    // Launch the build thread
    let tx_draw_for_build = tx_draw.clone();
    thread::Builder::new()
        .name("nail-build".to_string())
        .stack_size(nail::common::COMPILER_STACK_BYTES)
        .spawn(move || {
            build_thread_logic(editor_for_build, rx_build, tx_draw_for_build);
        })
        .expect("the build thread starts");

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

    // Launch the file watcher thread, which reloads a tab when something
    // else writes its file, so edits made outside the IDE show up as they
    // happen
    let editor_for_file_watch = Arc::clone(&shared_editor);
    thread::spawn(move || {
        file_watcher_thread_logic(editor_for_file_watch, rx_file_watch);
    });

    // Main draw thread (this runs on the main thread)
    draw_thread_logic(terminal.clone(), editor_for_draw, rx_draw);
    
    // Clean up terminal on exit
    restore_terminal();

    Ok(())
}

/// Whether the terminal agreed to the progressive keyboard protocol, so that
/// the teardown knows whether there is a stack entry to pop. It is a static
/// because the panic hook has to be able to read it, and a hook cannot borrow
/// anything from main.
static ENHANCED_KEYS: AtomicBool = AtomicBool::new(false);

/// Puts the terminal back the way it was found: out of the keyboard protocol,
/// out of mouse reporting, off the alternate screen, out of raw mode. Called
/// on the way out and again from the panic hook, and safe to call twice,
/// because a terminal that is already back where it started ignores being
/// told so a second time.
fn restore_terminal() {
    if ENHANCED_KEYS.swap(false, Ordering::Relaxed) {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    // Mouse reporting is switched off before anything else, and then the
    // input queue is read dry. The key thread stopped reading the moment quit
    // was decided, but the terminal keeps reporting every mouse movement
    // until it processes this switch-off, and anything it reports in between
    // sits unread in the input buffer. Whatever is left there when this
    // process exits, the shell reads as typed keys, which showed up as
    // 35;60;5M spray at the prompt. The drain stops at the first quiet gap,
    // with a deadline so a hostile stream of input cannot hold exit hostage.
    let _ = execute!(io::stdout(), DisableMouseCapture);
    let deadline = Instant::now() + std::time::Duration::from_millis(500);
    while matches!(event::poll(std::time::Duration::from_millis(25)), Ok(true)) {
        let _ = event::read();
        if Instant::now() > deadline {
            break;
        }
    }
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

#[derive(Debug, Clone)]
enum CompletionContext {
    None,
    Identifier(String),        // Typing an identifier, show matching functions/variables
    FunctionCall(String),       // Inside function call, show parameter hints
}

/// What a key asked the completion list to do, held until the typing pauses.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CompletionRequest {
    /// A character was typed, so a list may be owed.
    Open,
    /// Something was deleted, which narrows a list that is open and never
    /// opens one.
    Narrow,
}

// Anything unreadable or unrecognized falls back to dark, so a hand-edited
// config can never leave the editor without colors
fn stored_theme() -> &'static ColorScheme {
    return match crate::utils::read_config_value("theme").as_deref() {
        Some(name) => crate::colorizer::theme_by_name(name).unwrap_or(&DARK_THEME),
        None => &DARK_THEME,
    };
}

/// The keymap the user picked, or None if they have never been asked. A value
/// nobody recognizes counts as never asked, so a hand-edited config puts the
/// question back rather than quietly picking an answer.
fn stored_keymap() -> Option<nail::keymap::Keymap> {
    return match crate::utils::read_config_value("keymap").as_deref() {
        Some("normal") | Some("cua") => Some(nail::keymap::Keymap::Cua),
        Some("vim") => Some(nail::keymap::Keymap::Vim),
        Some("emacs") => Some(nail::keymap::Keymap::Emacs),
        _ => None,
    };
}

/// How a keymap is spelled in the config file.
fn keymap_name(keymap: nail::keymap::Keymap) -> &'static str {
    return match keymap {
        nail::keymap::Keymap::Cua => "normal",
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
        if current_tab.cursor_x > columns_in(line) {
            return CompletionContext::None;
        }
        
        // Look for tokens around cursor position
        let cursor_line = current_tab.cursor_y + 1; // Lines are 1-indexed in CodeSpan
        let cursor_col = current_tab.cursor_x + 1;  // Columns are 1-indexed in CodeSpan
        
        // Check if we're inside a function call by looking for opening parenthesis
        let mut paren_depth = 0;
        let mut in_function_call = false;
        // Borrowed rather than copied: this walks every token in the file
        // ahead of the cursor, and copying each name out cost a fresh string
        // per identifier per keystroke.
        let mut function_name: &str = "";

        for token in &current_tab.tokens {
            // Check if token is before cursor
            if token.code_span.end_line < cursor_line || 
               (token.code_span.end_line == cursor_line && token.code_span.end_column <= cursor_col) {
                match &token.token_type {
                    lexer::TokenType::Identifier(name) => {
                        // Store potential function name
                        function_name = name;
                    }
                    lexer::TokenType::ParenthesisOpen => {
                        paren_depth += 1;
                        in_function_call = true;
                    }
                    lexer::TokenType::ParenthesisClose => {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            in_function_call = false;
                            function_name = "";
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
            return CompletionContext::FunctionCall(function_name.to_string());
        }
        
        // Check if we're typing an identifier
        let typed = self.completion_prefix_at_cursor();
        if !typed.is_empty() {
            return CompletionContext::Identifier(typed);
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
    
    /// How much of a word has been typed at the cursor. Only what is to the
    /// left of it counts: the rest of the word is already written, and reading
    /// it as a prefix made the list offer to finish a name the cursor had
    /// merely landed in front of.
    fn completion_prefix_at_cursor(&self) -> String {
        let current_tab = self.get_current_tab();
        if current_tab.cursor_y >= current_tab.content.len() {
            return String::new();
        }

        let characters: Vec<char> = current_tab.content[current_tab.cursor_y].chars().collect();
        if current_tab.cursor_x > characters.len() {
            return String::new();
        }

        let mut start = current_tab.cursor_x;
        while start > 0 && (characters[start - 1].is_alphanumeric() || characters[start - 1] == '_') {
            start -= 1;
        }

        return characters[start..current_tab.cursor_x].iter().collect();
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
                
                // Get stdlib functions. The registry is asked for the names a
                // prefix reaches rather than read from end to end, so what
                // this costs is the length of the answer.
                let mut completions = Vec::new();

                for (name, func) in crate::stdlib_registry::functions_starting_with(&prefix) {
                    let signature = func.nail_signature(name);

                    completions.push(CompletionItem {
                        label: name.to_string(),
                        detail: signature,
                        description: func.description.to_string(),
                        example: func.example.to_string(),
                        kind: CompletionKind::Function,
                    });
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
                        .map(|p| format!("{}:{}", p.name, p.param_type))
                        .collect();
                    
                    let hint = CompletionItem {
                        label: format!("{}({})", func_name, params.join(", ")),
                        detail: format!("Returns: {}", func.return_type),
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
    
    /// A deletion narrows a list that is already open and never opens one.
    /// Backspace is how a line that was pushed down gets pulled back up, and
    /// that should leave the screen as it was rather than start suggesting
    /// names for a word nobody is typing.
    fn refresh_open_completions(&mut self) {
        if self.show_completions {
            self.update_completions();
        }
    }

    /// A character was typed. Building the list reads the standard library
    /// and the symbols in scope, which is work the person typing should not
    /// wait on, so it is noted here and done once the keys stop coming. The
    /// key loop carries the clock: see `COMPLETION_DELAY`.
    fn request_completions(&mut self) {
        self.completion_request = Some(CompletionRequest::Open);
    }

    /// The same, for a deletion, which only ever narrows a list that is open.
    fn request_completion_refresh(&mut self) {
        self.completion_request = Some(CompletionRequest::Narrow);
    }

    /// Builds whatever list was asked for, if one was. Anything that reads the
    /// list rather than typing into it calls this first, so that the keys that
    /// pick from it are never a moment behind the word they belong to.
    fn flush_completion_request(&mut self) {
        match self.completion_request.take() {
            Some(CompletionRequest::Open) => self.update_completions(),
            Some(CompletionRequest::Narrow) => self.refresh_open_completions(),
            None => {}
        }
    }

    /// Drops a list that was asked for and never shown, which is what Escape
    /// means when the request is still in flight.
    fn cancel_completion_request(&mut self) {
        self.completion_request = None;
    }

    fn accept_completion(&mut self) {
        self.accept_completion_with(ExampleForm::Example);
    }

    /// Shift with the same key asks for the whole program rather than the one
    /// line that calls the function.
    fn accept_completion_full(&mut self) {
        self.accept_completion_with(ExampleForm::FullExample);
    }

    fn accept_completion_with(&mut self, form: ExampleForm) {
        // What goes in is whatever the list is offering, so the list has to be
        // the one for the word actually typed rather than the one from a
        // keystroke ago.
        self.flush_completion_request();
        if !self.show_completions || self.completions.is_empty() {
            return;
        }

        let completion = &self.completions[self.completion_index];

        // Only complete if it's an identifier completion
        if let CompletionContext::Identifier(_) = self.get_completion_context() {
            // Generate insertion text based on completion kind (before any mutable borrows)
            let insertion_text = self.generate_insertion_text(&completion, form);
            let land_inside_parentheses = insertion_text.ends_with("()") && !insertion_text.contains('\n');

            // Asking for the full example while the call is already half
            // written wants its setup, not a second copy of the statement.
            // The declarations go in above and the call is finished in place.
            let split_around_the_statement = if form == ExampleForm::FullExample && insertion_text.contains('\n') {
                crate::stdlib_registry::get_stdlib_function(&completion.label).filter(|func| !func.example.is_empty()).map(|func| {
                    (
                        crate::stdlib_registry::example_setup(&completion.label, func.example).to_string(),
                        crate::stdlib_registry::example_snippet(&completion.label, func.example).to_string(),
                    )
                })
            } else {
                None
            };

            // Take out the half-typed word first, so that what goes in is
            // simply what was generated, however many lines that turns out
            // to be.
            let tab = self.get_current_tab_mut();
            if tab.cursor_y >= tab.content.len() {
                tab.content.push(String::new());
                tab.cursor_y = tab.content.len() - 1;
                tab.cursor_x = 0;
            }
            let characters: Vec<char> = tab.content[tab.cursor_y].chars().collect();
            let mut start = tab.cursor_x.min(characters.len());
            while start > 0 && (characters[start - 1].is_alphanumeric() || characters[start - 1] == '_') {
                start -= 1;
            }
            let mut end = tab.cursor_x.min(characters.len());
            while end < characters.len() && (characters[end].is_alphanumeric() || characters[end] == '_') {
                end += 1;
            }
            tab.content[tab.cursor_y] = characters[..start].iter().chain(characters[end..].iter()).collect();
            tab.cursor_x = start;

            let starts_the_line = characters[..start].iter().all(|character| character.is_whitespace());
            match split_around_the_statement {
                Some((setup, call)) if !starts_the_line && !setup.is_empty() => {
                    let tab = self.get_current_tab_mut();
                    for (offset, line) in setup.lines().enumerate() {
                        tab.content.insert(tab.cursor_y + offset, line.to_string());
                    }
                    tab.cursor_y += setup.lines().count();
                    self.insert_lines_at_cursor(&call);
                }
                _ => self.insert_lines_at_cursor(&insertion_text),
            }

            if land_inside_parentheses {
                let tab = self.get_current_tab_mut();
                tab.cursor_x = tab.cursor_x.saturating_sub(1);
            }
        }

        self.show_completions = false;
        self.show_detail_view = false;
        self.completions.clear();
    }
    
    fn next_completion(&mut self) {
        self.flush_completion_request();
        if !self.completions.is_empty() {
            self.completion_index = (self.completion_index + 1) % self.completions.len();
        }
    }

    fn previous_completion(&mut self) {
        self.flush_completion_request();
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

    /// What a completion puts in the file.
    ///
    /// Two answers, because there are two things a person can be short of.
    /// Tab gives the call with real arguments in it, `array_sum(numbers)`,
    /// which drops into a half-written statement and shows the argument
    /// shape an empty pair of parentheses never did. Shift+Tab gives the
    /// whole worked program: the inputs declared, any handed-over function
    /// defined, the result named.
    ///
    /// Both mean the same thing wherever the completion list is showing, so
    /// neither key changes meaning depending on whether the documentation is
    /// open. Both come from the one curated example, so they cannot disagree.
    ///
    /// Guessing either shape from the signature is what this used to do, and
    /// a signature does not know what a `fn` parameter or a type variable
    /// looks like written down, so the guess taught a syntax the compiler
    /// refuses.
    fn generate_insertion_text(&self, completion: &CompletionItem, form: ExampleForm) -> String {
        match completion.kind {
            CompletionKind::Function => {
                use crate::stdlib_registry::{example_snippet, get_stdlib_function};
                if let Some(func) = get_stdlib_function(&completion.label) {
                    if !func.example.is_empty() {
                        return match form {
                            ExampleForm::Example => example_snippet(&completion.label, func.example).to_string(),
                            ExampleForm::FullExample => func.example.to_string(),
                        };
                    }
                }
                format!("{}()", completion.label)
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
                let start_x = byte_of_column(line, start_pos.0);
                let end_x = byte_of_column(line, end_pos.0).max(start_x);
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
                    result.push_str(&line[byte_of_column(line, start_pos.0)..]);
                } else if line_idx == end_pos.1 {
                    // Last line - from beginning to end_x
                    result.push_str(&line[..byte_of_column(line, end_pos.0)]);
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
                let start_x = byte_of_column(line, start_pos.0);
                let end_x = byte_of_column(line, end_pos.0).max(start_x);
                line.drain(start_x..end_x);
                current_tab.cursor_x = start_pos.0;
                current_tab.cursor_y = start_pos.1;
            }
        } else {
            // Multi-line deletion
            if start_pos.1 < current_tab.content.len() && end_pos.1 < current_tab.content.len() {
                // Get the remaining parts of first and last lines
                let first_line = &current_tab.content[start_pos.1];
                let first_line_start = first_line[..byte_of_column(first_line, start_pos.0)].to_string();
                let last_line = &current_tab.content[end_pos.1];
                let last_line_end = last_line[byte_of_column(last_line, end_pos.0)..].to_string();
                
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
        let last_line_len = columns_in(&current_tab.content[last_line_idx]);
        current_tab.selection_end = Some((last_line_len, last_line_idx));
        current_tab.selection_mode = true;
    }
    
    fn copy_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_text = self.get_selected_text();
        if !selected_text.is_empty() {
            match self.clipboard() {
                Some(clipboard) => clipboard.set_text(selected_text)?,
                None => return Err("no clipboard is available".into()),
            }
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
        let text = match self.clipboard() {
            Some(clipboard) => clipboard.get_text()?,
            None => return Err("no clipboard is available".into()),
        };

        self.paste_text(&text);

        Ok(())
    }
    
    fn paste_text(&mut self, text: &str) {
        self.get_current_tab_mut().settle_cursor();
        // A selection pasted over belongs to the same edit as the paste, so
        // what it held is carried to the undo entry below.
        let removed = self.take_selection_for_replacement();

        if text.is_empty() {
            // Nothing arrives to record the removal alongside, so it stands as
            // its own entry rather than being lost.
            if let Some(removal) = removed {
                self.get_current_tab_mut().record_operation(removal);
            }
            return;
        }

        let current_tab = self.get_current_tab_mut();
        let position = (current_tab.cursor_x, current_tab.cursor_y);
        // A tab arrives as four spaces and a control character not at all, so
        // the entry holds what actually went into the file. Undo counts the
        // characters it wrote, and would leave three spaces behind for every
        // tab if it counted the ones that were pasted instead.
        let mut inserted = String::new();

        // Insert the text character by character
        for c in text.chars() {
            if c == '\n' {
                let split = byte_of_column(&current_tab.content[current_tab.cursor_y], current_tab.cursor_x);
                let remaining = current_tab.content[current_tab.cursor_y].split_off(split);
                current_tab.cursor_y += 1;
                current_tab.content.insert(current_tab.cursor_y, remaining);
                current_tab.cursor_x = 0;
                inserted.push('\n');
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
                    inserted.push(' ');
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
                inserted.push(c);
            }
        }

        current_tab.modified = true;
        self.record_replacing_selection(removed, EditOperation::InsertText { position, text: inserted });
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
                            Some((field_name.clone(), data_type.to_string()))
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
                    data_type: Some(data_type.to_string()),
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
            // The unrecorded delete is what this wants: the one entry recorded
            // below covers the whole swap.
            tab.delete_selected_text_unrecorded();
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
        self.get_current_tab_mut().settle_cursor();
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

    /// Every message the overlay is showing, grouped per line the way the
    /// overlay groups them, each followed by the line of code it points at so
    /// the copy stands on its own when pasted somewhere else. Notices with no
    /// line in the file are transient and stay out of it.
    fn errors_as_text(&self) -> String {
        use std::collections::BTreeMap;
        let mut messages_by_line: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
        for error in &self.code_errors {
            if error.code_span.start_line > 0 {
                messages_by_line.entry(error.code_span.start_line).or_default().push(error.message.as_str());
            }
        }
        let content = &self.get_current_tab().content;
        let mut blocks: Vec<String> = Vec::new();
        for (line_number, messages) in messages_by_line {
            let mut block = format!("line {}: {}", line_number, messages.join(" | "));
            if let Some(code) = content.get(line_number - 1) {
                if !code.trim().is_empty() {
                    block.push_str("\n    ");
                    block.push_str(code.trim());
                }
            }
            blocks.push(block);
        }
        return blocks.join("\n");
    }

    /// The error overlay is drawn over the buffer rather than stored in it,
    /// so no drag can ever select it. This is how the messages get out: the
    /// whole list lands on the clipboard as plain text.
    fn copy_errors(&mut self) {
        let text = self.errors_as_text();
        if text.is_empty() {
            self.push_copy_receipt("No errors to copy".to_string());
            return;
        }
        let count = self.code_errors.iter().filter(|error| error.code_span.start_line > 0).count();
        self.set_clipboard(&text);
        let noun = if count == 1 { "error" } else { "errors" };
        self.push_copy_receipt(format!("Copied {} {}", count, noun));
    }

    /// Whatever is on screen is copyable as plain text, and this is the ask.
    /// The keyboard cannot answer it alone, because overlays, popups and
    /// dialogs exist only in the painted frame, so the draw thread picks the
    /// request up and answers with the frame it just finished.
    fn request_screen_copy(&mut self) {
        self.screen_copy_requested = true;
    }

    /// The draw thread's side of the handshake: the finished frame as plain
    /// text lands on the clipboard, and the receipt replaces any earlier
    /// copy receipt rather than stacking under it.
    pub fn finish_screen_copy(&mut self, text: &str) {
        self.screen_copy_requested = false;
        self.set_clipboard(text);
        self.push_copy_receipt("Copied the screen as text".to_string());
    }

    /// The selected code with every annotation woven onto the end of its
    /// own line, exactly the way the IDE paints them, plus how many came
    /// along. No selection means the current line, because the annotation
    /// under the cursor is the one being asked about. An annotation whose
    /// display is turned off stays out, because what is copied is what is
    /// displayed.
    fn selection_with_annotations_text(&self) -> (String, usize) {
        let selection = self.get_selected_text();
        let tab = self.get_current_tab();
        let (first_line, code) = if selection.is_empty() {
            (tab.cursor_y, tab.content.get(tab.cursor_y).cloned().unwrap_or_default())
        } else {
            match (tab.selection_start, tab.selection_end) {
                (Some((_, start_y)), Some((_, end_y))) => (start_y.min(end_y), selection),
                _ => (tab.cursor_y, selection),
            }
        };
        let line_count = code.split('\n').count();
        let annotations = crate::utils::line_annotation_texts(self, first_line + 1, first_line + line_count);
        return (weave_annotations(&code, first_line + 1, &annotations), annotations.len());
    }

    /// The debugging copy: code and what the IDE says about it, together,
    /// ready to paste into a bug report or a conversation.
    fn copy_selection_with_annotations(&mut self) {
        let (text, count) = self.selection_with_annotations_text();
        if text.is_empty() {
            self.push_copy_receipt("Nothing to copy".to_string());
            return;
        }
        self.set_clipboard(&text);
        let noun = if count == 1 { "annotation" } else { "annotations" };
        self.push_copy_receipt(format!("Copied the selection with {} {}", count, noun));
    }

    /// The whole buffer as plain text, exactly as saving would write it.
    fn copy_file_text(&mut self) {
        let text = self.get_current_tab().content.join("\n");
        self.set_clipboard(&text);
        self.push_copy_receipt("Copied the file".to_string());
    }

    /// The whole buffer with every annotation woven onto the end of its
    /// own line, errors and timings alike: the largest of the debugging
    /// copies.
    fn copy_file_with_annotations(&mut self) {
        let tab = self.get_current_tab();
        let text = tab.content.join("\n");
        let last_line = tab.content.len();
        let annotations = crate::utils::line_annotation_texts(self, 1, last_line);
        let count = annotations.len();
        if count == 0 {
            self.set_clipboard(&text);
            self.push_copy_receipt("Copied the file, no annotations on it".to_string());
            return;
        }
        self.set_clipboard(&weave_annotations(&text, 1, &annotations));
        let noun = if count == 1 { "annotation" } else { "annotations" };
        self.push_copy_receipt(format!("Copied the file with {} {}", count, noun));
    }

    /// One receipt row for every copy command: the newest replaces whatever
    /// receipt was there before rather than stacking under it.
    fn push_copy_receipt(&mut self, message: String) {
        self.code_errors.retain(|error| !(error.code_span.start_line == 0 && (error.message.starts_with("Copied ") || error.message == "No errors to copy" || error.message == "Nothing to copy")));
        self.code_errors.push(CodeError::from(message));
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
        // One task per file: each file is read and scanned independently, and
        // the reads are what the time goes to. The per-file lists are
        // flattened in the walk's order, so the result is the same as the
        // sequential loop gave.
        let files = crate::utils::scan_project_files(std::path::Path::new(&self.project_root));
        let per_file: Vec<Vec<FileSymbol>> = files
            .par_iter()
            .map(|relative| {
                if !relative.ends_with(".nail") {
                    return Vec::new();
                }
                let lines = match self.project_lines(relative) {
                    Some(lines) => lines,
                    None => return Vec::new(),
                };
                lines
                    .iter()
                    .enumerate()
                    .filter_map(|(index, line)| declaration_label(line).map(|label| FileSymbol { label, line: index + 1, file: Some(relative.clone()) }))
                    .collect()
            })
            .collect();
        return per_file.into_iter().flatten().collect();
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
        // One task per file, since the lowercasing of every line is where the
        // time goes. Each file collects at most the overall limit, the
        // flatten below keeps the walk's order, and the final truncate gives
        // the same first two hundred hits the sequential loop found. Files
        // past the limit are still read, which is the price of the split, and
        // is smaller than the win of scanning them all at once.
        let files = crate::utils::scan_project_files(std::path::Path::new(&self.project_root));
        let per_file: Vec<Vec<FileSymbol>> = files
            .par_iter()
            .map(|relative| {
                let lines = match self.project_lines(relative) {
                    Some(lines) => lines,
                    None => return Vec::new(),
                };
                lines
                    .iter()
                    .enumerate()
                    .filter(|(_, line)| line.to_lowercase().contains(&needle))
                    // Long lines are cut rather than wrapped, because a result
                    // list is for choosing between places and not for reading
                    // the code that is about to be opened anyway.
                    .map(|(index, line)| FileSymbol { label: line.trim().chars().take(160).collect(), line: index + 1, file: Some(relative.clone()) })
                    .take(Self::PROJECT_SEARCH_LIMIT)
                    .collect()
            })
            .collect();
        self.symbol_entries = per_file.into_iter().flatten().take(Self::PROJECT_SEARCH_LIMIT).collect();
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
        // Scored in parallel on every keystroke, one task per symbol. The
        // collect keeps collection order, so the stable sort below holds.
        let mut scored: Vec<(i32, usize)> = self
            .symbol_entries
            .par_iter()
            .enumerate()
            .filter_map(|(index, symbol)| {
                // The name is what was asked for, so a match in it counts for
                // more than the same letters found in the path. A match in the
                // path still counts, because that is how one file's declarations
                // are picked out of the whole project's.
                let by_name = crate::utils::fuzzy_score(&symbol.label.to_lowercase(), &needle).map(|score| score + 40);
                let by_file = match &symbol.file {
                    Some(file) => crate::utils::fuzzy_score(&format!("{} {}", symbol.label, file).to_lowercase(), &needle),
                    None => None,
                };
                by_name.into_iter().chain(by_file).max().map(|score| (score, index))
            })
            .collect();
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
                if tab.disk_changed_underneath {
                    // The disk moved under unsaved edits, which is worth
                    // more alarm than a plain star.
                    title.push('!');
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

    /// Scrolls to the part of the file a minimap row stands for, landing it in
    /// the middle of the view. The cursor stays where it was, the same as the
    /// scroll keys: the minimap moves the view, not the text.
    fn minimap_jump(&mut self, row: u16) {
        let minimap = self.view.minimap;
        let visible = self.view.text.height as usize;
        let lines_per_row = minimap_lines_per_row(self.get_current_tab().content.len(), minimap.height);
        let tab = self.get_current_tab_mut();
        let furthest = tab.content.len().saturating_sub(1);
        let row_in_map = row.saturating_sub(minimap.y) as usize;
        let target = (row_in_map * lines_per_row + lines_per_row / 2).min(furthest);
        tab.scroll_position = target.saturating_sub(visible / 2).min(furthest) as u16;
    }

    fn mouse_press(&mut self, column: u16, row: u16) {
        if point_in_rect(column, row, self.view.tabs) {
            if let Some(index) = self.tab_at_column(column) {
                self.switch_to_tab(index);
            }
            return;
        }
        if point_in_rect(column, row, self.view.minimap) {
            self.minimap_dragging = true;
            self.minimap_jump(row);
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
        if self.minimap_dragging {
            let minimap = self.view.minimap;
            if minimap.height == 0 {
                return;
            }
            self.minimap_jump(row.clamp(minimap.y, minimap.y + minimap.height - 1));
            return;
        }
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
        self.minimap_dragging = false;
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
    // A cursor sitting past the end of the line is still asking about the word
    // at the end of it, not about a character the line does not have.
    let at = at.min(text.len());
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

    /// Every way of replacing a selection used to clear it through a delete
    /// that wrote nothing to the undo stack, so one keystroke over a selected
    /// block put that block out of reach of Ctrl+Z for good. Each of these
    /// replacements is one edit, and one undo takes the whole thing back.
    /// A closing brace on a line indented less than one level used to work out
    /// where to put the cursor by subtracting a level from it, which underflows
    /// and takes the editor with it.
    #[test]
    fn a_brace_on_a_shallow_line_dedents_what_there_is_of_it() {
        let mut editor = editor_with(&["  "]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_x = 2;
        }
        editor.insert_char('}');
        assert_eq!(editor.get_current_tab().content, vec!["  }"]);
        assert_eq!(editor.get_current_tab().cursor_x, 3);

        let mut editor = editor_with(&["        "]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_x = 8;
        }
        editor.insert_char('}');
        assert_eq!(editor.get_current_tab().content, vec!["    }"]);
        assert_eq!(editor.get_current_tab().cursor_x, 5);
    }

    /// The line commands, run over text that is not all ASCII. Each of these
    /// used to work on bytes somewhere along the way, and bracket matching at
    /// the end of such a line indexed off the end of the line and panicked.
    #[test]
    fn the_line_commands_all_survive_accented_text() {
        let accented = ["caf\u{e9} au lait", "na\u{ef}ve r\u{e9}sum\u{e9}", "plain line"];
        let at_end = |editor: &mut Editor, y: usize| {
            let tab = editor.get_current_tab_mut();
            tab.cursor_y = y;
            tab.cursor_x = tab.content[y].chars().count();
        };

        let mut editor = editor_with(&accented);
        at_end(&mut editor, 0);
        editor.update_bracket_matching();

        let mut editor = editor_with(&accented);
        at_end(&mut editor, 0);
        editor.toggle_comment();
        assert_eq!(editor.get_current_tab().content[0], "// caf\u{e9} au lait");

        let mut editor = editor_with(&accented);
        at_end(&mut editor, 1);
        editor.duplicate_line();
        assert_eq!(editor.get_current_tab().content[2], "na\u{ef}ve r\u{e9}sum\u{e9}");

        let mut editor = editor_with(&accented);
        at_end(&mut editor, 0);
        editor.delete_word_left();
        assert_eq!(editor.get_current_tab().content[0], "caf\u{e9} au ");

        let mut editor = editor_with(&accented);
        editor.get_current_tab_mut().cursor_x = 0;
        editor.move_cursor_right_word();
        let accented_stop = editor.get_current_tab().cursor_x;
        let mut editor = editor_with(&["cafe au lait"]);
        editor.get_current_tab_mut().cursor_x = 0;
        editor.move_cursor_right_word();
        assert_eq!(accented_stop, editor.get_current_tab().cursor_x);

        let mut editor = editor_with(&accented);
        at_end(&mut editor, 0);
        editor.kill_to_line_start();
        assert_eq!(editor.get_current_tab().content[0], "");

        let mut editor = editor_with(&accented);
        editor.get_current_tab_mut().cursor_x = 4;
        editor.kill_to_line_end();
        assert_eq!(editor.get_current_tab().content[0], "caf\u{e9}");

        let mut editor = editor_with(&accented);
        editor.join_lines();
        assert_eq!(editor.get_current_tab().content[0], "caf\u{e9} au lait na\u{ef}ve r\u{e9}sum\u{e9}");

        let mut editor = editor_with(&accented);
        select(&mut editor, (0, 0), (5, 1));
        editor.indent_selection();
        assert_eq!(editor.get_current_tab().content[0], "    caf\u{e9} au lait");
        editor.dedent_selection();
        assert_eq!(editor.get_current_tab().content[0], "caf\u{e9} au lait");

        let mut editor = editor_with(&accented);
        select(&mut editor, (0, 0), (4, 0));
        editor.insert_char('(');
        assert_eq!(editor.get_current_tab().content[0], "(caf\u{e9}) au lait");

        let mut editor = editor_with(&accented);
        at_end(&mut editor, 0);
        editor.expand_selection();
        editor.delete_line();
        editor.move_line_down();
        editor.sort_lines();
    }

    /// Every edit the editor can make, undone back to the file it started
    /// from and redone forward to the file it made. An entry that describes
    /// its edit loosely passes the eye and fails here: the line break used to
    /// record itself without the indentation it carried, so undoing it left
    /// the indentation behind in the middle of the joined line.
    #[test]
    fn every_edit_can_be_undone_and_redone_exactly() {
        let start = ["f main():v {", "    print(`one`);", "    print(`two`);", "}"];
        let commands: Vec<(&str, fn(&mut Editor))> = vec![
            ("insert_char", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 4; e.insert_char('x'); }),
            ("insert_newline", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 6; e.insert_newline(); }),
            ("delete_char", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 6; e.delete_char(); }),
            ("delete_char_join", |e| { e.get_current_tab_mut().cursor_y = 2; e.get_current_tab_mut().cursor_x = 0; e.delete_char(); }),
            ("delete_forward", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 4; e.delete_forward(); }),
            ("delete_forward_join", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 20; e.delete_forward(); }),
            ("paste_text", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 4; e.paste_text("a\nb"); }),
            ("duplicate_line", |e| { e.get_current_tab_mut().cursor_y = 1; e.duplicate_line(); }),
            ("delete_line", |e| { e.get_current_tab_mut().cursor_y = 1; e.delete_line(); }),
            ("move_line_up", |e| { e.get_current_tab_mut().cursor_y = 2; e.move_line_up(); }),
            ("move_line_down", |e| { e.get_current_tab_mut().cursor_y = 1; e.move_line_down(); }),
            ("toggle_comment", |e| { e.get_current_tab_mut().cursor_y = 1; e.toggle_comment(); }),
            ("indent_selection", |e| { select(e, (0, 1), (5, 2)); e.indent_selection(); }),
            ("dedent_selection", |e| { select(e, (0, 1), (5, 2)); e.dedent_selection(); }),
            ("join_lines", |e| { e.get_current_tab_mut().cursor_y = 1; e.join_lines(); }),
            ("sort_lines", |e| { select(e, (0, 0), (0, 2)); e.sort_lines(); }),
            ("delete_word_left", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 9; e.delete_word_left(); }),
            ("delete_word_right", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 4; e.delete_word_right(); }),
            ("kill_to_line_end", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 6; e.kill_to_line_end(); }),
            ("kill_to_line_start", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 6; e.kill_to_line_start(); }),
            ("delete_selection", |e| { select(e, (2, 1), (6, 2)); e.delete_selected_text(); }),
            ("type_over_selection", |e| { select(e, (2, 1), (6, 2)); e.insert_char('z'); }),
            ("paste_over_selection", |e| { select(e, (2, 1), (6, 2)); e.paste_text("QQ"); }),
            ("newline_over_selection", |e| { select(e, (2, 1), (6, 2)); e.insert_newline(); }),
            ("open_line_below", |e| { e.get_current_tab_mut().cursor_y = 1; e.open_line_below(); }),
            ("open_line_above", |e| { e.get_current_tab_mut().cursor_y = 1; e.open_line_above(); }),
            ("change_line", |e| { e.get_current_tab_mut().cursor_y = 1; e.change_line(); }),
            ("replace_current", |e| { e.search_query = "print".to_string(); e.find_all_matches(); e.replace_text = "log".to_string(); e.replace_current(); }),
            ("replace_all", |e| { e.search_query = "print".to_string(); e.find_all_matches(); e.replace_text = "log".to_string(); e.replace_all(); }),
            ("delete_char_at_cursor", |e| { e.get_current_tab_mut().cursor_y = 1; e.get_current_tab_mut().cursor_x = 4; e.delete_char_at_cursor(); }),
        ];

        let mut broken = Vec::new();
        for (name, command) in commands {
            let mut editor = editor_with(&start);
            command(&mut editor);
            let after: Vec<String> = editor.get_current_tab().content.clone();
            if after == start.iter().map(|l| l.to_string()).collect::<Vec<_>>() {
                broken.push(format!("{}: changed nothing", name));
                continue;
            }
            let mut guard = 0;
            while editor.undo() { guard += 1; if guard > 50 { break; } }
            let undone: Vec<String> = editor.get_current_tab().content.clone();
            if undone != start.iter().map(|l| l.to_string()).collect::<Vec<_>>() {
                broken.push(format!("{}: undo gave {:?}", name, undone));
                continue;
            }
            let mut guard = 0;
            while editor.redo() { guard += 1; if guard > 50 { break; } }
            let redone: Vec<String> = editor.get_current_tab().content.clone();
            if redone != after {
                broken.push(format!("{}: redo gave {:?}, wanted {:?}", name, redone, after));
            }
        }
        assert!(broken.is_empty(), "\n{}", broken.join("\n"));
    }

    /// Every command against the buffers that have no middle: empty, one
    /// line, the last column, and a cursor that has somehow ended up outside
    /// the file. None of them may panic, and none may leave no buffer at all.
    #[test]
    fn no_command_falls_off_the_end_of_a_buffer() {
        let commands: Vec<(&str, fn(&mut Editor))> = vec![
            ("insert_char", |e| e.insert_char('x')),
            ("insert_newline", |e| e.insert_newline()),
            ("delete_char", |e| e.delete_char()),
            ("delete_forward", |e| e.delete_forward()),
            ("delete_char_at_cursor", |e| e.delete_char_at_cursor()),
            ("paste_text", |e| e.paste_text("a\nb")),
            ("duplicate_line", |e| e.duplicate_line()),
            ("delete_line", |e| e.delete_line()),
            ("move_line_up", |e| e.move_line_up()),
            ("move_line_down", |e| e.move_line_down()),
            ("toggle_comment", |e| e.toggle_comment()),
            ("indent_selection", |e| e.indent_selection()),
            ("dedent_selection", |e| e.dedent_selection()),
            ("join_lines", |e| e.join_lines()),
            ("sort_lines", |e| e.sort_lines()),
            ("delete_word_left", |e| e.delete_word_left()),
            ("delete_word_right", |e| e.delete_word_right()),
            ("kill_to_line_end", |e| e.kill_to_line_end()),
            ("kill_to_line_start", |e| e.kill_to_line_start()),
            ("select_all", |e| e.select_all()),
            ("expand_selection", |e| e.expand_selection()),
            ("open_line_below", |e| e.open_line_below()),
            ("open_line_above", |e| e.open_line_above()),
            ("change_line", |e| e.change_line()),
            ("move_to_line_end", |e| e.move_to_line_end()),
            ("move_to_file_end", |e| e.move_to_file_end()),
            ("move_cursor_right", |e| e.move_cursor_right()),
            ("move_cursor_left", |e| e.move_cursor_left()),
            ("move_cursor_up", |e| e.move_cursor_up()),
            ("move_cursor_down", |e| e.move_cursor_down()),
            ("move_cursor_right_word", |e| e.move_cursor_right_word()),
            ("move_cursor_left_word", |e| e.move_cursor_left_word()),
            ("update_bracket_matching", |e| e.update_bracket_matching()),
            ("jump_to_matching_bracket", |e| e.jump_to_matching_bracket()),
            ("undo", |e| { e.undo(); }),
            ("redo", |e| { e.redo(); }),
            ("scroll_down", |e| e.scroll_down()),
            ("scroll_up", |e| e.scroll_up()),
        ];

        let buffers: Vec<(&str, Vec<&str>, (usize, usize))> = vec![
            ("empty file", vec![""], (0, 0)),
            ("one line, cursor at end", vec!["only"], (4, 0)),
            ("last line last column", vec!["a", "bb"], (2, 1)),
            ("cursor past the end of a line", vec!["a", "bb"], (9, 0)),
            ("blank lines", vec!["", "", ""], (0, 1)),
            ("one brace", vec!["{"], (0, 0)),
            ("cursor past the last line", vec!["a", "b"], (0, 7)),
            ("cursor past every column", vec!["short"], (99, 0)),
        ];

        for (buffer_name, lines, cursor) in buffers {
            for (command_name, command) in commands.iter() {
                let mut editor = editor_with(&lines);
                {
                    let tab = editor.get_current_tab_mut();
                    tab.cursor_x = cursor.0;
                    tab.cursor_y = cursor.1;
                }
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| command(&mut editor)));
                assert!(outcome.is_ok(), "{} panicked on {}", command_name, buffer_name);
                assert!(!editor.get_current_tab().content.is_empty(), "{} emptied the buffer on {}", command_name, buffer_name);
            }
        }
    }

    /// Finding and replacing keeps a list of positions and then edits the
    /// lines underneath it, so the positions have to be in the same units as
    /// everything else. They were byte offsets, which put the highlight and
    /// the cursor on the wrong characters once a line held an accent.
    #[test]
    fn finding_and_replacing_keeps_its_positions_in_columns() {
        let mut broken: Vec<String> = Vec::new();

        // Replace every match on one line, longer replacement than match
        let mut editor = editor_with(&["one one one", "one"]);
        editor.search_query = "one".to_string();
        editor.find_all_matches();
        editor.replace_text = "three".to_string();
        editor.replace_all();
        if editor.get_current_tab().content != vec!["three three three", "three"] {
            broken.push(format!("replace_all: {:?}", editor.get_current_tab().content));
        }
        while editor.undo() {}
        if editor.get_current_tab().content != vec!["one one one", "one"] {
            broken.push(format!("replace_all undo: {:?}", editor.get_current_tab().content));
        }

        // One at a time, with the remaining matches shifting under each one
        let mut editor = editor_with(&["one one one"]);
        editor.search_query = "one".to_string();
        editor.find_all_matches();
        editor.replace_text = "seven".to_string();
        editor.replace_current();
        editor.replace_current();
        editor.replace_current();
        if editor.get_current_tab().content != vec!["seven seven seven"] {
            broken.push(format!("replace_current thrice: {:?}", editor.get_current_tab().content));
        }

        // Shorter replacement, so later matches move left
        let mut editor = editor_with(&["alpha alpha alpha"]);
        editor.search_query = "alpha".to_string();
        editor.find_all_matches();
        editor.replace_text = "hi".to_string();
        editor.replace_current();
        editor.replace_current();
        if editor.get_current_tab().content != vec!["hi hi alpha"] {
            broken.push(format!("shorter replacement: {:?}", editor.get_current_tab().content));
        }

        // Walking the matches wraps at both ends
        let mut editor = editor_with(&["a", "target", "b", "target"]);
        editor.search_query = "target".to_string();
        editor.find_all_matches();
        let first = editor.current_match_index;
        editor.find_next();
        editor.find_next();
        if editor.current_match_index != first {
            broken.push(format!("find_next did not wrap: {} then {}", first, editor.current_match_index));
        }

        // A search that matches nothing leaves the file and the cursor alone
        let mut editor = editor_with(&["nothing here"]);
        editor.search_query = "zzz".to_string();
        editor.find_all_matches();
        editor.replace_text = "x".to_string();
        editor.replace_all();
        if editor.get_current_tab().content != vec!["nothing here"] {
            broken.push(format!("empty search: {:?}", editor.get_current_tab().content));
        }

        // Matches on an accented line, one before the accent and one after
        let mut editor = editor_with(&["ab caf\u{e9} ab"]);
        editor.search_query = "ab".to_string();
        editor.find_all_matches();
        if editor.search_results != vec![(0, 0, 2), (0, 8, 10)] {
            broken.push(format!("accented columns: {:?}", editor.search_results));
        }
        editor.replace_text = "xy".to_string();
        editor.replace_all();
        if editor.get_current_tab().content != vec!["xy caf\u{e9} xy"] {
            broken.push(format!("accented replace_all: {:?}", editor.get_current_tab().content));
        }

        assert!(broken.is_empty(), "\n{}", broken.join("\n"));
    }

    /// A column counts characters and a Rust string index counts bytes, and
    /// the two stop agreeing the moment a line holds anything but ASCII.
    /// Every selection used to be sliced by column directly, so selecting
    /// across an accent took the whole editor down with a panic.
    #[test]
    fn a_selection_over_accented_text_is_taken_whole() {
        let mut editor = editor_with(&["caf\u{e9} au lait"]);
        select(&mut editor, (0, 0), (8, 0));
        assert_eq!(editor.get_selected_text(), "caf\u{e9} au ");
        editor.delete_selected_text();
        assert_eq!(editor.get_current_tab().content, vec!["lait"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["caf\u{e9} au lait"]);
    }

    /// The boundary that used to panic outright: a column landing in the
    /// middle of a character rather than after it.
    #[test]
    fn a_selection_ending_inside_a_multibyte_character_does_not_panic() {
        let mut editor = editor_with(&["\u{e9}\u{e9}\u{e9}"]);
        select(&mut editor, (0, 0), (3, 0));
        assert_eq!(editor.get_selected_text(), "\u{e9}\u{e9}\u{e9}");
        editor.delete_selected_text();
        assert_eq!(editor.get_current_tab().content, vec![""]);

        let mut editor = editor_with(&["\u{e9}\u{e9}\u{e9}", "second"]);
        select(&mut editor, (1, 0), (3, 1));
        editor.delete_selected_text();
        assert_eq!(editor.get_current_tab().content, vec!["\u{e9}ond"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["\u{e9}\u{e9}\u{e9}", "second"]);
    }

    /// Selecting everything and typing over it is the same edit on an
    /// accented file as on an ASCII one, including the way back.
    #[test]
    fn select_all_covers_accented_lines_to_their_last_character() {
        let mut editor = editor_with(&["\u{e9}\u{e9}", "na\u{ef}ve"]);
        editor.select_all();
        assert_eq!(editor.get_selected_text(), "\u{e9}\u{e9}\nna\u{ef}ve");
        editor.insert_char('x');
        assert_eq!(editor.get_current_tab().content, vec!["x"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["\u{e9}\u{e9}", "na\u{ef}ve"]);
    }

    /// The end of a line is a column, not a byte count. Landing past the end
    /// of an accented line left the cursor in a place the file has no room
    /// for, and typing there padded the line with spaces to reach it.
    #[test]
    fn the_end_of_an_accented_line_is_its_last_column() {
        let mut editor = editor_with(&["caf\u{e9}"]);
        editor.move_to_line_end();
        assert_eq!(editor.get_current_tab().cursor_x, 4);
        editor.insert_char('!');
        assert_eq!(editor.get_current_tab().content, vec!["caf\u{e9}!"]);
    }

    /// A match is found over bytes and used as a column everywhere else, so
    /// one after an accent used to select and replace the wrong letters.
    #[test]
    fn a_search_match_after_an_accent_is_found_at_its_column() {
        let mut editor = editor_with(&["caf\u{e9} beans"]);
        editor.search_query = "beans".to_string();
        editor.find_all_matches();
        assert_eq!(editor.search_results, vec![(0, 5, 10)]);
        assert_eq!(editor.get_selected_text(), "beans");

        editor.replace_text = "leaves".to_string();
        editor.replace_current();
        assert_eq!(editor.get_current_tab().content, vec!["caf\u{e9} leaves"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["caf\u{e9} beans"]);
    }

    #[test]
    fn typing_over_a_selection_is_one_undo_away_from_the_text_it_replaced() {
        let mut editor = editor_with(&["print(greeting);", "second line"]);
        select(&mut editor, (0, 0), (16, 0));
        editor.insert_char('x');
        assert_eq!(editor.get_current_tab().content, vec!["x", "second line"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["print(greeting);", "second line"]);
        assert!(editor.redo());
        assert_eq!(editor.get_current_tab().content, vec!["x", "second line"]);
    }

    #[test]
    fn pasting_over_a_selection_is_one_undo_away_from_the_text_it_replaced() {
        let mut editor = editor_with(&["alpha", "beta", "gamma"]);
        select(&mut editor, (2, 0), (2, 2));
        editor.paste_text("XY");
        assert_eq!(editor.get_current_tab().content, vec!["alXYmma"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["alpha", "beta", "gamma"]);
        assert!(editor.redo());
        assert_eq!(editor.get_current_tab().content, vec!["alXYmma"]);
    }

    /// A pasted tab arrives as four spaces, so the undo entry has to hold the
    /// spaces. Holding the tab instead left three of them behind.
    #[test]
    fn undoing_a_paste_takes_back_the_spaces_a_tab_turned_into() {
        let mut editor = editor_with(&["ab"]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_x = 1;
        }
        editor.paste_text("\tx");
        assert_eq!(editor.get_current_tab().content, vec!["a    xb"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["ab"]);
    }

    #[test]
    fn enter_over_a_selection_is_one_undo_away_from_the_text_it_replaced() {
        let mut editor = editor_with(&["alpha beta", "gamma"]);
        select(&mut editor, (5, 0), (5, 1));
        editor.insert_newline();
        assert_eq!(editor.get_current_tab().content, vec!["alpha", ""]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["alpha beta", "gamma"]);
    }

    #[test]
    fn the_delete_key_over_a_selection_is_one_undo_away_from_it() {
        let mut editor = editor_with(&["alpha", "beta", "gamma"]);
        select(&mut editor, (1, 0), (2, 1));
        editor.delete_forward();
        assert_eq!(editor.get_current_tab().content, vec!["ata", "gamma"]);
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["alpha", "beta", "gamma"]);
    }

    /// A closing brace typed over a selection dedents the line instead of
    /// going in where the cursor is, and that path records the line it
    /// rewrote. The selection it swept out of the way needs an entry too.
    #[test]
    fn a_brace_typed_over_a_selection_still_leaves_a_way_back() {
        let mut editor = editor_with(&["    ", "        keep"]);
        select(&mut editor, (0, 1), (12, 1));
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_y = 1;
            tab.cursor_x = 12;
        }
        editor.insert_char('}');
        while editor.undo() {}
        assert_eq!(editor.get_current_tab().content, vec!["    ", "        keep"]);
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

    /// The overlay cannot be selected with the mouse, so the palette's copy
    /// hands over the same messages as text: file order, grouped per line the
    /// way the overlay groups them, each with the code it points at. Notices
    /// with no line in the file stay out of it.
    #[test]
    fn copying_errors_writes_them_out_in_file_order_with_their_code() {
        let mut editor = editor_with(&["one", "bad two", "three", "worse four"]);
        editor.code_errors = vec![
            CodeError { message: "later".to_string(), code_span: CodeSpan { start_line: 4, start_column: 1, end_line: 4, end_column: 2 } },
            CodeError { message: "earlier".to_string(), code_span: CodeSpan { start_line: 2, start_column: 1, end_line: 2, end_column: 2 } },
            CodeError { message: "also line two".to_string(), code_span: CodeSpan { start_line: 2, start_column: 5, end_line: 2, end_column: 6 } },
            CodeError::from("a notice with no line".to_string()),
        ];
        assert_eq!(editor.errors_as_text(), "line 2: earlier | also line two\n    bad two\nline 4: later\n    worse four");
    }

    /// Asking for a copy with nothing to copy answers on the bottom row
    /// rather than silently doing nothing, and asking twice replaces the
    /// receipt rather than stacking another one.
    #[test]
    fn copying_no_errors_says_so_instead_of_staying_silent() {
        let mut editor = editor_with(&["fine"]);
        editor.copy_errors();
        assert_eq!(editor.errors_as_text(), "", "a notice is not an error to copy");
        assert!(editor.code_errors.iter().any(|error| error.message == "No errors to copy"));
        editor.copy_errors();
        assert_eq!(editor.code_errors.len(), 1);
    }

    /// The debugging copy: the selected code and what the IDE says about
    /// those lines travel together, so one paste into a conversation
    /// carries the code and its diagnosis.
    #[test]
    fn copying_the_selection_carries_the_annotations_on_its_lines() {
        let mut editor = editor_with(&["one", "bad two", "three"]);
        editor.code_errors = vec![CodeError { message: "boom".to_string(), code_span: CodeSpan { start_line: 2, start_column: 1, end_line: 2, end_column: 2 } }];
        select(&mut editor, (0, 0), (7, 1));
        let (text, count) = editor.selection_with_annotations_text();
        assert_eq!(count, 1);
        assert_eq!(text, "one\nbad two  ◀ boom");
    }

    /// With nothing selected the line under the cursor is the selection,
    /// because the annotation being asked about is the one in front of
    /// the cursor. An annotation on any other line stays out.
    #[test]
    fn copying_with_no_selection_takes_the_line_under_the_cursor() {
        let mut editor = editor_with(&["one", "bad two", "three"]);
        editor.code_errors = vec![
            CodeError { message: "boom".to_string(), code_span: CodeSpan { start_line: 2, start_column: 1, end_line: 2, end_column: 2 } },
            CodeError { message: "elsewhere".to_string(), code_span: CodeSpan { start_line: 3, start_column: 1, end_line: 3, end_column: 2 } },
        ];
        editor.get_current_tab_mut().cursor_y = 1;
        let (text, count) = editor.selection_with_annotations_text();
        assert_eq!(count, 1);
        assert_eq!(text, "bad two  ◀ boom");
    }

    /// The largest debugging copy: the whole file and every annotation on
    /// any line of it, so one keypress hands a conversation the full
    /// picture even when the file runs past the window.
    #[test]
    fn copying_the_file_carries_every_annotation() {
        let mut editor = editor_with(&["one", "bad two", "three"]);
        editor.code_errors = vec![
            CodeError { message: "boom".to_string(), code_span: CodeSpan { start_line: 2, start_column: 1, end_line: 2, end_column: 2 } },
            CodeError { message: "also".to_string(), code_span: CodeSpan { start_line: 3, start_column: 1, end_line: 3, end_column: 2 } },
        ];
        editor.copy_file_with_annotations();
        assert!(editor.code_errors.iter().any(|error| error.message == "Copied the file with 2 annotations"));
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
        editor.view = ViewLayout { tabs: ratatui::layout::Rect::new(0, 0, 80, 3), text: ratatui::layout::Rect::new(5, 4, 40, 10), minimap: ratatui::layout::Rect::default() };
        editor.get_current_tab_mut().scroll_position = 1;

        assert_eq!(editor.text_position_at(8, 5), Some((3, 2)));
        // Past the end of a line is the end of that line, not a column that
        // has no character in it
        assert_eq!(editor.text_position_at(40, 4), Some((11, 1)));
        // Outside the text area entirely
        assert_eq!(editor.text_position_at(2, 1), None);
    }

    /// The file watcher pulls in what something else wrote. The text swaps,
    /// everything that pointed into the old text is dropped, and the cursor
    /// is clamped back inside the file instead of being flung to the top.
    #[test]
    fn an_external_reload_swaps_the_text_and_keeps_the_view_sane() {
        let mut editor = editor_with(&["first line", "second line", "third line"]);
        select(&mut editor, (0, 0), (4, 2));
        let tab = editor.get_current_tab_mut();
        tab.cursor_y = 2;
        tab.cursor_x = 8;
        tab.undo_stack.push(EditOperation::InsertChar { position: (0, 2), char: 'x' });
        tab.modified = true;
        tab.disk_changed_underneath = true;

        tab.reload_from_disk(vec!["short".to_string()], None);

        assert_eq!(tab.content, vec!["short"]);
        assert_eq!((tab.cursor_x, tab.cursor_y), (5, 0));
        assert!(!tab.has_selection());
        assert!(tab.undo_stack.is_empty());
        assert!(!tab.modified);
        assert!(!tab.disk_changed_underneath);
    }

    /// The watcher thread end to end: a clean tab follows the file as
    /// something else rewrites it, and a tab with unsaved edits keeps them
    /// and raises the flag instead.
    #[test]
    fn the_watcher_follows_the_disk_and_never_clobbers_edits() {
        let dir = std::env::temp_dir().join(format!("nail_ide_watch_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir for the watched file");
        let path = dir.join("watched.nail").to_string_lossy().to_string();
        std::fs::write(&path, "before").expect("the file being watched");

        let mut editor = Editor::new_with_debug(false);
        editor.tabs = vec![Tab::new_with_file(path.clone(), vec!["before".to_string()])];
        editor.tab_index = 0;
        let editor = std::sync::Arc::new(std::sync::Mutex::new(editor));

        let (tx, rx) = std::sync::mpsc::channel();
        let editor_for_watcher = std::sync::Arc::clone(&editor);
        let watcher = std::thread::spawn(move || crate::utils::file_watcher_thread_logic(editor_for_watcher, rx));

        let wait_for = |accept: &dyn Fn(&Tab) -> bool| -> bool {
            for _ in 0..100 {
                if accept(&editor.lock().expect("editor lock").tabs[0]) {
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            return false;
        };

        // A clean buffer follows the disk. The pause first is because two
        // writes inside one kernel clock tick share an mtime, and this test
        // gets from its first write to here faster than any real editor or
        // agent ever rewrites a file.
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(&path, "after").expect("rewriting the watched file");
        assert!(wait_for(&|tab| tab.content == vec!["after".to_string()]), "the reload never arrived");

        // A dirty buffer stays put and says so
        {
            let mut editor = editor.lock().expect("editor lock");
            editor.tabs[0].content = vec!["mine".to_string()];
            editor.tabs[0].modified = true;
        }
        std::fs::write(&path, "theirs").expect("rewriting under unsaved edits");
        assert!(wait_for(&|tab| tab.disk_changed_underneath), "the flag never went up");
        assert_eq!(editor.lock().expect("editor lock").tabs[0].content, vec!["mine".to_string()]);

        drop(tx);
        watcher.join().expect("the watcher thread shuts down when its channel closes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// F9 takes the disk copy, and what makes that safe to put on one key is
    /// that the thrown-away edits are a single undo step, not gone.
    #[test]
    fn taking_the_disk_copy_is_one_undo_away_from_regret() {
        let mut editor = editor_with(&["mine, half typed", "second line"]);
        {
            let tab = editor.get_current_tab_mut();
            tab.modified = true;
            tab.disk_changed_underneath = true;
            tab.take_disk_copy(vec!["theirs".to_string()], None);
            assert_eq!(tab.content, vec!["theirs"]);
            assert!(!tab.modified);
            assert!(!tab.disk_changed_underneath);
        }

        assert!(editor.undo());

        let tab = editor.get_current_tab();
        assert_eq!(tab.content, vec!["mine, half typed".to_string(), "second line".to_string()]);
        assert!(tab.modified);
    }

    /// A file truncated to nothing still has to leave a line for the cursor
    /// to be on, the same as opening an empty file does.
    #[test]
    fn reloading_an_emptied_file_leaves_one_empty_line() {
        let mut editor = editor_with(&["something"]);
        let tab = editor.get_current_tab_mut();
        tab.reload_from_disk(Vec::new(), None);
        assert_eq!(tab.content, vec![String::new()]);
        assert_eq!((tab.cursor_x, tab.cursor_y), (0, 0));
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

    /// The point of an example is that it is complete, so putting one in the
    /// file has to put all of it there, and leave the line it landed in the
    /// middle of still whole.
    #[test]
    fn an_inserted_example_arrives_whole_and_keeps_the_rest_of_the_line() {
        let mut editor = editor_with(&["before after", "next"]);
        editor.get_current_tab_mut().cursor_x = 7;
        editor.insert_lines_at_cursor("one\ntwo\nthree");
        assert_eq!(editor.get_current_tab().content, vec!["before one", "two", "threeafter", "next"]);
        assert_eq!(editor.get_current_tab().cursor_y, 2);
        assert_eq!(editor.get_current_tab().cursor_x, 5);

        let mut editor = editor_with(&["ab"]);
        editor.get_current_tab_mut().cursor_x = 1;
        editor.insert_lines_at_cursor("XY");
        assert_eq!(editor.get_current_tab().content, vec!["aXYb"]);
        assert_eq!(editor.get_current_tab().cursor_x, 3);
    }

    /// An insert past the end of the last line, and into a file with no lines
    /// at all, both used to be reachable ways to panic.
    #[test]
    fn an_insert_survives_a_cursor_past_the_end_of_the_file() {
        let mut editor = editor_with(&["short"]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_y = 5;
            tab.cursor_x = 99;
        }
        editor.insert_lines_at_cursor("here");
        assert_eq!(editor.get_current_tab().content.last().map(|line| line.as_str()), Some("here"));
    }

    /// The library browser is the one place a person is reading about a
    /// function rather than typing it, so it hands over the worked example
    /// instead of an empty pair of parentheses.
    #[test]
    fn the_library_browser_inserts_the_functions_example() {
        let expected = crate::stdlib_registry::get_stdlib_function("array_sum").expect("array_sum is registered").example;
        assert!(!expected.is_empty());

        let mut editor = editor_with(&[""]);
        editor.insert_stdlib_function("array_sum");
        assert_eq!(editor.get_current_tab().content.join("\n"), expected);
    }

    /// An example is a program, so it never gets joined onto the end of the
    /// statement the cursor happened to be after.
    #[test]
    fn an_inserted_example_starts_on_a_line_of_its_own() {
        let mut editor = editor_with(&["total:i = 1;"]);
        editor.get_current_tab_mut().cursor_x = 12;
        editor.insert_stdlib_function("array_sum");
        assert_eq!(editor.get_current_tab().content[0], "total:i = 1;");
        assert_eq!(editor.get_current_tab().content[1], "numbers:a:i = [1, 2, 3];");

        // Nothing typed on the line yet, so nothing is pushed down.
        let mut editor = editor_with(&["    "]);
        editor.get_current_tab_mut().cursor_x = 4;
        editor.insert_stdlib_function("array_sum");
        assert_eq!(editor.get_current_tab().content[0], "    numbers:a:i = [1, 2, 3];");
    }

    /// Three answers from one example. The completion list is someone typing,
    /// so it gives the name. The documentation view gives the call on TAB and
    /// the whole worked program on SHIFT+TAB.
    #[test]
    fn the_documentation_view_offers_the_call_and_the_whole_program() {
        let function = crate::stdlib_registry::get_stdlib_function("array_sum").expect("array_sum is registered");
        let completion = CompletionItem {
            label: "array_sum".to_string(),
            detail: String::new(),
            description: String::new(),
            example: function.example.to_string(),
            kind: CompletionKind::Function,
        };

        // Neither key changes meaning depending on whether the documentation
        // happens to be open, so both answers are the same either way.
        for showing_documentation in [false, true] {
            let mut editor = editor_with(&[""]);
            editor.show_detail_view = showing_documentation;

            let call = editor.generate_insertion_text(&completion, ExampleForm::Example);
            assert_eq!(call, "array_sum(numbers)");
            assert!(!call.contains('\n'));
            assert!(function.example.contains(&call));

            assert_eq!(editor.generate_insertion_text(&completion, ExampleForm::FullExample), function.example);
        }
        // The whole program declares the array the call reads, so it says
        // more than the call does.
        assert!(function.example.contains('\n'));
    }

    fn editor_completing(line: &str, name: &str) -> Editor {
        let function = crate::stdlib_registry::get_stdlib_function(name).expect("a registered function");
        let mut editor = editor_with(&[line]);
        editor.get_current_tab_mut().cursor_x = line.chars().count();
        editor.show_completions = true;
        editor.completion_index = 0;
        editor.completions = vec![CompletionItem {
            label: name.to_string(),
            detail: String::new(),
            description: String::new(),
            example: function.example.to_string(),
            kind: CompletionKind::Function,
        }];
        return editor;
    }

    /// Asking for the full example halfway through writing the call wants the
    /// declarations it needs, not a second copy of the statement. Pasting the
    /// program as it stands would leave `total:i = ` dangling above it.
    #[test]
    fn a_full_example_brings_its_setup_above_the_statement_being_typed() {
        let mut editor = editor_completing("total:i = array_su", "array_sum");
        editor.accept_completion_full();
        assert_eq!(editor.get_current_tab().content, vec!["numbers:a:i = [1, 2, 3];", "total:i = array_sum(numbers)"]);
    }

    /// On a line of its own there is no statement to work around, so the
    /// whole program arrives as written.
    #[test]
    fn a_full_example_on_an_empty_line_arrives_whole() {
        let function = crate::stdlib_registry::get_stdlib_function("array_sum").expect("array_sum is registered");
        let mut editor = editor_completing("array_su", "array_sum");
        editor.accept_completion_full();
        assert_eq!(editor.get_current_tab().content.join("\n"), function.example);
    }

    /// Tab is the same key in the middle of a statement as anywhere else, so
    /// what it inserts has to be an expression rather than a statement.
    #[test]
    fn tab_completes_the_call_in_place() {
        let mut editor = editor_completing("total:i = array_su", "array_sum");
        editor.accept_completion();
        assert_eq!(editor.get_current_tab().content, vec!["total:i = array_sum(numbers)"]);
    }

    /// Pressing Enter to push a line down and Backspace to pull it back up is
    /// two edits that cancel out, so the screen has to end where it started.
    /// The list used to appear on the way back, because the cursor sat in
    /// front of a name it read as half typed.
    #[test]
    fn pushing_a_line_down_and_pulling_it_back_up_leaves_the_list_closed() {
        let mut editor = editor_with(&["nail 0.1.0", "print(`hello from nail`);"]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_y = 1;
            tab.cursor_x = 0;
        }

        editor.insert_newline();
        editor.refresh_open_completions();
        assert!(!editor.show_completions);

        editor.delete_char();
        editor.refresh_open_completions();
        assert_eq!(editor.get_current_tab().content, vec!["nail 0.1.0", "print(`hello from nail`);"]);
        assert_eq!(editor.cursor_position(), (0, 1));
        assert!(!editor.show_completions);
    }

    /// The same two keys around a name that is already finished. A whole
    /// library function behind the cursor is not a word being typed, so
    /// nothing is offered to finish it with.
    #[test]
    fn the_same_two_keys_after_a_finished_name_leave_the_list_closed() {
        let mut editor = editor_with(&["nail 0.1.0", "total:i = array_sum(numbers);"]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_y = 1;
            tab.cursor_x = 19;
        }

        editor.insert_newline();
        editor.refresh_open_completions();
        assert!(!editor.show_completions);

        editor.delete_char();
        editor.refresh_open_completions();
        assert_eq!(editor.get_current_tab().content, vec!["nail 0.1.0", "total:i = array_sum(numbers);"]);
        assert_eq!(editor.cursor_position(), (19, 1));
        assert!(!editor.show_completions);
    }

    /// The prefix is what has been typed, which is what is behind the cursor.
    /// A name the cursor is sitting in front of, or in the middle of, is
    /// already written.
    #[test]
    fn the_completion_prefix_is_only_what_is_left_of_the_cursor() {
        let mut editor = editor_with(&["print(`hi`);"]);

        editor.get_current_tab_mut().cursor_x = 0;
        assert_eq!(editor.completion_prefix_at_cursor(), "");
        assert!(matches!(editor.get_completion_context(), CompletionContext::None));

        editor.get_current_tab_mut().cursor_x = 3;
        assert_eq!(editor.completion_prefix_at_cursor(), "pri");

        editor.get_current_tab_mut().cursor_x = 5;
        assert_eq!(editor.completion_prefix_at_cursor(), "print");
    }

    /// Typing asks for a list rather than building one. Building it reads the
    /// standard library, and a burst of typing asks many times over, so the
    /// asking has to be the cheap half.
    #[test]
    fn typing_asks_for_a_list_and_does_not_build_one() {
        let mut editor = editor_with(&[""]);
        for character in "array_su".chars() {
            editor.insert_char(character);
            editor.request_completions();
        }
        assert!(!editor.show_completions);
        assert!(editor.completions.is_empty());

        editor.flush_completion_request();
        assert!(editor.show_completions);
        assert!(editor.completions.iter().any(|completion| completion.label == "array_sum"));
        // One list per burst: the request is spent by building it.
        assert!(editor.completion_request.is_none());
    }

    /// The keys that pick from the list build it first, so a list asked for a
    /// moment ago can never hand back the word from before it.
    #[test]
    fn taking_a_completion_builds_the_list_that_was_still_owed() {
        let mut editor = editor_with(&[""]);
        for character in "array_su".chars() {
            editor.insert_char(character);
            editor.request_completions();
        }
        assert!(!editor.show_completions);

        editor.accept_completion();
        assert_eq!(editor.get_current_tab().content, vec!["array_sum(numbers)"]);
    }

    /// A deletion that was never built into a list is still a deletion: it
    /// narrows a list that is open, and opens nothing when none is.
    #[test]
    fn a_pending_deletion_narrows_rather_than_opens() {
        let mut editor = editor_with(&["array_sum"]);
        editor.get_current_tab_mut().cursor_x = 9;
        editor.delete_char();
        editor.request_completion_refresh();
        editor.flush_completion_request();
        assert!(!editor.show_completions);

        editor.update_completions();
        assert!(editor.show_completions);
        editor.delete_char();
        editor.request_completion_refresh();
        editor.flush_completion_request();
        assert!(editor.show_completions);
        assert_eq!(editor.completion_prefix, "array_s");
    }

    /// Typing still opens the list, and backspacing through what was typed
    /// still narrows it, so the list only ever closes early once the prefix is
    /// too short to mean anything.
    #[test]
    fn a_deletion_narrows_a_list_that_is_already_open() {
        let mut editor = editor_with(&[""]);
        for character in "array_su".chars() {
            editor.insert_char(character);
            editor.update_completions();
        }
        assert!(editor.show_completions);
        assert!(editor.completions.iter().any(|completion| completion.label == "array_sum"));

        editor.delete_char();
        editor.refresh_open_completions();
        assert!(editor.show_completions);
        assert_eq!(editor.completion_prefix, "array_s");

        for _ in 0..6 {
            editor.delete_char();
            editor.refresh_open_completions();
        }
        assert!(!editor.show_completions);
    }

    /// Vim's `o`. The new line inherits the indentation of the one it follows,
    /// which is what makes it worth having over Enter at the end of a line.
    #[test]
    fn opening_a_line_below_lands_indented_under_the_one_it_follows() {
        let mut editor = editor_with(&["f fn():v {", "    print(`hi`);", "}"]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_y = 1;
            tab.cursor_x = 2;
        }
        editor.open_line_below();
        let tab = editor.get_current_tab();
        assert_eq!(tab.content, vec!["f fn():v {", "    print(`hi`);", "    ", "}"]);
        assert_eq!((tab.cursor_x, tab.cursor_y), (4, 2));
    }

    /// Vim's `O`, which has to work on the first line of the file too, where
    /// there is no line above to open one under.
    #[test]
    fn opening_a_line_above_works_at_the_top_of_the_file() {
        let mut editor = editor_with(&["    second"]);
        editor.open_line_above();
        assert_eq!(editor.get_current_tab().content, vec!["", "    second"]);
        assert_eq!(editor.cursor_position(), (0, 0));
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["    second"]);

        let mut editor = editor_with(&["    first", "second"]);
        {
            let tab = editor.get_current_tab_mut();
            tab.cursor_y = 1;
        }
        editor.open_line_above();
        assert_eq!(editor.get_current_tab().content, vec!["    first", "    ", "second"]);
        assert_eq!(editor.cursor_position(), (4, 1));
    }

    /// Deleting forward joins the next line up, which is what Delete is for
    /// and never what `x` means.
    #[test]
    fn deleting_the_character_under_the_cursor_stops_at_the_end_of_the_line() {
        let mut editor = editor_with(&["ab", "cd"]);
        editor.delete_char_at_cursor();
        assert_eq!(editor.get_current_tab().content, vec!["b", "cd"]);

        editor.get_current_tab_mut().cursor_x = 1;
        editor.delete_char_at_cursor();
        assert_eq!(editor.get_current_tab().content, vec!["b", "cd"]);
    }

    /// Vim's `cc` empties the line and leaves the cursor where the typing
    /// goes, keeping the indentation it had.
    #[test]
    fn changing_a_line_keeps_its_indentation_and_nothing_else() {
        let mut editor = editor_with(&["    print(`hi`);"]);
        editor.change_line();
        assert_eq!(editor.get_current_tab().content, vec!["    "]);
        assert_eq!(editor.cursor_position(), (4, 0));
        assert!(editor.undo());
        assert_eq!(editor.get_current_tab().content, vec!["    print(`hi`);"]);
    }

    #[test]
    fn killing_to_the_line_start_takes_what_is_behind_the_cursor() {
        let mut editor = editor_with(&["one two"]);
        editor.get_current_tab_mut().cursor_x = 4;
        editor.kill_to_line_start();
        assert_eq!(editor.get_current_tab().content, vec!["two"]);
        assert_eq!(editor.cursor_position(), (0, 0));
        // Nothing behind the cursor means nothing happens, rather than the
        // line above being pulled up.
        editor.kill_to_line_start();
        assert_eq!(editor.get_current_tab().content, vec!["two"]);
    }

    /// Visual line mode is charwise selection with both ends pushed out to the
    /// line boundaries, and which end goes where depends on which way the
    /// selection was drawn.
    #[test]
    fn a_line_selection_covers_whole_lines_in_both_directions() {
        let mut editor = editor_with(&["one", "two", "three"]);
        select(&mut editor, (1, 0), (2, 2));
        editor.snap_selection_to_lines();
        let tab = editor.get_current_tab();
        assert_eq!(tab.selection_start, Some((0, 0)));
        assert_eq!(tab.selection_end, Some((5, 2)));

        select(&mut editor, (2, 2), (1, 0));
        editor.snap_selection_to_lines();
        let tab = editor.get_current_tab();
        assert_eq!(tab.selection_start, Some((5, 2)));
        assert_eq!(tab.selection_end, Some((0, 0)));
    }
}
