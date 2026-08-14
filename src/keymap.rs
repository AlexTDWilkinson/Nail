//! Which key bindings to start with, read off the machine the IDE is running
//! on.
//!
//! A vim user has already told several programs that they are a vim user, and
//! an emacs user likewise. Reading those answers means the IDE gets the
//! bindings right on first launch, with nothing to configure and no settings
//! file of ours to keep in sync. CUA (the arrow keys and Ctrl-C, Ctrl-V that
//! every other editor uses) is the default, so a machine that says nothing
//! lands somewhere familiar.
//!
//! Signals are checked in order of how deliberate they are. `NAIL_KEYMAP` is
//! about this program specifically, so it wins. `VISUAL` and `EDITOR` say
//! which editor the user wants handed control, which is a strong statement
//! about their fingers. The readline `editing-mode` is last because it is a
//! shell preference that a user may never have revisited.
//!
//! An unrecognized value at any step is not an error and not an answer, it
//! just falls through to the next signal. `EDITOR=nano` says nothing about
//! modal editing, and ending at CUA is the right outcome for it anyway.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keymap {
    Cua,
    Vim,
    Emacs,
}

impl Keymap {
    /// What the status bar announces. CUA has no label because it is the
    /// default, and a mode indicator only earns its space when it is telling
    /// the user something they did not already assume.
    ///
    /// Vim reports which mode it is in rather than that it is vim, because
    /// which mode is on is the thing a vim user needs the bar to tell them and
    /// the only thing that changes while they work.
    pub fn label(&self, vim: VimMode) -> Option<&'static str> {
        return match self {
            Keymap::Cua => None,
            Keymap::Vim => Some(vim.label()),
            Keymap::Emacs => Some(" emacs "),
        };
    }
}

/// Which of vim's modes the keys are being read in.
///
/// Insert is the odd one out: it is an ordinary editor with one way back, so
/// it borrows the CUA table whole. In the other three a letter is a command,
/// which is why a letter with no command behind it has to be swallowed there
/// rather than typed into the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
}

impl VimMode {
    pub fn label(&self) -> &'static str {
        return match self {
            VimMode::Normal => " -- NORMAL -- ",
            VimMode::Insert => " -- INSERT -- ",
            VimMode::Visual => " -- VISUAL -- ",
            VimMode::VisualLine => " -- VISUAL LINE -- ",
        };
    }

    /// Both visual modes answer to the same keys and differ only in what the
    /// selection is allowed to cover, so most of the table asks this rather
    /// than naming them one at a time.
    pub fn is_visual(&self) -> bool {
        return matches!(self, VimMode::Visual | VimMode::VisualLine);
    }
}

pub fn detect() -> Keymap {
    if let Some(keymap) = std::env::var("NAIL_KEYMAP").ok().and_then(|value| from_keymap_name(&value)) {
        return keymap;
    }
    // VISUAL beats EDITOR by long convention: EDITOR names a line editor that
    // works anywhere, VISUAL the full screen one the user actually prefers.
    for variable in ["VISUAL", "EDITOR"] {
        if let Some(keymap) = std::env::var(variable).ok().and_then(|value| from_editor_value(&value)) {
            return keymap;
        }
    }
    if let Some(keymap) = read_inputrc().as_deref().and_then(from_inputrc_text) {
        return keymap;
    }
    return Keymap::Cua;
}

fn read_inputrc() -> Option<String> {
    let path = match std::env::var_os("INPUTRC") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".inputrc"),
    };
    // A missing or unreadable file is the common case, not a problem, so it
    // reads as no signal rather than anything the user hears about.
    return std::fs::read_to_string(path).ok();
}

fn from_keymap_name(value: &str) -> Option<Keymap> {
    return match value.trim().to_ascii_lowercase().as_str() {
        "vim" => Some(Keymap::Vim),
        "emacs" => Some(Keymap::Emacs),
        // "cua" is the name of the standard this follows, and nobody outside
        // a specification has ever called their keyboard that. "normal" is
        // what it is called anywhere a user can see it, and both are accepted
        // so a config written under the old name still loads.
        "normal" | "cua" => Some(Keymap::Cua),
        _ => None,
    };
}

/// The variable holds a command line, not a program name: `vim`,
/// `/usr/bin/nvim`, `emacsclient -t`, `vim -f`. Only the file name of the
/// first word says anything about key bindings, so the rest is dropped.
fn from_editor_value(value: &str) -> Option<Keymap> {
    let command = value.split_whitespace().next()?;
    let name = Path::new(command).file_name()?.to_str()?.to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    return match name {
        "vim" | "nvim" | "vi" | "gvim" | "vimx" => Some(Keymap::Vim),
        "emacs" | "emacsclient" => Some(Keymap::Emacs),
        _ => None,
    };
}

/// Reads readline's `set editing-mode` out of an inputrc. The last setting
/// wins, because that is the one readline itself would end up applying.
/// Conditional `$if` blocks are not interpreted, so a file that sets vi mode
/// only for some other program still reads as vi here. That costs a wrong
/// guess in a rare file and buys a parser with nothing in it to rot.
fn from_inputrc_text(text: &str) -> Option<Keymap> {
    return text.lines().rev().find_map(|line| {
        let line = line.trim();
        if line.starts_with('#') {
            return None;
        }
        let mut words = line.split_whitespace();
        if (words.next(), words.next()) != (Some("set"), Some("editing-mode")) {
            return None;
        }
        let mode = words.next()?;
        if words.next().is_some() {
            return None;
        }
        return match mode {
            "vi" => Some(Keymap::Vim),
            "emacs" => Some(Keymap::Emacs),
            _ => None,
        };
    });
}

/// One thing the editor can be asked to do, named for the intent rather than
/// the keys that ask for it. Keeping the two apart is what lets a second set
/// of bindings exist at all: a vim table and a CUA table disagree about which
/// keys mean `DeleteLine`, never about what deleting a line is.
///
/// Text input is deliberately absent. Typing a character, Enter, Backspace,
/// Tab and Escape all mean different things depending on whether a dialog or
/// the completion list is open, and that routing belongs to the editor's own
/// state rather than to a binding table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Save,
    ReloadFromDisk,
    CycleExampleFiles,
    ToggleTheme,
    Build,
    BuildRelease,
    ToggleLineNumbers,
    ToggleCurrentLineHighlight,
    ToggleBracketMatching,
    ToggleWhitespace,
    ToggleIndentationGuides,
    ToggleMinimap,
    SelectAll,
    Cut,
    Paste,
    Undo,
    Redo,
    GoToLineDialog,
    FindDialog,
    ReplaceDialog,
    ToggleCaseSensitivity,
    FindNext,
    FindPrevious,
    OpenFileDialog,
    CloseTab,
    NewTab,
    StdLibBrowser,
    NextTab,
    PreviousTab,
    SwitchToTab(usize),
    ToggleComment,
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    DeleteLine,
    JumpToMatchingBracket,
    ToggleCompletionDetail,
    ScrollUp,
    ScrollDown,
    CursorUp { extend: bool },
    CursorDown { extend: bool },
    CursorLeft { extend: bool },
    CursorRight { extend: bool },
    CursorWordLeft { extend: bool },
    CursorWordRight { extend: bool },
    SmartHome,
    LineStart { extend: bool },
    LineEnd { extend: bool },
    FileStart { extend: bool },
    FileEnd { extend: bool },
    Copy,
    SetMark,
    ClearMark,
    KillToLineEnd,
    DeleteForward,
    OpenSettings,
    DeleteWordLeft,
    DeleteWordRight,
    NextError,
    PreviousError,
    CopyErrors,
    CopyScreen,
    CopySelectionWithAnnotations,
    CopyFileText,
    CopyFileWithAnnotations,
    CommandPalette,
    SymbolPicker,
    ProjectSymbolPicker,
    ProjectSearch,
    OpenImportedFile,
    ExpandSelection,
    ShrinkSelection,
    JoinLines,
    SortLines,
    ToggleWholeWord,
    ToggleRegex,
    ToggleMouse,
    ScrollLineUp,
    ScrollLineDown,
    // The edits vim asks for that no other keymap has a key for. Each is named
    // for the whole intent rather than the keys: `o` is "start a line under
    // this one and type on it", and the vim table is the only thing that has
    // to know `o` is how that gets asked for.
    EnterInsertMode,
    EnterNormalMode,
    EnterVisualMode,
    EnterVisualLineMode,
    InsertAfterCursor,
    InsertAtLineStart,
    InsertAtLineEnd,
    OpenLineBelow,
    OpenLineAbove,
    DeleteCharAtCursor,
    DeleteBackward,
    DeleteToLineStart,
    ChangeToLineEnd,
    ChangeLine,
    ChangeWord,
    SubstituteChar,
    YankLine,
    YankWord,
    YankToLineEnd,
    PasteAfter,
    PasteBefore,
    CutSelection,
    YankSelection,
    ChangeSelection,
    PasteOverSelection,
    Indent,
    Dedent,
    SaveAndQuit,
    ClearSearchHighlight,
    /// A key that vim gives a meaning this editor cannot honour, carrying what
    /// to say about it. Silence would be the alternative, and a key that does
    /// nothing without saying why is a key the user presses again harder.
    Unsupported(&'static str),
}

/// One row of the command palette: what the command is called, which keys
/// also reach it, and what it does.
pub struct Command {
    pub name: &'static str,
    pub keys: Keys,
    pub action: Action,
}

/// What a command's keys are called under each keymap, because they are not
/// the same keys. Saving is Ctrl+S, `ZZ` and `C-x C-s` depending on who is
/// typing, and a palette that says Ctrl+S to all three is lying to two of
/// them.
///
/// An empty string means the command has no key at all under that keymap and
/// the palette is the way to it. That is a real answer and worth showing: the
/// alternative is a user hunting for a chord that was never there.
pub struct Keys {
    cua: &'static str,
    vim: &'static str,
    emacs: &'static str,
}

impl Keys {
    /// One key in every keymap, which is the common case: the function keys,
    /// the Alt keys, and every chord none of the modal tables wanted.
    const fn same(keys: &'static str) -> Keys {
        return Keys { cua: keys, vim: keys, emacs: keys };
    }

    const fn new(cua: &'static str, vim: &'static str, emacs: &'static str) -> Keys {
        return Keys { cua, vim, emacs };
    }

    pub fn for_keymap(&self, keymap: Keymap) -> &'static str {
        return match keymap {
            Keymap::Cua => self.cua,
            Keymap::Vim => self.vim,
            Keymap::Emacs => self.emacs,
        };
    }
}

