use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub highlight_fg: Color,
    pub highlight_bg: Color,
    pub accent: Color,
    pub selected: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}

// ---------------------------------------------------------------------------
// Named-color helpers (kept for serialization / config file round-trips)
// ---------------------------------------------------------------------------

/// Backward-compatible alias for [`NAMED_COLORS`].
pub const COLORS: &[Color] = NAMED_COLORS;

pub const NAMED_COLORS: &[Color] = &[
    Color::Reset,
    Color::Black,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Gray,
    Color::DarkGray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::White,
];

pub fn color_name(c: Color) -> &'static str {
    match c {
        Color::Reset => "Default/Reset",
        Color::Black => "Black",
        Color::Red => "Red",
        Color::Green => "Green",
        Color::Yellow => "Yellow",
        Color::Blue => "Blue",
        Color::Magenta => "Magenta",
        Color::Cyan => "Cyan",
        Color::Gray => "Gray",
        Color::DarkGray => "Dark Gray",
        Color::LightRed => "Light Red",
        Color::LightGreen => "Light Green",
        Color::LightYellow => "Light Yellow",
        Color::LightBlue => "Light Blue",
        Color::LightMagenta => "Light Magenta",
        Color::LightCyan => "Light Cyan",
        Color::White => "White",
        Color::Rgb(r, g, b) => {
            // Leaks a tiny string — acceptable for a debug/display helper.
            Box::leak(format!("#{r:02X}{g:02X}{b:02X}").into_boxed_str())
        }
        _ => "Unknown",
    }
}

/// Serialize a `Color` to the string form used in config files.
/// `Rgb` values are written as `#RRGGBB`.
pub fn color_to_string(c: Color) -> String {
    match c {
        Color::Reset => "reset".into(),
        Color::Black => "black".into(),
        Color::Red => "red".into(),
        Color::Green => "green".into(),
        Color::Yellow => "yellow".into(),
        Color::Blue => "blue".into(),
        Color::Magenta => "magenta".into(),
        Color::Cyan => "cyan".into(),
        Color::Gray => "gray".into(),
        Color::DarkGray => "darkgray".into(),
        Color::LightRed => "lightred".into(),
        Color::LightGreen => "lightgreen".into(),
        Color::LightYellow => "lightyellow".into(),
        Color::LightBlue => "lightblue".into(),
        Color::LightMagenta => "lightmagenta".into(),
        Color::LightCyan => "lightcyan".into(),
        Color::White => "white".into(),
        Color::Rgb(r, g, b) => format!("#{r:02X}{g:02X}{b:02X}"),
        _ => "reset".into(),
    }
}

/// Parse a color from a config-file string.
/// Accepts named colors **and** `#RRGGBB` / `RRGGBB` hex strings.
pub fn string_to_color(s: &str) -> Color {
    let s = s.trim();

    // Try hex first: #RRGGBB or RRGGBB
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }

    match s.to_lowercase().as_str() {
        "reset" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "darkgray" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Reset,
    }
}

// ---------------------------------------------------------------------------
// Convenience RGB constructor (keeps theme definitions tidy)
// ---------------------------------------------------------------------------

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

// ---------------------------------------------------------------------------
// Themes
// ---------------------------------------------------------------------------

/// Terminal default colors — no RGB assumptions.
pub const THEME_DEFAULT: Theme = Theme {
    name: "default",
    background: Color::Reset,
    foreground: Color::White,
    border: Color::DarkGray,
    highlight_fg: Color::Black,
    highlight_bg: Color::Cyan,
    accent: Color::Cyan,
    selected: Color::Yellow,
    success: Color::Green,
    warning: Color::Yellow,
    error: Color::Red,
};

/// Dracula — purple-tinted dark theme.
/// Palette: https://draculatheme.com/contribute
pub const THEME_DRACULA: Theme = Theme {
    name: "dracula",
    background: rgb(40, 42, 54),
    foreground: rgb(248, 248, 242),
    border: rgb(189, 147, 249),
    highlight_fg: rgb(40, 42, 54),
    highlight_bg: rgb(189, 147, 249),
    accent: rgb(255, 121, 198),
    selected: rgb(241, 250, 140),
    success: rgb(80, 250, 123),
    warning: rgb(255, 184, 108),
    error: rgb(255, 85, 85),
};

