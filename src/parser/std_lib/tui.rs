//! Full-screen terminal programs, described rather than drawn.
//!
//! Every other terminal library in every other language is retained-mode: you
//! build widget objects, hold onto them, and mutate them as things change.
//! That is not possible to make the default in a language where nothing is
//! mutable, and it turns out not to be necessary. A program here supplies two
//! ordinary functions:
//!
//!   f view(state:App):TUI_Screen      - what the screen looks like right now
//!   f update(state:App, event:TUI_Event):App - the state after something happened
//!
//! and `tui_run` owns everything else: raw mode, the alternate screen, polling
//! for input without blocking the runtime, redrawing, resizing, and - the part
//! every hand-written terminal program gets wrong - putting the terminal back
//! exactly as it was, including when the program panics. A Nail program cannot
//! leave someone's shell in raw mode with no echo, because it never turns raw
//! mode on itself.
//!
//! `App` is whatever struct the program wants; `tui_run` takes the starting
//! one and returns the last one, so a program can carry a result out of its
//! own interface.
//!
//! Quitting is a field on the screen rather than a special return: when `view`
//! reports `quit = true`, the loop paints that last frame and stops.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{cursor, terminal, ExecutableCommand};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::time::Duration;

use super::term::TERM_Color;

/// One line of the screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TUI_Line {
    pub text: String,
    pub color: TERM_Color,
    pub bold: bool,
    /// Drawn with the foreground and background swapped - what a selected row
    /// in a list looks like.
    pub selected: bool,
}

/// Everything on the screen at one moment.
///
/// `title` is drawn at the top and `status` at the bottom, both optional -
/// leave them empty and the lines get the whole screen. Lines beyond the
/// bottom of the terminal are not drawn; that is the program's cue to scroll
/// by choosing different lines, which is a decision `view` is in a far better
/// position to make than the library is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TUI_Screen {
    pub title: String,
    pub lines: Vec<TUI_Line>,
    pub status: String,
    pub quit: bool,
}

/// Something that happened.
///
/// `key` is the name of the key pressed: a single character for an ordinary
/// key, or one of `Enter`, `Esc`, `Up`, `Down`, `Left`, `Right`, `Backspace`,
/// `Tab`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`, or a control
/// combination like `Ctrl+c`.
///
/// `tick` is true for the event delivered when nothing was pressed, which is
/// what drives a clock, a progress bar, or anything else that has to move on
/// its own. `width` and `height` are the terminal's size, so a resize needs no
/// special handling - the next frame simply knows more.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TUI_Event {
    pub key: String,
    pub tick: bool,
    pub width: i64,
    pub height: i64,
}

/// A plain line in the terminal's own colour.
pub fn line(text: String) -> TUI_Line {
    return TUI_Line { text, color: TERM_Color::White, bold: false, selected: false };
}

/// A line with everything about its appearance said explicitly.
pub fn styled(text: String, color: TERM_Color, bold: bool, selected: bool) -> TUI_Line {
    return TUI_Line { text, color, bold, selected };
}

/// The name this library gives a key, which is what `TUI_Event.key` holds.
fn key_name(key: KeyEvent) -> String {
    let base = match key.code {
        KeyCode::Char(character) => character.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::F(number) => format!("F{}", number),
        _ => String::new(),
    };

    if base.is_empty() {
        return base;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return format!("Ctrl+{}", base);
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        return format!("Alt+{}", base);
    }
    return base;
}

/// Owns the terminal's mode for as long as the program is drawing, and puts it
/// back when dropped.
///
/// This is the whole reason a program should not do this by hand. Raw mode
/// turns off echo and line buffering; leaving it on means the shell the
/// program was run from is left unusable, with typed characters invisible.
/// Because this restores in `Drop`, it runs on a normal exit, on an early
/// return, and while a panic unwinds - which is exactly when a hand-written
/// version forgets.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<TerminalGuard, String> {
        terminal::enable_raw_mode().map_err(|e| format!("tui_run: could not take control of the terminal: {}", e))?;
        let mut out = std::io::stdout();
        out.execute(terminal::EnterAlternateScreen).map_err(|e| format!("tui_run: could not open a full-screen view: {}", e))?;
        out.execute(cursor::Hide).map_err(|e| format!("tui_run: could not hide the cursor: {}", e))?;
        return Ok(TerminalGuard);
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Every step is attempted even if an earlier one failed: a terminal
        // left in raw mode is worse than an error nobody reads.
        let mut out = std::io::stdout();
        let _ = out.execute(cursor::Show);
        let _ = out.execute(terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        let _ = out.flush();
    }
}