/// Every command a user can ask for by name. The palette is the answer to a
/// keyboard shortcut nobody can be expected to remember, so the list is
/// written here beside the tables rather than off in the editor: a binding
/// and its name are the same fact twice, and keeping them adjacent is what
/// makes a stale key hint obvious.
///
/// A command with no keys is reachable only from here, which is a fine place
/// for the ones that are worth having and not worth a chord.
pub const COMMANDS: &[Command] = &[
    Command { name: "Save file", keys: Keys::new("Ctrl+S", "Ctrl+S", "Ctrl+X Ctrl+S"), action: Action::Save },
    Command { name: "Open file", keys: Keys::new("Ctrl+O", "Ctrl+O", "Ctrl+X Ctrl+F"), action: Action::OpenFileDialog },
    // The other half of the file watcher: when the disk moved under unsaved
    // edits, this is how to say the disk's copy is the one to keep. The
    // discard is one recorded edit, so undo brings the edits back.
    Command { name: "Reload file from disk (discard edits, undo restores them)", keys: Keys::same("F9"), action: Action::ReloadFromDisk },
    Command { name: "Build (quick)", keys: Keys::same("F7"), action: Action::Build },
    Command { name: "Build for shipping (release binary beside the source)", keys: Keys::same("Shift+F7"), action: Action::BuildRelease },
    // Ctrl+G is the emacs cancel key, so going to a line has no emacs chord.
    Command { name: "Go to line", keys: Keys::new("Ctrl+G", "Ctrl+G", ""), action: Action::GoToLineDialog },
    Command { name: "Go to symbol", keys: Keys::new("Ctrl+R", "gO", ""), action: Action::SymbolPicker },
    Command { name: "Go to symbol in project", keys: Keys::new("Ctrl+T", "Ctrl+T", ""), action: Action::ProjectSymbolPicker },
    // Ctrl+E scrolls a line in vim and does nothing in emacs, so both of those
    // reach the project-wide search by name.
    Command { name: "Search the project", keys: Keys::new("Ctrl+E", "", ""), action: Action::ProjectSearch },
    Command { name: "Go to imported file", keys: Keys::new("F12", "gd", "F12"), action: Action::OpenImportedFile },
    Command { name: "Find", keys: Keys::new("Ctrl+F", "/", "Ctrl+S"), action: Action::FindDialog },
    Command { name: "Find and replace", keys: Keys::new("Ctrl+H", "Ctrl+H", "Alt+%"), action: Action::ReplaceDialog },
    Command { name: "Find next", keys: Keys::new("F3", "n", "F3"), action: Action::FindNext },
    Command { name: "Find previous", keys: Keys::new("Shift+F3", "N", "Ctrl+R"), action: Action::FindPrevious },
    Command { name: "Clear search highlight", keys: Keys::new("", "Ctrl+L", ""), action: Action::ClearSearchHighlight },
    Command { name: "Next error", keys: Keys::new("F8", "]d", "F8"), action: Action::NextError },
    Command { name: "Previous error", keys: Keys::new("Shift+F8", "[d", "Shift+F8"), action: Action::PreviousError },
    // The error overlay is drawn over the buffer rather than stored in it, so
    // no selection can ever pick it up. This is how the messages get out.
    Command { name: "Copy errors", keys: Keys::same(""), action: Action::CopyErrors },
    // The rule the copy commands serve: anything the IDE displays is
    // copyable as plain text. This one takes the whole painted frame,
    // overlays, popups and all, so nothing on screen is ever out of reach.
    Command { name: "Copy screen as text", keys: Keys::same("F10"), action: Action::CopyScreen },
    // The debugging copy: the selected code and whatever the IDE has to say
    // about those lines, errors and timings alike, in one paste.
    Command { name: "Copy selection with errors and timings", keys: Keys::same("Shift+F10"), action: Action::CopySelectionWithAnnotations },
    Command { name: "Copy file text", keys: Keys::same(""), action: Action::CopyFileText },
    Command { name: "Copy file with errors and timings", keys: Keys::same(""), action: Action::CopyFileWithAnnotations },
    // Ctrl+B pages up in vim, so the library is on the key vim already uses
    // for looking a word up.
    Command { name: "Standard library browser", keys: Keys::new("Ctrl+B", "K", ""), action: Action::StdLibBrowser },
    Command { name: "New tab", keys: Keys::new("Ctrl+N", "Ctrl+N", ""), action: Action::NewTab },
    Command { name: "Close tab", keys: Keys::new("Ctrl+W", "Ctrl+W c", "Ctrl+X k"), action: Action::CloseTab },
    Command { name: "Next tab", keys: Keys::new("Ctrl+Tab", "gt", "Ctrl+X b"), action: Action::NextTab },
    Command { name: "Previous tab", keys: Keys::new("Ctrl+Shift+Tab", "gT", "Ctrl+Shift+Tab"), action: Action::PreviousTab },
    Command { name: "Undo", keys: Keys::new("Ctrl+Z", "u", "Ctrl+/"), action: Action::Undo },
    Command { name: "Redo", keys: Keys::new("Ctrl+Y", "Ctrl+R", ""), action: Action::Redo },
    Command { name: "Cut", keys: Keys::new("Ctrl+X", "d in visual", "Ctrl+W"), action: Action::Cut },
    Command { name: "Copy", keys: Keys::new("Ctrl+C", "y in visual", "Alt+W"), action: Action::Copy },
    Command { name: "Paste", keys: Keys::new("Ctrl+V", "p", "Ctrl+Y"), action: Action::Paste },
    Command { name: "Select all", keys: Keys::new("Ctrl+A", "Ctrl+A", "Ctrl+X h"), action: Action::SelectAll },
    Command { name: "Expand selection", keys: Keys::same("Shift+Alt+Right"), action: Action::ExpandSelection },
    Command { name: "Shrink selection", keys: Keys::same("Shift+Alt+Left"), action: Action::ShrinkSelection },
    Command { name: "Toggle comment", keys: Keys::new("Ctrl+/", "gcc", "Alt+;"), action: Action::ToggleComment },
    Command { name: "Duplicate line", keys: Keys::new("Ctrl+D", "", ""), action: Action::DuplicateLine },
    Command { name: "Delete line", keys: Keys::new("Ctrl+Shift+K", "dd", ""), action: Action::DeleteLine },
    Command { name: "Move line up", keys: Keys::same("Alt+Up"), action: Action::MoveLineUp },
    Command { name: "Move line down", keys: Keys::same("Alt+Down"), action: Action::MoveLineDown },
    Command { name: "Join lines", keys: Keys::new("Ctrl+J", "J", ""), action: Action::JoinLines },
    Command { name: "Sort lines", keys: Keys::same(""), action: Action::SortLines },
    // Alt+Backspace rather than Ctrl+Backspace, which most terminals cannot
    // tell apart from Ctrl+H and so send as that instead. The Ctrl binding is
    // still there for the terminals that report it properly, but the hint
    // names the one that works everywhere.
    Command { name: "Delete word left", keys: Keys::new("Alt+Backspace", "db", "Alt+Backspace"), action: Action::DeleteWordLeft },
    Command { name: "Delete word right", keys: Keys::new("Ctrl+Delete", "dw", "Ctrl+Delete"), action: Action::DeleteWordRight },
    Command { name: "Jump to matching bracket", keys: Keys::new("Ctrl+]", "%", ""), action: Action::JumpToMatchingBracket },
    Command { name: "Toggle theme", keys: Keys::same("F6"), action: Action::ToggleTheme },
    Command { name: "Toggle mouse", keys: Keys::same("F4"), action: Action::ToggleMouse },
    Command { name: "Toggle line numbers", keys: Keys::new("Ctrl+L", "", ""), action: Action::ToggleLineNumbers },
    Command { name: "Toggle current line highlight", keys: Keys::new("Ctrl+Shift+H", "Ctrl+Shift+H", ""), action: Action::ToggleCurrentLineHighlight },
    Command { name: "Toggle bracket matching", keys: Keys::new("Ctrl+Shift+B", "", ""), action: Action::ToggleBracketMatching },
    Command { name: "Toggle whitespace", keys: Keys::new("Ctrl+Shift+W", "", ""), action: Action::ToggleWhitespace },
    Command { name: "Toggle indentation guides", keys: Keys::new("Ctrl+Shift+G", "Ctrl+Shift+G", ""), action: Action::ToggleIndentationGuides },
    Command { name: "Toggle minimap", keys: Keys::new("Ctrl+Shift+M", "Ctrl+Shift+M", ""), action: Action::ToggleMinimap },
    Command { name: "Toggle case sensitive search", keys: Keys::same("Alt+C"), action: Action::ToggleCaseSensitivity },
    // Alt+W is how emacs copies, so the whole word switch is out of its reach.
    Command { name: "Toggle whole word search", keys: Keys::new("Alt+W", "Alt+W", ""), action: Action::ToggleWholeWord },
    Command { name: "Toggle regular expression search", keys: Keys::same("Alt+R"), action: Action::ToggleRegex },
    Command { name: "Cycle example files", keys: Keys::same("F5"), action: Action::CycleExampleFiles },
    Command { name: "Save and quit", keys: Keys::new("", "ZZ", ""), action: Action::SaveAndQuit },
    Command { name: "Quit", keys: Keys::new("Esc", "ZQ", "Ctrl+X Ctrl+C"), action: Action::Quit },
];

/// A key that means nothing on its own and waits for the one after it. Emacs
/// spells most of its file commands this way and vim spells every operator
/// that way, while CUA has none at all, which is the reason resolving a key
/// cannot be a plain lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    ControlX,
    // Vim's operators. Each is a verb waiting to hear what it applies to, and
    // `dd`, `cc` and `yy` are what that verb spells when it applies to the
    // line the cursor is on.
    Delete,
    Change,
    Yank,
    Go,
    Indent,
    Dedent,
    // `gc`, which is an operator reached through another one, so `gcc` is
    // three keys and two prefixes deep.
    Comment,
    // Ctrl+W, which in neovim is the window commands. This editor has tabs
    // where neovim has windows, so the three that mean the same thing about a
    // tab are here and the rest of the family is not.
    Window,
    // `Z`, which is only ever the first half of leaving.
    Exit,
    // Neovim's bracket pairs, which are "the thing of this kind before the
    // cursor" and "the one after it". `]d` and `[d` are the diagnostics, which
    // here are the compiler's errors.
    PreviousOf,
    NextOf,
}

/// What a key press turned out to be. `Pending` and `Swallowed` both differ
/// from `Unbound` in the same way: the key belongs to the bindings, so it must
/// not fall through and be typed into the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Run(Action),
    Pending(Prefix),
    /// The key was read by the bindings and asked for nothing. Vim's normal
    /// mode is full of these: `q` is not a command this editor has, and while
    /// normal mode is on it is not the letter q either.
    Swallowed,
    Unbound,
}

/// Turns a key press into what the editor should do about it, given which
/// bindings are in force, which vim mode is on, and whether a prefix key is
/// already waiting. The mode is ignored by the keymaps that do not have modes.
pub fn resolve(keymap: Keymap, mode: VimMode, pending: Option<Prefix>, key: KeyEvent) -> Resolution {
    return match keymap {
        Keymap::Cua => match cua(key) {
            Some(action) => Resolution::Run(action),
            None => Resolution::Unbound,
        },
        Keymap::Vim => vim(mode, pending, key),
        Keymap::Emacs => emacs(pending, key),
    };
}

/// The bindings every other editor has taught people: arrow keys move, Ctrl
/// with a letter runs a command. This is the table a machine that gave no
/// signal lands on, and the one a vim or emacs table will be a sibling of
/// rather than a patch over.
///
/// A key with no entry here is not an error. It falls through to the editor's
/// text input handling, which is how ordinary typing still reaches the buffer.
pub fn cua(key: KeyEvent) -> Option<Action> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    return match key.code {
        KeyCode::Char('c') if control => Some(Action::Copy),
        KeyCode::Char('s') if control => Some(Action::Save),
        KeyCode::Char('l') if control => Some(Action::ToggleLineNumbers),
        // Every Ctrl+Shift pair is tested before the plain Ctrl one that
        // shares its letter, because `contains` asks whether a modifier is
        // held and not whether it is the only one held.
        KeyCode::Char('h') if control && shift => Some(Action::ToggleCurrentLineHighlight),
        KeyCode::Char('b') if control && shift => Some(Action::ToggleBracketMatching),
        KeyCode::Char('w') if control && shift => Some(Action::ToggleWhitespace),
        KeyCode::Char('g') if control && shift => Some(Action::ToggleIndentationGuides),
        KeyCode::Char('m') if control && shift => Some(Action::ToggleMinimap),
        KeyCode::Char('k') if control && shift => Some(Action::DeleteLine),
        KeyCode::Char('z') if control && shift => Some(Action::Redo),
        // The standard library browser, finding a symbol and finding a
        // phrase. The shifted spellings are the ones other editors use, and
        // they are here for the terminals that can send them. Most cannot:
        // Ctrl+Shift+P arrives as plain Ctrl+P unless the terminal speaks one
        // of the newer keyboard protocols, which is why each of these also
        // has a plain chord of its own below.
        KeyCode::Char('p') if control && shift => Some(Action::StdLibBrowser),
        KeyCode::Char('r') if control && shift => Some(Action::ProjectSymbolPicker),
        KeyCode::Char('f') if control && shift => Some(Action::ProjectSearch),
        KeyCode::Char('a') if control => Some(Action::SelectAll),
        KeyCode::Char('x') if control => Some(Action::Cut),
        KeyCode::Char('v') if control => Some(Action::Paste),
        KeyCode::Char('z') if control => Some(Action::Undo),
        KeyCode::Char('y') if control => Some(Action::Redo),
        KeyCode::Char('g') if control => Some(Action::GoToLineDialog),
        KeyCode::Char('f') if control => Some(Action::FindDialog),
        KeyCode::Char('h') if control => Some(Action::ReplaceDialog),
        KeyCode::Char('i') if control => Some(Action::ToggleCaseSensitivity),
        KeyCode::Char('o') if control => Some(Action::OpenFileDialog),
        KeyCode::Char('w') if control => Some(Action::CloseTab),
        KeyCode::Char('n') if control => Some(Action::NewTab),
        KeyCode::Char('/') if control => Some(Action::ToggleComment),
        KeyCode::Char('d') if control => Some(Action::DuplicateLine),
        KeyCode::Char(']') if control => Some(Action::JumpToMatchingBracket),
        // The two lists that answer "what can this thing do" and "what is in
        // this file". Both are the same fuzzy list, pointed at different
        // contents, and both are where a key nobody remembers goes to be
        // found by name instead.
        KeyCode::Char('p') if control => Some(Action::CommandPalette),
        KeyCode::Char('r') if control => Some(Action::SymbolPicker),
        // The project-wide pair, on keys a terminal can actually deliver.
        // Ctrl+T is where other editors put a search for a symbol by name,
        // and Ctrl+E is next to it and free.
        KeyCode::Char('t') if control => Some(Action::ProjectSymbolPicker),
        KeyCode::Char('e') if control => Some(Action::ProjectSearch),
        // Browsing the standard library, on a key a terminal can deliver.
        // Ctrl+B was free, and B is for browse.
        KeyCode::Char('b') if control => Some(Action::StdLibBrowser),
        KeyCode::Char('j') if control => Some(Action::JoinLines),
        // Ctrl with a digit picks a tab directly. Tab one is the leftmost, so
        // the digit is one ahead of the index it names.
        KeyCode::Char(digit) if control && digit.is_ascii_digit() => {
            let position = digit.to_digit(10)? as usize;
            Some(Action::SwitchToTab(position.saturating_sub(1)))
        }
        _ => common(key),
    };
}

