//! Color module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Color:
        "color_rgb" => "std_lib::color::rgb", (red: i, green: i, blue: i) -> (s!e),
            "Builds `#rrggbb` from red, green and blue components, each 0 to 255. A component outside that range is an error naming which one.",
            "steel:s = danger(color_rgb(26, 43, 60));";
        "color_red" => "std_lib::color::red", (color: s) -> (i!e),
            "The red component of a hex color, 0 to 255. Accepts `#rrggbb` or 3-digit shorthand, with or without the `#`.",
            "red_part:i = danger(color_red(`#1a2b3c`));";
        "color_green" => "std_lib::color::green", (color: s) -> (i!e),
            "The green component of a hex color, 0 to 255.",
            "green_part:i = danger(color_green(`#1a2b3c`));";
        "color_blue" => "std_lib::color::blue", (color: s) -> (i!e),
            "The blue component of a hex color, 0 to 255.",
            "blue_part:i = danger(color_blue(`#1a2b3c`));";
        "color_lighten" => "std_lib::color::lighten", (color: s, amount: f) -> (s!e),
            "Blends a color toward white. Amount runs 0 (unchanged) to 1 (white).",
            "hover:s = danger(color_lighten(`#1a2b3c`, 0.2));";
        "color_darken" => "std_lib::color::darken", (color: s, amount: f) -> (s!e),
            "Blends a color toward black. Amount runs 0 (unchanged) to 1 (black).",
            "pressed:s = danger(color_darken(`#1a2b3c`, 0.2));";
        "color_mix" => "std_lib::color::mix", (first: s, second: s, share: f) -> (s!e),
            "Mixes two colors channel by channel. Share is how much of the second color, 0 to 1.",
            "purple:s = danger(color_mix(`#ff0000`, `#0000ff`, 0.5));";
        "color_invert" => "std_lib::color::invert", (color: s) -> (s!e),
            "Flips every channel to its opposite: 255 minus each component.",
            "opposite:s = danger(color_invert(`#1a2b3c`));";
        "color_grayscale" => "std_lib::color::grayscale", (color: s) -> (s!e),
            "The color's brightness as an equal-component gray, using the ITU-R 601 luma weights.",
            "gray:s = danger(color_grayscale(`#3366cc`));";
        "color_is_dark" => "std_lib::color::is_dark", (color: s) -> (b!e),
            "Whether the WCAG relative luminance falls below 0.5 - the cue for putting light text on it.",
            "needs_light_text:b = danger(color_is_dark(`#1a2b3c`));";
        "color_contrast_ratio" => "std_lib::color::contrast_ratio", (first: s, second: s) -> (f!e),
            "The WCAG 2 contrast ratio between two colors, 1 (identical) to 21 (black on white). Accessibility guidelines ask for at least 4.5 for body text.",
            "ratio:f = danger(color_contrast_ratio(`#1a2b3c`, `#ffffff`));";
        "color_text_on" => "std_lib::color::text_on", (background: s) -> (s!e),
            "`#000000` or `#ffffff`, whichever has the higher contrast ratio against the given background.",
            "ink:s = danger(color_text_on(`#1a2b3c`));";
        "color_hsl" => "std_lib::color::hsl", (hue: f, saturation: f, lightness: f) -> (s!e),
            "Builds a hex color from the HSL wheel: hue in degrees (circular, so 380 reads as 20), saturation and lightness both 0 to 1.",
            "lime:s = danger(color_hsl(120.0, 1.0, 0.5));";
        "color_rotate_hue" => "std_lib::color::rotate_hue", (color: s, degrees: f) -> (s!e),
            "Rotates a color around the HSL wheel by the given degrees, keeping its saturation and lightness - 180 lands on the complement, and stepping by 30 or 120 walks out a palette.",
            "complement:s = danger(color_rotate_hue(`#ff0000`, 180.0));";
    }
}