/// Paints one frame.
///
/// The whole frame is built in memory and written in a single call. Writing it
/// piece by piece is what makes a terminal program flicker, because the
/// terminal draws whatever has arrived so far.
fn paint(screen: &TUI_Screen, width: u16, height: u16) -> Result<(), String> {
    let mut frame = String::new();
    frame.push_str("\u{1b}[H\u{1b}[2J");

    let mut rows_left = height as usize;

    if !screen.title.is_empty() && rows_left > 0 {
        frame.push_str(&format!("\u{1b}[1m{}\u{1b}[0m\r\n", truncate(&screen.title, width as usize)));
        rows_left -= 1;
    }

    // The status line is kept back so it can sit at the bottom.
    let status_rows = if screen.status.is_empty() { 0 } else { 1 };
    let body_rows = rows_left.saturating_sub(status_rows);

    for line in screen.lines.iter().take(body_rows) {
        let mut styled_text = String::new();
        if line.selected {
            styled_text.push_str("\u{1b}[7m");
        }
        if line.bold {
            styled_text.push_str("\u{1b}[1m");
        }
        styled_text.push_str(&format!("\u{1b}[{}m", color_code(line.color)));
        styled_text.push_str(&truncate(&line.text, width as usize));
        styled_text.push_str("\u{1b}[0m");
        frame.push_str(&styled_text);
        frame.push_str("\r\n");
    }

    if status_rows == 1 {
        let painted = screen.lines.len().min(body_rows);
        for _ in painted..body_rows {
            frame.push_str("\r\n");
        }
        frame.push_str(&format!("\u{1b}[2m{}\u{1b}[0m", truncate(&screen.status, width as usize)));
    }

    let mut out = std::io::stdout();
    out.write_all(frame.as_bytes()).map_err(|e| format!("tui_run: could not draw to the terminal: {}", e))?;
    out.flush().map_err(|e| format!("tui_run: could not draw to the terminal: {}", e))?;
    return Ok(());
}

/// Cuts a line to the terminal's width, counting characters rather than bytes
/// so a line of accented text is not cut mid-character.
fn truncate(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    return text.chars().take(width).collect();
}

/// The SGR number for a colour, shared with the term module's own table.
fn color_code(color: TERM_Color) -> u8 {
    return match color {
        TERM_Color::Black => 30,
        TERM_Color::Red => 31,
        TERM_Color::Green => 32,
        TERM_Color::Yellow => 33,
        TERM_Color::Blue => 34,
        TERM_Color::Magenta => 35,
        TERM_Color::Cyan => 36,
        TERM_Color::White => 37,
        TERM_Color::BrightBlack => 90,
        TERM_Color::BrightRed => 91,
        TERM_Color::BrightGreen => 92,
        TERM_Color::BrightYellow => 93,
        TERM_Color::BrightBlue => 94,
        TERM_Color::BrightMagenta => 95,
        TERM_Color::BrightCyan => 96,
        TERM_Color::BrightWhite => 97,
    };
}

pub type ViewFuture = Pin<Box<dyn Future<Output = TUI_Screen> + Send>>;
pub type UpdateFuture<S> = Pin<Box<dyn Future<Output = S> + Send>>;

/// How long the loop waits for a key before delivering a tick instead.
const TICK: Duration = Duration::from_millis(100);

