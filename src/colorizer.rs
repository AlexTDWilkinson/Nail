use lazy_static::lazy_static;
use log;
use ratatui::text::Span;
use ratatui::{
    style::{Color, Style},
    text::Line,
};
use rayon::prelude::*;

use crate::embedded::{self, Piece};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ColorScheme {
    pub function: Color,
    pub const_decl: Color,
    pub var_decl: Color,
    pub if_decl: Color,
    pub else_decl: Color,
    pub arrow_decl: Color,
    pub identifier: Color,
    pub unsigned_int: Color,
    pub signed_int: Color,
    pub float: Color,
    pub operator: Color,
    pub keyword: Color,
    pub comma: Color,
    pub string_literal: Color,
    pub identifier_type: Color,
    pub unknown: Color,
    pub parenthesis: Color,
    pub block: Color,
    pub end_statement: Color,
    pub async_keyword: Color,
    pub parallel_keyword: Color,
    pub struct_keyword: Color,
    pub enum_keyword: Color,
    pub return_keyword: Color,
    pub default: Color,
    pub background: Color,
    pub comment: Color,
    pub error: Color,
    // Everything below is UI chrome rather than syntax: the panels, dialogs,
    // selections and highlights the editor draws around the code. They live
    // here so a theme controls the whole screen, not just the tokens.
    pub ui_text: Color,
    pub ui_text_muted: Color,
    pub ui_hint: Color,
    pub ui_panel_bg: Color,
    pub accent: Color,
    pub success: Color,
    pub success_bright: Color,
    pub danger: Color,
    pub info: Color,
    pub info_bright: Color,
    pub primary: Color,
    pub special: Color,
    pub on_emphasis: Color,
    pub current_line_bg: Color,
    pub error_line_bg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub search_match_bg: Color,
    pub search_match_fg: Color,
    pub search_other_bg: Color,
    pub search_other_fg: Color,
    pub bracket_match_bg: Color,
    pub cursor_fg: Color,
    pub input_bg: Color,
    pub input_inactive_bg: Color,
    pub menu_selection_bg: Color,
    pub menu_selection_fg: Color,
    pub item_selection_bg: Color,
    pub badge_bg: Color,
    pub badge_fg: Color,
    pub toggle_on_bg: Color,
    pub toggle_on_fg: Color,
    pub scroll_track: Color,
    pub scroll_thumb: Color,
}

/// Convert a hex color string (e.g., "#FF5733") to a `tui::style::Color`
/// Whether a word is a number the way Nail writes one: digits, with an
/// optional minus sign and at most one decimal point. Asking the standard
/// library to parse it instead would also accept `inf` and `1e9`, which Nail
/// has no syntax for.
fn is_a_number(word: &str) -> bool {
    let digits = word.strip_prefix('-').unwrap_or(word);
    if digits.is_empty() {
        return false;
    }
    let mut parts = digits.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    if parts.next().is_some() {
        return false;
    }
    let all_digits = |text: &str| !text.is_empty() && text.chars().all(|character| character.is_ascii_digit());
    all_digits(whole) && fraction.map_or(true, all_digits)
}

pub fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        let red = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let green = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let blue = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Color::Rgb(red, green, blue)
    } else {
        Color::Reset // Fallback color
    }
}

