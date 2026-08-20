//! Audio module stdlib registry entries.
//!
//! Behind the `audio` feature: a sound device is not something every machine
//! has, and on Linux building against one needs ALSA's development headers. A
//! server that will never make a sound should not have to install them, so the
//! dependency is optional and `nailc --cargo-toml` turns it on only for a
//! program that actually calls one of these.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Audio:
        "audio_play_file" [Rodio] => "std_lib::audio::play_file", (path: s) -> (v!e),
            "Plays a sound file and returns when it has finished. WAV, MP3, FLAC and Ogg Vorbis are understood. To carry on while it plays, run it in a c block beside the other work.",
            "danger(audio_play_file(`done.wav`));";
        "audio_play_tone" [Rodio] => "std_lib::audio::play_tone", (hertz: f, seconds: f, volume: f) -> (v!e),
            "Plays a single tone and returns when it has finished. 440.0 hertz is a concert A. A volume of 0.2 is a better starting point for a notification than 1.0.",
            "danger(audio_play_tone(440.0, 0.2, 0.2));";
        "audio_tone_start" [Rodio] => "std_lib::audio::tone_start", (hertz: f, seconds: f, volume: f) -> (v!e),
            "Starts a tone and returns at once, without waiting for it to finish. This is the one a game loop can call, since waiting out a sound would stutter the picture. In a browser the page stays silent until the player has first touched or clicked it, which is the browser's rule, not an error.",
            "danger(audio_tone_start(880.0, 0.06, 0.15));";
        "audio_tone_after" [Rodio] => "std_lib::audio::tone_after", (hertz: f, seconds: f, volume: f, wait_seconds: f) -> (v!e),
            "Starts a tone after a wait and returns at once. Several of these with growing waits are a short tune, queued in one go and played in time without the program keeping a clock of its own.",
            "danger(audio_tone_after(659.0, 0.12, 0.14, 0.11));";
        "audio_is_available" [Rodio] => "std_lib::audio::is_available", () -> b,
            "Returns whether this machine has a sound device to play through. Ask before playing anything on a server, where the answer is usually no.",
            "can_beep:b = audio_is_available();";
    }
}