/// The keys that mean the same thing whichever bindings are in force. Arrows,
/// Home, End and the function keys are not anybody's idea of a mode: an emacs
/// user reaching for an arrow key wants the cursor to move, and taking that
/// away in the name of purity would only make the editor harder to use.
fn common(key: KeyEvent) -> Option<Action> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    return match key.code {
        KeyCode::F(5) => Some(Action::CycleExampleFiles),
        KeyCode::F(6) => Some(Action::ToggleTheme),
        // Building answers two different questions. Unshifted is the fast
        // one, "does it compile and how does it run": the quick profile,
        // rebuilt in under a second. Shifted is the slow one, "give me the
        // binary to ship": full release, copied beside the source. Only the
        // shifted build leaves a binary there, so a binary sitting next to
        // its .nail file is always the shippable one.
        KeyCode::F(7) if shift => Some(Action::BuildRelease),
        KeyCode::F(7) => Some(Action::Build),
        KeyCode::F(3) if shift => Some(Action::FindPrevious),
        KeyCode::F(3) => Some(Action::FindNext),
        KeyCode::F(1) => Some(Action::ToggleCompletionDetail),
        KeyCode::F(2) => Some(Action::OpenSettings),
        KeyCode::F(4) => Some(Action::ToggleMouse),
        // Where every other editor puts going to a definition, which is what
        // following an import is in a language whose files name each other.
        KeyCode::F(12) => Some(Action::OpenImportedFile),
        KeyCode::F(8) if shift => Some(Action::PreviousError),
        KeyCode::F(8) => Some(Action::NextError),
        // The answer to "the file changed on disk under my edits" has to be
        // a key the status bar can name in the moment, not a command to go
        // hunting for. Undo takes it back, which is why one keypress is safe.
        KeyCode::F(9) => Some(Action::ReloadFromDisk),
        // The whole painted frame as plain text, because error popups and
        // timing annotations exist only on screen and a drag cannot reach
        // them. Shifted, the selection instead, with what the IDE says
        // about its lines. Some terminals keep F10 for their own menu, so
        // the command palette carries both of these too.
        KeyCode::F(10) if shift => Some(Action::CopySelectionWithAnnotations),
        KeyCode::F(10) => Some(Action::CopyScreen),
        KeyCode::Tab if control && shift => Some(Action::PreviousTab),
        KeyCode::Tab if control => Some(Action::NextTab),
        // The other copy and paste keys, from before Ctrl+C settled the
        // question. They are the only way to reach copy in this editor,
        // because Ctrl+C is spoken for.
        KeyCode::Insert if control => Some(Action::Copy),
        KeyCode::Insert if shift => Some(Action::Paste),
        // Deleting a word answers to both modifiers, because which one a
        // terminal sends for Ctrl+Backspace is the terminal's opinion and not
        // the user's.
        KeyCode::Backspace if control || alt => Some(Action::DeleteWordLeft),
        KeyCode::Delete if control || alt => Some(Action::DeleteWordRight),
        // The find dialog's own switches. They are Alt keys because Ctrl with
        // the same letters is spoken for, and Alt+C, Alt+W and Alt+R are what
        // the editors that have these switches already use.
        KeyCode::Char('c') if alt => Some(Action::ToggleCaseSensitivity),
        KeyCode::Char('w') if alt => Some(Action::ToggleWholeWord),
        KeyCode::Char('r') if alt => Some(Action::ToggleRegex),
        // Growing a selection outward one syntactic step at a time, and the
        // same key with the other arrow to take a step back.
        KeyCode::Right if alt && shift => Some(Action::ExpandSelection),
        KeyCode::Left if alt && shift => Some(Action::ShrinkSelection),
        KeyCode::Up if alt => Some(Action::MoveLineUp),
        KeyCode::Down if alt => Some(Action::MoveLineDown),
        // Ctrl with an arrow scrolls the page under a cursor that stays put,
        // which is how a reader looks somewhere else without losing their place.
        KeyCode::Up if control => Some(Action::ScrollLineUp),
        KeyCode::Down if control => Some(Action::ScrollLineDown),
        KeyCode::Up => Some(Action::CursorUp { extend: shift }),
        KeyCode::Down => Some(Action::CursorDown { extend: shift }),
        KeyCode::Left if control => Some(Action::CursorWordLeft { extend: shift }),
        KeyCode::Left => Some(Action::CursorLeft { extend: shift }),
        KeyCode::Right if control => Some(Action::CursorWordRight { extend: shift }),
        KeyCode::Right => Some(Action::CursorRight { extend: shift }),
        KeyCode::Home if control => Some(Action::FileStart { extend: shift }),
        // Shift+Home extends to the line start rather than running smart home,
        // because a selection that stopped at the first non-whitespace and
        // then had to be extended again would be the slower of the two.
        KeyCode::Home if shift => Some(Action::LineStart { extend: true }),
        KeyCode::Home => Some(Action::SmartHome),
        KeyCode::End if control => Some(Action::FileEnd { extend: shift }),
        KeyCode::End => Some(Action::LineEnd { extend: shift }),
        KeyCode::PageUp => Some(Action::ScrollUp),
        KeyCode::PageDown => Some(Action::ScrollDown),
        _ => None,
    };
}

/// The emacs chords, as far as this editor can honour them.
///
/// Present: the motions, the mark and the kill ring's two useful halves, undo,
/// search, and the `C-x` file commands. Absent: `C-t`, `M-d`, registers,
/// macros and anything else that needs machinery the editor does not have.
/// A chord with nothing behind it is left unbound rather than aimed at the
/// nearest approximation, because a key that does almost the right thing is
/// worse than one that does nothing.
fn emacs(pending: Option<Prefix>, key: KeyEvent) -> Resolution {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let meta = key.modifiers.contains(KeyModifiers::ALT);

    // The second key of a C-x chord. It never falls through to the buffer:
    // C-x followed by something meaningless is a cancelled command, not text.
    if pending == Some(Prefix::ControlX) {
        return match key.code {
            KeyCode::Char('s') if control => Resolution::Run(Action::Save),
            KeyCode::Char('c') if control => Resolution::Run(Action::Quit),
            KeyCode::Char('f') if control => Resolution::Run(Action::OpenFileDialog),
            KeyCode::Char('k') => Resolution::Run(Action::CloseTab),
            KeyCode::Char('b') => Resolution::Run(Action::NextTab),
            KeyCode::Char('u') => Resolution::Run(Action::Undo),
            KeyCode::Char('h') => Resolution::Run(Action::SelectAll),
            _ => Resolution::Unbound,
        };
    }

    let action = match key.code {
        KeyCode::Char('x') if control => return Resolution::Pending(Prefix::ControlX),
        KeyCode::Char('a') if control => Some(Action::LineStart { extend: false }),
        KeyCode::Char('e') if control => Some(Action::LineEnd { extend: false }),
        KeyCode::Char('f') if control => Some(Action::CursorRight { extend: false }),
        KeyCode::Char('b') if control => Some(Action::CursorLeft { extend: false }),
        KeyCode::Char('n') if control => Some(Action::CursorDown { extend: false }),
        KeyCode::Char('p') if control => Some(Action::CursorUp { extend: false }),
        KeyCode::Char('d') if control => Some(Action::DeleteForward),
        KeyCode::Char('k') if control => Some(Action::KillToLineEnd),
        KeyCode::Char('y') if control => Some(Action::Paste),
        KeyCode::Char('w') if control => Some(Action::Cut),
        KeyCode::Char('s') if control => Some(Action::FindDialog),
        KeyCode::Char('r') if control => Some(Action::FindPrevious),
        KeyCode::Char('v') if control => Some(Action::ScrollDown),
        KeyCode::Char('g') if control => Some(Action::ClearMark),
        // Both spellings of undo. C-_ is what the terminal usually sends for
        // C-/, and which one arrives depends on the terminal, not the user.
        KeyCode::Char('/') if control => Some(Action::Undo),
        KeyCode::Char('_') if control => Some(Action::Undo),
        KeyCode::Char(' ') if control => Some(Action::SetMark),
        KeyCode::Null if control => Some(Action::SetMark),
        KeyCode::Char('w') if meta => Some(Action::Copy),
        // M-x is how emacs has always run a command by name, and this table
        // takes Ctrl+P for moving the cursor, so without it the palette would
        // be the one thing an emacs user could not reach.
        KeyCode::Char('x') if meta => Some(Action::CommandPalette),
        KeyCode::Char('%') if meta => Some(Action::ReplaceDialog),
        KeyCode::Char(';') if meta => Some(Action::ToggleComment),
        KeyCode::Char('f') if meta => Some(Action::CursorWordRight { extend: false }),
        KeyCode::Char('b') if meta => Some(Action::CursorWordLeft { extend: false }),
        KeyCode::Char('v') if meta => Some(Action::ScrollUp),
        KeyCode::Char('<') if meta => Some(Action::FileStart { extend: false }),
        KeyCode::Char('>') if meta => Some(Action::FileEnd { extend: false }),
        _ => common(key),
    };
    return match action {
        Some(action) => Resolution::Run(action),
        None => Resolution::Unbound,
    };
}

