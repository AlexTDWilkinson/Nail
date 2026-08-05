//! Making a noise.
//!
//! Two things a program actually wants from sound: play this file, and make a
//! beep when something finishes. Both are here, and nothing else is - mixing,
//! effects and streaming belong to a program about audio, not to the standard
//! library of a general-purpose language.
//!
//! This module is behind the `audio` feature, because a sound device is not
//! something every machine has. Linux needs ALSA's development headers to
//! build against, and a server that will never make a sound should not have to
//! install them. `nailc --cargo-toml` turns the feature on only when a program
//! actually calls one of these functions.
//!
//! Playing is synchronous: the call returns when the sound has finished. That
//! is what makes it usable at all in a language with no way to hold a handle
//! to a playing sound, and it is what a notification beep wants anyway. To
//! carry on while something plays, put it in a `spawn` block. A game loop
//! cannot afford to wait at all, so `audio_tone_start` is the one call that
//! returns immediately.
//!
//! A browser has its own sound machinery, so the browser build of this module
//! speaks web audio and the rodio stack underneath the desktop build never
//! reaches a wasm binary.

#[cfg(not(target_arch = "wasm32"))]
use std::io::Cursor;

/// The checks every tone passes before any sound device is touched, so they
/// answer the same way on a machine with no sound card and in a browser.
fn check_tone(hertz: f64, seconds: f64, volume: f64, what: &str) -> Result<(), String> {
    if hertz <= 0.0 || hertz > 20000.0 {
        return Err(format!("{}: {} hertz is outside what a person can hear", what, hertz));
    }
    if seconds <= 0.0 || seconds > 60.0 {
        return Err(format!("{}: a tone of {} seconds is not something to play", what, seconds));
    }
    if !(0.0..=1.0).contains(&volume) {
        return Err(format!("{}: a volume of {} is outside 0.0 to 1.0", what, volume));
    }
    return Ok(());
}

/// Everything needed to make a sound, opened fresh each time.
///
/// Holding the device open between calls would be faster, and would also mean
/// a Nail program had a piece of hidden global state whose lifetime nobody
/// controls. Opening it per call costs a few milliseconds and keeps the
/// library honest.
#[cfg(not(target_arch = "wasm32"))]
fn open_output() -> Result<(rodio::OutputStream, rodio::OutputStreamHandle), String> {
    return rodio::OutputStream::try_default().map_err(|e| format!("audio: no sound device this program can play through: {}", e));
}

/// Plays a sound file and returns when it has finished.
///
/// WAV, MP3, FLAC and Ogg Vorbis are understood. The file is read into memory
/// first, so this is for sound effects and notifications rather than for an
/// hour of music.
#[cfg(not(target_arch = "wasm32"))]
pub fn play_file(path: String) -> Result<(), String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("audio_play_file: could not read '{}': {}", path, e))?;

    let (_stream, handle) = open_output()?;
    let sink = rodio::Sink::try_new(&handle).map_err(|e| format!("audio_play_file: could not open the sound device: {}", e))?;

    let source = rodio::Decoder::new(Cursor::new(bytes)).map_err(|e| format!("audio_play_file: '{}' is not a sound file this can read - WAV, MP3, FLAC and Ogg Vorbis are understood: {}", path, e))?;

    sink.append(source);
    sink.sleep_until_end();
    return Ok(());
}

/// Plays a single tone of the given pitch and length, and returns when it has
/// finished. 440.0 hertz is a concert A; 880.0 is the A above it.
///
/// The volume runs from 0.0 to 1.0. A tone at full volume is louder than most
/// people expect from a notification, so 0.2 is a better starting point than
/// 1.0.
#[cfg(not(target_arch = "wasm32"))]
pub fn play_tone(hertz: f64, seconds: f64, volume: f64) -> Result<(), String> {
    check_tone(hertz, seconds, volume, "audio_play_tone")?;

    let (_stream, handle) = open_output()?;
    let sink = rodio::Sink::try_new(&handle).map_err(|e| format!("audio_play_tone: could not open the sound device: {}", e))?;

    use rodio::Source;
    let tone = rodio::source::SineWave::new(hertz as f32).take_duration(std::time::Duration::from_secs_f64(seconds)).amplify(volume as f32);

    sink.append(tone);
    sink.sleep_until_end();
    return Ok(());
}