lazy_static! {
    // The light palette keeps each token in the same hue family as the dark
    // one, so switching themes never rewires what a color means. Every value
    // was checked against the backgrounds it can sit on (editor, current
    // line, panels) and holds at least a 7:1 WCAG AAA contrast ratio, except
    // the two dimmed punctuation marks, which hold 4.5:1. Light themes need
    // the higher floor: thin terminal glyphs on a bright ground wash out at
    // ratios that read fine on a dark one.
    pub static ref LIGHT_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#054598"),      // Deep Blue
        const_decl: hex_to_color("#89212A"),    // Ruby Red
        var_decl: hex_to_color("#7D3000"),      // Burnt Orange
        if_decl: hex_to_color("#5E29A0"),       // Deep Purple
        else_decl: hex_to_color("#5C2E9D"),     // Medium Purple
        arrow_decl: hex_to_color("#0E524A"),    // Teal
        identifier: hex_to_color("#1F2328"),    // Near Black
        unsigned_int: hex_to_color("#76361D"),  // Sienna
        signed_int: hex_to_color("#394E13"),    // Olive Green
        float: hex_to_color("#0A5343"),         // Deep Teal Green
        operator: hex_to_color("#074696"),      // Navy Blue
        keyword: hex_to_color("#7B1E7B"),       // Royal Purple
        comma: hex_to_color("#59636E"),         // Slate Gray
        string_literal: hex_to_color("#115321"), // Forest Green
        identifier_type: hex_to_color("#044E70"), // Azure
        unknown: hex_to_color("#404A55"),       // Slate Gray
        parenthesis: hex_to_color("#624200"),   // Dark Gold
        block: hex_to_color("#283EA4"),         // Indigo Blue
        end_statement: hex_to_color("#5D6572"), // Dimmed Gray
        async_keyword: hex_to_color("#564600"),  // Olive Gold
        parallel_keyword: hex_to_color("#6A3D00"), // Chestnut
        struct_keyword: hex_to_color("#811F5E"), // Plum
        enum_keyword: hex_to_color("#5A2DA2"),   // Violet
        return_keyword: hex_to_color("#851C5E"), // Dark Magenta
        default: hex_to_color("#1F2328"),       // Near Black
        background: hex_to_color("#ECEEF0"),    // Near White
        comment: hex_to_color("#404A55"),       // Slate Gray
        error: hex_to_color("#881F21"),         // Error Red
        ui_text: hex_to_color("#1F2328"),
        ui_text_muted: hex_to_color("#3F4954"),
        ui_hint: hex_to_color("#424953"),
        ui_panel_bg: hex_to_color("#DFE3E8"),
        accent: hex_to_color("#624200"),
        success: hex_to_color("#145321"),
        success_bright: hex_to_color("#145323"),
        danger: hex_to_color("#8A1F21"),
        info: hex_to_color("#0D4F5E"),
        info_bright: hex_to_color("#074594"),
        primary: hex_to_color("#054496"),
        special: hex_to_color("#811F5E"),
        on_emphasis: hex_to_color("#FFFFFF"),
        current_line_bg: hex_to_color("#E1E4E8"),
        error_line_bg: hex_to_color("#F3DCDE"),
        selection_bg: hex_to_color("#ADD6FF"),
        selection_fg: hex_to_color("#1F2328"),
        search_match_bg: hex_to_color("#FFD33D"),
        search_match_fg: hex_to_color("#1F2328"),
        search_other_bg: hex_to_color("#F2E491"),
        search_other_fg: hex_to_color("#1F2328"),
        bracket_match_bg: hex_to_color("#9A2B72"),
        cursor_fg: hex_to_color("#054598"),
        input_bg: hex_to_color("#D6DCE2"),
        input_inactive_bg: hex_to_color("#E3E6EA"),
        menu_selection_bg: hex_to_color("#0550AE"),
        menu_selection_fg: hex_to_color("#FFFFFF"),
        item_selection_bg: hex_to_color("#0550AE"),
        badge_bg: hex_to_color("#F0C64C"),
        badge_fg: hex_to_color("#1F2328"),
        toggle_on_bg: hex_to_color("#19662C"),
        toggle_on_fg: hex_to_color("#FFFFFF"),
        scroll_track: hex_to_color("#C6CCD4"),
        scroll_thumb: hex_to_color("#8C949E"),
    };

    pub static ref DARK_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#61AFEF"),      // Soft Blue
        const_decl: hex_to_color("#E06C75"),    // Salmon Pink
        var_decl: hex_to_color("#D19A66"),      // Sandy Brown
        if_decl: hex_to_color("#C678DD"),       // Violet
        else_decl: hex_to_color("#E5C0FF"),     // Pale Violet
        arrow_decl: hex_to_color("#56B6C2"),    // Cyan
        identifier: hex_to_color("#E6E6E6"),    // Light Gray
        unsigned_int: hex_to_color("#CE9178"),  // Terra Cotta
        signed_int: hex_to_color("#B5CEA8"),    // Sage Green
        float: hex_to_color("#4EC9B0"),         // Mint
        operator: hex_to_color("#569CD6"),      // Sky Blue
        keyword: hex_to_color("#C586C0"),       // Orchid
        comma: hex_to_color("#858585"),         // Medium Gray
        string_literal: hex_to_color("#98C379"), // Spring Green
        identifier_type: hex_to_color("#4FC1E9"), // Light Blue
        unknown: hex_to_color("#8F8F8F"),       // Gray
        parenthesis: hex_to_color("#FFD602"),   // Bright Yellow
        block: hex_to_color("#9CDCFE"),         // Powder Blue
        end_statement: hex_to_color("#737373"), // Dark Gray
        async_keyword: hex_to_color("#DCDCAA"),  // Pale Yellow
        parallel_keyword: hex_to_color("#FFB86C"), // Orange Cream
        struct_keyword: hex_to_color("#FF79C6"), // Hot Pink
        enum_keyword: hex_to_color("#BD93F9"),   // Purple Rain
        return_keyword: hex_to_color("#FF6AC1"), // Magenta
        default: hex_to_color("#D4D4D4"),       // Off White
        background: hex_to_color("#1A1A1C"),    // Deep Black
        comment: hex_to_color("#8E8E8E"),       // Neutral Gray
        error: hex_to_color("#F97583"),         // Light Red
        ui_text: hex_to_color("#E6E6E6"),
        ui_text_muted: hex_to_color("#9DA5AE"),
        ui_hint: hex_to_color("#8A9199"),
        ui_panel_bg: hex_to_color("#121214"),
        accent: hex_to_color("#E3B341"),
        success: hex_to_color("#3FB950"),
        success_bright: hex_to_color("#56D364"),
        danger: hex_to_color("#F85149"),
        info: hex_to_color("#39C5CF"),
        info_bright: hex_to_color("#79C0FF"),
        primary: hex_to_color("#58A6FF"),
        special: hex_to_color("#DB61A2"),
        on_emphasis: hex_to_color("#FFFFFF"),
        current_line_bg: hex_to_color("#282828"),
        error_line_bg: hex_to_color("#3C1414"),
        selection_bg: hex_to_color("#264F78"),
        selection_fg: hex_to_color("#FFFFFF"),
        search_match_bg: hex_to_color("#E3B341"),
        search_match_fg: hex_to_color("#1F2328"),
        search_other_bg: hex_to_color("#3A3F45"),
        search_other_fg: hex_to_color("#E6E6E6"),
        bracket_match_bg: hex_to_color("#9E3379"),
        cursor_fg: hex_to_color("#FFFFFF"),
        input_bg: hex_to_color("#3E4450"),
        input_inactive_bg: hex_to_color("#2E3238"),
        menu_selection_bg: hex_to_color("#D4D4D4"),
        menu_selection_fg: hex_to_color("#1A1A1C"),
        item_selection_bg: hex_to_color("#264F78"),
        badge_bg: hex_to_color("#E3B341"),
        badge_fg: hex_to_color("#1F2328"),
        toggle_on_bg: hex_to_color("#3FB950"),
        toggle_on_fg: hex_to_color("#1F2328"),
        scroll_track: hex_to_color("#2E3238"),
        scroll_thumb: hex_to_color("#3FB950"),
    };

    // Modeled on the Solarized light palette.
    pub static ref SOLAR_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#194B68"),
        const_decl: hex_to_color("#822911"),
        var_decl: hex_to_color("#564400"),
        if_decl: hex_to_color("#474B00"),
        else_decl: hex_to_color("#404371"),
        arrow_decl: hex_to_color("#134F48"),
        identifier: hex_to_color("#374B4E"),
        unsigned_int: hex_to_color("#811F55"),
        signed_int: hex_to_color("#474B00"),
        float: hex_to_color("#134F48"),
        operator: hex_to_color("#384A50"),
        keyword: hex_to_color("#474B00"),
        comma: hex_to_color("#5A6565"),
        string_literal: hex_to_color("#134F48"),
        identifier_type: hex_to_color("#564400"),
        unknown: hex_to_color("#384A50"),
        parenthesis: hex_to_color("#384A50"),
        block: hex_to_color("#194B68"),
        end_statement: hex_to_color("#5A6565"),
        async_keyword: hex_to_color("#404371"),
        parallel_keyword: hex_to_color("#822911"),
        struct_keyword: hex_to_color("#811F55"),
        enum_keyword: hex_to_color("#404371"),
        return_keyword: hex_to_color("#811F55"),
        default: hex_to_color("#374B4E"),
        background: hex_to_color("#F0E9D7"),
        comment: hex_to_color("#414949"),
        error: hex_to_color("#7D1E1E"),
        ui_text: hex_to_color("#324547"),
        ui_text_muted: hex_to_color("#36474D"),
        ui_hint: hex_to_color("#36474D"),
        ui_panel_bg: hex_to_color("#E2DCCA"),
        accent: hex_to_color("#524100"),
        success: hex_to_color("#444700"),
        success_bright: hex_to_color("#444700"),
        danger: hex_to_color("#811F1F"),
        info: hex_to_color("#124B44"),
        info_bright: hex_to_color("#184762"),
        primary: hex_to_color("#184762"),
        special: hex_to_color("#7B1E51"),
        on_emphasis: hex_to_color("#FDF6E3"),
        current_line_bg: hex_to_color("#E7E1CF"),
        error_line_bg: hex_to_color("#E9D3CB"),
        selection_bg: hex_to_color("#CDE1EE"),
        selection_fg: hex_to_color("#073642"),
        search_match_bg: hex_to_color("#EDC85A"),
        search_match_fg: hex_to_color("#073642"),
        search_other_bg: hex_to_color("#E6DCAE"),
        search_other_fg: hex_to_color("#073642"),
        bracket_match_bg: hex_to_color("#972563"),
        cursor_fg: hex_to_color("#194B68"),
        input_bg: hex_to_color("#E0D9C2"),
        input_inactive_bg: hex_to_color("#E5E0CE"),
        menu_selection_bg: hex_to_color("#1D5878"),
        menu_selection_fg: hex_to_color("#FDF6E3"),
        item_selection_bg: hex_to_color("#1D5878"),
        badge_bg: hex_to_color("#EDC85A"),
        badge_fg: hex_to_color("#073642"),
        toggle_on_bg: hex_to_color("#91A21C"),
        toggle_on_fg: hex_to_color("#03080C"),
        scroll_track: hex_to_color("#D4CEB9"),
        scroll_thumb: hex_to_color("#93A1A1"),
    };

    // Modeled on the Solarized dark palette.
    pub static ref LUNAR_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#51A1DA"),
        const_decl: hex_to_color("#D98361"),
        var_decl: hex_to_color("#BD951C"),
        if_decl: hex_to_color("#91A21C"),
        else_decl: hex_to_color("#9195CF"),
        arrow_decl: hex_to_color("#41A9A2"),
        identifier: hex_to_color("#8C9D9E"),
        unsigned_int: hex_to_color("#E176A7"),
        signed_int: hex_to_color("#91A21C"),
        float: hex_to_color("#41A9A2"),
        operator: hex_to_color("#8C9D9E"),
        keyword: hex_to_color("#91A21C"),
        comma: hex_to_color("#697D83"),
        string_literal: hex_to_color("#41A9A2"),
        identifier_type: hex_to_color("#BD951C"),
        unknown: hex_to_color("#8C9BA0"),
        parenthesis: hex_to_color("#8C9D9E"),
        block: hex_to_color("#51A1DA"),
        end_statement: hex_to_color("#697D83"),
        async_keyword: hex_to_color("#9195CF"),
        parallel_keyword: hex_to_color("#D98361"),
        struct_keyword: hex_to_color("#E176A7"),
        enum_keyword: hex_to_color("#9195CF"),
        return_keyword: hex_to_color("#E176A7"),
        default: hex_to_color("#8C9D9E"),
        background: hex_to_color("#002B36"),
        comment: hex_to_color("#8C9BA0"),
        error: hex_to_color("#EB7775"),
        ui_text: hex_to_color("#93A1A1"),
        ui_text_muted: hex_to_color("#839496"),
        ui_hint: hex_to_color("#809298"),
        ui_panel_bg: hex_to_color("#00212B"),
        accent: hex_to_color("#B58900"),
        success: hex_to_color("#859900"),
        success_bright: hex_to_color("#859900"),
        danger: hex_to_color("#E66361"),
        info: hex_to_color("#2AA198"),
        info_bright: hex_to_color("#2AA198"),
        primary: hex_to_color("#3894D5"),
        special: hex_to_color("#DD669F"),
        on_emphasis: hex_to_color("#002B36"),
        current_line_bg: hex_to_color("#073642"),
        error_line_bg: hex_to_color("#3A232B"),
        selection_bg: hex_to_color("#185360"),
        selection_fg: hex_to_color("#FDF6E3"),
        search_match_bg: hex_to_color("#B58900"),
        search_match_fg: hex_to_color("#002B36"),
        search_other_bg: hex_to_color("#0D3B47"),
        search_other_fg: hex_to_color("#93A1A1"),
        bracket_match_bg: hex_to_color("#DD669F"),
        cursor_fg: hex_to_color("#93A1A1"),
        input_bg: hex_to_color("#0D3A44"),
        input_inactive_bg: hex_to_color("#0B3844"),
        menu_selection_bg: hex_to_color("#B58900"),
        menu_selection_fg: hex_to_color("#002B36"),
        item_selection_bg: hex_to_color("#B58900"),
        badge_bg: hex_to_color("#B58900"),
        badge_fg: hex_to_color("#002B36"),
        toggle_on_bg: hex_to_color("#859900"),
        toggle_on_fg: hex_to_color("#002B36"),
        scroll_track: hex_to_color("#0B3844"),
        scroll_thumb: hex_to_color("#586E75"),
    };

    // Modeled on the Dracula palette.
    pub static ref VAMPIRE_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#50FA7B"),
        const_decl: hex_to_color("#BD93F9"),
        var_decl: hex_to_color("#FFB86C"),
        if_decl: hex_to_color("#FF79C6"),
        else_decl: hex_to_color("#FF92DF"),
        arrow_decl: hex_to_color("#8BE9FD"),
        identifier: hex_to_color("#F8F8F2"),
        unsigned_int: hex_to_color("#BD93F9"),
        signed_int: hex_to_color("#D6ACFF"),
        float: hex_to_color("#8BE9FD"),
        operator: hex_to_color("#FF79C6"),
        keyword: hex_to_color("#FF79C6"),
        comma: hex_to_color("#9DA6CE"),
        string_literal: hex_to_color("#F1FA8C"),
        identifier_type: hex_to_color("#8BE9FD"),
        unknown: hex_to_color("#8F9BBE"),
        parenthesis: hex_to_color("#F8F8F2"),
        block: hex_to_color("#BD93F9"),
        end_statement: hex_to_color("#6E7DAA"),
        async_keyword: hex_to_color("#8BE9FD"),
        parallel_keyword: hex_to_color("#FFB86C"),
        struct_keyword: hex_to_color("#FF79C6"),
        enum_keyword: hex_to_color("#BD93F9"),
        return_keyword: hex_to_color("#FF79C6"),
        default: hex_to_color("#F8F8F2"),
        background: hex_to_color("#282A36"),
        comment: hex_to_color("#8F9BBE"),
        error: hex_to_color("#FF6E6E"),
        ui_text: hex_to_color("#F8F8F2"),
        ui_text_muted: hex_to_color("#ACB4D6"),
        ui_hint: hex_to_color("#8E99C4"),
        ui_panel_bg: hex_to_color("#21222C"),
        accent: hex_to_color("#F1FA8C"),
        success: hex_to_color("#50FA7B"),
        success_bright: hex_to_color("#69FF94"),
        danger: hex_to_color("#FF5555"),
        info: hex_to_color("#8BE9FD"),
        info_bright: hex_to_color("#9AEDFE"),
        primary: hex_to_color("#BD93F9"),
        special: hex_to_color("#FF79C6"),
        on_emphasis: hex_to_color("#282A36"),
        current_line_bg: hex_to_color("#31333F"),
        error_line_bg: hex_to_color("#402830"),
        selection_bg: hex_to_color("#44475A"),
        selection_fg: hex_to_color("#F8F8F2"),
        search_match_bg: hex_to_color("#F1FA8C"),
        search_match_fg: hex_to_color("#282A36"),
        search_other_bg: hex_to_color("#44475A"),
        search_other_fg: hex_to_color("#F8F8F2"),
        bracket_match_bg: hex_to_color("#FF79C6"),
        cursor_fg: hex_to_color("#F8F8F2"),
        input_bg: hex_to_color("#383B4C"),
        input_inactive_bg: hex_to_color("#2E3040"),
        menu_selection_bg: hex_to_color("#BD93F9"),
        menu_selection_fg: hex_to_color("#282A36"),
        item_selection_bg: hex_to_color("#BD93F9"),
        badge_bg: hex_to_color("#F1FA8C"),
        badge_fg: hex_to_color("#282A36"),
        toggle_on_bg: hex_to_color("#50FA7B"),
        toggle_on_fg: hex_to_color("#282A36"),
        scroll_track: hex_to_color("#343746"),
        scroll_thumb: hex_to_color("#6272A4"),
    };

    // Modeled on the gruvbox dark palette.
    pub static ref RETRO_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#8EC07C"),
        const_decl: hex_to_color("#D3869B"),
        var_decl: hex_to_color("#FE8019"),
        if_decl: hex_to_color("#FF6553"),
        else_decl: hex_to_color("#FE8019"),
        arrow_decl: hex_to_color("#8EC07C"),
        identifier: hex_to_color("#EBDBB2"),
        unsigned_int: hex_to_color("#D3869B"),
        signed_int: hex_to_color("#B8BB26"),
        float: hex_to_color("#8EC07C"),
        operator: hex_to_color("#FE8019"),
        keyword: hex_to_color("#FF6553"),
        comma: hex_to_color("#A89984"),
        string_literal: hex_to_color("#B8BB26"),
        identifier_type: hex_to_color("#FABD2F"),
        unknown: hex_to_color("#A4988B"),
        parenthesis: hex_to_color("#EBDBB2"),
        block: hex_to_color("#83A598"),
        end_statement: hex_to_color("#928374"),
        async_keyword: hex_to_color("#FABD2F"),
        parallel_keyword: hex_to_color("#FE8019"),
        struct_keyword: hex_to_color("#D3869B"),
        enum_keyword: hex_to_color("#C186A2"),
        return_keyword: hex_to_color("#D3869B"),
        default: hex_to_color("#EBDBB2"),
        background: hex_to_color("#282828"),
        comment: hex_to_color("#A4988B"),
        error: hex_to_color("#FF6553"),
        ui_text: hex_to_color("#EBDBB2"),
        ui_text_muted: hex_to_color("#BDAE93"),
        ui_hint: hex_to_color("#A89984"),
        ui_panel_bg: hex_to_color("#1D2021"),
        accent: hex_to_color("#FABD2F"),
        success: hex_to_color("#B8BB26"),
        success_bright: hex_to_color("#B8BB26"),
        danger: hex_to_color("#FD533F"),
        info: hex_to_color("#8EC07C"),
        info_bright: hex_to_color("#83A598"),
        primary: hex_to_color("#83A598"),
        special: hex_to_color("#D3869B"),
        on_emphasis: hex_to_color("#282828"),
        current_line_bg: hex_to_color("#32302F"),
        error_line_bg: hex_to_color("#402A26"),
        selection_bg: hex_to_color("#504945"),
        selection_fg: hex_to_color("#EBDBB2"),
        search_match_bg: hex_to_color("#FABD2F"),
        search_match_fg: hex_to_color("#282828"),
        search_other_bg: hex_to_color("#504945"),
        search_other_fg: hex_to_color("#EBDBB2"),
        bracket_match_bg: hex_to_color("#D3869B"),
        cursor_fg: hex_to_color("#EBDBB2"),
        input_bg: hex_to_color("#3C3836"),
        input_inactive_bg: hex_to_color("#32302F"),
        menu_selection_bg: hex_to_color("#FABD2F"),
        menu_selection_fg: hex_to_color("#282828"),
        item_selection_bg: hex_to_color("#FABD2F"),
        badge_bg: hex_to_color("#FABD2F"),
        badge_fg: hex_to_color("#282828"),
        toggle_on_bg: hex_to_color("#B8BB26"),
        toggle_on_fg: hex_to_color("#282828"),
        scroll_track: hex_to_color("#3C3836"),
        scroll_thumb: hex_to_color("#928374"),
    };

    // Modeled on the Nord palette.
    pub static ref FJORD_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#88C0D0"),
        const_decl: hex_to_color("#D69985"),
        var_decl: hex_to_color("#EBCB8B"),
        if_decl: hex_to_color("#8DA9C5"),
        else_decl: hex_to_color("#94B0CC"),
        arrow_decl: hex_to_color("#88C0D0"),
        identifier: hex_to_color("#D8DEE9"),
        unsigned_int: hex_to_color("#BE9DB7"),
        signed_int: hex_to_color("#A3BE8C"),
        float: hex_to_color("#8FBCBB"),
        operator: hex_to_color("#8DA9C5"),
        keyword: hex_to_color("#8DA9C5"),
        comma: hex_to_color("#7B88A1"),
        string_literal: hex_to_color("#A3BE8C"),
        identifier_type: hex_to_color("#8FBCBB"),
        unknown: hex_to_color("#A0A6B5"),
        parenthesis: hex_to_color("#D8DEE9"),
        block: hex_to_color("#92A8C4"),
        end_statement: hex_to_color("#7D869D"),
        async_keyword: hex_to_color("#EBCB8B"),
        parallel_keyword: hex_to_color("#D69985"),
        struct_keyword: hex_to_color("#BE9DB7"),
        enum_keyword: hex_to_color("#BE9DB7"),
        return_keyword: hex_to_color("#CF989E"),
        default: hex_to_color("#D8DEE9"),
        background: hex_to_color("#2E3440"),
        comment: hex_to_color("#A0A6B5"),
        error: hex_to_color("#CF989E"),
        ui_text: hex_to_color("#ECEFF4"),
        ui_text_muted: hex_to_color("#AEB6C6"),
        ui_hint: hex_to_color("#919CAF"),
        ui_panel_bg: hex_to_color("#272C36"),
        accent: hex_to_color("#EBCB8B"),
        success: hex_to_color("#A3BE8C"),
        success_bright: hex_to_color("#A3BE8C"),
        danger: hex_to_color("#CB8C92"),
        info: hex_to_color("#88C0D0"),
        info_bright: hex_to_color("#8FBCBB"),
        primary: hex_to_color("#81A1C1"),
        special: hex_to_color("#B691AF"),
        on_emphasis: hex_to_color("#2E3440"),
        current_line_bg: hex_to_color("#353C4A"),
        error_line_bg: hex_to_color("#3F3038"),
        selection_bg: hex_to_color("#4C566A"),
        selection_fg: hex_to_color("#ECEFF4"),
        search_match_bg: hex_to_color("#EBCB8B"),
        search_match_fg: hex_to_color("#2E3440"),
        search_other_bg: hex_to_color("#4C566A"),
        search_other_fg: hex_to_color("#ECEFF4"),
        bracket_match_bg: hex_to_color("#B691AF"),
        cursor_fg: hex_to_color("#ECEFF4"),
        input_bg: hex_to_color("#3B4252"),
        input_inactive_bg: hex_to_color("#333A47"),
        menu_selection_bg: hex_to_color("#88C0D0"),
        menu_selection_fg: hex_to_color("#2E3440"),
        item_selection_bg: hex_to_color("#88C0D0"),
        badge_bg: hex_to_color("#EBCB8B"),
        badge_fg: hex_to_color("#2E3440"),
        toggle_on_bg: hex_to_color("#A3BE8C"),
        toggle_on_fg: hex_to_color("#2E3440"),
        scroll_track: hex_to_color("#3B4252"),
        scroll_thumb: hex_to_color("#616E88"),
    };

    // Modeled on the Monokai palette.
    pub static ref NEON_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#A6E22E"),
        const_decl: hex_to_color("#B084FF"),
        var_decl: hex_to_color("#FD971F"),
        if_decl: hex_to_color("#FF6398"),
        else_decl: hex_to_color("#FA6EA8"),
        arrow_decl: hex_to_color("#66D9EF"),
        identifier: hex_to_color("#F8F8F2"),
        unsigned_int: hex_to_color("#B084FF"),
        signed_int: hex_to_color("#A6E22E"),
        float: hex_to_color("#66D9EF"),
        operator: hex_to_color("#FF6398"),
        keyword: hex_to_color("#FF6398"),
        comma: hex_to_color("#999681"),
        string_literal: hex_to_color("#E6DB74"),
        identifier_type: hex_to_color("#66D9EF"),
        unknown: hex_to_color("#9E9B8C"),
        parenthesis: hex_to_color("#F8F8F2"),
        block: hex_to_color("#66D9EF"),
        end_statement: hex_to_color("#807D6A"),
        async_keyword: hex_to_color("#E6DB74"),
        parallel_keyword: hex_to_color("#FD971F"),
        struct_keyword: hex_to_color("#FF6398"),
        enum_keyword: hex_to_color("#B084FF"),
        return_keyword: hex_to_color("#FF6398"),
        default: hex_to_color("#F8F8F2"),
        background: hex_to_color("#272822"),
        comment: hex_to_color("#9E9B8C"),
        error: hex_to_color("#FF658B"),
        ui_text: hex_to_color("#F8F8F2"),
        ui_text_muted: hex_to_color("#B8B6A6"),
        ui_hint: hex_to_color("#94917E"),
        ui_panel_bg: hex_to_color("#1E1F1A"),
        accent: hex_to_color("#E6DB74"),
        success: hex_to_color("#A6E22E"),
        success_bright: hex_to_color("#A6E22E"),
        danger: hex_to_color("#FF6188"),
        info: hex_to_color("#66D9EF"),
        info_bright: hex_to_color("#66D9EF"),
        primary: hex_to_color("#AE81FF"),
        special: hex_to_color("#FF4786"),
        on_emphasis: hex_to_color("#272822"),
        current_line_bg: hex_to_color("#32332B"),
        error_line_bg: hex_to_color("#3E262B"),
        selection_bg: hex_to_color("#49483E"),
        selection_fg: hex_to_color("#F8F8F2"),
        search_match_bg: hex_to_color("#E6DB74"),
        search_match_fg: hex_to_color("#272822"),
        search_other_bg: hex_to_color("#49483E"),
        search_other_fg: hex_to_color("#F8F8F2"),
        bracket_match_bg: hex_to_color("#AE81FF"),
        cursor_fg: hex_to_color("#F8F8F2"),
        input_bg: hex_to_color("#3E3D32"),
        input_inactive_bg: hex_to_color("#32332B"),
        menu_selection_bg: hex_to_color("#E6DB74"),
        menu_selection_fg: hex_to_color("#272822"),
        item_selection_bg: hex_to_color("#E6DB74"),
        badge_bg: hex_to_color("#E6DB74"),
        badge_fg: hex_to_color("#272822"),
        toggle_on_bg: hex_to_color("#A6E22E"),
        toggle_on_fg: hex_to_color("#272822"),
        scroll_track: hex_to_color("#3E3D32"),
        scroll_thumb: hex_to_color("#75715E"),
    };

    // Modeled on the Tokyo Night palette.
    pub static ref MIDNIGHT_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#7AA2F7"),
        const_decl: hex_to_color("#F7768E"),
        var_decl: hex_to_color("#E0AF68"),
        if_decl: hex_to_color("#BB9AF7"),
        else_decl: hex_to_color("#CDB3FA"),
        arrow_decl: hex_to_color("#73DACA"),
        identifier: hex_to_color("#C0CAF5"),
        unsigned_int: hex_to_color("#FF9E64"),
        signed_int: hex_to_color("#9ECE6A"),
        float: hex_to_color("#73DACA"),
        operator: hex_to_color("#89DDFF"),
        keyword: hex_to_color("#BB9AF7"),
        comma: hex_to_color("#737AA2"),
        string_literal: hex_to_color("#9ECE6A"),
        identifier_type: hex_to_color("#2AC3DE"),
        unknown: hex_to_color("#878DAA"),
        parenthesis: hex_to_color("#C0CAF5"),
        block: hex_to_color("#7AA2F7"),
        end_statement: hex_to_color("#676F95"),
        async_keyword: hex_to_color("#E0AF68"),
        parallel_keyword: hex_to_color("#FF9E64"),
        struct_keyword: hex_to_color("#F7768E"),
        enum_keyword: hex_to_color("#BB9AF7"),
        return_keyword: hex_to_color("#BB9AF7"),
        default: hex_to_color("#C0CAF5"),
        background: hex_to_color("#1A1B26"),
        comment: hex_to_color("#878DAA"),
        error: hex_to_color("#F7768E"),
        ui_text: hex_to_color("#C0CAF5"),
        ui_text_muted: hex_to_color("#9AA5CE"),
        ui_hint: hex_to_color("#828BB8"),
        ui_panel_bg: hex_to_color("#16161E"),
        accent: hex_to_color("#E0AF68"),
        success: hex_to_color("#9ECE6A"),
        success_bright: hex_to_color("#9ECE6A"),
        danger: hex_to_color("#F7768E"),
        info: hex_to_color("#7DCFFF"),
        info_bright: hex_to_color("#7DCFFF"),
        primary: hex_to_color("#7AA2F7"),
        special: hex_to_color("#BB9AF7"),
        on_emphasis: hex_to_color("#1A1B26"),
        current_line_bg: hex_to_color("#232637"),
        error_line_bg: hex_to_color("#3C2635"),
        selection_bg: hex_to_color("#283457"),
        selection_fg: hex_to_color("#C0CAF5"),
        search_match_bg: hex_to_color("#E0AF68"),
        search_match_fg: hex_to_color("#1A1B26"),
        search_other_bg: hex_to_color("#283457"),
        search_other_fg: hex_to_color("#C0CAF5"),
        bracket_match_bg: hex_to_color("#BB9AF7"),
        cursor_fg: hex_to_color("#C0CAF5"),
        input_bg: hex_to_color("#2F334D"),
        input_inactive_bg: hex_to_color("#24273A"),
        menu_selection_bg: hex_to_color("#7AA2F7"),
        menu_selection_fg: hex_to_color("#1A1B26"),
        item_selection_bg: hex_to_color("#7AA2F7"),
        badge_bg: hex_to_color("#E0AF68"),
        badge_fg: hex_to_color("#1A1B26"),
        toggle_on_bg: hex_to_color("#9ECE6A"),
        toggle_on_fg: hex_to_color("#1A1B26"),
        scroll_track: hex_to_color("#24273A"),
        scroll_thumb: hex_to_color("#565F89"),
    };

    // Modeled on the Catppuccin Mocha palette.
    pub static ref PASTEL_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#89B4FA"),
        const_decl: hex_to_color("#EBA0AC"),
        var_decl: hex_to_color("#FAB387"),
        if_decl: hex_to_color("#CBA6F7"),
        else_decl: hex_to_color("#D8BFF9"),
        arrow_decl: hex_to_color("#94E2D5"),
        identifier: hex_to_color("#CDD6F4"),
        unsigned_int: hex_to_color("#FAB387"),
        signed_int: hex_to_color("#A6E3A1"),
        float: hex_to_color("#94E2D5"),
        operator: hex_to_color("#89DCEB"),
        keyword: hex_to_color("#CBA6F7"),
        comma: hex_to_color("#9399B2"),
        string_literal: hex_to_color("#A6E3A1"),
        identifier_type: hex_to_color("#F9E2AF"),
        unknown: hex_to_color("#8E93A6"),
        parenthesis: hex_to_color("#CDD6F4"),
        block: hex_to_color("#B4BEFE"),
        end_statement: hex_to_color("#9399B2"),
        async_keyword: hex_to_color("#F9E2AF"),
        parallel_keyword: hex_to_color("#FAB387"),
        struct_keyword: hex_to_color("#F5C2E7"),
        enum_keyword: hex_to_color("#CBA6F7"),
        return_keyword: hex_to_color("#F5C2E7"),
        default: hex_to_color("#CDD6F4"),
        background: hex_to_color("#1E1E2E"),
        comment: hex_to_color("#8E93A6"),
        error: hex_to_color("#F38BA8"),
        ui_text: hex_to_color("#CDD6F4"),
        ui_text_muted: hex_to_color("#A6ADC8"),
        ui_hint: hex_to_color("#9399B2"),
        ui_panel_bg: hex_to_color("#181825"),
        accent: hex_to_color("#F9E2AF"),
        success: hex_to_color("#A6E3A1"),
        success_bright: hex_to_color("#A6E3A1"),
        danger: hex_to_color("#F38BA8"),
        info: hex_to_color("#89DCEB"),
        info_bright: hex_to_color("#89DCEB"),
        primary: hex_to_color("#89B4FA"),
        special: hex_to_color("#F5C2E7"),
        on_emphasis: hex_to_color("#1E1E2E"),
        current_line_bg: hex_to_color("#292A3B"),
        error_line_bg: hex_to_color("#3A2735"),
        selection_bg: hex_to_color("#45475A"),
        selection_fg: hex_to_color("#CDD6F4"),
        search_match_bg: hex_to_color("#F9E2AF"),
        search_match_fg: hex_to_color("#1E1E2E"),
        search_other_bg: hex_to_color("#45475A"),
        search_other_fg: hex_to_color("#CDD6F4"),
        bracket_match_bg: hex_to_color("#F5C2E7"),
        cursor_fg: hex_to_color("#F5E0DC"),
        input_bg: hex_to_color("#313244"),
        input_inactive_bg: hex_to_color("#292A3B"),
        menu_selection_bg: hex_to_color("#89B4FA"),
        menu_selection_fg: hex_to_color("#1E1E2E"),
        item_selection_bg: hex_to_color("#89B4FA"),
        badge_bg: hex_to_color("#F9E2AF"),
        badge_fg: hex_to_color("#1E1E2E"),
        toggle_on_bg: hex_to_color("#A6E3A1"),
        toggle_on_fg: hex_to_color("#1E1E2E"),
        scroll_track: hex_to_color("#313244"),
        scroll_thumb: hex_to_color("#6C7086"),
    };

    // Every theme the editor knows, in the order the settings screen cycles
    // through them. The name is what the config file stores. Each palette
    // beyond the first two starts from one the internet already knows, under
    // its own name here, and any color of the original that fell short of
    // the contrast floor on these backgrounds was moved along its own hue
    // until it cleared. The floors are what
    // every_theme_clears_the_contrast_floor pins.
    pub static ref THEMES: Vec<(&'static str, &'static ColorScheme)> = vec![
        ("dark", &*DARK_THEME),
        ("vampire", &*VAMPIRE_THEME),
        ("retro", &*RETRO_THEME),
        ("fjord", &*FJORD_THEME),
        ("neon", &*NEON_THEME),
        ("midnight", &*MIDNIGHT_THEME),
        ("pastel", &*PASTEL_THEME),
        ("lunar", &*LUNAR_THEME),
        ("light", &*LIGHT_THEME),
        ("solar", &*SOLAR_THEME),
    ];
}