/// The vim bindings, as far as this editor can honour them.
///
/// Neovim's defaults rather than vim's, because that is where the users are
/// and because the two only disagree in one place: `Y` yanks to the end of the
/// line here, the way neovim has spelled it since 0.6, rather than yanking the
/// whole line. Everything neovim added on top of vim and this editor can
/// already do is here too: `gcc` and visual `gc` toggle comments, `gO` lists
/// what the file declares, and `]d` and `[d` walk the diagnostics, which in
/// this editor are the compiler's errors.
///
/// Present: the motions, the operators that pair with them, both halves of
/// visual mode, the four ways into insert mode, undo and redo, search, and the
/// scroll chords. Absent: counts (`3j`), registers, marks, macros, text
/// objects (`ciw`), blockwise visual and `.`, each of which needs machinery
/// the editor does not have. The rest of neovim's own defaults are the ones
/// that need a language server this editor does not talk to: `grn`, `gra`,
/// `grr` and `K` have nothing to answer with, so they answer with nothing.
///
/// A key whose vim meaning is missing is swallowed rather than left to
/// whatever the CUA table would have made of it. Ctrl+V is the reason: a
/// blockwise selection that pastes instead is the exact surprise a mode
/// indicator exists to prevent. The chords with no vim meaning at all do fall
/// through, which is how Ctrl+S still saves and F7 still builds.
///
/// The cursor is the editor's own, sitting between characters rather than on
/// one, so `$` lands after the last character rather than on it. Every other
/// key is written to suit that rather than to fight it.
fn vim(mode: VimMode, pending: Option<Prefix>, key: KeyEvent) -> Resolution {
    // Ctrl+[ is Escape by another name, and a hand that learned vim knows both
    // spellings. Escape itself is left to the editor, because it has dialogs
    // and lists to dismiss before it gets to the mode.
    if key.code == KeyCode::Char('[') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Resolution::Run(Action::EnterNormalMode);
    }

    if mode == VimMode::Insert {
        return match insert(key) {
            Some(action) => Resolution::Run(action),
            None => Resolution::Unbound,
        };
    }

    // An operator that does not get a motion it knows is a cancelled command,
    // never a letter to type.
    if let Some(prefix) = pending {
        return match after_operator(mode, prefix, key) {
            Some(resolution) => resolution,
            None => Resolution::Swallowed,
        };
    }

    if mode.is_visual() {
        if let Some(resolution) = visual(key) {
            return resolution;
        }
        // A motion is all a visual mode key may borrow from the normal table.
        // The rest of that table edits at the cursor, and doing that with a
        // selection open is not what any of those keys mean here.
        if let Some(resolution) = normal(key) {
            return match resolution {
                // Saying why a key does nothing is not an edit, so it survives
                // the filter that keeps the editing keys out of visual mode.
                Resolution::Run(Action::Unsupported(message)) => Resolution::Run(Action::Unsupported(message)),
                Resolution::Run(action) if !is_motion(action) => Resolution::Swallowed,
                resolution => resolution,
            };
        }
    } else if let Some(resolution) = normal(key) {
        return resolution;
    }

    return match cua(key) {
        Some(action) => Resolution::Run(action),
        // Escape is the way out of every mode and out of every dialog, so it
        // goes on to the editor's own cancel handling rather than dying here.
        None if key.code == KeyCode::Esc => Resolution::Unbound,
        None => Resolution::Swallowed,
    };
}

/// Insert mode is the CUA table with two chords taken back. Ctrl+W is the one
/// that matters: a vim user expects it to rub out the word behind the cursor,
/// and the CUA table would close the tab.
fn insert(key: KeyEvent) -> Option<Action> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    return match key.code {
        KeyCode::Char('w') if control => Some(Action::DeleteWordLeft),
        KeyCode::Char('u') if control => Some(Action::DeleteToLineStart),
        _ => cua(key),
    };
}

/// Normal mode: every letter is a command, and the ones that are not are
/// swallowed by the caller rather than typed.
fn normal(key: KeyEvent) -> Option<Resolution> {
    // Alt is not a vim modifier. Declining those keys here is what leaves the
    // find dialog's Alt+C, Alt+W and Alt+R reachable from normal mode.
    if key.modifiers.contains(KeyModifiers::ALT) {
        return None;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        let action = match key.code {
            KeyCode::Char('r') => Action::Redo,
            // A page and half a page are the same page here, because the
            // editor scrolls by a screen and has no half of one to offer.
            KeyCode::Char('f') | KeyCode::Char('d') => Action::ScrollDown,
            KeyCode::Char('b') | KeyCode::Char('u') => Action::ScrollUp,
            KeyCode::Char('e') => Action::ScrollLineDown,
            KeyCode::Char('y') => Action::ScrollLineUp,
            // Neovim's Ctrl+L puts the screen right and drops the search
            // highlighting, and the highlighting is the half of that a
            // terminal editor can do something about.
            KeyCode::Char('l') => Action::ClearSearchHighlight,
            // The window commands, on an editor whose windows are tabs.
            KeyCode::Char('w') => return Some(Resolution::Pending(Prefix::Window)),
            // Blockwise visual would paste if it fell through to the CUA
            // table, so it says what it is instead.
            KeyCode::Char('v') => Action::Unsupported("Blockwise visual is not built. Use v or V."),
            _ => return None,
        };
        return Some(Resolution::Run(action));
    }

    let action = match key.code {
        KeyCode::Char('h') => Action::CursorLeft { extend: false },
        KeyCode::Char('j') => Action::CursorDown { extend: false },
        KeyCode::Char('k') => Action::CursorUp { extend: false },
        KeyCode::Char('l') => Action::CursorRight { extend: false },
        // A word and a WORD are the same word here. The editor has one idea of
        // where a word ends, and it is the punctuation-aware one.
        KeyCode::Char('w') | KeyCode::Char('W') => Action::CursorWordRight { extend: false },
        KeyCode::Char('b') | KeyCode::Char('B') => Action::CursorWordLeft { extend: false },
        KeyCode::Char('0') => Action::LineStart { extend: false },
        KeyCode::Char('^') => Action::SmartHome,
        KeyCode::Char('$') => Action::LineEnd { extend: false },
        KeyCode::Char('G') => Action::FileEnd { extend: false },
        KeyCode::Char('%') => Action::JumpToMatchingBracket,
        // The two keys that would edit the file if they fell through, and the
        // two a hand reaches for without deciding to.
        KeyCode::Backspace => Action::CursorLeft { extend: false },
        KeyCode::Enter => Action::CursorDown { extend: false },
        KeyCode::Delete => Action::DeleteCharAtCursor,
        // The ways in to insert mode
        KeyCode::Char('i') => Action::EnterInsertMode,
        KeyCode::Char('I') => Action::InsertAtLineStart,
        KeyCode::Char('a') => Action::InsertAfterCursor,
        KeyCode::Char('A') => Action::InsertAtLineEnd,
        KeyCode::Char('o') => Action::OpenLineBelow,
        KeyCode::Char('O') => Action::OpenLineAbove,
        // The edits that need no motion after them
        KeyCode::Char('x') => Action::DeleteCharAtCursor,
        KeyCode::Char('X') => Action::DeleteBackward,
        KeyCode::Char('D') => Action::KillToLineEnd,
        KeyCode::Char('C') => Action::ChangeToLineEnd,
        KeyCode::Char('s') => Action::SubstituteChar,
        KeyCode::Char('S') => Action::ChangeLine,
        KeyCode::Char('J') => Action::JoinLines,
        KeyCode::Char('p') => Action::PasteAfter,
        KeyCode::Char('P') => Action::PasteBefore,
        KeyCode::Char('u') => Action::Undo,
        // Neovim's `Y`, which takes the rest of the line rather than the whole
        // of it. This is the one key where following neovim means not
        // following vim, and it is the one vim itself calls a wart.
        KeyCode::Char('Y') => Action::YankToLineEnd,
        // The operators, each waiting to hear what it applies to
        KeyCode::Char('d') => return Some(Resolution::Pending(Prefix::Delete)),
        KeyCode::Char('c') => return Some(Resolution::Pending(Prefix::Change)),
        KeyCode::Char('y') => return Some(Resolution::Pending(Prefix::Yank)),
        KeyCode::Char('g') => return Some(Resolution::Pending(Prefix::Go)),
        KeyCode::Char('>') => return Some(Resolution::Pending(Prefix::Indent)),
        KeyCode::Char('<') => return Some(Resolution::Pending(Prefix::Dedent)),
        KeyCode::Char(']') => return Some(Resolution::Pending(Prefix::NextOf)),
        KeyCode::Char('[') => return Some(Resolution::Pending(Prefix::PreviousOf)),
        KeyCode::Char('/') => Action::FindDialog,
        KeyCode::Char('n') => Action::FindNext,
        KeyCode::Char('N') => Action::FindPrevious,
        // In vim, `:` is where every command that is not a key gets typed by
        // name. That is what the palette is, and quitting is one of the names
        // in it, which is the only way out of the editor in this keymap.
        KeyCode::Char(':') => Action::CommandPalette,
        KeyCode::Char('v') => Action::EnterVisualMode,
        KeyCode::Char('V') => Action::EnterVisualLineMode,
        // `K` looks a word up, which in an editor for a language with no
        // imports is the standard library and nothing else.
        KeyCode::Char('K') => Action::StdLibBrowser,
        KeyCode::Char('Z') => return Some(Resolution::Pending(Prefix::Exit)),
        // The four famous keys behind machinery this editor does not have.
        // Each says so rather than doing nothing, because a vim user pressing
        // `.` and getting silence has no way to tell a missing feature from a
        // dropped keystroke.
        KeyCode::Char('.') => Action::Unsupported("Repeating the last change is not built."),
        KeyCode::Char('q') => Action::Unsupported("Macros are not built. Quitting is ZZ or ZQ."),
        KeyCode::Char('m') => Action::Unsupported("Marks are not built. Ctrl+T finds a symbol by name."),
        KeyCode::Char('"') => Action::Unsupported("Registers are not built. There is one clipboard."),
        _ => return None,
    };
    return Some(Resolution::Run(action));
}

/// The keys that mean something else once there is a selection. Everything not
/// named here falls back to the normal table, which is where the motions are.
fn visual(key: KeyEvent) -> Option<Resolution> {
    if key.modifiers.intersects(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) {
        return None;
    }
    let action = match key.code {
        KeyCode::Char('d') | KeyCode::Char('x') => Action::CutSelection,
        KeyCode::Char('y') => Action::YankSelection,
        KeyCode::Char('c') | KeyCode::Char('s') => Action::ChangeSelection,
        KeyCode::Char('p') | KeyCode::Char('P') => Action::PasteOverSelection,
        // Vim drops out of visual mode after indenting. Staying is the better
        // of the two here, because `>` pressed three times is what indenting
        // three levels looks like and there is no `gv` to come back with.
        KeyCode::Char('>') => Action::Indent,
        KeyCode::Char('<') => Action::Dedent,
        KeyCode::Char('v') => Action::EnterVisualMode,
        KeyCode::Char('V') => Action::EnterVisualLineMode,
        // The one motion the normal table cannot lend: smart home moves
        // without extending, and would drop the selection on its way.
        KeyCode::Char('^') => Action::LineStart { extend: true },
        // Text objects, the case switches and swapping the ends of the
        // selection are not built, and every one of these letters means
        // something destructive in the normal table.
        KeyCode::Char('i') | KeyCode::Char('a') | KeyCode::Char('o') | KeyCode::Char('O') | KeyCode::Char('r') | KeyCode::Char('u') | KeyCode::Char('U') | KeyCode::Char('~') => return Some(Resolution::Swallowed),
        _ => return None,
    };
    return Some(Resolution::Run(action));
}

