use ratatui::style::Color;

#[derive(Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub working_copy: Color,
    pub immutable: Color,
    pub immutable_icon: Color,
    pub conflict: Color,
    pub empty: Color,
    pub selected: Color,
    pub rebase_src: Color,
    pub change_id: Color,
    pub author: Color,
    pub desc: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub status_ok: Color,
    pub status_err: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub help_key: Color,
    pub help_section: Color,
}

impl Theme {
    pub fn override_from_config(&mut self, colors: &std::collections::HashMap<String, String>) {
        if let Some(c) = colors.get("bg").and_then(|s| parse_hex(s)) {
            self.bg = c;
        }
        if let Some(c) = colors.get("working_copy").and_then(|s| parse_hex(s)) {
            self.working_copy = c;
        }
        if let Some(c) = colors.get("immutable").and_then(|s| parse_hex(s)) {
            self.immutable = c;
        }
        if let Some(c) = colors.get("immutable_icon").and_then(|s| parse_hex(s)) {
            self.immutable_icon = c;
        }
        if let Some(c) = colors.get("conflict").and_then(|s| parse_hex(s)) {
            self.conflict = c;
        }
        if let Some(c) = colors.get("empty").and_then(|s| parse_hex(s)) {
            self.empty = c;
        }
        if let Some(c) = colors.get("selected").and_then(|s| parse_hex(s)) {
            self.selected = c;
        }
        if let Some(c) = colors.get("rebase_src").and_then(|s| parse_hex(s)) {
            self.rebase_src = c;
        }
        if let Some(c) = colors.get("change_id").and_then(|s| parse_hex(s)) {
            self.change_id = c;
        }
        if let Some(c) = colors.get("author").and_then(|s| parse_hex(s)) {
            self.author = c;
        }
        if let Some(c) = colors.get("desc").and_then(|s| parse_hex(s)) {
            self.desc = c;
        }
        if let Some(c) = colors.get("border_focused").and_then(|s| parse_hex(s)) {
            self.border_focused = c;
        }
        if let Some(c) = colors.get("border_unfocused").and_then(|s| parse_hex(s)) {
            self.border_unfocused = c;
        }
        if let Some(c) = colors.get("status_ok").and_then(|s| parse_hex(s)) {
            self.status_ok = c;
        }
        if let Some(c) = colors.get("status_err").and_then(|s| parse_hex(s)) {
            self.status_err = c;
        }
        if let Some(c) = colors.get("header_bg").and_then(|s| parse_hex(s)) {
            self.header_bg = c;
        }
        if let Some(c) = colors.get("header_fg").and_then(|s| parse_hex(s)) {
            self.header_fg = c;
        }
        if let Some(c) = colors.get("help_key").and_then(|s| parse_hex(s)) {
            self.help_key = c;
        }
        if let Some(c) = colors.get("help_section").and_then(|s| parse_hex(s)) {
            self.help_section = c;
        }
    }
}

fn parse_hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[derive(Clone, Copy, PartialEq)]
pub enum ThemeKind {
    CatppuccinMocha,
    CatppuccinLatte,
    TokyoNight,
    Dracula,
    GruvboxDark,
    Nord,
    OneDark,
    SolarizedDark,
}