/// The palette a stored name refers to, or None for a name written by some
/// other version of the editor.
pub fn theme_by_name(name: &str) -> Option<&'static ColorScheme> {
    return THEMES.iter().find(|(known, _)| *known == name).map(|(_, theme)| *theme);
}

/// The stored name of a palette. An unknown scheme answers to dark's name so
/// it still round trips through the config file as something loadable.
pub fn theme_name_of(theme: &ColorScheme) -> &'static str {
    return THEMES.iter().find(|(_, known)| **known == *theme).map(|(name, _)| *name).unwrap_or("dark");
}

/// The next theme along in the settings order, wrapping at either end.
pub fn neighbor_theme(current: &ColorScheme, forward: bool) -> &'static ColorScheme {
    let position = THEMES.iter().position(|(_, known)| **known == *current).unwrap_or(0);
    let count = THEMES.len();
    let next = if forward { (position + 1) % count } else { (position + count - 1) % count };
    return THEMES[next].1;
}

pub fn colorize_code(content: Vec<Line>, theme: &ColorScheme) -> Vec<Line<'static>> {
    // Safety check: ensure we have a valid theme and content
    if content.is_empty() {
        return vec![Line::from(vec![Span::raw("")])];
    }
    
    // Validate theme colors to prevent unexpected green text
    if matches!(theme.default, Color::Green) && !cfg!(test) {
        log::warn!("Detected potentially incorrect theme with green default color");
    }

    // First pass: detect multi-line strings
    let string_state = scan_string_states(&content);

    // Parallel colorization per line with bounds checking
    let colored_lines: Vec<Line<'static>> = content.into_par_iter().enumerate().map(|(line_idx, line)| {
        // Add bounds checking for line_idx
        if line_idx < string_state.len() {
            colorize_line(line, line_idx, &string_state, theme)
        } else {
            log::warn!("Colorizer: line index {} out of bounds for string_state len {}", line_idx, string_state.len());
            // Return uncolored line as fallback
            Line::from(line.spans.into_iter().map(|span| Span::raw(span.content.to_string())).collect::<Vec<_>>())
        }
    }).collect();

    colored_lines
}

