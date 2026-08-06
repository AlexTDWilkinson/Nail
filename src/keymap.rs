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
    /// Vim says what it actually does rather than what it is called. There is
    /// no vim table yet, so a detected vim user is getting CUA keys, and a
    /// bar reading `-- VIM --` over CUA behaviour would be the one thing a
    /// mode indicator exists to prevent.
    pub fn label(&self) -> Option<&'static str> {
        return match self {
            Keymap::Cua => None,
            Keymap::Vim => Some(" vim (cua keys for now) "),
            Keymap::Emacs => Some(" emacs "),
        };
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
        "cua" => Some(Keymap::Cua),
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
    CycleExampleFiles,
    ToggleTheme,
    Build,
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
}

/// One row of the command palette: what the command is called, which keys
/// also reach it, and what it does.
pub struct Command {
    pub name: &'static str,
    pub keys: &'static str,
    pub action: Action,
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
    Command { name: "Save file", keys: "Ctrl+S", action: Action::Save },
    Command { name: "Open file", keys: "Ctrl+O", action: Action::OpenFileDialog },
    Command { name: "Build and run", keys: "F7", action: Action::Build },
    Command { name: "Go to line", keys: "Ctrl+G", action: Action::GoToLineDialog },
    Command { name: "Go to symbol", keys: "Ctrl+R", action: Action::SymbolPicker },
    Command { name: "Go to symbol in project", keys: "Ctrl+T", action: Action::ProjectSymbolPicker },
    Command { name: "Search the project", keys: "Ctrl+E", action: Action::ProjectSearch },
    Command { name: "Go to imported file", keys: "F12", action: Action::OpenImportedFile },
    Command { name: "Find", keys: "Ctrl+F", action: Action::FindDialog },
    Command { name: "Find and replace", keys: "Ctrl+H", action: Action::ReplaceDialog },
    Command { name: "Find next", keys: "F3", action: Action::FindNext },
    Command { name: "Find previous", keys: "Shift+F3", action: Action::FindPrevious },
    Command { name: "Next error", keys: "F8", action: Action::NextError },
    Command { name: "Previous error", keys: "Shift+F8", action: Action::PreviousError },
    Command { name: "Standard library browser", keys: "Ctrl+Shift+P", action: Action::StdLibBrowser },
    Command { name: "New tab", keys: "Ctrl+N", action: Action::NewTab },
    Command { name: "Close tab", keys: "Ctrl+W", action: Action::CloseTab },
    Command { name: "Next tab", keys: "Ctrl+Tab", action: Action::NextTab },
    Command { name: "Previous tab", keys: "Ctrl+Shift+Tab", action: Action::PreviousTab },
    Command { name: "Undo", keys: "Ctrl+Z", action: Action::Undo },
    Command { name: "Redo", keys: "Ctrl+Y", action: Action::Redo },
    Command { name: "Cut", keys: "Ctrl+X", action: Action::Cut },
    Command { name: "Copy", keys: "Ctrl+C", action: Action::Copy },
    Command { name: "Paste", keys: "Ctrl+V", action: Action::Paste },
    Command { name: "Select all", keys: "Ctrl+A", action: Action::SelectAll },
    Command { name: "Expand selection", keys: "Shift+Alt+Right", action: Action::ExpandSelection },
    Command { name: "Shrink selection", keys: "Shift+Alt+Left", action: Action::ShrinkSelection },
    Command { name: "Toggle comment", keys: "Ctrl+/", action: Action::ToggleComment },
    Command { name: "Duplicate line", keys: "Ctrl+D", action: Action::DuplicateLine },
    Command { name: "Delete line", keys: "Ctrl+Shift+K", action: Action::DeleteLine },
    Command { name: "Move line up", keys: "Alt+Up", action: Action::MoveLineUp },
    Command { name: "Move line down", keys: "Alt+Down", action: Action::MoveLineDown },
    Command { name: "Join lines", keys: "Ctrl+J", action: Action::JoinLines },
    Command { name: "Sort lines", keys: "", action: Action::SortLines },
    // Alt+Backspace rather than Ctrl+Backspace, which most terminals cannot
    // tell apart from Ctrl+H and so send as that instead. The Ctrl binding is
    // still there for the terminals that report it properly, but the hint
    // names the one that works everywhere.
    Command { name: "Delete word left", keys: "Alt+Backspace", action: Action::DeleteWordLeft },
    Command { name: "Delete word right", keys: "Ctrl+Delete", action: Action::DeleteWordRight },
    Command { name: "Jump to matching bracket", keys: "Ctrl+]", action: Action::JumpToMatchingBracket },
    Command { name: "Toggle theme", keys: "F6", action: Action::ToggleTheme },
    Command { name: "Toggle mouse", keys: "F4", action: Action::ToggleMouse },
    Command { name: "Toggle line numbers", keys: "Ctrl+L", action: Action::ToggleLineNumbers },
    Command { name: "Toggle current line highlight", keys: "Ctrl+Shift+H", action: Action::ToggleCurrentLineHighlight },
    Command { name: "Toggle bracket matching", keys: "Ctrl+Shift+B", action: Action::ToggleBracketMatching },
    Command { name: "Toggle whitespace", keys: "Ctrl+Shift+W", action: Action::ToggleWhitespace },
    Command { name: "Toggle indentation guides", keys: "Ctrl+Shift+G", action: Action::ToggleIndentationGuides },
    Command { name: "Toggle minimap", keys: "Ctrl+Shift+M", action: Action::ToggleMinimap },
    Command { name: "Toggle case sensitive search", keys: "Alt+C", action: Action::ToggleCaseSensitivity },
    Command { name: "Toggle whole word search", keys: "Alt+W", action: Action::ToggleWholeWord },
    Command { name: "Toggle regular expression search", keys: "Alt+R", action: Action::ToggleRegex },
    Command { name: "Cycle example files", keys: "F5", action: Action::CycleExampleFiles },
    Command { name: "Quit", keys: "Esc", action: Action::Quit },
];

