//! Colors as the hex strings the web writes them in. `#1a2b3c` goes in - with
//! or without the `#`, three digits or six, either case - and six lowercase
//! digits always come out. Build a color from components or from the HSL
//! wheel, take it apart, blend it, and ask the accessibility questions a
//! designer actually checks: is this dark, what is the contrast ratio, which
//! text color survives on it.

/// A hex string to its three channels. Forgives a missing `#`, 3-digit
/// shorthand and upper case; anything else is an error naming the caller.
fn parse(color: &str, fn_name: &str) -> Result<(u8, u8, u8), String> {
    let trimmed = color.trim();
    let digits = trimmed.strip_prefix('#').unwrap_or(trimmed);
    let characters: Vec<char> = digits.chars().collect();
    let expanded: Vec<char> = match characters.len() {
        3 => characters.iter().flat_map(|character| [*character, *character]).collect(),
        6 => characters,
        other => return Err(format!("{}: `{}` is not a hex color - it has {} digits and a color has 3 or 6", fn_name, trimmed, other)),
    };

    let mut channels = [0u8; 3];
    for (index, pair) in expanded.chunks(2).enumerate() {
        match (pair[0].to_digit(16), pair[1].to_digit(16)) {
            (Some(high), Some(low)) => channels[index] = (high * 16 + low) as u8,
            _ => return Err(format!("{}: `{}` is not a hex color - `{}{}` is not a hex byte", fn_name, trimmed, pair[0], pair[1])),
        }
    }
    return Ok((channels[0], channels[1], channels[2]));
}

/// Three channels back to the one spelling every function here emits.
fn format_hex(red: u8, green: u8, blue: u8) -> String {
    return format!("#{:02x}{:02x}{:02x}", red, green, blue);
}

/// A share that must sit in 0..=1. The comparison also rejects NaN.
fn fraction(value: f64, name: &str, fn_name: &str) -> Result<f64, String> {
    if !(value >= 0.0 && value <= 1.0) {
        return Err(format!("{}: {} is {} and must be between 0 and 1", fn_name, name, value));
    }
    return Ok(value);
}

/// One channel moved `share` of the way from its value toward a target.
/// With share in 0..=1 the result stays in 0..=255, so the cast is safe.
fn blend(from: u8, toward: u8, share: f64) -> u8 {
    return ((from as f64) * (1.0 - share) + (toward as f64) * share).round() as u8;
}

/// One sRGB channel into linear light, per the WCAG 2 definition.
fn linearize(channel: u8) -> f64 {
    let scaled = channel as f64 / 255.0;
    if scaled <= 0.03928 {
        return scaled / 12.92;
    }
    return ((scaled + 0.055) / 1.055).powf(2.4);
}

/// WCAG 2 relative luminance: 0.0 for black, 1.0 for white.
fn relative_luminance(red: u8, green: u8, blue: u8) -> f64 {
    return 0.2126 * linearize(red) + 0.7152 * linearize(green) + 0.0722 * linearize(blue);
}