/// The second key of an operator. Delete and change take the same motions and
/// differ only in where they leave the user, which is what makes `cw` a `dw`
/// that carries on typing.
///
/// One of these is an operator itself: `gc` takes a motion of its own, so in
/// normal mode it waits again and `gcc` is what comes of that. Visual mode
/// already has the lines it applies to, so there it runs at once.
fn after_operator(mode: VimMode, prefix: Prefix, key: KeyEvent) -> Option<Resolution> {
    // The window commands are the only ones that answer with control still
    // held, because Ctrl+W Ctrl+W is how a hand actually types the second one.
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    if key.modifiers.contains(KeyModifiers::ALT) || (control && prefix != Prefix::Window) {
        return None;
    }
    let action = match (prefix, key.code) {
        (Prefix::Delete, KeyCode::Char('d')) => Action::DeleteLine,
        (Prefix::Delete, KeyCode::Char('w')) => Action::DeleteWordRight,
        (Prefix::Delete, KeyCode::Char('b')) => Action::DeleteWordLeft,
        (Prefix::Delete, KeyCode::Char('$')) => Action::KillToLineEnd,
        (Prefix::Delete, KeyCode::Char('0')) => Action::DeleteToLineStart,
        (Prefix::Change, KeyCode::Char('c')) => Action::ChangeLine,
        (Prefix::Change, KeyCode::Char('w')) => Action::ChangeWord,
        (Prefix::Change, KeyCode::Char('$')) => Action::ChangeToLineEnd,
        (Prefix::Yank, KeyCode::Char('y')) => Action::YankLine,
        (Prefix::Yank, KeyCode::Char('w')) => Action::YankWord,
        (Prefix::Go, KeyCode::Char('g')) => Action::FileStart { extend: false },
        // Where every other editor puts going to a definition, which in a
        // language whose files name each other is the import under the cursor.
        (Prefix::Go, KeyCode::Char('d')) => Action::OpenImportedFile,
        (Prefix::Go, KeyCode::Char('t')) => Action::NextTab,
        (Prefix::Go, KeyCode::Char('T')) => Action::PreviousTab,
        // Neovim lists what a file declares with `gO`, which is this editor's
        // go to symbol pointed at the file the cursor is in.
        (Prefix::Go, KeyCode::Char('O')) => Action::SymbolPicker,
        (Prefix::Go, KeyCode::Char('c')) if mode.is_visual() => Action::ToggleComment,
        (Prefix::Go, KeyCode::Char('c')) => return Some(Resolution::Pending(Prefix::Comment)),
        (Prefix::Comment, KeyCode::Char('c')) => Action::ToggleComment,
        (Prefix::NextOf, KeyCode::Char('d')) => Action::NextError,
        (Prefix::PreviousOf, KeyCode::Char('d')) => Action::PreviousError,
        // The window commands that mean the same thing about a tab. The rest
        // of the family needs split windows, which this editor does not have.
        (Prefix::Window, KeyCode::Char('c')) | (Prefix::Window, KeyCode::Char('q')) => Action::CloseTab,
        (Prefix::Window, KeyCode::Char('w')) => Action::NextTab,
        (Prefix::Window, KeyCode::Char('p')) => Action::PreviousTab,
        // Both halves of leaving. `ZZ` writes the file first, `ZQ` does not,
        // and the editor asks before throwing unsaved work away.
        (Prefix::Exit, KeyCode::Char('Z')) => Action::SaveAndQuit,
        (Prefix::Exit, KeyCode::Char('Q')) => Action::Quit,
        (Prefix::Indent, KeyCode::Char('>')) => Action::Indent,
        (Prefix::Dedent, KeyCode::Char('<')) => Action::Dedent,
        _ => return None,
    };
    return Some(Resolution::Run(action));
}

/// Whether an action moves the cursor and does nothing else. Visual mode has
/// to know, because a motion grows the selection and anything else would be
/// an edit at one end of it.
///
/// Searching is not on the list even though it moves the cursor. Finding a
/// match puts the cursor on it without extending anything, so a selection that
/// survived the jump would be pointing at somewhere the user has left.
fn is_motion(action: Action) -> bool {
    return matches!(
        action,
        Action::CursorUp { .. }
            | Action::CursorDown { .. }
            | Action::CursorLeft { .. }
            | Action::CursorRight { .. }
            | Action::CursorWordLeft { .. }
            | Action::CursorWordRight { .. }
            | Action::LineStart { .. }
            | Action::LineEnd { .. }
            | Action::FileStart { .. }
            | Action::FileEnd { .. }
            | Action::SmartHome
            | Action::ScrollUp
            | Action::ScrollDown
            | Action::JumpToMatchingBracket
    );
}