/// Starts a tone and returns at once, without waiting for it to finish.
///
/// This is the one a game loop can call. A frame has about sixteen
/// milliseconds to spend and a tone lasts longer than that, so waiting for
/// the sound would stutter the picture. The sound plays itself out while the
/// game carries on.
#[cfg(not(target_arch = "wasm32"))]
pub fn tone_start(hertz: f64, seconds: f64, volume: f64) -> Result<(), String> {
    check_tone(hertz, seconds, volume, "audio_tone_start")?;

    // The stream has to outlive this call for the sound to finish playing, and
    // a Nail program has nowhere to keep it, so the playing thread owns it and
    // drops it when the tone ends.
    std::thread::spawn(move || {
        let Ok((_stream, handle)) = open_output() else { return };
        let Ok(sink) = rodio::Sink::try_new(&handle) else { return };
        use rodio::Source;
        let tone = rodio::source::SineWave::new(hertz as f32).take_duration(std::time::Duration::from_secs_f64(seconds)).amplify(volume as f32);
        sink.append(tone);
        sink.sleep_until_end();
    });
    return Ok(());
}

/// Whether this machine has a sound device to play through. Ask before
/// playing anything on a server, where the answer is usually no.
#[cfg(not(target_arch = "wasm32"))]
pub fn is_available() -> bool {
    return rodio::OutputStream::try_default().is_ok();
}

/// The browser build. Sound goes through web audio, which every browser has,
/// so there is no device to open and nothing to install.
///
/// One rule shapes all of it: a browser refuses to make a sound until the
/// person has touched the page. That is not an error a game should die of, so
/// a refused sound is simply silence and the call still succeeds.
#[cfg(target_arch = "wasm32")]
mod browser {
    use std::cell::RefCell;

    thread_local! {
        /// Browsers limit how many audio contexts a page may open, so the
        /// first sound makes one and every later sound reuses it.
        static CONTEXT: RefCell<Option<web_sys::AudioContext>> = RefCell::new(None);
    }

    /// Runs `play` with the page's audio context, or does nothing at all if
    /// the browser will not give one.
    fn with_context(play: impl FnOnce(&web_sys::AudioContext)) {
        CONTEXT.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_none() {
                slot.replace(match web_sys::AudioContext::new() {
                    Ok(context) => context,
                    Err(_) => return,
                });
            }
            let Some(context) = slot.as_ref() else { return };
            // A context made before the first click starts suspended, and
            // resuming it is what the first touch is for.
            let _ = context.resume();
            play(context);
        });
    }

    pub fn tone_start(hertz: f64, seconds: f64, volume: f64) -> Result<(), String> {
        super::check_tone(hertz, seconds, volume, "audio_tone_start")?;

        with_context(|context| {
            let (Ok(oscillator), Ok(gain)) = (context.create_oscillator(), context.create_gain()) else { return };
            oscillator.set_type(web_sys::OscillatorType::Sine);
            oscillator.frequency().set_value(hertz as f32);

            // A tone that starts and stops at full volume clicks, so it opens
            // fast and fades to nearly nothing by the end.
            let start = context.current_time();
            let level = gain.gain();
            let _ = level.set_value_at_time(0.0, start);
            let _ = level.linear_ramp_to_value_at_time(volume as f32, start + 0.005);
            let _ = level.exponential_ramp_to_value_at_time(0.0001, start + seconds);

            if oscillator.connect_with_audio_node(&gain).is_err() || gain.connect_with_audio_node(&context.destination()).is_err() {
                return;
            }
            let _ = oscillator.start();
            let _ = oscillator.stop_with_when(start + seconds);
        });
        return Ok(());
    }

    /// A browser cannot stop and wait for a sound, so this starts the tone the
    /// same way `tone_start` does and comes straight back.
    pub fn play_tone(hertz: f64, seconds: f64, volume: f64) -> Result<(), String> {
        return tone_start(hertz, seconds, volume);
    }

    /// The path is a URL the page can reach, and the sound starts rather than
    /// finishes before this returns, for the same reason.
    pub fn play_file(path: String) -> Result<(), String> {
        let element = web_sys::HtmlAudioElement::new_with_src(&path).map_err(|_| format!("audio_play_file: the browser would not load '{}'", path))?;
        let _ = element.play();
        return Ok(());
    }

    /// Every browser can make a sound, though not before the person has
    /// touched the page.
    pub fn is_available() -> bool {
        return true;
    }
}