/// HSL to channels. The hue wheel is circular, so any number of degrees
/// lands somewhere on it; saturation and lightness are already validated.
fn hsl_to_channels(hue: f64, saturation: f64, lightness: f64) -> (u8, u8, u8) {
    let wheel = hue.rem_euclid(360.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let secondary = chroma * (1.0 - ((wheel / 60.0).rem_euclid(2.0) - 1.0).abs());
    let base = lightness - chroma / 2.0;
    let (red, green, blue) = match wheel {
        position if position < 60.0 => (chroma, secondary, 0.0),
        position if position < 120.0 => (secondary, chroma, 0.0),
        position if position < 180.0 => (0.0, chroma, secondary),
        position if position < 240.0 => (0.0, secondary, chroma),
        position if position < 300.0 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let channel = |value: f64| -> u8 { return ((value + base) * 255.0).round() as u8 };
    return (channel(red), channel(green), channel(blue));
}

/// Channels to HSL, the inverse of `hsl_to_channels`, for hue rotation.
/// A gray has no hue direction, so it reports hue 0 and saturation 0.
fn channels_to_hsl(red: u8, green: u8, blue: u8) -> (f64, f64, f64) {
    let red_scaled = red as f64 / 255.0;
    let green_scaled = green as f64 / 255.0;
    let blue_scaled = blue as f64 / 255.0;
    let maximum = red_scaled.max(green_scaled).max(blue_scaled);
    let minimum = red_scaled.min(green_scaled).min(blue_scaled);
    let lightness = (maximum + minimum) / 2.0;
    let delta = maximum - minimum;
    if delta == 0.0 {
        return (0.0, 0.0, lightness);
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue = if maximum == red_scaled {
        60.0 * ((green_scaled - blue_scaled) / delta).rem_euclid(6.0)
    } else if maximum == green_scaled {
        60.0 * ((blue_scaled - red_scaled) / delta + 2.0)
    } else {
        60.0 * ((red_scaled - green_scaled) / delta + 4.0)
    };
    return (hue, saturation, lightness);
}

/// Build `#rrggbb` from red, green and blue, each 0 to 255. A component
/// outside that range is an error naming which one.
pub fn rgb(red: i64, green: i64, blue: i64) -> Result<String, String> {
    let channel = |value: i64, name: &str| -> Result<u8, String> {
        if !(0..=255).contains(&value) {
            return Err(format!("color_rgb: {} is {} and a channel runs 0 to 255", name, value));
        }
        return Ok(value as u8);
    };
    return Ok(format_hex(channel(red, "red")?, channel(green, "green")?, channel(blue, "blue")?));
}

/// The red component of a color, 0 to 255.
pub fn red(color: String) -> Result<i64, String> {
    let (red, _, _) = parse(&color, "color_red")?;
    return Ok(red as i64);
}

/// The green component of a color, 0 to 255.
pub fn green(color: String) -> Result<i64, String> {
    let (_, green, _) = parse(&color, "color_green")?;
    return Ok(green as i64);
}

/// The blue component of a color, 0 to 255.
pub fn blue(color: String) -> Result<i64, String> {
    let (_, _, blue) = parse(&color, "color_blue")?;
    return Ok(blue as i64);
}

/// Blend a color toward white. Amount runs 0 (unchanged) to 1 (white),
/// moving each channel linearly.
pub fn lighten(color: String, amount: f64) -> Result<String, String> {
    let (red, green, blue) = parse(&color, "color_lighten")?;
    let share = fraction(amount, "amount", "color_lighten")?;
    return Ok(format_hex(blend(red, 255, share), blend(green, 255, share), blend(blue, 255, share)));
}

/// Blend a color toward black. Amount runs 0 (unchanged) to 1 (black).
pub fn darken(color: String, amount: f64) -> Result<String, String> {
    let (red, green, blue) = parse(&color, "color_darken")?;
    let share = fraction(amount, "amount", "color_darken")?;
    return Ok(format_hex(blend(red, 0, share), blend(green, 0, share), blend(blue, 0, share)));
}

/// Mix two colors channel by channel. Share is how much of the second color
/// ends up in the result: 0 is all first, 1 is all second.
pub fn mix(first: String, second: String, share: f64) -> Result<String, String> {
    let (first_red, first_green, first_blue) = parse(&first, "color_mix")?;
    let (second_red, second_green, second_blue) = parse(&second, "color_mix")?;
    let portion = fraction(share, "share", "color_mix")?;
    return Ok(format_hex(blend(first_red, second_red, portion), blend(first_green, second_green, portion), blend(first_blue, second_blue, portion)));
}

/// Flip every channel to its opposite: 255 minus each component.
pub fn invert(color: String) -> Result<String, String> {
    let (red, green, blue) = parse(&color, "color_invert")?;
    return Ok(format_hex(255 - red, 255 - green, 255 - blue));
}

/// The color's brightness as an equal-component gray, using the ITU-R 601
/// luma weights (0.299, 0.587, 0.114).
pub fn grayscale(color: String) -> Result<String, String> {
    let (red, green, blue) = parse(&color, "color_grayscale")?;
    let luma = (0.299 * red as f64 + 0.587 * green as f64 + 0.114 * blue as f64).round() as u8;
    return Ok(format_hex(luma, luma, luma));
}

/// Whether the WCAG relative luminance falls below 0.5. That is linear
/// light, not the eight-bit midpoint, so the line sits near `#bcbcbc` and
/// most mid-tones count as dark.
pub fn is_dark(color: String) -> Result<bool, String> {
    let (red, green, blue) = parse(&color, "color_is_dark")?;
    return Ok(relative_luminance(red, green, blue) < 0.5);
}

/// The WCAG 2 contrast ratio between two colors, from 1.0 (identical) to
/// 21.0 (black on white). Order does not matter. Accessibility guidelines
/// ask for at least 4.5 for body text.
pub fn contrast_ratio(first: String, second: String) -> Result<f64, String> {
    let (first_red, first_green, first_blue) = parse(&first, "color_contrast_ratio")?;
    let (second_red, second_green, second_blue) = parse(&second, "color_contrast_ratio")?;
    let first_luminance = relative_luminance(first_red, first_green, first_blue);
    let second_luminance = relative_luminance(second_red, second_green, second_blue);
    let lighter = first_luminance.max(second_luminance);
    let darker = first_luminance.min(second_luminance);
    return Ok((lighter + 0.05) / (darker + 0.05));
}

/// `#000000` or `#ffffff`, whichever has the higher contrast ratio against
/// the given background.
pub fn text_on(background: String) -> Result<String, String> {
    let (red, green, blue) = parse(&background, "color_text_on")?;
    let luminance = relative_luminance(red, green, blue);
    let against_white = (1.0 + 0.05) / (luminance + 0.05);
    let against_black = (luminance + 0.05) / 0.05;
    if against_white >= against_black {
        return Ok("#ffffff".to_string());
    }
    return Ok("#000000".to_string());
}

/// Build a hex color from the HSL wheel: hue in degrees (the wheel is
/// circular, so 380 reads as 20), saturation and lightness both 0 to 1.
pub fn hsl(hue: f64, saturation: f64, lightness: f64) -> Result<String, String> {
    if !hue.is_finite() {
        return Err(format!("color_hsl: hue is {} and must be a number of degrees", hue));
    }
    let saturation_share = fraction(saturation, "saturation", "color_hsl")?;
    let lightness_share = fraction(lightness, "lightness", "color_hsl")?;
    let (red, green, blue) = hsl_to_channels(hue, saturation_share, lightness_share);
    return Ok(format_hex(red, green, blue));
}

/// Rotate a color around the HSL wheel by the given degrees, keeping its
/// saturation and lightness - 180 lands on the complement, and stepping by
/// 30 or 120 walks out a palette.
pub fn rotate_hue(color: String, degrees: f64) -> Result<String, String> {
    let (red, green, blue) = parse(&color, "color_rotate_hue")?;
    if !degrees.is_finite() {
        return Err(format!("color_rotate_hue: degrees is {} and must be a number", degrees));
    }
    let (hue, saturation, lightness) = channels_to_hsl(red, green, blue);
    let (rotated_red, rotated_green, rotated_blue) = hsl_to_channels(hue + degrees, saturation, lightness);
    return Ok(format_hex(rotated_red, rotated_green, rotated_blue));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 0.01;
    }

    /// Same color to within one count per channel - rounding room for
    /// round-trips through HSL.
    fn channels_close(left: &str, right: &str) -> bool {
        let (left_red, left_green, left_blue) = parse(left, "test").unwrap();
        let (right_red, right_green, right_blue) = parse(right, "test").unwrap();
        return (left_red as i64 - right_red as i64).abs() <= 1
            && (left_green as i64 - right_green as i64).abs() <= 1
            && (left_blue as i64 - right_blue as i64).abs() <= 1;
    }

    #[test]
    fn components_go_in_and_come_back_out() {
        assert_eq!(rgb(26, 43, 60).unwrap(), "#1a2b3c");
        assert_eq!(red("#ff0000".to_string()).unwrap(), 255);
        assert_eq!(green("#1a2b3c".to_string()).unwrap(), 43);
        assert_eq!(blue("#1a2b3c".to_string()).unwrap(), 60);
    }

    #[test]
    fn shorthand_case_and_the_missing_hash_are_forgiven() {
        assert_eq!(red("#abc".to_string()).unwrap(), 0xaa);
        assert_eq!(lighten("#abc".to_string(), 0.0).unwrap(), "#aabbcc");
        assert_eq!(red("1A2B3C".to_string()).unwrap(), 26);
    }

    #[test]
    fn blending_moves_toward_white_black_and_each_other() {
        assert_eq!(mix("#000000".to_string(), "#ffffff".to_string(), 0.5).unwrap(), "#808080");
        assert_eq!(lighten("#000000".to_string(), 1.0).unwrap(), "#ffffff");
        assert_eq!(darken("#ffffff".to_string(), 1.0).unwrap(), "#000000");
        assert_eq!(mix("#ff0000".to_string(), "#0000ff".to_string(), 0.0).unwrap(), "#ff0000");
        assert_eq!(mix("#ff0000".to_string(), "#0000ff".to_string(), 1.0).unwrap(), "#0000ff");
    }

    #[test]
    fn the_hsl_wheel_lands_on_the_primaries() {
        assert_eq!(hsl(0.0, 1.0, 0.5).unwrap(), "#ff0000");
        assert_eq!(hsl(120.0, 1.0, 0.5).unwrap(), "#00ff00");
        assert_eq!(hsl(240.0, 1.0, 0.5).unwrap(), "#0000ff");
        assert_eq!(hsl(360.0, 1.0, 0.5).unwrap(), "#ff0000");
    }

    #[test]
    fn luminance_answers_the_accessibility_questions() {
        assert!(close(contrast_ratio("#ffffff".to_string(), "#000000".to_string()).unwrap(), 21.0));
        assert!(close(contrast_ratio("#123456".to_string(), "#123456".to_string()).unwrap(), 1.0));
        assert!(is_dark("#000000".to_string()).unwrap());
        assert!(!is_dark("#ffffff".to_string()).unwrap());
        assert_eq!(text_on("#000000".to_string()).unwrap(), "#ffffff");
        assert_eq!(text_on("#ffff00".to_string()).unwrap(), "#000000");
    }

    #[test]
    fn grayscale_and_invert_hold_their_anchors() {
        assert_eq!(grayscale("#ffffff".to_string()).unwrap(), "#ffffff");
        assert_eq!(grayscale("#000000".to_string()).unwrap(), "#000000");
        assert_eq!(grayscale("#3366cc".to_string()).unwrap(), "#626262");
        assert_eq!(invert("#1a2b3c".to_string()).unwrap(), "#e5d4c3");
        assert_eq!(invert("#ffffff".to_string()).unwrap(), "#000000");
    }

    #[test]
    fn a_full_turn_of_the_wheel_comes_home() {
        assert!(channels_close(&rotate_hue("#3366cc".to_string(), 360.0).unwrap(), "#3366cc"));
        assert!(channels_close(&rotate_hue("#ff0000".to_string(), 360.0).unwrap(), "#ff0000"));
        assert_eq!(rotate_hue("#ff0000".to_string(), 180.0).unwrap(), "#00ffff");
    }

    #[test]
    fn junk_input_errors_name_the_problem() {
        assert!(red("#12".to_string()).unwrap_err().contains("3 or 6"));
        assert!(red("#zzzzzz".to_string()).unwrap_err().contains("not a hex byte"));
        assert!(rgb(300, 0, 0).unwrap_err().contains("0 to 255"));
        assert!(rgb(0, -1, 0).unwrap_err().contains("green"));
        assert!(lighten("#123456".to_string(), 1.5).unwrap_err().contains("between 0 and 1"));
        assert!(hsl(0.0, 2.0, 0.5).unwrap_err().contains("saturation"));
    }
}
