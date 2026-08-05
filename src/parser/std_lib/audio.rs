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
//! carry on while something plays, put it in a `spawn` block.

use std::io::Cursor;

/// Everything needed to make a sound, opened fresh each time.
///
/// Holding the device open between calls would be faster, and would also mean
/// a Nail program had a piece of hidden global state whose lifetime nobody
/// controls. Opening it per call costs a few milliseconds and keeps the
/// library honest.
fn open_output() -> Result<(rodio::OutputStream, rodio::OutputStreamHandle), String> {
    return rodio::OutputStream::try_default().map_err(|e| format!("audio: no sound device this program can play through: {}", e));
}

/// Plays a sound file and returns when it has finished.
///
/// WAV, MP3, FLAC and Ogg Vorbis are understood. The file is read into memory
/// first, so this is for sound effects and notifications rather than for an
/// hour of music.
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
pub fn play_tone(hertz: f64, seconds: f64, volume: f64) -> Result<(), String> {
    if hertz <= 0.0 || hertz > 20000.0 {
        return Err(format!("audio_play_tone: {} hertz is outside what a person can hear", hertz));
    }
    if seconds <= 0.0 || seconds > 60.0 {
        return Err(format!("audio_play_tone: a tone of {} seconds is not something to play", seconds));
    }
    if !(0.0..=1.0).contains(&volume) {
        return Err(format!("audio_play_tone: a volume of {} is outside 0.0 to 1.0", volume));
    }

    let (_stream, handle) = open_output()?;
    let sink = rodio::Sink::try_new(&handle).map_err(|e| format!("audio_play_tone: could not open the sound device: {}", e))?;

    use rodio::Source;
    let tone = rodio::source::SineWave::new(hertz as f32).take_duration(std::time::Duration::from_secs_f64(seconds)).amplify(volume as f32);

    sink.append(tone);
    sink.sleep_until_end();
    return Ok(());
}

/// Whether this machine has a sound device to play through. Ask before
/// playing anything on a server, where the answer is usually no.
pub fn is_available() -> bool {
    return rodio::OutputStream::try_default().is_ok();
}

#[cfg(test)]
mod tests {
    use super::*;

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