/// Runs a full-screen program until its `view` reports `quit`, and returns the
/// state it finished with.
///
/// The loop is: draw what `view` says, wait for something to happen, hand that
/// to `update`, repeat. Input is polled rather than waited on, so the async
/// runtime this shares a thread with is never blocked - a program can be
/// serving HTTP in a `spawn` block while its interface stays responsive.
pub async fn run<S, V, U>(initial: S, view: V, update: U) -> Result<S, String>
where
    S: Clone + Send + 'static,
    V: Fn(S) -> ViewFuture + Send + Sync + 'static,
    U: Fn(S, TUI_Event) -> UpdateFuture<S> + Send + Sync + 'static,
{
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return Err("tui_run: standard output is not a terminal, so there is nothing to draw on - a full-screen program cannot be piped to a file".to_string());
    }

    // From here until this returns, the terminal is ours; the guard hands it
    // back however this ends.
    let _guard = TerminalGuard::enter()?;

    let mut state = initial;
    loop {
        let (width, height) = terminal::size().unwrap_or((80, 24));

        let screen = view(state.clone()).await;
        paint(&screen, width, height)?;
        if screen.quit {
            return Ok(state);
        }

        // Poll rather than block, so the executor stays free. An empty poll
        // becomes a tick, which is what lets a clock tick and a spinner spin.
        let mut event_key = String::new();
        let mut waited = Duration::ZERO;
        while waited < TICK {
            let ready = event::poll(Duration::ZERO).map_err(|e| format!("tui_run: could not read from the terminal: {}", e))?;
            if ready {
                match event::read().map_err(|e| format!("tui_run: could not read from the terminal: {}", e))? {
                    Event::Key(key) => {
                        let name = key_name(key);
                        if !name.is_empty() {
                            event_key = name;
                            break;
                        }
                    }
                    // A resize needs no special handling: the next frame reads
                    // the new size and lays itself out again.
                    Event::Resize(_, _) => break,
                    _ => {}
                }
            }
            tokio::time::sleep(Duration::from_millis(8)).await;
            waited += Duration::from_millis(8);
        }

        let (width, height) = terminal::size().unwrap_or((80, 24));
        let happened = TUI_Event { key: event_key.clone(), tick: event_key.is_empty(), width: width as i64, height: height as i64 };
        state = update(state, happened).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen_of(lines: Vec<&str>) -> TUI_Screen {
        return TUI_Screen { title: "Title".to_string(), lines: lines.iter().map(|text| line(text.to_string())).collect(), status: "Status".to_string(), quit: false };
    }

    #[test]
    fn a_plain_line_is_unstyled_and_a_styled_one_keeps_what_it_was_given() {
        let plain = line("hello".to_string());
        assert_eq!(plain.text, "hello");
        assert!(!plain.bold);
        assert!(!plain.selected);

        let fancy = styled("hello".to_string(), TERM_Color::Red, true, true);
        assert_eq!(fancy.color, TERM_Color::Red);
        assert!(fancy.bold);
        assert!(fancy.selected);
    }

    #[test]
    fn keys_are_named_the_way_a_program_would_compare_them() {
        assert_eq!(key_name(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)), "q");
        assert_eq!(key_name(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)), "Enter");
        assert_eq!(key_name(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)), "Up");
        assert_eq!(key_name(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)), "Esc");
        assert_eq!(key_name(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE)), "F5");
    }

    #[test]
    fn a_control_combination_is_named_as_one() {
        assert_eq!(key_name(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), "Ctrl+c");
        assert_eq!(key_name(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)), "Alt+x");
    }

    #[test]
    fn a_key_with_no_name_is_reported_as_nothing_rather_than_as_a_tick() {
        // An unnamed key must produce an empty name, which the loop then skips
        // instead of delivering as a keypress nobody can match on.
        assert_eq!(key_name(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)), "");
    }

    #[test]
    fn lines_are_cut_to_the_width_by_characters_not_bytes() {
        assert_eq!(truncate("hello", 3), "hel");
        assert_eq!(truncate("hello", 99), "hello");
        assert_eq!(truncate("hello", 0), "");
        // Four accented characters are eight bytes; cutting by bytes would
        // split one in half and produce mojibake.
        assert_eq!(truncate("ééé", 2), "éé");
    }

    #[test]
    fn every_colour_has_its_own_code() {
        let colors = [
            TERM_Color::Black,
            TERM_Color::Red,
            TERM_Color::Green,
            TERM_Color::Yellow,
            TERM_Color::Blue,
            TERM_Color::Magenta,
            TERM_Color::Cyan,
            TERM_Color::White,
            TERM_Color::BrightBlack,
            TERM_Color::BrightRed,
            TERM_Color::BrightGreen,
            TERM_Color::BrightYellow,
            TERM_Color::BrightBlue,
            TERM_Color::BrightMagenta,
            TERM_Color::BrightCyan,
            TERM_Color::BrightWhite,
        ];
        let mut codes: Vec<u8> = colors.iter().map(|color| color_code(*color)).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), colors.len());
    }

    /// The screen model is data, so what a program would draw can be asserted
    /// without a terminal to draw it on - which is the point of describing a
    /// screen rather than drawing one.
    #[test]
    fn a_screen_is_data_that_can_be_checked_without_a_terminal() {
        let screen = screen_of(vec!["one", "two", "three"]);
        assert_eq!(screen.lines.len(), 3);
        assert_eq!(screen.lines[1].text, "two");
        assert!(!screen.quit);

        let quitting = TUI_Screen { quit: true, ..screen_of(vec![]) };
        assert!(quitting.quit);
    }

    /// The same property a real program depends on: view is a pure function of
    /// state, so the same state always draws the same screen.
    #[test]
    fn the_same_state_always_draws_the_same_screen() {
        fn view(count: i64) -> TUI_Screen {
            return TUI_Screen {
                title: "Counter".to_string(),
                lines: vec![line(format!("count: {}", count))],
                status: "press q to quit".to_string(),
                quit: count > 3,
            };
        }

        assert_eq!(view(2).lines[0].text, view(2).lines[0].text);
        assert!(!view(2).quit);
        assert!(view(4).quit, "the state, not the library, decides when it is over");
    }

    #[tokio::test]
    async fn running_without_a_terminal_says_so_instead_of_taking_over_the_output() {
        // Under a test harness stdout is captured, so this is the piped case -
        // and it must refuse rather than write escape codes into the capture.
        let outcome = run(
            0i64,
            |count: i64| Box::pin(async move { TUI_Screen { title: String::new(), lines: vec![], status: String::new(), quit: count >= 0 } }) as ViewFuture,
            |count: i64, _event: TUI_Event| Box::pin(async move { count }) as UpdateFuture<i64>,
        )
        .await;

        match outcome {
            Err(message) => assert!(message.contains("not a terminal"), "got: {}", message),
            Ok(_) => {
                // A machine that does give the test harness a terminal is
                // fine too; the first frame quits immediately.
            }
        }
    }
}