/// Nord — arctic, blue-toned dark theme.
/// Palette: https://www.nordtheme.com
pub const THEME_NORD: Theme = Theme {
    name: "nord",
    background: rgb(46, 52, 64),
    foreground: rgb(236, 239, 244),
    border: rgb(136, 192, 208),
    highlight_fg: rgb(46, 52, 64),
    highlight_bg: rgb(136, 192, 208),
    accent: rgb(129, 161, 193),
    selected: rgb(235, 203, 139),
    success: rgb(163, 190, 140),
    warning: rgb(235, 203, 139),
    error: rgb(191, 97, 106),
};

/// Gruvbox Dark — warm retro look.
/// Palette: https://github.com/morhetz/gruvbox
pub const THEME_GRUVBOX: Theme = Theme {
    name: "gruvbox",
    background: rgb(40, 40, 40),
    foreground: rgb(235, 219, 178),
    border: rgb(168, 153, 132),
    highlight_fg: rgb(40, 40, 40),
    highlight_bg: rgb(215, 153, 33),
    accent: rgb(250, 189, 47),
    selected: rgb(184, 187, 38),
    success: rgb(184, 187, 38),
    warning: rgb(250, 189, 47),
    error: rgb(251, 73, 52),
};

/// Solarized Dark.
/// Palette: https://ethanschoonover.com/solarized
pub const THEME_SOLARIZED: Theme = Theme {
    name: "solarized",
    background: rgb(0, 43, 54),
    foreground: rgb(131, 148, 150),
    border: rgb(42, 161, 152),
    highlight_fg: rgb(0, 43, 54),
    highlight_bg: rgb(42, 161, 152),
    accent: rgb(38, 139, 210),
    selected: rgb(181, 137, 0),
    success: rgb(133, 153, 0),
    warning: rgb(181, 137, 0),
    error: rgb(220, 50, 47),
};

/// Monokai — vibrant dark theme popularised by Sublime Text.
/// Palette: https://monokai.pro
pub const THEME_MONOKAI: Theme = Theme {
    name: "monokai",
    background: rgb(39, 40, 34),
    foreground: rgb(248, 248, 242),
    border: rgb(174, 129, 255),
    highlight_fg: rgb(39, 40, 34),
    highlight_bg: rgb(166, 226, 46),
    accent: rgb(174, 129, 255),
    selected: rgb(230, 219, 116),
    success: rgb(166, 226, 46),
    warning: rgb(253, 151, 31),
    error: rgb(249, 38, 114),
};

/// Matrix — classic green-on-black terminal aesthetic.
pub const THEME_MATRIX: Theme = Theme {
    name: "matrix",
    background: rgb(0, 0, 0),
    foreground: rgb(0, 255, 65),
    border: rgb(0, 187, 48),
    highlight_fg: rgb(0, 0, 0),
    highlight_bg: rgb(0, 255, 65),
    accent: rgb(0, 187, 48),
    selected: rgb(255, 255, 255),
    success: rgb(0, 255, 65),
    warning: rgb(255, 204, 0),
    error: rgb(255, 50, 50),
};

/// Catppuccin Mocha — pastel dark theme.
/// Palette: https://github.com/catppuccin/catppuccin
pub const THEME_CATPPUCCIN: Theme = Theme {
    name: "catppuccin",
    background: rgb(30, 30, 46),
    foreground: rgb(205, 214, 244),
    border: rgb(137, 180, 250),
    highlight_fg: rgb(30, 30, 46),
    highlight_bg: rgb(137, 180, 250),
    accent: rgb(203, 166, 247),
    selected: rgb(249, 226, 175),
    success: rgb(166, 227, 161),
    warning: rgb(249, 226, 175),
    error: rgb(243, 139, 168),
};