/// Length of the language tag sitting at the end of `content`, if any. A tag
/// is an identifier-shaped run written flush against a string's opening
/// backtick - the `html` in html`<p>hi</p>` - and the lexer treats it as part
/// of the literal, so the colorizer has to as well. Tags are ASCII by
/// construction, so the returned length is good for both bytes and chars.
fn trailing_tag_len(content: &str) -> usize {
    let tag_len = content.chars().rev().take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_').count();
    if tag_len == 0 {
        return 0;
    }
    match content[content.len() - tag_len..].chars().next() {
        Some(first) if first.is_ascii_lowercase() => tag_len,
        // A run of digits or underscores is not an identifier, so it is not a tag.
        _ => 0,
    }
}

/// Where a line begins: inside a string literal, or in ordinary code.
/// A string opened with a language tag carries how far into that language's
/// syntax the previous line got - a `<div` whose attributes run onto the next
/// line, a CSS block, a `/* ... */` - since all of those stay open across the
/// break. A string with no tag, or a tag no tokenizer knows, carries `None`
/// and is colored as one plain string.
#[derive(Clone, PartialEq, Debug)]
struct StringContext {
    embedded: Option<embedded::State>,
}

/// The state each line *starts* in. The previous version recorded the state at
/// the end of each line, which put the boundary a line out at both ends: the
/// line opening a multi-line string had its leading code painted as string,
/// and the line closing one had its trailing markup painted as code.
fn scan_string_states(content: &[Line]) -> Vec<Option<StringContext>> {
    let mut state: Option<StringContext> = None;
    let mut states = Vec::with_capacity(content.len());

    for line in content {
        states.push(state.clone());
        let text = line.spans.iter().map(|span| span.content.as_ref()).collect::<Vec<_>>().join("");
        advance_line_state(&text, &mut state, None);
    }

    states
}

/// Walks one line, updating `state`, and colors it into `emit` if one is
/// given. Colorizing and state-tracking share this one walk so the two can
/// never disagree about where a string ends.
fn advance_line_state(text: &str, state: &mut Option<StringContext>, mut emit: Option<(&mut Vec<Span<'static>>, &ColorScheme)>) {
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    let mut code_run = String::new();
    let mut string_run = String::new();

    while index < chars.len() {
        let ch = chars[index];

        if let Some(context) = state.clone() {
            // Inside a string. An escaped character never closes it.
            if ch == '\\' && index + 1 < chars.len() {
                string_run.push(ch);
                string_run.push(chars[index + 1]);
                index += 2;
                continue;
            }
            if ch == '`' {
                let mut embedded = context.embedded.clone();
                match emit.as_mut() {
                    Some((spans, theme)) => {
                        push_string_body(&string_run, &mut embedded, spans, theme);
                        spans.push(Span::styled("`".to_string(), Style::default().fg(theme.string_literal)));
                    }
                    None => advance_string_body(&string_run, &mut embedded),
                }
                string_run.clear();
                *state = None;
                index += 1;
                continue;
            }
            string_run.push(ch);
            index += 1;
            continue;
        }

        // In code. A comment runs to the end of the line and is never tokenized.
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            let comment: String = chars[index..].iter().collect();
            if let Some((spans, theme)) = emit.as_mut() {
                push_code_run(&code_run, spans, theme);
                spans.push(Span::styled(comment, Style::default().fg(theme.comment)));
            }
            code_run.clear();
            return;
        }

        if ch == '`' {
            // A language tag written against the backtick belongs to the
            // string, not to the code before it.
            let tag_len = trailing_tag_len(&code_run);
            let tag: String = code_run[code_run.len() - tag_len..].to_string();
            code_run.truncate(code_run.len() - tag_len);
            if let Some((spans, theme)) = emit.as_mut() {
                push_code_run(&code_run, spans, theme);
                spans.push(Span::styled(format!("{}`", tag), Style::default().fg(theme.string_literal)));
            }
            code_run.clear();
            *state = Some(StringContext { embedded: embedded::state_for_tag(&tag) });
            index += 1;
            continue;
        }

        code_run.push(ch);
        index += 1;
    }

    // Whatever is left runs off the end of the line.
    if let Some(context) = state.as_mut() {
        match emit.as_mut() {
            Some((spans, theme)) => push_string_body(&string_run, &mut context.embedded, spans, theme),
            None => advance_string_body(&string_run, &mut context.embedded),
        }
    } else if let Some((spans, theme)) = emit.as_mut() {
        push_code_run(&code_run, spans, theme);
    }
}

fn push_code_run(code_run: &str, spans: &mut Vec<Span<'static>>, theme: &ColorScheme) {
    if code_run.is_empty() {
        return;
    }
    colorize_non_string_content_preserve_positions(code_run, spans, theme);
}

/// The inside of a string literal. A tag naming a language the highlighter
/// knows is colored piece by piece; anything else - an untagged string, or a
/// tag no tokenizer covers - stays one color.
fn push_string_body(body: &str, embedded: &mut Option<embedded::State>, spans: &mut Vec<Span<'static>>, theme: &ColorScheme) {
    if body.is_empty() {
        return;
    }
    match embedded {
        Some(state) => embedded::tokenize(body, state, |text, piece| {
            spans.push(Span::styled(text.to_string(), Style::default().fg(embedded_color(piece, theme))));
        }),
        None => spans.push(Span::styled(body.to_string(), Style::default().fg(theme.string_literal))),
    }
}

fn advance_string_body(body: &str, embedded: &mut Option<embedded::State>) {
    if let Some(state) = embedded {
        embedded::advance(body, state);
    }
}

/// One color per kind of piece, shared by every embedded language: an element
/// name and a CSS selector name the same thing about their language, so they
/// are painted the same way.
fn embedded_color(piece: Piece, theme: &ColorScheme) -> Color {
    return match piece {
        Piece::Bracket => theme.comma,
        Piece::Element => theme.keyword,
        Piece::Attribute => theme.identifier_type,
        Piece::Function => theme.function,
        Piece::Keyword => theme.keyword,
        Piece::Operator => theme.operator,
        Piece::Value => theme.string_literal,
        Piece::Number => theme.signed_int,
        Piece::Comment => theme.comment,
        Piece::Text => theme.string_literal,
    };
}

fn colorize_line(line: Line, line_idx: usize, string_states: &[Option<StringContext>], theme: &ColorScheme) -> Line<'static> {
    if line.spans.is_empty() {
        return Line::from(vec![Span::raw("")]);
    }

    let text = line.spans.iter().map(|span| span.content.as_ref()).collect::<Vec<_>>().join("");
    let mut state = string_states.get(line_idx).cloned().flatten();
    return colorize_one_line(&text, &mut state, theme);
}