/// These cover the pure halves of detection only. `detect()` itself reads
/// process wide environment variables, and cargo runs tests in threads of one
/// process, so a test that set one would change what every other test sees.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_editor_name_is_recognized() {
        assert_eq!(from_editor_value("vim"), Some(Keymap::Vim));
        assert_eq!(from_editor_value("emacs"), Some(Keymap::Emacs));
    }

    #[test]
    fn an_absolute_path_is_reduced_to_its_file_name() {
        assert_eq!(from_editor_value("/usr/bin/nvim"), Some(Keymap::Vim));
        assert_eq!(from_editor_value("/usr/local/bin/emacs"), Some(Keymap::Emacs));
    }

    #[test]
    fn arguments_after_the_command_are_ignored() {
        assert_eq!(from_editor_value("emacsclient -t"), Some(Keymap::Emacs));
        assert_eq!(from_editor_value("vim -f"), Some(Keymap::Vim));
        assert_eq!(from_editor_value("  vim   -u NONE  "), Some(Keymap::Vim));
    }

    #[test]
    fn a_non_modal_editor_is_no_signal() {
        assert_eq!(from_editor_value("nano"), None);
        assert_eq!(from_editor_value("code --wait"), None);
        assert_eq!(from_editor_value("micro"), None);
        assert_eq!(from_editor_value("helix"), None);
    }

    #[test]
    fn an_empty_editor_value_is_no_signal() {
        assert_eq!(from_editor_value(""), None);
        assert_eq!(from_editor_value("   "), None);
    }

    #[test]
    fn a_windows_style_suffix_is_stripped() {
        assert_eq!(from_editor_value("vim.exe"), Some(Keymap::Vim));
        assert_eq!(from_editor_value("C:/tools/vim/gvim.EXE"), Some(Keymap::Vim));
    }

    #[test]
    fn inputrc_vi_mode_is_read() {
        assert_eq!(from_inputrc_text("set editing-mode vi\n"), Some(Keymap::Vim));
        assert_eq!(from_inputrc_text("set bell-style none\nset editing-mode vi\nset show-mode-in-prompt on\n"), Some(Keymap::Vim));
    }

    #[test]
    fn a_commented_inputrc_line_is_no_signal() {
        assert_eq!(from_inputrc_text("# set editing-mode vi\n"), None);
        assert_eq!(from_inputrc_text("   #set editing-mode vi\n"), None);
    }

    #[test]
    fn inputrc_whitespace_is_arbitrary() {
        assert_eq!(from_inputrc_text("   set    editing-mode\tvi   \n"), Some(Keymap::Vim));
    }

    #[test]
    fn inputrc_emacs_mode_is_read() {
        assert_eq!(from_inputrc_text("set editing-mode emacs\n"), Some(Keymap::Emacs));
    }

    #[test]
    fn an_unrelated_inputrc_is_no_signal() {
        assert_eq!(from_inputrc_text(""), None);
        assert_eq!(from_inputrc_text("set completion-ignore-case on\n"), None);
        assert_eq!(from_inputrc_text("set editing-mode\n"), None);
        assert_eq!(from_inputrc_text("set editing-mode vi please\n"), None);
    }

    #[test]
    fn the_last_inputrc_setting_wins() {
        assert_eq!(from_inputrc_text("set editing-mode vi\nset editing-mode emacs\n"), Some(Keymap::Emacs));
    }

    #[test]
    fn the_keymap_variable_ignores_case() {
        assert_eq!(from_keymap_name("VIM"), Some(Keymap::Vim));
        assert_eq!(from_keymap_name("Emacs"), Some(Keymap::Emacs));
        assert_eq!(from_keymap_name("normal"), Some(Keymap::Cua));
        assert_eq!(from_keymap_name("cua"), Some(Keymap::Cua));
    }

    #[test]
    fn an_unknown_keymap_name_is_no_signal() {
        assert_eq!(from_keymap_name("nonsense"), None);
        assert_eq!(from_keymap_name(""), None);
    }

    #[test]
    fn only_the_modal_keymaps_announce_themselves() {
        assert_eq!(Keymap::Vim.label(VimMode::Normal), Some(" -- NORMAL -- "));
        assert_eq!(Keymap::Vim.label(VimMode::Insert), Some(" -- INSERT -- "));
        assert_eq!(Keymap::Emacs.label(VimMode::Normal), Some(" emacs "));
        assert_eq!(Keymap::Cua.label(VimMode::Normal), None);
    }

    fn plain(code: KeyCode) -> Option<Action> {
        return cua(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn with(code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
        return cua(KeyEvent::new(code, modifiers));
    }

    const CONTROL_SHIFT: KeyModifiers = KeyModifiers::CONTROL.union(KeyModifiers::SHIFT);

    #[test]
    fn a_control_chord_names_its_command() {
        assert_eq!(with(KeyCode::Char('s'), KeyModifiers::CONTROL), Some(Action::Save));
        assert_eq!(with(KeyCode::Char('f'), KeyModifiers::CONTROL), Some(Action::FindDialog));
        assert_eq!(with(KeyCode::Char('o'), KeyModifiers::CONTROL), Some(Action::OpenFileDialog));
        assert_eq!(with(KeyCode::Char(']'), KeyModifiers::CONTROL), Some(Action::JumpToMatchingBracket));
    }

    /// The whole reason the Ctrl+Shift arms are written first. Testing for a
    /// modifier with `contains` means a plain Ctrl arm matches a Ctrl+Shift
    /// press too, so a wrong order silently swallows six commands.
    #[test]
    fn adding_shift_reaches_a_different_command_than_control_alone() {
        assert_eq!(with(KeyCode::Char('h'), CONTROL_SHIFT), Some(Action::ToggleCurrentLineHighlight));
        assert_eq!(with(KeyCode::Char('h'), KeyModifiers::CONTROL), Some(Action::ReplaceDialog));
        assert_eq!(with(KeyCode::Char('g'), CONTROL_SHIFT), Some(Action::ToggleIndentationGuides));
        assert_eq!(with(KeyCode::Char('g'), KeyModifiers::CONTROL), Some(Action::GoToLineDialog));
        assert_eq!(with(KeyCode::Char('w'), CONTROL_SHIFT), Some(Action::ToggleWhitespace));
        assert_eq!(with(KeyCode::Char('w'), KeyModifiers::CONTROL), Some(Action::CloseTab));
    }

    /// Searching this file and searching the project are different commands
    /// on different keys, and the shifted spellings other editors use reach
    /// the project ones too, on the terminals that can send them.
    #[test]
    fn the_project_wide_searches_have_keys_of_their_own() {
        assert_eq!(with(KeyCode::Char('r'), KeyModifiers::CONTROL), Some(Action::SymbolPicker));
        assert_eq!(with(KeyCode::Char('t'), KeyModifiers::CONTROL), Some(Action::ProjectSymbolPicker));
        assert_eq!(with(KeyCode::Char('r'), CONTROL_SHIFT), Some(Action::ProjectSymbolPicker));
        assert_eq!(with(KeyCode::Char('f'), KeyModifiers::CONTROL), Some(Action::FindDialog));
        assert_eq!(with(KeyCode::Char('e'), KeyModifiers::CONTROL), Some(Action::ProjectSearch));
        assert_eq!(with(KeyCode::Char('f'), CONTROL_SHIFT), Some(Action::ProjectSearch));
    }

    #[test]
    fn following_an_import_answers_to_the_go_to_definition_key() {
        assert_eq!(plain(KeyCode::F(12)), Some(Action::OpenImportedFile));
    }

    #[test]
    fn undo_and_redo_are_told_apart_by_shift() {
        assert_eq!(with(KeyCode::Char('z'), KeyModifiers::CONTROL), Some(Action::Undo));
        assert_eq!(with(KeyCode::Char('z'), CONTROL_SHIFT), Some(Action::Redo));
        assert_eq!(with(KeyCode::Char('y'), KeyModifiers::CONTROL), Some(Action::Redo));
    }

    /// Escape is not in the table at all. It is the last step of the editor's
    /// cancel cascade, reached only once there is no dialog, no completion
    /// list and no selection left to dismiss, and even then it asks first.
    #[test]
    fn control_c_copies_and_no_chord_quits_outright() {
        assert_eq!(with(KeyCode::Char('c'), KeyModifiers::CONTROL), Some(Action::Copy));
        assert_eq!(with(KeyCode::Esc, KeyModifiers::CONTROL), None);
        assert_eq!(plain(KeyCode::Esc), None);
        // Emacs still spells it out in full, and a chord that deliberate is
        // taken at its word.
        assert_eq!(after_control_x(KeyCode::Char('c'), KeyModifiers::CONTROL), Resolution::Run(Action::Quit));
    }

    /// Everything the editor routes through its own state has to reach it
    /// untouched, so the table must decline these rather than bind them.
    #[test]
    fn text_input_and_dialog_keys_are_left_to_the_editor() {
        assert_eq!(plain(KeyCode::Char('a')), None);
        assert_eq!(plain(KeyCode::Esc), None);
        assert_eq!(plain(KeyCode::Enter), None);
        assert_eq!(plain(KeyCode::Backspace), None);
        assert_eq!(plain(KeyCode::Delete), None);
        assert_eq!(plain(KeyCode::Tab), None);
        assert_eq!(plain(KeyCode::BackTab), None);
        assert_eq!(with(KeyCode::Char('A'), KeyModifiers::SHIFT), None);
    }

    #[test]
    fn a_digit_picks_the_tab_of_that_position() {
        assert_eq!(with(KeyCode::Char('1'), KeyModifiers::CONTROL), Some(Action::SwitchToTab(0)));
        assert_eq!(with(KeyCode::Char('3'), KeyModifiers::CONTROL), Some(Action::SwitchToTab(2)));
        // Zero has no tab before it, so it stays on the first rather than
        // wrapping into an index that does not exist.
        assert_eq!(with(KeyCode::Char('0'), KeyModifiers::CONTROL), Some(Action::SwitchToTab(0)));
    }

    #[test]
    fn shift_turns_a_motion_into_a_selection() {
        assert_eq!(plain(KeyCode::Up), Some(Action::CursorUp { extend: false }));
        assert_eq!(with(KeyCode::Up, KeyModifiers::SHIFT), Some(Action::CursorUp { extend: true }));
        assert_eq!(with(KeyCode::Left, KeyModifiers::SHIFT), Some(Action::CursorLeft { extend: true }));
        assert_eq!(with(KeyCode::Left, CONTROL_SHIFT), Some(Action::CursorWordLeft { extend: true }));
        assert_eq!(with(KeyCode::Right, KeyModifiers::CONTROL), Some(Action::CursorWordRight { extend: false }));
    }

    #[test]
    fn alt_with_an_arrow_moves_the_line_rather_than_the_cursor() {
        assert_eq!(with(KeyCode::Up, KeyModifiers::ALT), Some(Action::MoveLineUp));
        assert_eq!(with(KeyCode::Down, KeyModifiers::ALT), Some(Action::MoveLineDown));
    }

    #[test]
    fn home_and_end_answer_to_three_reaches() {
        assert_eq!(plain(KeyCode::Home), Some(Action::SmartHome));
        assert_eq!(with(KeyCode::Home, KeyModifiers::SHIFT), Some(Action::LineStart { extend: true }));
        assert_eq!(with(KeyCode::Home, KeyModifiers::CONTROL), Some(Action::FileStart { extend: false }));
        assert_eq!(with(KeyCode::Home, CONTROL_SHIFT), Some(Action::FileStart { extend: true }));
        assert_eq!(plain(KeyCode::End), Some(Action::LineEnd { extend: false }));
        assert_eq!(with(KeyCode::End, KeyModifiers::CONTROL), Some(Action::FileEnd { extend: false }));
    }

    #[test]
    fn tab_cycling_needs_control_so_plain_tab_still_indents() {
        assert_eq!(with(KeyCode::Tab, KeyModifiers::CONTROL), Some(Action::NextTab));
        assert_eq!(with(KeyCode::Tab, CONTROL_SHIFT), Some(Action::PreviousTab));
    }

    fn emacs_key(code: KeyCode, modifiers: KeyModifiers) -> Resolution {
        return resolve(Keymap::Emacs, VimMode::Normal, None, KeyEvent::new(code, modifiers));
    }

    fn after_control_x(code: KeyCode, modifiers: KeyModifiers) -> Resolution {
        return resolve(Keymap::Emacs, VimMode::Normal, Some(Prefix::ControlX), KeyEvent::new(code, modifiers));
    }

    #[test]
    fn the_emacs_motions_reach_the_same_actions_as_the_cua_ones() {
        assert_eq!(emacs_key(KeyCode::Char('a'), KeyModifiers::CONTROL), Resolution::Run(Action::LineStart { extend: false }));
        assert_eq!(emacs_key(KeyCode::Char('e'), KeyModifiers::CONTROL), Resolution::Run(Action::LineEnd { extend: false }));
        assert_eq!(emacs_key(KeyCode::Char('f'), KeyModifiers::CONTROL), Resolution::Run(Action::CursorRight { extend: false }));
        assert_eq!(emacs_key(KeyCode::Char('p'), KeyModifiers::CONTROL), Resolution::Run(Action::CursorUp { extend: false }));
        assert_eq!(emacs_key(KeyCode::Char('f'), KeyModifiers::ALT), Resolution::Run(Action::CursorWordRight { extend: false }));
        assert_eq!(emacs_key(KeyCode::Char('<'), KeyModifiers::ALT), Resolution::Run(Action::FileStart { extend: false }));
    }

    #[test]
    fn the_kill_and_yank_keys_reach_cut_copy_and_paste() {
        assert_eq!(emacs_key(KeyCode::Char('w'), KeyModifiers::CONTROL), Resolution::Run(Action::Cut));
        assert_eq!(emacs_key(KeyCode::Char('w'), KeyModifiers::ALT), Resolution::Run(Action::Copy));
        assert_eq!(emacs_key(KeyCode::Char('y'), KeyModifiers::CONTROL), Resolution::Run(Action::Paste));
        assert_eq!(emacs_key(KeyCode::Char('k'), KeyModifiers::CONTROL), Resolution::Run(Action::KillToLineEnd));
    }

    #[test]
    fn control_x_waits_for_the_key_after_it() {
        assert_eq!(emacs_key(KeyCode::Char('x'), KeyModifiers::CONTROL), Resolution::Pending(Prefix::ControlX));
        assert_eq!(after_control_x(KeyCode::Char('s'), KeyModifiers::CONTROL), Resolution::Run(Action::Save));
        assert_eq!(after_control_x(KeyCode::Char('c'), KeyModifiers::CONTROL), Resolution::Run(Action::Quit));
        assert_eq!(after_control_x(KeyCode::Char('f'), KeyModifiers::CONTROL), Resolution::Run(Action::OpenFileDialog));
        assert_eq!(after_control_x(KeyCode::Char('k'), KeyModifiers::NONE), Resolution::Run(Action::CloseTab));
    }

    /// A cancelled C-x chord must not leave its second key in the buffer.
    #[test]
    fn a_meaningless_key_after_control_x_is_swallowed_rather_than_typed() {
        assert_eq!(after_control_x(KeyCode::Char('q'), KeyModifiers::NONE), Resolution::Unbound);
        assert_eq!(after_control_x(KeyCode::Esc, KeyModifiers::NONE), Resolution::Unbound);
    }

    /// Without this, C-s in emacs mode would save rather than search, because
    /// the CUA meaning of the chord would still be reachable.
    #[test]
    fn an_emacs_chord_is_not_also_read_as_its_cua_meaning() {
        assert_eq!(emacs_key(KeyCode::Char('s'), KeyModifiers::CONTROL), Resolution::Run(Action::FindDialog));
        assert_eq!(emacs_key(KeyCode::Char('n'), KeyModifiers::CONTROL), Resolution::Run(Action::CursorDown { extend: false }));
        assert_eq!(emacs_key(KeyCode::Char('v'), KeyModifiers::CONTROL), Resolution::Run(Action::ScrollDown));
        assert_eq!(emacs_key(KeyCode::Char('d'), KeyModifiers::CONTROL), Resolution::Run(Action::DeleteForward));
    }

    #[test]
    fn the_mark_is_set_by_either_spelling_of_control_space() {
        assert_eq!(emacs_key(KeyCode::Char(' '), KeyModifiers::CONTROL), Resolution::Run(Action::SetMark));
        assert_eq!(emacs_key(KeyCode::Null, KeyModifiers::CONTROL), Resolution::Run(Action::SetMark));
        assert_eq!(emacs_key(KeyCode::Char('g'), KeyModifiers::CONTROL), Resolution::Run(Action::ClearMark));
    }

    /// An emacs user still has arrow keys and still expects them to work.
    #[test]
    fn the_shared_keys_survive_in_both_keymaps() {
        assert_eq!(emacs_key(KeyCode::Up, KeyModifiers::NONE), Resolution::Run(Action::CursorUp { extend: false }));
        assert_eq!(emacs_key(KeyCode::Home, KeyModifiers::NONE), Resolution::Run(Action::SmartHome));
        assert_eq!(emacs_key(KeyCode::F(7), KeyModifiers::NONE), Resolution::Run(Action::Build));
        assert_eq!(emacs_key(KeyCode::F(7), KeyModifiers::SHIFT), Resolution::Run(Action::BuildRelease));
        assert_eq!(emacs_key(KeyCode::Char('a'), KeyModifiers::NONE), Resolution::Unbound);
    }

    #[test]
    fn copy_answers_to_control_insert_in_every_keymap() {
        assert_eq!(with(KeyCode::Insert, KeyModifiers::CONTROL), Some(Action::Copy));
        assert_eq!(with(KeyCode::Insert, KeyModifiers::SHIFT), Some(Action::Paste));
        assert_eq!(emacs_key(KeyCode::Insert, KeyModifiers::CONTROL), Resolution::Run(Action::Copy));
    }

    /// A chord vim never claimed still reaches the command every other keymap
    /// puts it on, in every mode. This is what keeps saving, building and the
    /// pickers one key away from a vim user rather than three.
    #[test]
    fn the_chords_vim_never_claimed_still_reach_their_commands() {
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('s'), KeyModifiers::CONTROL), Resolution::Run(Action::Save));
        assert_eq!(vim_key(VimMode::Insert, KeyCode::Char('s'), KeyModifiers::CONTROL), Resolution::Run(Action::Save));
        assert_eq!(vim_key(VimMode::Visual, KeyCode::Char('s'), KeyModifiers::CONTROL), Resolution::Run(Action::Save));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::F(7), KeyModifiers::NONE), Resolution::Run(Action::Build));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::F(7), KeyModifiers::SHIFT), Resolution::Run(Action::BuildRelease));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('p'), KeyModifiers::CONTROL), Resolution::Run(Action::CommandPalette));
        assert_eq!(resolve(Keymap::Cua, VimMode::Normal, None, KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)), Resolution::Unbound);
    }

    #[test]
    fn the_function_keys_keep_their_commands() {
        assert_eq!(plain(KeyCode::F(3)), Some(Action::FindNext));
        assert_eq!(with(KeyCode::F(3), KeyModifiers::SHIFT), Some(Action::FindPrevious));
        assert_eq!(plain(KeyCode::F(5)), Some(Action::CycleExampleFiles));
        assert_eq!(plain(KeyCode::F(6)), Some(Action::ToggleTheme));
        assert_eq!(plain(KeyCode::F(7)), Some(Action::Build));
        assert_eq!(with(KeyCode::F(7), KeyModifiers::SHIFT), Some(Action::BuildRelease));
        assert_eq!(plain(KeyCode::F(1)), Some(Action::ToggleCompletionDetail));
        assert_eq!(plain(KeyCode::F(2)), Some(Action::OpenSettings));
        assert_eq!(emacs_key(KeyCode::F(2), KeyModifiers::NONE), Resolution::Run(Action::OpenSettings));
        assert_eq!(plain(KeyCode::F(4)), Some(Action::ToggleMouse));
        assert_eq!(plain(KeyCode::F(8)), Some(Action::NextError));
        assert_eq!(with(KeyCode::F(8), KeyModifiers::SHIFT), Some(Action::PreviousError));
        assert_eq!(plain(KeyCode::F(9)), Some(Action::ReloadFromDisk));
    }

    /// Backspace and Delete are text input until a modifier is held, and both
    /// modifiers have to work because which one a terminal sends for
    /// Ctrl+Backspace is the terminal's opinion rather than the user's.
    #[test]
    fn a_modifier_turns_a_delete_into_a_word() {
        assert_eq!(plain(KeyCode::Backspace), None);
        assert_eq!(with(KeyCode::Backspace, KeyModifiers::CONTROL), Some(Action::DeleteWordLeft));
        assert_eq!(with(KeyCode::Backspace, KeyModifiers::ALT), Some(Action::DeleteWordLeft));
        assert_eq!(with(KeyCode::Delete, KeyModifiers::CONTROL), Some(Action::DeleteWordRight));
        assert_eq!(emacs_key(KeyCode::Backspace, KeyModifiers::ALT), Resolution::Run(Action::DeleteWordLeft));
    }

    #[test]
    fn the_pickers_answer_to_their_own_keys() {
        assert_eq!(with(KeyCode::Char('p'), KeyModifiers::CONTROL), Some(Action::CommandPalette));
        assert_eq!(with(KeyCode::Char('p'), CONTROL_SHIFT), Some(Action::StdLibBrowser));
        assert_eq!(with(KeyCode::Char('r'), KeyModifiers::CONTROL), Some(Action::SymbolPicker));
        assert_eq!(with(KeyCode::Char('j'), KeyModifiers::CONTROL), Some(Action::JoinLines));
    }

    /// Ctrl+Shift+P reaches almost no terminal: it arrives as plain Ctrl+P,
    /// which is the palette, so the browser had a key that never opened it.
    /// The plain chord is the one that has to keep working.
    #[test]
    fn the_standard_library_opens_on_a_key_a_terminal_can_send() {
        assert_eq!(with(KeyCode::Char('b'), KeyModifiers::CONTROL), Some(Action::StdLibBrowser));
        let named = COMMANDS.iter().find(|command| command.action == Action::StdLibBrowser).expect("the browser is in the palette");
        assert_eq!(named.keys.for_keymap(Keymap::Cua), "Ctrl+B");
    }

    /// Alt with an arrow already moved a line, so growing a selection has to
    /// be told apart by shift, and both have to keep working.
    #[test]
    fn alt_and_shift_together_grow_the_selection() {
        const ALT_SHIFT: KeyModifiers = KeyModifiers::ALT.union(KeyModifiers::SHIFT);
        assert_eq!(with(KeyCode::Right, ALT_SHIFT), Some(Action::ExpandSelection));
        assert_eq!(with(KeyCode::Left, ALT_SHIFT), Some(Action::ShrinkSelection));
        assert_eq!(with(KeyCode::Up, KeyModifiers::ALT), Some(Action::MoveLineUp));
        assert_eq!(with(KeyCode::Right, KeyModifiers::ALT), Some(Action::CursorRight { extend: false }));
    }

    #[test]
    fn control_with_a_vertical_arrow_scrolls_instead_of_moving() {
        assert_eq!(with(KeyCode::Up, KeyModifiers::CONTROL), Some(Action::ScrollLineUp));
        assert_eq!(with(KeyCode::Down, KeyModifiers::CONTROL), Some(Action::ScrollLineDown));
        assert_eq!(plain(KeyCode::Up), Some(Action::CursorUp { extend: false }));
    }

    /// The find switches are Alt keys, and emacs' own M-w has to survive
    /// having one of those letters taken.
    #[test]
    fn the_search_switches_do_not_take_a_key_emacs_wanted() {
        assert_eq!(with(KeyCode::Char('c'), KeyModifiers::ALT), Some(Action::ToggleCaseSensitivity));
        assert_eq!(with(KeyCode::Char('w'), KeyModifiers::ALT), Some(Action::ToggleWholeWord));
        assert_eq!(with(KeyCode::Char('r'), KeyModifiers::ALT), Some(Action::ToggleRegex));
        assert_eq!(emacs_key(KeyCode::Char('w'), KeyModifiers::ALT), Resolution::Run(Action::Copy));
    }

    /// The palette is a list of names for things that already have keys, so a
    /// name that appears twice is two rows doing one job, and a key hint that
    /// no longer reaches its command is a lie the user cannot check.
    fn vim_key(mode: VimMode, code: KeyCode, modifiers: KeyModifiers) -> Resolution {
        return resolve(Keymap::Vim, mode, None, KeyEvent::new(code, modifiers));
    }

    fn vim_letter(mode: VimMode, letter: char) -> Resolution {
        // An upper case letter arrives with shift held, which is how a
        // terminal spells it and how the table has to read it.
        let modifiers = if letter.is_uppercase() { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
        return resolve(Keymap::Vim, mode, None, KeyEvent::new(KeyCode::Char(letter), modifiers));
    }

    fn after_operator_key(prefix: Prefix, letter: char) -> Resolution {
        let modifiers = if letter.is_uppercase() { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
        return resolve(Keymap::Vim, VimMode::Normal, Some(prefix), KeyEvent::new(KeyCode::Char(letter), modifiers));
    }

    #[test]
    fn the_home_row_moves_the_cursor_in_normal_mode() {
        assert_eq!(vim_letter(VimMode::Normal, 'h'), Resolution::Run(Action::CursorLeft { extend: false }));
        assert_eq!(vim_letter(VimMode::Normal, 'j'), Resolution::Run(Action::CursorDown { extend: false }));
        assert_eq!(vim_letter(VimMode::Normal, 'k'), Resolution::Run(Action::CursorUp { extend: false }));
        assert_eq!(vim_letter(VimMode::Normal, 'l'), Resolution::Run(Action::CursorRight { extend: false }));
        assert_eq!(vim_letter(VimMode::Normal, 'w'), Resolution::Run(Action::CursorWordRight { extend: false }));
        assert_eq!(vim_letter(VimMode::Normal, 'b'), Resolution::Run(Action::CursorWordLeft { extend: false }));
        assert_eq!(vim_letter(VimMode::Normal, '$'), Resolution::Run(Action::LineEnd { extend: false }));
        assert_eq!(vim_letter(VimMode::Normal, '0'), Resolution::Run(Action::LineStart { extend: false }));
        assert_eq!(vim_letter(VimMode::Normal, 'G'), Resolution::Run(Action::FileEnd { extend: false }));
    }

    /// The whole reason normal mode needs a resolution of its own. A letter
    /// with no command behind it must not reach the buffer, or every typo in
    /// normal mode would be an edit.
    #[test]
    fn a_letter_with_no_command_is_swallowed_rather_than_typed() {
        assert_eq!(vim_letter(VimMode::Normal, 'z'), Resolution::Swallowed);
        assert_eq!(vim_letter(VimMode::Normal, 'f'), Resolution::Swallowed);
        assert_eq!(vim_letter(VimMode::Normal, '9'), Resolution::Swallowed);
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Tab, KeyModifiers::NONE), Resolution::Swallowed);
    }

    /// Backspace and Enter edit the file everywhere else in this editor, so
    /// normal mode has to claim them before they get the chance.
    #[test]
    fn the_editing_keys_only_move_in_normal_mode() {
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Backspace, KeyModifiers::NONE), Resolution::Run(Action::CursorLeft { extend: false }));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Enter, KeyModifiers::NONE), Resolution::Run(Action::CursorDown { extend: false }));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Delete, KeyModifiers::NONE), Resolution::Run(Action::DeleteCharAtCursor));
    }

    #[test]
    fn the_four_ways_into_insert_mode_are_told_apart() {
        assert_eq!(vim_letter(VimMode::Normal, 'i'), Resolution::Run(Action::EnterInsertMode));
        assert_eq!(vim_letter(VimMode::Normal, 'I'), Resolution::Run(Action::InsertAtLineStart));
        assert_eq!(vim_letter(VimMode::Normal, 'a'), Resolution::Run(Action::InsertAfterCursor));
        assert_eq!(vim_letter(VimMode::Normal, 'A'), Resolution::Run(Action::InsertAtLineEnd));
        assert_eq!(vim_letter(VimMode::Normal, 'o'), Resolution::Run(Action::OpenLineBelow));
        assert_eq!(vim_letter(VimMode::Normal, 'O'), Resolution::Run(Action::OpenLineAbove));
    }

    /// Insert mode is where typing has to reach the buffer, which is the one
    /// thing normal mode may never do.
    #[test]
    fn insert_mode_leaves_typing_alone() {
        assert_eq!(vim_letter(VimMode::Insert, 'q'), Resolution::Unbound);
        assert_eq!(vim_letter(VimMode::Insert, 'i'), Resolution::Unbound);
        assert_eq!(vim_key(VimMode::Insert, KeyCode::Enter, KeyModifiers::NONE), Resolution::Unbound);
        assert_eq!(vim_key(VimMode::Insert, KeyCode::Backspace, KeyModifiers::NONE), Resolution::Unbound);
        assert_eq!(vim_key(VimMode::Insert, KeyCode::Tab, KeyModifiers::NONE), Resolution::Unbound);
    }

    /// Ctrl+W in insert mode is the one chord where the CUA meaning would be a
    /// disaster: a vim user rubbing out a word would close the tab instead.
    #[test]
    fn the_insert_mode_chords_are_taken_back_from_the_cua_table() {
        assert_eq!(vim_key(VimMode::Insert, KeyCode::Char('w'), KeyModifiers::CONTROL), Resolution::Run(Action::DeleteWordLeft));
        assert_eq!(vim_key(VimMode::Insert, KeyCode::Char('u'), KeyModifiers::CONTROL), Resolution::Run(Action::DeleteToLineStart));
        assert_eq!(with(KeyCode::Char('w'), KeyModifiers::CONTROL), Some(Action::CloseTab));
    }

    /// Escape is the way out of every mode, and it is also how a dialog and a
    /// completion list get dismissed, so it has to reach the editor's own
    /// cancel handling rather than resolve to anything here.
    #[test]
    fn escape_is_left_to_the_editor_in_every_vim_mode() {
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Esc, KeyModifiers::NONE), Resolution::Unbound);
        assert_eq!(vim_key(VimMode::Insert, KeyCode::Esc, KeyModifiers::NONE), Resolution::Unbound);
        assert_eq!(vim_key(VimMode::Visual, KeyCode::Esc, KeyModifiers::NONE), Resolution::Unbound);
        assert_eq!(vim_key(VimMode::VisualLine, KeyCode::Esc, KeyModifiers::NONE), Resolution::Unbound);
    }

    /// The other spelling of Escape, which does resolve here because there is
    /// no dialog it could have been aimed at.
    #[test]
    fn control_bracket_leaves_the_mode_the_way_escape_does() {
        assert_eq!(vim_key(VimMode::Insert, KeyCode::Char('['), KeyModifiers::CONTROL), Resolution::Run(Action::EnterNormalMode));
        assert_eq!(vim_key(VimMode::Visual, KeyCode::Char('['), KeyModifiers::CONTROL), Resolution::Run(Action::EnterNormalMode));
    }

    #[test]
    fn an_operator_waits_for_the_key_after_it() {
        assert_eq!(vim_letter(VimMode::Normal, 'd'), Resolution::Pending(Prefix::Delete));
        assert_eq!(vim_letter(VimMode::Normal, 'c'), Resolution::Pending(Prefix::Change));
        assert_eq!(vim_letter(VimMode::Normal, 'y'), Resolution::Pending(Prefix::Yank));
        assert_eq!(vim_letter(VimMode::Normal, 'g'), Resolution::Pending(Prefix::Go));
        assert_eq!(after_operator_key(Prefix::Delete, 'd'), Resolution::Run(Action::DeleteLine));
        assert_eq!(after_operator_key(Prefix::Delete, 'w'), Resolution::Run(Action::DeleteWordRight));
        assert_eq!(after_operator_key(Prefix::Delete, '$'), Resolution::Run(Action::KillToLineEnd));
        assert_eq!(after_operator_key(Prefix::Change, 'c'), Resolution::Run(Action::ChangeLine));
        assert_eq!(after_operator_key(Prefix::Change, 'w'), Resolution::Run(Action::ChangeWord));
        assert_eq!(after_operator_key(Prefix::Yank, 'y'), Resolution::Run(Action::YankLine));
        assert_eq!(after_operator_key(Prefix::Go, 'g'), Resolution::Run(Action::FileStart { extend: false }));
        assert_eq!(after_operator_key(Prefix::Go, 'd'), Resolution::Run(Action::OpenImportedFile));
        assert_eq!(after_operator_key(Prefix::Go, 't'), Resolution::Run(Action::NextTab));
        assert_eq!(after_operator_key(Prefix::Indent, '>'), Resolution::Run(Action::Indent));
    }

    /// The keys neovim added on top of vim that this editor can answer. `gc`
    /// is an operator behind an operator, so it waits twice in normal mode and
    /// once in visual mode, where it already knows what it applies to.
    #[test]
    fn the_neovim_defaults_reach_what_the_editor_has_for_them() {
        assert_eq!(vim_letter(VimMode::Normal, 'Y'), Resolution::Run(Action::YankToLineEnd));
        assert_eq!(after_operator_key(Prefix::Go, 'c'), Resolution::Pending(Prefix::Comment));
        assert_eq!(after_operator_key(Prefix::Comment, 'c'), Resolution::Run(Action::ToggleComment));
        assert_eq!(resolve(Keymap::Vim, VimMode::Visual, Some(Prefix::Go), KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)), Resolution::Run(Action::ToggleComment));
        assert_eq!(after_operator_key(Prefix::Go, 'O'), Resolution::Run(Action::SymbolPicker));
        assert_eq!(vim_letter(VimMode::Normal, ']'), Resolution::Pending(Prefix::NextOf));
        assert_eq!(vim_letter(VimMode::Normal, '['), Resolution::Pending(Prefix::PreviousOf));
        assert_eq!(after_operator_key(Prefix::NextOf, 'd'), Resolution::Run(Action::NextError));
        assert_eq!(after_operator_key(Prefix::PreviousOf, 'd'), Resolution::Run(Action::PreviousError));
    }

    /// Vim has no Escape that quits, so it has to have the two keys it has
    /// always used for leaving, and the editor still asks before throwing
    /// unsaved work away on the one that does not write first.
    #[test]
    fn both_of_vims_ways_out_are_two_deliberate_keys() {
        assert_eq!(vim_letter(VimMode::Normal, 'Z'), Resolution::Pending(Prefix::Exit));
        assert_eq!(after_operator_key(Prefix::Exit, 'Z'), Resolution::Run(Action::SaveAndQuit));
        assert_eq!(after_operator_key(Prefix::Exit, 'Q'), Resolution::Run(Action::Quit));
        assert_eq!(after_operator_key(Prefix::Exit, 'x'), Resolution::Swallowed);
    }

    /// This editor has tabs where neovim has windows, so the three window
    /// commands that mean the same thing about a tab answer, and Ctrl+W stops
    /// being the key that closed a tab by surprise.
    #[test]
    fn the_window_prefix_reaches_the_tab_commands() {
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('w'), KeyModifiers::CONTROL), Resolution::Pending(Prefix::Window));
        assert_eq!(after_operator_key(Prefix::Window, 'c'), Resolution::Run(Action::CloseTab));
        assert_eq!(after_operator_key(Prefix::Window, 'q'), Resolution::Run(Action::CloseTab));
        assert_eq!(after_operator_key(Prefix::Window, 'p'), Resolution::Run(Action::PreviousTab));
        // Ctrl+W Ctrl+W is how the hand types it, so control still being held
        // for the second key cannot cancel the command.
        assert_eq!(resolve(Keymap::Vim, VimMode::Normal, Some(Prefix::Window), KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL)), Resolution::Run(Action::NextTab));
    }

    /// A key vim gives a meaning this editor cannot honour says so. Silence
    /// leaves the user unable to tell a missing feature from a dropped key.
    #[test]
    fn the_keys_behind_missing_machinery_say_what_is_missing() {
        let spoken = |mode, letter| match vim_letter(mode, letter) {
            Resolution::Run(Action::Unsupported(message)) => message,
            other => panic!("expected an explanation, got {:?}", other),
        };
        assert!(spoken(VimMode::Normal, '.').contains("Repeating"));
        assert!(spoken(VimMode::Normal, 'q').contains("Macros"));
        assert!(spoken(VimMode::Normal, 'm').contains("Marks"));
        assert!(spoken(VimMode::Normal, '"').contains("Registers"));
        match vim_key(VimMode::Normal, KeyCode::Char('v'), KeyModifiers::CONTROL) {
            Resolution::Run(Action::Unsupported(message)) => assert!(message.contains("Blockwise")),
            other => panic!("expected an explanation, got {:?}", other),
        }
    }

    #[test]
    fn the_search_highlighting_has_a_key_to_put_it_out() {
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('l'), KeyModifiers::CONTROL), Resolution::Run(Action::ClearSearchHighlight));
        // Which is the CUA key for something else, so that keymap keeps it.
        assert_eq!(with(KeyCode::Char('l'), KeyModifiers::CONTROL), Some(Action::ToggleLineNumbers));
    }

    /// Looking a word up is what `K` is for, and in a language with no imports
    /// the only place a word can be looked up is the standard library.
    #[test]
    fn shift_k_opens_the_library() {
        assert_eq!(vim_letter(VimMode::Normal, 'K'), Resolution::Run(Action::StdLibBrowser));
    }

    /// Emacs takes Ctrl+P for the cursor, so without M-x the list of every
    /// command by name would be the one thing that keymap could not reach.
    #[test]
    fn the_emacs_meta_keys_reach_the_commands_that_lost_their_chord() {
        assert_eq!(emacs_key(KeyCode::Char('x'), KeyModifiers::ALT), Resolution::Run(Action::CommandPalette));
        assert_eq!(emacs_key(KeyCode::Char('%'), KeyModifiers::ALT), Resolution::Run(Action::ReplaceDialog));
        assert_eq!(emacs_key(KeyCode::Char(';'), KeyModifiers::ALT), Resolution::Run(Action::ToggleComment));
    }

    /// The palette hint is the only place a user finds out which key runs a
    /// command, so it has to name the key their keymap actually uses.
    #[test]
    fn a_command_names_the_key_of_the_keymap_that_is_on() {
        let find = COMMANDS.iter().find(|command| command.action == Action::FindDialog).expect("find is in the palette");
        assert_eq!(find.keys.for_keymap(Keymap::Cua), "Ctrl+F");
        assert_eq!(find.keys.for_keymap(Keymap::Vim), "/");
        assert_eq!(find.keys.for_keymap(Keymap::Emacs), "Ctrl+S");
        let build = COMMANDS.iter().find(|command| command.action == Action::Build).expect("build is in the palette");
        assert_eq!(build.keys.for_keymap(Keymap::Vim), build.keys.for_keymap(Keymap::Cua));
        // A command with no key under a keymap says so with an empty hint
        // rather than naming a chord that reaches something else there.
        let numbers = COMMANDS.iter().find(|command| command.action == Action::ToggleLineNumbers).expect("line numbers are in the palette");
        assert_eq!(numbers.keys.for_keymap(Keymap::Vim), "");
    }

    /// An operator that never hears a motion it knows is a cancelled command.
    /// Letting its second key through would type that letter into the file.
    #[test]
    fn a_motion_an_operator_does_not_know_cancels_it() {
        assert_eq!(after_operator_key(Prefix::Delete, 'q'), Resolution::Swallowed);
        assert_eq!(after_operator_key(Prefix::Yank, 'd'), Resolution::Swallowed);
        assert_eq!(resolve(Keymap::Vim, VimMode::Normal, Some(Prefix::Change), KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)), Resolution::Swallowed);
    }

    #[test]
    fn visual_mode_turns_the_operators_on_the_selection() {
        assert_eq!(vim_letter(VimMode::Visual, 'd'), Resolution::Run(Action::CutSelection));
        assert_eq!(vim_letter(VimMode::Visual, 'x'), Resolution::Run(Action::CutSelection));
        assert_eq!(vim_letter(VimMode::Visual, 'y'), Resolution::Run(Action::YankSelection));
        assert_eq!(vim_letter(VimMode::Visual, 'c'), Resolution::Run(Action::ChangeSelection));
        assert_eq!(vim_letter(VimMode::Visual, 'p'), Resolution::Run(Action::PasteOverSelection));
        assert_eq!(vim_letter(VimMode::VisualLine, '>'), Resolution::Run(Action::Indent));
        assert_eq!(vim_letter(VimMode::VisualLine, '<'), Resolution::Run(Action::Dedent));
    }

    /// Motions are all visual mode borrows from the normal table. Everything
    /// else there edits at the cursor, and `o` opening a line under a
    /// selection is not what a vim user pressing it means.
    #[test]
    fn visual_mode_keeps_the_motions_and_drops_the_edits() {
        assert_eq!(vim_letter(VimMode::Visual, 'j'), Resolution::Run(Action::CursorDown { extend: false }));
        assert_eq!(vim_letter(VimMode::Visual, 'w'), Resolution::Run(Action::CursorWordRight { extend: false }));
        assert_eq!(vim_letter(VimMode::Visual, 'G'), Resolution::Run(Action::FileEnd { extend: false }));
        assert_eq!(vim_letter(VimMode::Visual, '^'), Resolution::Run(Action::LineStart { extend: true }));
        assert_eq!(vim_letter(VimMode::Visual, 'o'), Resolution::Swallowed);
        assert_eq!(vim_letter(VimMode::Visual, 'i'), Resolution::Swallowed);
        assert_eq!(vim_letter(VimMode::Visual, 'a'), Resolution::Swallowed);
        assert_eq!(vim_letter(VimMode::Visual, 'A'), Resolution::Swallowed);
        assert_eq!(vim_letter(VimMode::Visual, 'u'), Resolution::Swallowed);
    }

    #[test]
    fn the_visual_modes_reach_each_other_and_normal_mode() {
        assert_eq!(vim_letter(VimMode::Normal, 'v'), Resolution::Run(Action::EnterVisualMode));
        assert_eq!(vim_letter(VimMode::Normal, 'V'), Resolution::Run(Action::EnterVisualLineMode));
        assert_eq!(vim_letter(VimMode::Visual, 'V'), Resolution::Run(Action::EnterVisualLineMode));
        assert_eq!(vim_letter(VimMode::VisualLine, 'v'), Resolution::Run(Action::EnterVisualMode));
    }

    /// Both of these mean something in vim that the CUA table would answer
    /// with something else entirely. Pasting when the user asked for a block
    /// selection, or closing a tab when they asked for a window, is the
    /// surprise a mode indicator exists to prevent.
    #[test]
    fn the_chords_vim_spells_differently_never_reach_their_cua_meaning() {
        assert!(matches!(vim_key(VimMode::Normal, KeyCode::Char('v'), KeyModifiers::CONTROL), Resolution::Run(Action::Unsupported(_))));
        assert!(matches!(vim_key(VimMode::Visual, KeyCode::Char('v'), KeyModifiers::CONTROL), Resolution::Run(Action::Unsupported(_))));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('w'), KeyModifiers::CONTROL), Resolution::Pending(Prefix::Window));
        assert_eq!(with(KeyCode::Char('w'), KeyModifiers::CONTROL), Some(Action::CloseTab));
    }

    #[test]
    fn the_scroll_and_undo_chords_are_vims_own() {
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('r'), KeyModifiers::CONTROL), Resolution::Run(Action::Redo));
        assert_eq!(vim_letter(VimMode::Normal, 'u'), Resolution::Run(Action::Undo));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('d'), KeyModifiers::CONTROL), Resolution::Run(Action::ScrollDown));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('u'), KeyModifiers::CONTROL), Resolution::Run(Action::ScrollUp));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('e'), KeyModifiers::CONTROL), Resolution::Run(Action::ScrollLineDown));
    }

    /// The find switches are Alt keys, and normal mode has to let them past to
    /// keep them reachable from a keymap where every plain letter is spoken
    /// for.
    #[test]
    fn alt_keys_are_not_vims_and_reach_the_search_switches() {
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('c'), KeyModifiers::ALT), Resolution::Run(Action::ToggleCaseSensitivity));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Char('w'), KeyModifiers::ALT), Resolution::Run(Action::ToggleWholeWord));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Up, KeyModifiers::ALT), Resolution::Run(Action::MoveLineUp));
    }

    /// A vim user still has arrow keys, and the search keys still mean search.
    #[test]
    fn the_shared_keys_survive_in_vim_too() {
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Up, KeyModifiers::NONE), Resolution::Run(Action::CursorUp { extend: false }));
        assert_eq!(vim_key(VimMode::Normal, KeyCode::Home, KeyModifiers::NONE), Resolution::Run(Action::SmartHome));
        assert_eq!(vim_letter(VimMode::Normal, '/'), Resolution::Run(Action::FindDialog));
        assert_eq!(vim_letter(VimMode::Normal, 'n'), Resolution::Run(Action::FindNext));
        assert_eq!(vim_letter(VimMode::Normal, 'N'), Resolution::Run(Action::FindPrevious));
        // The command line is the palette, which is where quitting is named.
        assert_eq!(vim_letter(VimMode::Normal, ':'), Resolution::Run(Action::CommandPalette));
    }

    #[test]
    fn every_command_is_named_once() {
        let mut names: Vec<&str> = COMMANDS.iter().map(|command| command.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
        assert!(COMMANDS.iter().all(|command| !command.name.is_empty()));
    }
}
