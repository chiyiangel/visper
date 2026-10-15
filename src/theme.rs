use gpui::WindowAppearance;

/// Semantic colors shared by every screen; values follow visual-spec-v1.md.
#[derive(Clone, Copy)]
pub struct Palette {
    pub main: u32,
    pub sidebar: u32,
    pub raised: u32,
    /// The recessed track behind the language pill.
    pub track: u32,
    pub line: u32,
    pub ink: u32,
    pub secondary: u32,
    pub tertiary: u32,
    pub accent: u32,
    pub accent_hover: u32,
    pub accent_pressed: u32,
    pub on_accent: u32,
    /// The Mark's vee and its two lines.
    pub mark: u32,
    pub mark_lines: u32,
    /// The Mark when it is only company, with nothing selected.
    pub mark_faint: u32,
    pub mark_lines_faint: u32,
    pub selected: u32,
    pub playing: u32,
    pub warning: u32,
    pub warning_surface: u32,
    pub danger: u32,
    pub danger_surface: u32,
    pub disabled: u32,
}

impl Palette {
    pub fn from_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self {
                main: 0x222623,
                sidebar: 0x1b1f1c,
                raised: 0x2b312c,
                track: 0x1b1f1c,
                line: 0x3b433c,
                ink: 0xf1f3ee,
                secondary: 0xb5bdb3,
                tertiary: 0x9ba59a,
                accent: 0xabc4ad,
                accent_hover: 0xbbd0bd,
                accent_pressed: 0x91b294,
                on_accent: 0x19221c,
                mark: 0xabc4ad,
                mark_lines: 0x5d7a63,
                mark_faint: 0x3b433c,
                mark_lines_faint: 0x313831,
                selected: 0x344538,
                playing: 0x2d3b30,
                warning: 0xd1b06f,
                warning_surface: 0x332f23,
                danger: 0xe29e94,
                danger_surface: 0x3a2927,
                disabled: 0x647065,
            },
            _ => Self {
                main: 0xfaf9f6,
                sidebar: 0xefeeea,
                raised: 0xffffff,
                track: 0xe9e8e3,
                line: 0xddded8,
                ink: 0x202622,
                secondary: 0x5d655f,
                tertiary: 0x626a64,
                accent: 0x426650,
                accent_hover: 0x365940,
                accent_pressed: 0x2d4c3d,
                on_accent: 0xffffff,
                mark: 0x426650,
                mark_lines: 0xa3bea4,
                mark_faint: 0xc9ccc4,
                mark_lines_faint: 0xdcddd7,
                selected: 0xe0e7de,
                playing: 0xeef2ec,
                warning: 0x8a6429,
                warning_surface: 0xf5f0e5,
                danger: 0x9c443f,
                danger_surface: 0xf7ece9,
                disabled: 0x929990,
            },
        }
    }
}

/// `a` moved toward `b` by `t`, channel by channel.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    [16, 8, 0].iter().fold(0, |out, shift| {
        let (from, to) = ((a >> shift) & 0xff, (b >> shift) & 0xff);
        let channel = from as f32 + (to as f32 - from as f32) * t;
        out | ((channel.round() as u32) << shift)
    })
}

/// `color` laid over whatever is beneath at `alpha`.
pub fn tint(color: u32, alpha: f32) -> gpui::Rgba {
    gpui::Rgba {
        a: alpha.clamp(0.0, 1.0),
        ..gpui::rgb(color)
    }
}

/// Logical-pixel measurements, independent of display scale and font rem size.
pub mod metrics {
    pub const WINDOW_WIDTH: f32 = 1120.0;
    pub const WINDOW_HEIGHT: f32 = 760.0;
    pub const MIN_WIDTH: f32 = 920.0;
    pub const MIN_HEIGHT: f32 = 620.0;
    pub const TITLEBAR: f32 = 52.0;
    pub const BUTTON: f32 = 32.0;
    pub const BUTTON_RADIUS: f32 = 8.0;
    pub const ROW_RADIUS: f32 = 8.0;
    pub const SIDEBAR: f32 = 280.0;
    pub const NARROW_SIDEBAR: f32 = 264.0;
    pub const CONTENT: f32 = 32.0;
    pub const NARROW_CONTENT: f32 = 24.0;
    pub const READING_WIDTH: f32 = 880.0;
    pub const TIMESTAMP: f32 = 64.0;
    pub const BODY_SIZE: f32 = 15.0;
    pub const BODY_LINE: f32 = 24.0;
    pub const LABEL_SIZE: f32 = 13.0;
    pub const LABEL_LINE: f32 = 20.0;
    pub const SMALL_SIZE: f32 = 12.0;
    pub const SMALL_LINE: f32 = 18.0;
    pub const MONO_FONT: &str = "Menlo";
    pub const UI_FONT: &str = ".SystemUIFont";
}