#[cfg(target_arch = "wasm32")]
pub use browser::{is_available, play_file, play_tone, tone_start};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// A tone that never reaches a speaker still has to be refused the same
    /// way, so the game loop call checks its arguments like the blocking one.
    #[test]
    fn a_tone_that_starts_and_leaves_is_checked_the_same_way() {
        assert!(tone_start(0.0, 0.1, 0.5).unwrap_err().contains("outside what a person can hear"));
        assert!(tone_start(440.0, 0.0, 0.5).unwrap_err().contains("not something to play"));
        assert!(tone_start(440.0, 0.1, 2.0).unwrap_err().contains("outside 0.0 to 1.0"));
    }

    /// The checks that happen before any device is touched, so these run on a
    /// machine with no sound card - which is what continuous integration is.
    #[test]
    fn a_tone_nobody_could_hear_is_refused() {
        assert!(play_tone(0.0, 1.0, 0.5).unwrap_err().contains("outside what a person can hear"));
        assert!(play_tone(-440.0, 1.0, 0.5).unwrap_err().contains("outside what a person can hear"));
        assert!(play_tone(30000.0, 1.0, 0.5).unwrap_err().contains("outside what a person can hear"));
    }

    #[test]
    fn a_length_that_is_not_a_length_is_refused() {
        assert!(play_tone(440.0, 0.0, 0.5).unwrap_err().contains("not something to play"));
        assert!(play_tone(440.0, -1.0, 0.5).unwrap_err().contains("not something to play"));
        assert!(play_tone(440.0, 3600.0, 0.5).unwrap_err().contains("not something to play"));
    }

    #[test]
    fn a_volume_outside_the_range_is_refused() {
        assert!(play_tone(440.0, 0.1, 1.5).unwrap_err().contains("outside 0.0 to 1.0"));
        assert!(play_tone(440.0, 0.1, -0.1).unwrap_err().contains("outside 0.0 to 1.0"));
    }

    #[test]
    fn a_file_that_is_not_there_is_reported_as_such() {
        let failure = play_file("/nowhere/at/all/beep.wav".to_string()).unwrap_err();
        assert!(failure.contains("could not read"), "got: {}", failure);
        assert!(failure.contains("/nowhere/at/all/beep.wav"), "the error names the file: {}", failure);
    }

    #[test]
    fn a_file_that_is_not_a_sound_is_reported_as_such() {
        let path = format!("{}/nail_audio_not_a_sound.wav", std::env::temp_dir().to_string_lossy());
        std::fs::write(&path, "this is text, not audio").expect("a writable temporary directory");

        let failure = play_file(path.clone()).unwrap_err();
        // Without a sound device the device error comes first; with one, the
        // decoder rejects the file. Either way it is not silently ignored.
        assert!(failure.contains("audio"), "got: {}", failure);

        std::fs::remove_file(path).expect("a removable file");
    }

    #[test]
    fn asking_whether_there_is_a_sound_device_never_fails() {
        // The answer depends on the machine; that it answers at all is the point.
        let _ = is_available();
    }
}