/// A key that means nothing on its own and waits for the one after it. Emacs
/// spells most of its file commands this way, and CUA spells none of them
/// that way, which is the reason resolving a key cannot be a plain lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    ControlX,
}

/// What a key press turned out to be. Separating `Pending` from `Unbound`
/// matters: a pending prefix has to swallow the key rather than let it fall
/// through and be typed into the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Run(Action),
    Pending(Prefix),
    Unbound,
}

/// Turns a key press into what the editor should do about it, given which
/// bindings are in force and whether a prefix key is already waiting.
pub fn resolve(keymap: Keymap, pending: Option<Prefix>, key: KeyEvent) -> Resolution {
    return match keymap {
        // Vim has no table yet. Falling back to CUA rather than pretending is
        // deliberate, and the settings screen does not offer it.
        Keymap::Cua | Keymap::Vim => match cua(key) {
            Some(action) => Resolution::Run(action),
            None => Resolution::Unbound,
        },
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
        KeyCode::Char('p') if control && shift => Some(Action::StdLibBrowser),
        KeyCode::Char('k') if control && shift => Some(Action::DeleteLine),
        KeyCode::Char('z') if control && shift => Some(Action::Redo),
        // Finding a symbol and finding a phrase, widened from this file to
        // every file in the project. The shifted spellings are the ones other
        // editors use, and they are here for the terminals that can send
        // them. Most cannot: Ctrl+Shift+R arrives as plain Ctrl+R unless the
        // terminal speaks one of the newer keyboard protocols, which is why
        // each of these also has a plain chord of its own below.
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
        assert_eq!(from_keymap_name("cua"), Some(Keymap::Cua));
    }

    #[test]
    fn an_unknown_keymap_name_is_no_signal() {
        assert_eq!(from_keymap_name("nonsense"), None);
        assert_eq!(from_keymap_name(""), None);
    }

    #[test]
    fn only_the_modal_keymaps_announce_themselves() {
        assert_eq!(Keymap::Vim.label(), Some(" vim (cua keys for now) "));
        assert_eq!(Keymap::Emacs.label(), Some(" emacs "));
        assert_eq!(Keymap::Cua.label(), None);
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
        return resolve(Keymap::Emacs, None, KeyEvent::new(code, modifiers));
    }

    fn after_control_x(code: KeyCode, modifiers: KeyModifiers) -> Resolution {
        return resolve(Keymap::Emacs, Some(Prefix::ControlX), KeyEvent::new(code, modifiers));
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
        assert_eq!(emacs_key(KeyCode::Char('a'), KeyModifiers::NONE), Resolution::Unbound);
    }

    #[test]
    fn copy_answers_to_control_insert_in_every_keymap() {
        assert_eq!(with(KeyCode::Insert, KeyModifiers::CONTROL), Some(Action::Copy));
        assert_eq!(with(KeyCode::Insert, KeyModifiers::SHIFT), Some(Action::Paste));
        assert_eq!(emacs_key(KeyCode::Insert, KeyModifiers::CONTROL), Resolution::Run(Action::Copy));
    }

    /// Vim has no table, so it must land on CUA rather than on nothing.
    #[test]
    fn a_keymap_without_a_table_still_answers_keys() {
        assert_eq!(resolve(Keymap::Vim, None, KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)), Resolution::Run(Action::Save));
        assert_eq!(resolve(Keymap::Cua, None, KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)), Resolution::Unbound);
    }

    #[test]
    fn the_function_keys_keep_their_commands() {
        assert_eq!(plain(KeyCode::F(3)), Some(Action::FindNext));
        assert_eq!(with(KeyCode::F(3), KeyModifiers::SHIFT), Some(Action::FindPrevious));
        assert_eq!(plain(KeyCode::F(5)), Some(Action::CycleExampleFiles));
        assert_eq!(plain(KeyCode::F(6)), Some(Action::ToggleTheme));
        assert_eq!(plain(KeyCode::F(7)), Some(Action::Build));
        assert_eq!(plain(KeyCode::F(1)), Some(Action::ToggleCompletionDetail));
        assert_eq!(plain(KeyCode::F(2)), Some(Action::OpenSettings));
        assert_eq!(emacs_key(KeyCode::F(2), KeyModifiers::NONE), Resolution::Run(Action::OpenSettings));
        assert_eq!(plain(KeyCode::F(4)), Some(Action::ToggleMouse));
        assert_eq!(plain(KeyCode::F(8)), Some(Action::NextError));
        assert_eq!(with(KeyCode::F(8), KeyModifiers::SHIFT), Some(Action::PreviousError));
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