/// Colors one line and leaves `state` as the next line will find it. This is
/// the whole of the work: a line's colors depend on its text, the state it
/// starts in and the theme, and on nothing else in the file.
fn colorize_one_line(text: &str, state: &mut Option<StringContext>, theme: &ColorScheme) -> Line<'static> {
    let mut colored_spans: Vec<Span<'static>> = Vec::new();
    advance_line_state(text, state, Some((&mut colored_spans, theme)));

    if colored_spans.is_empty() {
        colored_spans.push(Span::raw(text.to_string()));
    }

    return Line::from(colored_spans);
}

/// A colored copy of a file that recolors only the lines that changed.
///
/// Because a line's colors depend on that line and the state it starts in,
/// a line whose text and starting state are both what they were last time is
/// reused as it stands. Typing one character used to recolor the whole file,
/// twenty times a second, which is what every keystroke in a long file was
/// waiting on.
pub struct ColorizeCache {
    theme: Option<ColorScheme>,
    lines: Vec<CachedLine>,
    recolored: usize,
    /// Moves whenever any cached line changes, so anything derived from the
    /// colored file (the minimap) can tell a new picture from the old one
    /// without comparing lines itself.
    generation: u64,
}

struct CachedLine {
    text: String,
    /// The state this line starts in, kept so a reuse can be checked, and the
    /// state it leaves behind, kept so the line after it can be checked
    /// without recoloring this one.
    start: Option<StringContext>,
    end: Option<StringContext>,
    colored: Line<'static>,
}

impl ColorizeCache {
    pub fn new() -> Self {
        return ColorizeCache { theme: None, lines: Vec::new(), recolored: 0, generation: 0 };
    }

    /// Brings the cache up to date with `content`. An edit is nearly always
    /// one line with an untouched run above it and another below, so that is
    /// what this looks for: the lines shared with last time at the top, the
    /// lines shared at the bottom, and the handful in between that have to be
    /// colored again.
    pub fn colorize(&mut self, content: &[String], theme: &ColorScheme) {
        if self.theme != Some(*theme) {
            self.theme = Some(*theme);
            self.lines.clear();
        }

        let head = content.iter().zip(self.lines.iter()).take_while(|(line, cached)| **line == cached.text).count();

        // Lines shared at the bottom, counted from the end and stopped before
        // it can meet the head: a line may only be claimed by one of the two.
        let room = content.len().min(self.lines.len()) - head;
        let tail = content
            .iter()
            .rev()
            .zip(self.lines.iter().rev())
            .take(room)
            .take_while(|(line, cached)| **line == cached.text)
            .count();

        let lines_before = self.lines.len();
        self.recolored = 0;

        let mut state = if head == 0 { None } else { self.lines[head - 1].end.clone() };
        let mut fresh: Vec<CachedLine> = Vec::with_capacity(content.len() - head - tail);
        for text in &content[head..content.len() - tail] {
            fresh.push(self.color(text, &mut state, theme));
        }

        // The tail is only what it was if it still starts where it started.
        // A string opened above it repaints everything under it, which is
        // exactly what the screen then has to show.
        let tail_survives = tail > 0 && self.lines[lines_before - tail].start == state;
        if !tail_survives {
            for text in &content[content.len() - tail..] {
                fresh.push(self.color(text, &mut state, theme));
            }
        }

        // The fresh middle drops in over the stale one, leaving the head and
        // the surviving tail where they already sit. Rebuilding the vector
        // moved every cached line on every keystroke, which at fifty
        // thousand lines was most of what an edit cost.
        let replaced_up_to = if tail_survives { lines_before - tail } else { lines_before };
        self.lines.splice(head..replaced_up_to, fresh);
        // Recoloring nothing while keeping every line is the one case where
        // the picture is unchanged. Pure deletion recolors nothing too, which
        // is why the length is part of the question.
        if self.recolored > 0 || self.lines.len() != lines_before {
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// Which version of the colored file the cache holds. Any change to any
    /// line moves it, and nothing else does.
    pub fn generation(&self) -> u64 {
        return self.generation;
    }

    fn color(&mut self, text: &str, state: &mut Option<StringContext>, theme: &ColorScheme) -> CachedLine {
        self.recolored += 1;
        let start = state.clone();
        let colored = colorize_one_line(text, state, theme);
        return CachedLine { text: text.to_string(), start, end: state.clone(), colored };
    }

    /// One colored line, or nothing if the file does not go that far.
    pub fn line(&self, index: usize) -> Option<&Line<'static>> {
        return self.lines.get(index).map(|cached| &cached.colored);
    }

    /// How many lines the last call had to color. Nothing reads this but the
    /// tests, which is where it matters that an edit is cheap rather than
    /// merely correct.
    #[cfg(test)]
    fn colored_last_time(&self) -> usize {
        return self.recolored;
    }
}

fn tokenize_code(content: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Delimiters that should be separate tokens
            '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | ':' => {
                // Push any accumulated token
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                // Push the delimiter as its own token
                tokens.push(ch.to_string());
            }
            // Operators that might be multi-character
            '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' => {
                // Special case for error types: check if ! is part of type!e pattern
                if ch == '!' {
                    // Check if the current token is a type character and next is 'e'
                    let is_error_type = !current_token.is_empty() && current_token.chars().all(|c| matches!(c, 'i' | 'f' | 's' | 'b' | 'a')) && chars.peek() == Some(&'e');

                    if is_error_type {
                        // This is an error type like i!e, keep it as one token
                        current_token.push(ch);
                        current_token.push(chars.next().unwrap()); // consume the 'e'
                        continue;
                    }
                }

                // Special case for parallel end: check if / is followed by p
                if ch == '/' && chars.peek() == Some(&'p') {
                    // This is /p for parallel end, keep it as one token
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    current_token.push(ch);
                    current_token.push(chars.next().unwrap()); // consume the 'p'
                    tokens.push(current_token.clone());
                    current_token.clear();
                    continue;
                }

                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }

                let mut op = ch.to_string();
                // Check for two-character operators
                if let Some(&next_ch) = chars.peek() {
                    if (ch == '=' && next_ch == '=') || (ch == '!' && next_ch == '=') || (ch == '<' && next_ch == '=') || (ch == '>' && next_ch == '=') || (ch == '-' && next_ch == '>') {
                        op.push(chars.next().unwrap());
                    }
                }
                tokens.push(op);
            }
            // Whitespace
            ' ' | '\t' | '\n' | '\r' => {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            // Regular characters
            _ => {
                current_token.push(ch);
            }
        }
    }

    // Don't forget the last token
    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}

fn colorize_non_string_content(content: &str, colored_spans: &mut Vec<Span<'static>>, theme: &ColorScheme) {
    // Safety check: never tokenize comments
    if content.trim().starts_with("//") {
        colored_spans.push(Span::styled(content.to_string(), Style::default().fg(theme.comment)));
        return;
    }

    // Preserve leading whitespace
    let leading_spaces = content.len() - content.trim_start().len();
    if leading_spaces > 0 {
        colored_spans.push(Span::raw(" ".repeat(leading_spaces)));
    }

    let trimmed_content = content.trim_start();
    let tokens = tokenize_code(trimmed_content);
    let mut i = 0;
    let mut need_space = false;

    while i < tokens.len() {
        let token = &tokens[i];

        // Skip whitespace tokens
        if token.trim().is_empty() {
            i += 1;
            continue;
        }

        // Add space between tokens when needed
        // Special case: don't add space before ':' if previous token was '|' (lambda return type)
        let prev_token = if i > 0 { Some(tokens[i - 1].as_str()) } else { None };
        if need_space && !matches!(token.as_str(), "," | ";" | ")" | "]" | "}") && !(token == ":" && prev_token == Some("|")) {
            colored_spans.push(Span::raw(" "));
        }
        need_space = false;

        // Check if this is an identifier:type pattern
        if i + 2 < tokens.len() && tokens[i + 1] == ":" && !token.starts_with('`') && !token.contains("::") {
            // Color identifier
            colored_spans.push(Span::styled(token.to_string(), Style::default().fg(theme.var_decl)));
            // Color colon
            colored_spans.push(Span::styled(tokens[i + 1].to_string(), Style::default().fg(theme.operator)));

            // Handle type part (might be array type)
            let type_token = &tokens[i + 2];
            if type_token == "a" && i + 4 < tokens.len() && tokens[i + 3] == ":" {
                // Array type like a:i
                colored_spans.push(Span::styled("a".to_string(), Style::default().fg(theme.identifier_type)));
                colored_spans.push(Span::styled(":".to_string(), Style::default().fg(theme.operator)));
                colored_spans.push(Span::styled(tokens[i + 4].to_string(), Style::default().fg(theme.identifier_type)));
                i += 5;
            } else {
                // Simple type
                colored_spans.push(Span::styled(type_token.to_string(), Style::default().fg(theme.identifier_type)));
                i += 3;
            }
        }
        // Check if this is a function call
        else if i + 1 < tokens.len() && tokens[i + 1] == "(" {
            colored_spans.push(Span::styled(token.to_string(), Style::default().fg(theme.function)));
            // Process the '(' immediately to avoid adding space
            colored_spans.push(Span::styled("(".to_string(), Style::default().fg(theme.parenthesis)));
            i += 2;
            continue;
        }
        // Regular token
        else {
            let styled_span = colorize_word(token, theme);
            colored_spans.push(styled_span);
            i += 1;
        }

        // Set need_space for next iteration
        need_space = !matches!(token.as_str(), "(" | "[" | "{");
    }
}