/// Tokyo Night — deep blue-purple dark theme.
/// Palette: https://github.com/enkia/tokyo-night-vscode-theme
pub const THEME_TOKYO_NIGHT: Theme = Theme {
    name: "tokyo-night",
    background: rgb(26, 27, 38),
    foreground: rgb(169, 177, 214),
    border: rgb(122, 162, 247),
    highlight_fg: rgb(26, 27, 38),
    highlight_bg: rgb(122, 162, 247),
    accent: rgb(187, 154, 247),
    selected: rgb(224, 175, 104),
    success: rgb(158, 206, 106),
    warning: rgb(224, 175, 104),
    error: rgb(247, 118, 142),
};

/// Rosé Pine — dark theme with muted, earthy tones.
/// Palette: https://rosepinetheme.com
pub const THEME_ROSE_PINE: Theme = Theme {
    name: "rose-pine",
    background: rgb(25, 23, 36),
    foreground: rgb(224, 222, 244),
    border: rgb(196, 167, 231),
    highlight_fg: rgb(25, 23, 36),
    highlight_bg: rgb(196, 167, 231),
    accent: rgb(235, 188, 186),
    selected: rgb(246, 193, 119),
    success: rgb(156, 207, 216),
    warning: rgb(246, 193, 119),
    error: rgb(235, 111, 146),
};

/// Everforest Dark — cozy green-tinted theme.
/// Palette: https://github.com/sainnhe/everforest
pub const THEME_EVERFOREST: Theme = Theme {
    name: "everforest",
    background: rgb(45, 53, 59),
    foreground: rgb(211, 198, 170),
    border: rgb(131, 165, 152),
    highlight_fg: rgb(45, 53, 59),
    highlight_bg: rgb(131, 165, 152),
    accent: rgb(127, 187, 179),
    selected: rgb(219, 188, 127),
    success: rgb(167, 192, 128),
    warning: rgb(219, 188, 127),
    error: rgb(230, 126, 128),
};

/// One Dark — Atom/VS Code staple.
/// Palette: https://github.com/atom/one-dark-ui
pub const THEME_ONE_DARK: Theme = Theme {
    name: "one-dark",
    background: rgb(40, 44, 52),
    foreground: rgb(171, 178, 191),
    border: rgb(97, 175, 239),
    highlight_fg: rgb(40, 44, 52),
    highlight_bg: rgb(97, 175, 239),
    accent: rgb(198, 120, 221),
    selected: rgb(229, 192, 123),
    success: rgb(152, 195, 121),
    warning: rgb(229, 192, 123),
    error: rgb(224, 108, 117),
};

pub const THEMES: &[Theme] = &[
    THEME_DEFAULT,
    THEME_DRACULA,
    THEME_NORD,
    THEME_GRUVBOX,
    THEME_SOLARIZED,
    THEME_MONOKAI,
    THEME_MATRIX,
    THEME_CATPPUCCIN,
    THEME_TOKYO_NIGHT,
    THEME_ROSE_PINE,
    THEME_EVERFOREST,
    THEME_ONE_DARK,
];

// ---------------------------------------------------------------------------
// Theme resolution
// ---------------------------------------------------------------------------

/// Return a `Theme` by name, or build a fully custom one from individual
/// color strings (named or `#RRGGBB` hex).
pub fn resolve_theme(
    theme_name: &str,
    bg: &str,
    fg: &str,
    border: &str,
    highlight_fg: &str,
    highlight_bg: &str,
    accent: &str,
    selected: &str,
    success: &str,
    warning: &str,
    error: &str,
) -> Theme {
    if theme_name.eq_ignore_ascii_case("custom") {
        Theme {
            name: "custom",
            background: string_to_color(bg),
            foreground: string_to_color(fg),
            border: string_to_color(border),
            highlight_fg: string_to_color(highlight_fg),
            highlight_bg: string_to_color(highlight_bg),
            accent: string_to_color(accent),
            selected: string_to_color(selected),
            success: string_to_color(success),
            warning: string_to_color(warning),
            error: string_to_color(error),
        }
    } else {
        THEMES
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(theme_name))
            .copied()
            .unwrap_or(THEME_DEFAULT)
    }
}

/// Convenience: look up a theme by name only.
pub fn theme_by_name(name: &str) -> Theme {
    THEMES
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
        .copied()
        .unwrap_or(THEME_DEFAULT)
}