impl ThemeKind {
    pub const ALL: &'static [ThemeKind] = &[
        ThemeKind::CatppuccinMocha,
        ThemeKind::CatppuccinLatte,
        ThemeKind::TokyoNight,
        ThemeKind::Dracula,
        ThemeKind::GruvboxDark,
        ThemeKind::Nord,
        ThemeKind::OneDark,
        ThemeKind::SolarizedDark,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            ThemeKind::CatppuccinMocha => "catppuccin-mocha",
            ThemeKind::CatppuccinLatte => "catppuccin-latte",
            ThemeKind::TokyoNight => "tokyo-night",
            ThemeKind::Dracula => "dracula",
            ThemeKind::GruvboxDark => "gruvbox-dark",
            ThemeKind::Nord => "nord",
            ThemeKind::OneDark => "one-dark",
            ThemeKind::SolarizedDark => "solarized-dark",
        }
    }

    pub fn from_slug(s: &str) -> ThemeKind {
        Self::ALL
            .iter()
            .find(|t| t.slug() == s)
            .copied()
            .unwrap_or(ThemeKind::CatppuccinMocha)
    }

    pub fn theme(self) -> Theme {
        match self {
            ThemeKind::CatppuccinMocha => catppuccin_mocha(),
            ThemeKind::CatppuccinLatte => catppuccin_latte(),
            ThemeKind::TokyoNight => tokyo_night(),
            ThemeKind::Dracula => dracula(),
            ThemeKind::GruvboxDark => gruvbox_dark(),
            ThemeKind::Nord => nord(),
            ThemeKind::OneDark => one_dark(),
            ThemeKind::SolarizedDark => solarized_dark(),
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

fn catppuccin_mocha() -> Theme {
    Theme {
        name: "Catppuccin Mocha",
        bg: rgb(30, 30, 46),                // Base
        working_copy: rgb(148, 226, 213),   // Teal
        immutable: rgb(108, 112, 134),      // Overlay0
        immutable_icon: rgb(250, 179, 135), // Peach
        conflict: rgb(243, 139, 168),       // Red
        empty: rgb(249, 226, 175),          // Yellow
        selected: rgb(137, 180, 250),       // Blue
        rebase_src: rgb(245, 194, 231),     // Pink
        change_id: rgb(116, 199, 236),      // Sapphire
        author: rgb(180, 190, 254),         // Lavender
        desc: rgb(205, 214, 244),           // Text
        border_focused: rgb(203, 166, 247), // Mauve
        border_unfocused: rgb(69, 71, 90),  // Surface1
        status_ok: rgb(166, 227, 161),      // Green
        status_err: rgb(243, 139, 168),     // Red
        header_bg: rgb(24, 24, 37),         // Mantle
        header_fg: rgb(205, 214, 244),      // Text
        help_key: rgb(250, 179, 135),       // Peach
        help_section: rgb(180, 190, 254),   // Lavender
    }
}

fn catppuccin_latte() -> Theme {
    Theme {
        name: "Catppuccin Latte",
        bg: rgb(239, 241, 245),            // Base
        working_copy: rgb(23, 146, 153),   // Teal
        immutable: rgb(156, 160, 176),     // Overlay1
        immutable_icon: rgb(254, 100, 11), // Peach
        conflict: rgb(210, 15, 57),        // Red
        empty: rgb(223, 142, 29),          // Yellow
        selected: rgb(64, 160, 43),        // Green
        rebase_src: rgb(254, 100, 11),     // Peach
        change_id: rgb(32, 159, 181),      // Sapphire
        author: rgb(136, 57, 239),         // Mauve
        desc: rgb(76, 79, 105),            // Text
        border_focused: rgb(23, 146, 153),
        border_unfocused: rgb(188, 192, 204), // Surface2
        status_ok: rgb(64, 160, 43),          // Green
        status_err: rgb(210, 15, 57),         // Red
        header_bg: rgb(220, 224, 232),        // Crust
        header_fg: rgb(76, 79, 105),          // Text
        help_key: rgb(254, 100, 11),          // Peach
        help_section: rgb(136, 57, 239),      // Mauve
    }
}

fn tokyo_night() -> Theme {
    Theme {
        name: "Tokyo Night",
        bg: rgb(26, 27, 38),
        working_copy: rgb(115, 218, 202),   // teal
        immutable: rgb(86, 95, 137),        // comment
        immutable_icon: rgb(255, 158, 100), // orange
        conflict: rgb(247, 118, 142),       // red
        empty: rgb(224, 175, 104),          // yellow
        selected: rgb(158, 206, 106),       // green
        rebase_src: rgb(255, 158, 100),     // orange
        change_id: rgb(122, 162, 247),      // blue
        author: rgb(187, 154, 247),         // purple
        desc: rgb(192, 202, 245),           // fg
        border_focused: rgb(115, 218, 202),
        border_unfocused: rgb(56, 62, 90),
        status_ok: rgb(158, 206, 106),
        status_err: rgb(247, 118, 142),
        header_bg: rgb(13, 17, 23),
        header_fg: rgb(192, 202, 245),
        help_key: rgb(255, 158, 100),
        help_section: rgb(187, 154, 247),
    }
}

fn dracula() -> Theme {
    Theme {
        name: "Dracula",
        bg: rgb(40, 42, 54),
        working_copy: rgb(80, 250, 123),    // green
        immutable: rgb(98, 114, 164),       // comment
        immutable_icon: rgb(255, 184, 108), // orange
        conflict: rgb(255, 85, 85),         // red
        empty: rgb(241, 250, 140),          // yellow
        selected: rgb(255, 184, 108),       // orange
        rebase_src: rgb(255, 121, 198),     // pink
        change_id: rgb(139, 233, 253),      // cyan
        author: rgb(189, 147, 249),         // purple
        desc: rgb(248, 248, 242),           // fg
        border_focused: rgb(80, 250, 123),
        border_unfocused: rgb(68, 71, 90), // current line
        status_ok: rgb(80, 250, 123),
        status_err: rgb(255, 85, 85),
        header_bg: rgb(21, 22, 30),
        header_fg: rgb(248, 248, 242),
        help_key: rgb(255, 184, 108),
        help_section: rgb(189, 147, 249),
    }
}

fn gruvbox_dark() -> Theme {
    Theme {
        name: "Gruvbox Dark",
        bg: rgb(40, 40, 40),
        working_copy: rgb(142, 192, 124),  // green
        immutable: rgb(146, 131, 116),     // gray
        immutable_icon: rgb(214, 153, 33), // yellow (fab428 is gruvbox yellow)
        conflict: rgb(251, 73, 52),        // red
        empty: rgb(250, 189, 47),          // yellow
        selected: rgb(254, 128, 25),       // orange
        rebase_src: rgb(211, 134, 155),    // pink
        change_id: rgb(131, 165, 152),     // aqua
        author: rgb(211, 134, 155),        // purple
        desc: rgb(235, 219, 178),          // fg1
        border_focused: rgb(142, 192, 124),
        border_unfocused: rgb(80, 73, 69), // bg3
        status_ok: rgb(142, 192, 124),
        status_err: rgb(251, 73, 52),
        header_bg: rgb(29, 32, 33), // bg0_hard
        header_fg: rgb(235, 219, 178),
        help_key: rgb(254, 128, 25),
        help_section: rgb(211, 134, 155),
    }
}

fn nord() -> Theme {
    Theme {
        name: "Nord",
        bg: rgb(46, 52, 64),
        working_copy: rgb(143, 188, 187),   // nord7
        immutable: rgb(76, 86, 106),        // nord3
        immutable_icon: rgb(235, 203, 139), // yellow (nord13)
        conflict: rgb(191, 97, 106),        // nord11
        empty: rgb(235, 203, 139),          // nord13
        selected: rgb(163, 190, 140),       // nord14
        rebase_src: rgb(208, 135, 112),     // nord12
        change_id: rgb(129, 161, 193),      // nord9
        author: rgb(180, 142, 173),         // nord15
        desc: rgb(236, 239, 244),           // nord6
        border_focused: rgb(143, 188, 187),
        border_unfocused: rgb(59, 66, 82), // nord1
        status_ok: rgb(163, 190, 140),
        status_err: rgb(191, 97, 106),
        header_bg: rgb(36, 41, 51), // nord0
        header_fg: rgb(236, 239, 244),
        help_key: rgb(208, 135, 112),
        help_section: rgb(180, 142, 173),
    }
}

fn one_dark() -> Theme {
    Theme {
        name: "One Dark",
        bg: rgb(40, 44, 52),
        working_copy: rgb(86, 182, 194),    // cyan
        immutable: rgb(92, 99, 112),        // comment
        immutable_icon: rgb(229, 192, 123), // yellow
        conflict: rgb(224, 108, 117),       // red
        empty: rgb(229, 192, 123),          // yellow
        selected: rgb(152, 195, 121),       // green
        rebase_src: rgb(209, 154, 102),     // orange
        change_id: rgb(97, 175, 239),       // blue
        author: rgb(198, 120, 221),         // purple
        desc: rgb(171, 178, 191),           // fg
        border_focused: rgb(86, 182, 194),
        border_unfocused: rgb(55, 59, 69), // bg2
        status_ok: rgb(152, 195, 121),
        status_err: rgb(224, 108, 117),
        header_bg: rgb(24, 26, 31),
        header_fg: rgb(171, 178, 191),
        help_key: rgb(209, 154, 102),
        help_section: rgb(198, 120, 221),
    }
}

fn solarized_dark() -> Theme {
    Theme {
        name: "Solarized Dark",
        bg: rgb(0, 43, 54),
        working_copy: rgb(42, 161, 152),  // cyan
        immutable: rgb(88, 110, 117),     // base01
        immutable_icon: rgb(181, 137, 0), // yellow
        conflict: rgb(220, 50, 47),       // red
        empty: rgb(181, 137, 0),          // yellow
        selected: rgb(133, 153, 0),       // green
        rebase_src: rgb(203, 75, 22),     // orange
        change_id: rgb(38, 139, 210),     // blue
        author: rgb(108, 113, 196),       // violet
        desc: rgb(131, 148, 150),         // base0
        border_focused: rgb(42, 161, 152),
        border_unfocused: rgb(7, 54, 66), // base02
        status_ok: rgb(133, 153, 0),
        status_err: rgb(220, 50, 47),
        header_bg: rgb(0, 43, 54), // base03
        header_fg: rgb(131, 148, 150),
        help_key: rgb(203, 75, 22),
        help_section: rgb(108, 113, 196),
    }
}