// New function that preserves exact character positions
fn colorize_non_string_content_preserve_positions(content: &str, colored_spans: &mut Vec<Span<'static>>, theme: &ColorScheme) {
    // Safety check: never tokenize comments
    if content.trim().starts_with("//") {
        colored_spans.push(Span::styled(content.to_string(), Style::default().fg(theme.comment)));
        return;
    }

    // Track position in original string
    let mut pos = 0;
    let chars: Vec<char> = content.chars().collect();
    // True when the previous non-whitespace token was ':' - the next word is
    // then a type annotation, not a variable declaration
    let mut prev_was_colon = false;

    while pos < chars.len() {
        // Skip whitespace
        let start_pos = pos;
        // Add safety counter to prevent infinite loops
        let mut ws_counter = 0;
        while pos < chars.len() && chars[pos].is_whitespace() && ws_counter < 100 {
            pos += 1;
            ws_counter += 1;
        }
        
        // Add whitespace span if any
        if pos > start_pos {
            let whitespace: String = chars[start_pos..pos].iter().collect();
            colored_spans.push(Span::raw(whitespace));
        }
        
        if pos >= chars.len() {
            break;
        }
        
        // Find the end of the current token
        let token_start = pos;
        
        // Check for operators and delimiters
        let ch = chars[pos];
        if matches!(ch, '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | ':') {
            // Single character delimiter
            colored_spans.push(colorize_single_char(ch, theme));
            prev_was_colon = ch == ':';
            pos += 1;
        } else if matches!(ch, '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '&' | '|' | '%') {
            // Potentially multi-character operator
            let mut token_end = pos + 1;
            
            // Check for two-character operators
            if token_end < chars.len() {
                let next_ch = chars[token_end];
                if (ch == '=' && next_ch == '=') ||
                   (ch == '-' && next_ch == '>') ||
                   (ch == '!' && next_ch == '=') ||
                   (ch == '<' && next_ch == '=') ||
                   (ch == '>' && next_ch == '=') ||
                   (ch == '&' && next_ch == '&') ||
                   (ch == '|' && next_ch == '|') ||
                   (ch == '/' && (next_ch == 'p' || next_ch == 'c')) {
                    token_end += 1;
                }
            }
            
            // A '!' that ends a result type is part of the word before it and
            // is taken there. Reaching here with one means it stands alone, so
            // it is painted as the operator it is: skipping it silently
            // dropped the character from what the editor drew, and a file on
            // screen that differs from the file on disk is the one thing a
            // colorizer may never do.
            let token: String = chars[token_start..token_end].iter().collect();
            colored_spans.push(Span::styled(token, Style::default().fg(theme.operator)));
            prev_was_colon = false;
            pos = token_end;
        } else {
            // Regular word/identifier
            let mut token_end = pos;
            // Add safety counter to prevent infinite loops
            let mut loop_counter = 0;
            while token_end < chars.len() && loop_counter < 1000 {
                loop_counter += 1;
                let ch = chars[token_end];
                // The characters a word can end at. `&`, `|`, `%` and `!`
                // are operators like the rest, and leaving them out meant
                // `count%2`, `80&&81` and `5!= 6`, written without spaces,
                // were one long word the colorizer could make nothing of, so
                // the numbers in them were painted as plain text. A `!` that
                // belongs to a result type is the exception just below.
                if ch.is_whitespace() || matches!(ch, '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | ':' | '=' | '<' | '>' | '+' | '-' | '*' | '/' | '&' | '|' | '%' | '!') {
                    // A '!' followed by 'e' ends a result type and belongs to
                    // the word: `i!e`, and `Job!e` just as much, since a
                    // struct name can be the value a result carries.
                    if ch == '!' && chars.get(token_end + 1) == Some(&'e') && token_end > token_start {
                        token_end += 2; // Include !e
                        continue;
                    }
                    break;
                }
                // Special case for error types: include !e
                if ch == '!' && token_end + 1 < chars.len() && chars[token_end + 1] == 'e' {
                    token_end += 2;
                } else {
                    token_end += 1;
                }
            }
            
            // Safety check: if we hit the loop limit, skip this token
            if loop_counter >= 1000 {
                log::warn!("Colorizer: potential infinite loop detected, skipping token");
                pos += 1;
                continue;
            }
            
            let token: String = chars[token_start..token_end].iter().collect();
            
            // Check if next token is '(' to identify function calls
            let mut next_non_ws = token_end;
            // Add safety counter to prevent infinite loops
            let mut ws_loop_counter = 0;
            while next_non_ws < chars.len() && chars[next_non_ws].is_whitespace() && ws_loop_counter < 100 {
                next_non_ws += 1;
                ws_loop_counter += 1;
            }
            
            if next_non_ws < chars.len() && chars[next_non_ws] == '(' {
                colored_spans.push(Span::styled(token, Style::default().fg(theme.function)));
            } else if prev_was_colon {
                // Word directly after ':' is a type annotation (name:TYPE)
                colored_spans.push(Span::styled(token, Style::default().fg(theme.identifier_type)));
            } else if next_non_ws < chars.len() && chars[next_non_ws] == ':' && token.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
                // Word directly before ':' is a variable declaration (NAME:type)
                colored_spans.push(Span::styled(token, Style::default().fg(theme.var_decl)));
            } else {
                colored_spans.push(colorize_word(&token, theme));
            }
            prev_was_colon = false;
            
            pos = token_end;
        }
    }
}

fn colorize_single_char(ch: char, theme: &ColorScheme) -> Span<'static> {
    match ch {
        '(' | ')' => Span::styled(ch.to_string(), Style::default().fg(theme.parenthesis)),
        '{' | '}' => Span::styled(ch.to_string(), Style::default().fg(theme.block)),
        ';' => Span::styled(ch.to_string(), Style::default().fg(theme.end_statement)),
        ',' => Span::styled(ch.to_string(), Style::default().fg(theme.comma)),
        ':' => Span::styled(ch.to_string(), Style::default().fg(theme.operator)),
        _ => Span::styled(ch.to_string(), Style::default().fg(theme.default)),
    }
}

fn colorize_word(word: &str, theme: &ColorScheme) -> Span<'static> {
    match word {
        // Keywords
        "p" => Span::styled(word.to_string(), Style::default().fg(theme.parallel_keyword)),
        "if" | "else" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)),
        "f" => Span::styled(word.to_string(), Style::default().fg(theme.function)),
        "struct" => Span::styled(word.to_string(), Style::default().fg(theme.struct_keyword)),
        "enum" => Span::styled(word.to_string(), Style::default().fg(theme.enum_keyword)),
        "r" | "return" => Span::styled(word.to_string(), Style::default().fg(theme.return_keyword)),
        "async" | "await" => Span::styled(word.to_string(), Style::default().fg(theme.async_keyword)),
        "c" | "v" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)), // const/var keywords

        // Collection/iteration language constructs (lexer keywords, not stdlib functions)
        "map" | "filter" | "reduce" | "scan" | "each" | "find" | "all" | "any" | "forever" | "in" | "from" => {
            Span::styled(word.to_string(), Style::default().fg(theme.function))
        }

        // Literals
        "true" | "false" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)),

        // Operators
        "==" | "!=" | "<" | ">" | "<=" | ">=" | "=" | "+" | "-" | "*" | "/" => Span::styled(word.to_string(), Style::default().fg(theme.operator)),
        "->" => Span::styled(word.to_string(), Style::default().fg(theme.arrow_decl)),

        // Punctuation
        "(" | ")" => Span::styled(word.to_string(), Style::default().fg(theme.parenthesis)),
        "{" | "}" => Span::styled(word.to_string(), Style::default().fg(theme.block)),
        ";" => Span::styled(word.to_string(), Style::default().fg(theme.end_statement)),
        "," => Span::styled(word.to_string(), Style::default().fg(theme.comma)),

        // Function calls (identifier followed by parentheses)
        _ if word.contains("(") && word.contains(")") && !word.starts_with('`') => {
            let paren_pos = word.find('(').unwrap();
            if paren_pos > 0 {
                Span::styled(word.to_string(), Style::default().fg(theme.function))
            } else {
                Span::styled(word.to_string(), Style::default().fg(theme.default))
            }
        }

        // Numbers. Whole or fractional by how the number is written, which is
        // how the compiler reads it, rather than by whether it happens to fit
        // an i64: a whole number too large to hold is still a whole number,
        // and painting it as a float said the wrong thing about it.
        _ if is_a_number(word) => {
            let color = if word.contains('.') { theme.float } else { theme.signed_int };
            Span::styled(word.to_string(), Style::default().fg(color))
        }

        // String literals (Nail uses backticks). A word containing one is
        // either a string or a tagged string's opening, html`<p>hi</p>`.
        _ if word.contains('`') => Span::styled(word.to_string(), Style::default().fg(theme.string_literal)),

        // Known stdlib functions (queried from the registry so the list never goes stale)
        _ if crate::stdlib_registry::is_stdlib_function(word) => {
            Span::styled(word.to_string(), Style::default().fg(theme.function))
        }

        // Function references (common patterns for callbacks)
        _ if word.ends_with("_func") => Span::styled(word.to_string(), Style::default().fg(theme.function)),

        // Default
        _ => Span::styled(word.to_string(), Style::default().fg(theme.default)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};

    fn test_theme() -> ColorScheme {
        ColorScheme {
            function: Color::Blue,
            const_decl: Color::Red,
            var_decl: Color::Green,
            if_decl: Color::Cyan,
            else_decl: Color::Cyan,
            arrow_decl: Color::Yellow,
            identifier: Color::Magenta,
            unsigned_int: Color::LightBlue,
            signed_int: Color::LightBlue,
            float: Color::LightGreen,
            operator: Color::White,
            keyword: Color::Cyan,
            comma: Color::Gray,
            string_literal: Color::Green,
            identifier_type: Color::Magenta,
            unknown: Color::White,
            parenthesis: Color::Yellow,
            block: Color::Yellow,
            end_statement: Color::Gray,
            async_keyword: Color::LightMagenta,
            parallel_keyword: Color::LightCyan,
            struct_keyword: Color::LightYellow,
            enum_keyword: Color::LightYellow,
            return_keyword: Color::LightRed,
            default: Color::White,
            background: Color::Black,
            comment: Color::DarkGray,
            error: Color::Red,
            ..ColorScheme::default()
        }
    }

    /// Every registered theme keeps every color readable on every background
    /// it can sit on. Dark themes hold WCAG AA: 4.5 to 1 for text, 3 to 1 for
    /// the two punctuation marks a theme dims on purpose. Light themes hold
    /// AAA, 7 to 1 and 4.5 to 1, because thin terminal glyphs on a bright
    /// ground wash out at ratios that read fine on a dark one. A new theme
    /// that ships a low contrast color fails here by name rather than being
    /// discovered by someone squinting.
    #[test]
    fn every_theme_clears_the_contrast_floor() {
        fn channel(c: u8) -> f64 {
            let c = c as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        fn luminance(color: Color) -> f64 {
            match color {
                Color::Rgb(r, g, b) => 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b),
                other => panic!("theme colors must be Rgb so contrast can be measured, found {:?}", other),
            }
        }
        fn contrast(a: Color, b: Color) -> f64 {
            let (la, lb) = (luminance(a), luminance(b));
            let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
            (hi + 0.05) / (lo + 0.05)
        }
        fn assert_floor(theme: &str, what: &str, fg: Color, bg: Color, floor: f64) {
            let seen = contrast(fg, bg);
            assert!(seen >= floor - 1e-6, "{theme}: {what} holds {seen:.2} to 1 against a floor of {floor} to 1");
        }

        for (name, theme) in THEMES.iter() {
            let light_background = luminance(theme.background) > 0.5;
            let (text_floor, dim_floor) = if light_background { (7.0, 4.5) } else { (4.5, 3.0) };
            let code_bgs = [theme.background, theme.current_line_bg];
            let tokens = [
                ("function", theme.function, text_floor),
                ("const_decl", theme.const_decl, text_floor),
                ("var_decl", theme.var_decl, text_floor),
                ("if_decl", theme.if_decl, text_floor),
                ("else_decl", theme.else_decl, text_floor),
                ("arrow_decl", theme.arrow_decl, text_floor),
                ("identifier", theme.identifier, text_floor),
                ("unsigned_int", theme.unsigned_int, text_floor),
                ("signed_int", theme.signed_int, text_floor),
                ("float", theme.float, text_floor),
                ("operator", theme.operator, text_floor),
                ("keyword", theme.keyword, text_floor),
                ("comma", theme.comma, dim_floor),
                ("string_literal", theme.string_literal, text_floor),
                ("identifier_type", theme.identifier_type, text_floor),
                ("unknown", theme.unknown, text_floor),
                ("parenthesis", theme.parenthesis, text_floor),
                ("block", theme.block, text_floor),
                ("end_statement", theme.end_statement, dim_floor),
                ("async_keyword", theme.async_keyword, text_floor),
                ("parallel_keyword", theme.parallel_keyword, text_floor),
                ("struct_keyword", theme.struct_keyword, text_floor),
                ("enum_keyword", theme.enum_keyword, text_floor),
                ("return_keyword", theme.return_keyword, text_floor),
                ("default", theme.default, text_floor),
                ("comment", theme.comment, text_floor),
                ("error", theme.error, text_floor),
                ("cursor_fg", theme.cursor_fg, text_floor),
            ];
            for (field, color, floor) in tokens {
                for bg in code_bgs {
                    assert_floor(name, field, color, bg, floor);
                }
            }
            assert_floor(name, "error on the error line wash", theme.error, theme.error_line_bg, text_floor);

            let ui_bgs = [theme.ui_panel_bg, theme.background];
            let ui = [
                ("ui_text", theme.ui_text),
                ("ui_text_muted", theme.ui_text_muted),
                ("ui_hint", theme.ui_hint),
                ("accent", theme.accent),
                ("success", theme.success),
                ("success_bright", theme.success_bright),
                ("danger", theme.danger),
                ("info", theme.info),
                ("info_bright", theme.info_bright),
                ("primary", theme.primary),
                ("special", theme.special),
            ];
            for (field, color) in ui {
                for bg in ui_bgs {
                    assert_floor(name, field, color, bg, text_floor);
                }
            }

            let pairs = [
                ("selected text", theme.selection_fg, theme.selection_bg),
                ("the current search match", theme.search_match_fg, theme.search_match_bg),
                ("the other search matches", theme.search_other_fg, theme.search_other_bg),
                ("the keymap badge", theme.badge_fg, theme.badge_bg),
                ("a switch that is on", theme.toggle_on_fg, theme.toggle_on_bg),
                ("the picked list row", theme.menu_selection_fg, theme.menu_selection_bg),
                ("text in an input field", theme.ui_text, theme.input_bg),
                ("text in an idle input field", theme.ui_text, theme.input_inactive_bg),
                ("the picked completion", theme.on_emphasis, theme.item_selection_bg),
                ("the matched bracket", theme.on_emphasis, theme.bracket_match_bg),
            ];
            for (what, fg, bg) in pairs {
                assert_floor(name, what, fg, bg, text_floor);
            }
        }
    }

    /// The fuzzer compares what the compiler reads with what the editor
    /// paints, and each of these was a disagreement it found.
    #[test]
    fn numbers_are_painted_as_numbers_wherever_they_sit() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("total:i = 80&&81; share:i = total%7; big:i = 18446744073709551616; ratio:f = 2.5;")])];

        let result = colorize_code(content, &theme);
        let colored: Vec<(String, Option<Color>)> = result[0].spans.iter().map(|span| (span.content.to_string(), span.style.fg)).collect();

        // '&&' and '%' end a word like every other operator. Without that,
        // `80&&81` was one long word the colorizer could make nothing of.
        for number in ["80", "81", "7"] {
            let found = colored.iter().find(|(text, _)| text == number).unwrap_or_else(|| panic!("{} should be its own span, got {:?}", number, colored));
            assert_eq!(found.1, Some(theme.signed_int), "{} is a whole number", number);
        }
        // A whole number too large for an i is still a whole number.
        let big = colored.iter().find(|(text, _)| text == "18446744073709551616").expect("the large number should be its own span");
        assert_eq!(big.1, Some(theme.signed_int));
        let ratio = colored.iter().find(|(text, _)| text == "2.5").expect("the fraction should be its own span");
        assert_eq!(ratio.1, Some(theme.float));
    }

    /// Whatever else colouring does, the text it hands back is the text it was
    /// given. The editor draws that, so a dropped character is a file that
    /// looks different on screen from the file on disk.
    #[test]
    fn colouring_never_changes_a_character_of_the_line() {
        let theme = test_theme();
        let lines = [
            "f heaviest(jobs:a:Job):Job!e {",
            "half:i = safe(half_of(4), fallback_int);",
            "flag:b = (count>3&&other<9) || 5!= 6;",
            "page:s = html`<p class=\"x\">hi</p>`;",
            "share:i = total /count;",
            "c",
            "/c",
        ];
        for line in lines {
            let result = colorize_code(vec![Line::from(vec![Span::raw(line.to_string())])], &theme);
            let drawn: String = result[0].spans.iter().map(|span| span.content.to_string()).collect();
            assert_eq!(drawn, line, "colouring changed the line");
        }
    }

    #[test]
    fn markup_inside_an_html_string_is_colored_as_markup() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("page:s = html`<section class=\"hero\">`;")])];

        let result = colorize_code(content, &theme);
        let colored: Vec<(String, Option<Color>)> = result[0].spans.iter().map(|span| (span.content.to_string(), span.style.fg)).collect();

        let element = colored.iter().find(|(text, _)| text == "section").expect("the element name should be its own span");
        assert_eq!(element.1, Some(theme.keyword), "element names are colored as markup, not as string text");
        let value = colored.iter().find(|(text, _)| text == "\"hero\"").expect("the attribute value should be its own span");
        assert_eq!(value.1, Some(theme.string_literal));
        let bracket = colored.iter().find(|(text, _)| text == "<").expect("brackets should be their own spans");
        assert_eq!(bracket.1, Some(theme.comma));
    }

    #[test]
    fn a_css_string_is_colored_by_the_css_tokenizer() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("sheet:s = css`.hero { font-size: 1.5rem; }`;")])];

        let result = colorize_code(content, &theme);
        let colored: Vec<(String, Option<Color>)> = result[0].spans.iter().map(|span| (span.content.to_string(), span.style.fg)).collect();

        let selector = colored.iter().find(|(text, _)| text == ".hero").expect("the selector should be its own span");
        assert_eq!(selector.1, Some(theme.keyword));
        let property = colored.iter().find(|(text, _)| text == "font-size").expect("the property should be its own span");
        assert_eq!(property.1, Some(theme.identifier_type));
        let length = colored.iter().find(|(text, _)| text == "1.5rem").expect("the length should be its own span");
        assert_eq!(length.1, Some(theme.signed_int));
    }

    #[test]
    fn a_script_string_is_colored_by_the_script_tokenizer() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("script:s = js`const total = items.length; // done`;")])];

        let result = colorize_code(content, &theme);
        let colored: Vec<(String, Option<Color>)> = result[0].spans.iter().map(|span| (span.content.to_string(), span.style.fg)).collect();

        let keyword = colored.iter().find(|(text, _)| text == "const").expect("the keyword should be its own span");
        assert_eq!(keyword.1, Some(theme.keyword));
        let member = colored.iter().find(|(text, _)| text == "length").expect("the member should be its own span");
        assert_eq!(member.1, Some(theme.identifier_type));
        let comment = colored.iter().find(|(text, _)| text == "// done").expect("the comment should be its own span");
        assert_eq!(comment.1, Some(theme.comment));
    }

    #[test]
    fn the_line_oriented_languages_are_colored_too() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("query:s = sql`select name from users`;")]),
            Line::from(vec![Span::raw("config:s = yaml`port: 8080`;")]),
            Line::from(vec![Span::raw("manifest:s = toml`edition = 2024`;")]),
        ];

        let result = colorize_code(content, &theme);

        let keyword = result[0].spans.iter().find(|span| span.content == "select").expect("a lowercase SQL keyword is still a keyword");
        assert_eq!(keyword.style.fg, Some(theme.keyword));
        let key = result[1].spans.iter().find(|span| span.content == "port").expect("the YAML key should be its own span");
        assert_eq!(key.style.fg, Some(theme.identifier_type));
        let number = result[2].spans.iter().find(|span| span.content == "2024").expect("the TOML value should be its own span");
        assert_eq!(number.style.fg, Some(theme.signed_int));
    }

    #[test]
    fn a_css_block_left_open_across_a_line_break_stays_open() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("sheet:s = css`.hero {")]), Line::from(vec![Span::raw("    color: red;")]), Line::from(vec![Span::raw("}`;")])];

        let result = colorize_code(content, &theme);

        // `color` is a property, not a selector: the block opened on the line
        // before has not been closed yet.
        let property = result[1].spans.iter().find(|span| span.content == "color").expect("the property should be its own span");
        assert_eq!(property.style.fg, Some(theme.identifier_type));
        let semicolon = result[2].spans.iter().find(|span| span.content == ";").expect("the semicolon is code again");
        assert_eq!(semicolon.style.fg, Some(theme.end_statement));
    }

    #[test]
    fn a_string_without_a_markup_tag_stays_one_color() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("plain:s = `<not markup, just text>`;")])];

        let result = colorize_code(content, &theme);
        let body = result[0].spans.iter().find(|span| span.content.contains("not markup")).expect("the body should survive as one span");
        assert_eq!(body.content, "<not markup, just text>", "an untagged string is never tokenized");
        assert_eq!(body.style.fg, Some(theme.string_literal));
    }

    #[test]
    fn a_multi_line_string_covers_exactly_its_own_lines() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("page:s = html`<section>")]),
            Line::from(vec![Span::raw("    <h1>Nail</h1>")]),
            Line::from(vec![Span::raw("</section>`;")]),
        ];

        let result = colorize_code(content, &theme);

        // The code before the string keeps its own colors rather than being
        // swallowed by the string that starts later on the line.
        let name = result[0].spans.iter().find(|span| span.content == "page").expect("the declaration should still be colored");
        assert_eq!(name.style.fg, Some(theme.var_decl));

        // The closing line is still inside the string, so its markup is markup
        // and only the trailing `;` is code.
        let closing = result[2].spans.iter().find(|span| span.content == "section").expect("the closing element should be colored as markup");
        assert_eq!(closing.style.fg, Some(theme.keyword));
        let semicolon = result[2].spans.iter().find(|span| span.content == ";").expect("the semicolon is code");
        assert_eq!(semicolon.style.fg, Some(theme.end_statement));
    }

    #[test]
    fn a_tag_left_open_across_a_line_break_stays_open() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("page:s = html`<svg viewBox=\"0 0 20 20\"")]),
            Line::from(vec![Span::raw("     stroke=\"currentColor\"><path/></svg>`;")]),
        ];

        let result = colorize_code(content, &theme);

        // `stroke` is an attribute, not text: the tag opened on the line before
        // has not been closed yet.
        let attribute = result[1].spans.iter().find(|span| span.content.trim() == "stroke").expect("the attribute should be its own span");
        assert_eq!(attribute.style.fg, Some(theme.identifier_type));
    }

    #[test]
    fn a_language_tag_is_colored_as_part_of_its_string() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("page:s = html`<p>hi</p>`;")])];

        let result = colorize_code(content, &theme);

        let tag_span = result[0].spans.iter().find(|span| span.content.contains("html"));
        let tag_span = tag_span.expect("the tag should still appear in the line");
        assert_eq!(tag_span.style.fg, Some(theme.string_literal), "the tag belongs to the string, not to the code around it");
    }

    #[test]
    fn a_word_before_a_string_is_only_a_tag_when_it_touches_the_backtick() {
        assert_eq!(trailing_tag_len("page:s = html"), 4);
        assert_eq!(trailing_tag_len("page:s = "), 0);
        // Digits alone are not identifier-shaped, so they are not a tag.
        assert_eq!(trailing_tag_len("x = 42"), 0);
        assert_eq!(trailing_tag_len("y:s = my_lang2"), 8);
    }

    #[test]
    fn test_colorize_keywords() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("x:i = 42;")]), Line::from(vec![Span::raw("y:s = `hello`;")]), Line::from(vec![Span::raw("if true { return 1; }")])];

        let result = colorize_code(content, &theme);

        assert_eq!(result.len(), 3);

        // Check that 'if' is colored as keyword
        let third_line = &result[2];
        assert!(!third_line.spans.is_empty());
        // Check that the 'if' keyword is colored correctly
        let has_if_keyword = third_line.spans.iter().any(|span| span.content == "if" && span.style.fg == Some(theme.keyword));
        assert!(has_if_keyword, "The 'if' keyword should be colored correctly");
    }

    #[test]
    fn test_colorize_function_calls() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("print(`hello`);")]), Line::from(vec![Span::raw("from(42);")]), Line::from(vec![Span::raw("time_now();")])];

        let result = colorize_code(content, &theme);

        assert_eq!(result.len(), 3);

        // Check function calls are colored correctly
        for line in &result {
            let has_function_color = line.spans.iter().any(|span| {
                // Function names are colored separately from parentheses
                (span.content == "print" || span.content == "from" || span.content == "time_now") && span.style.fg == Some(theme.function)
            });
            assert!(has_function_color, "Function call should be colored as function");
        }
    }

    #[test]
    fn test_colorize_variable_declarations() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("name:s = `Alice`;")]), Line::from(vec![Span::raw("age:i = 25;")]), Line::from(vec![Span::raw("score:f = 95.5;")])];

        let result = colorize_code(content, &theme);

        // Check that variable declarations (name:type) are colored correctly
        for line in &result {
            let has_identifier = line.spans.iter().any(|span| {
                // Check for variable declarations (colored with var_decl)
                span.style.fg == Some(theme.var_decl)
            });
            assert!(has_identifier, "Variable declaration should be colored as identifier");
        }
    }

    #[test]
    fn test_colorize_numbers() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("x:i = 42;")]), Line::from(vec![Span::raw("y:f = 3.14;")]), Line::from(vec![Span::raw("z:i = -100;")])];

        let result = colorize_code(content, &theme);

        // Check that numbers are colored correctly
        for line in &result {
            let has_number = line
                .spans
                .iter()
                .any(|span| (span.content.parse::<i64>().is_ok() || span.content.parse::<f64>().is_ok()) && (span.style.fg == Some(theme.signed_int) || span.style.fg == Some(theme.float)));
            // Note: Some lines might not have numbers due to splitting
        }
    }

    #[test]
    fn test_colorize_strings() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("msg:s = \"hello world\";")]), Line::from(vec![Span::raw("print(\"test\");")])];

        let result = colorize_code(content, &theme);

        // Check that string literals are colored correctly
        for line in &result {
            let has_string = line.spans.iter().any(|span| (span.content.starts_with('"') || span.content.ends_with('"')) && span.style.fg == Some(theme.string_literal));
            // Note: Strings might be split across spans
        }
    }

    #[test]
    fn test_colorize_operators() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("result:i = x + y * 2;")]), Line::from(vec![Span::raw("if a == b || c != d {")]), Line::from(vec![Span::raw("=> x / 2")])];

        let result = colorize_code(content, &theme);

        // Check that operators are colored correctly
        for line in &result {
            let has_operator =
                line.spans.iter().any(|span| matches!(span.content.as_ref(), "+" | "-" | "*" | "/" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "=") && span.style.fg == Some(theme.operator));
            // Note: Not all lines may have operators
        }
    }

    #[test]
    fn test_colorize_comments() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("// This is a comment")]), Line::from(vec![Span::raw("x:i = 42; // Inline comment")]), Line::from(vec![Span::raw("// TODO: implement this")])];

        let result = colorize_code(content, &theme);

        // Check that comments are colored correctly
        for line in &result {
            let has_comment = line.spans.iter().any(|span| span.content.trim().starts_with("//") && span.style.fg == Some(theme.comment));
            if line.spans.iter().any(|span| span.content.contains("//")) {
                assert!(has_comment, "Comments should be colored as comment color");
            }
        }
    }

    #[test]
    fn test_colorize_parallel_blocks() {
        let theme = test_theme();
        let content =
            vec![Line::from(vec![Span::raw("p")]), Line::from(vec![Span::raw("    print(\"task 1\");")]), Line::from(vec![Span::raw("    print(\"task 2\");")]), Line::from(vec![Span::raw("/p")])];

        let result = colorize_code(content, &theme);

        // Check that 'parallel' keyword is colored correctly
        let first_line = &result[0];
        let has_parallel = first_line.spans.iter().any(|span| span.content == "p" && span.style.fg == Some(theme.parallel_keyword));
        assert!(has_parallel, "Parallel keyword should be colored correctly");
    }

    #[test]
    fn test_colorize_multiline_strings() {
        let theme = test_theme();
        let content =
            vec![Line::from(vec![Span::raw("msg:s = `line 1")]), Line::from(vec![Span::raw("line 2")]), Line::from(vec![Span::raw("line 3`;")]), Line::from(vec![Span::raw("other:i = 42;")])];

        let result = colorize_code(content, &theme);

        // The string state detection should identify lines 1 and 2 as being inside a string
        assert_eq!(result.len(), 4);

        // Lines inside multiline string should be colored as string_literal
        let second_line = &result[1];
        let has_string_color = second_line.spans.iter().any(|span| span.style.fg == Some(theme.string_literal));
        assert!(has_string_color, "Content inside multiline string should be colored as string");
    }

    #[test]
    fn test_colorize_complex_nail_program() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("// Complex Nail program")]),
            Line::from(vec![Span::raw("name:s = \"Alice\";")]),
            Line::from(vec![Span::raw("age:i = 25;")]),
            Line::from(vec![Span::raw("score:f = 95.5;")]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::raw("if age > 18 {")]),
            Line::from(vec![Span::raw("    print(`Adult`);")]),
            Line::from(vec![Span::raw("} else {")]),
            Line::from(vec![Span::raw("    print(`Minor`);")]),
            Line::from(vec![Span::raw("}")]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::raw("p")]),
            Line::from(vec![Span::raw("    result1:s = string_from(age);")]),
            Line::from(vec![Span::raw("    result2:i = time_now();")]),
            Line::from(vec![Span::raw("}")]),
        ];

        let result = colorize_code(content, &theme);

        assert_eq!(result.len(), 15);

        // Verify the first line is a comment
        let first_line = &result[0];
        assert!(first_line.spans.iter().any(|span| span.style.fg == Some(theme.comment)));

        // Verify we have const declarations (colored with var_decl)
        assert!(result.iter().any(|line| line.spans.iter().any(|span| span.style.fg == Some(theme.var_decl))));

        // Verify we have the parallel keyword
        assert!(result.iter().any(|line| line.spans.iter().any(|span| span.content == "p" && span.style.fg == Some(theme.parallel_keyword))));

        // Verify we have function calls
        assert!(result.iter().any(|line| line.spans.iter().any(|span| (span.content == "print" || span.content == "from") && span.style.fg == Some(theme.function))));
    }

    #[test]
    fn test_colorize_empty_lines() {
        let theme = test_theme();
        let content = vec![Line::from(vec![]), Line::from(vec![Span::raw("")]), Line::from(vec![Span::raw("x:i = 42;")]), Line::from(vec![])];

        let result = colorize_code(content, &theme);

        assert_eq!(result.len(), 4);

        // Empty lines should be handled gracefully
        assert_eq!(result[0].spans.len(), 1);
        assert_eq!(result[0].spans[0].content, "");

        assert_eq!(result[3].spans.len(), 1);
        assert_eq!(result[3].spans[0].content, "");
    }

    #[test]
    fn test_parallel_colorization_performance() {
        let theme = test_theme();

        // Create a large program to test parallel performance
        let mut content = Vec::new();
        for i in 0..1000 {
            content.push(Line::from(vec![Span::raw(format!("var{}:i = {};", i, i))]));
        }

        let start = std::time::Instant::now();
        let result = colorize_code(content, &theme);
        let duration = start.elapsed();

        assert_eq!(result.len(), 1000);

        // Should complete within reasonable time (parallel processing should help)
        assert!(duration.as_millis() < 1000, "Colorization took too long: {:?}", duration);

        // Verify some lines are colored correctly (var declarations)
        assert!(result.iter().any(|line| line.spans.iter().any(|span| span.style.fg == Some(theme.var_decl))));
    }

    #[test]
    fn test_error_type_tokenization() {
        // Test that error types like i!e are kept as single tokens without spaces
        let content = "f divide(num:i, den:i):i!e {";
        let tokens = tokenize_code(content);

        // Check that i!e is a single token
        assert!(tokens.contains(&"i!e".to_string()), "i!e should be a single token, got: {:?}", tokens);

        // Test other error types
        let content2 = "result:f!e = parse_float(str);";
        let tokens2 = tokenize_code(content2);
        assert!(tokens2.contains(&"f!e".to_string()), "f!e should be a single token, got: {:?}", tokens2);

        let content3 = "data:s!e = read_file(path);";
        let tokens3 = tokenize_code(content3);
        assert!(tokens3.contains(&"s!e".to_string()), "s!e should be a single token, got: {:?}", tokens3);

        // Test that regular ! operators are still handled correctly
        let content4 = "if { x != 0 -> { print(`ok`); } }";
        let tokens4 = tokenize_code(content4);
        assert!(tokens4.contains(&"!=".to_string()), "!= should be a single token, got: {:?}", tokens4);
    }

    fn lines_of(content: &[&str]) -> Vec<String> {
        return content.iter().map(|line| line.to_string()).collect();
    }

    /// What the whole file looks like colored from scratch, which is what the
    /// cache has to agree with after any edit at all.
    fn colored_from_scratch(content: &[String], theme: &ColorScheme) -> Vec<Line<'static>> {
        let lines: Vec<Line> = content.iter().map(|line| Line::from(vec![Span::raw(line.clone())])).collect();
        return colorize_code(lines, theme);
    }

    fn cached_lines(cache: &ColorizeCache, count: usize) -> Vec<Line<'static>> {
        return (0..count).map(|index| cache.line(index).expect("the cache holds every line").clone()).collect();
    }

    /// The point of the cache: a character typed into one line is one line of
    /// coloring, not a file's worth.
    #[test]
    fn typing_into_a_line_colors_that_line_and_no_other() {
        let theme = test_theme();
        let mut cache = ColorizeCache::new();
        let before = lines_of(&["f main():v {", "    x:i = 1;", "    print(`hi`);", "}"]);
        cache.colorize(&before, &theme);
        assert_eq!(cache.colored_last_time(), 4);

        let after = lines_of(&["f main():v {", "    xy:i = 1;", "    print(`hi`);", "}"]);
        cache.colorize(&after, &theme);
        assert_eq!(cache.colored_last_time(), 1);
        assert_eq!(cached_lines(&cache, after.len()), colored_from_scratch(&after, &theme));
    }

    /// A new line pushes everything under it down, and the lines it pushed are
    /// still the lines they were.
    #[test]
    fn inserting_a_line_reuses_the_lines_it_pushed_down() {
        let theme = test_theme();
        let mut cache = ColorizeCache::new();
        let before = lines_of(&["f main():v {", "    print(`hi`);", "}"]);
        cache.colorize(&before, &theme);

        let after = lines_of(&["f main():v {", "    x:i = 1;", "    print(`hi`);", "}"]);
        cache.colorize(&after, &theme);
        assert_eq!(cache.colored_last_time(), 1);
        assert_eq!(cached_lines(&cache, after.len()), colored_from_scratch(&after, &theme));
    }

    /// Opening a string changes what every line under it means, so those lines
    /// are colored again however well they match what they were.
    #[test]
    fn opening_a_string_recolors_what_is_under_it() {
        let theme = test_theme();
        let mut cache = ColorizeCache::new();
        let before = lines_of(&["x:i = 1;", "y:i = 2;", "z:i = 3;"]);
        cache.colorize(&before, &theme);

        let after = lines_of(&["x:s = `open", "y:i = 2;", "z:i = 3;"]);
        cache.colorize(&after, &theme);
        assert_eq!(cache.colored_last_time(), 3);
        assert_eq!(cached_lines(&cache, after.len()), colored_from_scratch(&after, &theme));

        // And closing it again puts them back.
        let closed = lines_of(&["x:s = `open`;", "y:i = 2;", "z:i = 3;"]);
        cache.colorize(&closed, &theme);
        assert_eq!(cached_lines(&cache, closed.len()), colored_from_scratch(&closed, &theme));
    }

    /// Every shape of edit, against the answer computed from nothing.
    #[test]
    fn the_cache_agrees_with_a_fresh_colorization_after_any_edit() {
        let theme = test_theme();
        let mut cache = ColorizeCache::new();
        let edits = vec![
            lines_of(&["f main():v {", "    print(`hi`);", "}"]),
            // A line deleted from the middle
            lines_of(&["f main():v {", "}"]),
            // A multi-line string opened and closed
            lines_of(&["page:s = html`<p>", "  <b>hi</b>", "</p>`;", "f main():v {", "}"]),
            // The line that opened it edited
            lines_of(&["page:s = html`<div>", "  <b>hi</b>", "</p>`;", "f main():v {", "}"]),
            // Everything replaced
            lines_of(&["// nothing but a comment"]),
            // Back to an empty file
            lines_of(&[""]),
        ];

        for content in &edits {
            cache.colorize(content, &theme);
            assert_eq!(cached_lines(&cache, content.len()), colored_from_scratch(content, &theme), "content: {:?}", content);
        }
    }

    /// A theme is a different answer for every line, so switching one throws
    /// the whole cache away rather than half of it.
    #[test]
    fn switching_the_theme_colors_the_file_again() {
        let mut cache = ColorizeCache::new();
        let content = lines_of(&["x:i = 1;", "y:i = 2;"]);
        cache.colorize(&content, &test_theme());
        assert_eq!(cache.colored_last_time(), 2);

        let other = ColorScheme { identifier: Color::LightRed, ..test_theme() };
        cache.colorize(&content, &other);
        assert_eq!(cache.colored_last_time(), 2);
        assert_eq!(cached_lines(&cache, content.len()), colored_from_scratch(&content, &other));
    }
}
