//! KEY=VALUE config under ~/.config/cosmic-desk-log/config.

const DEFAULT_HEIGHT: u32 = 280;
const DEFAULT_MARGIN_LEFT: i32 = 24;
const DEFAULT_MARGIN_TOP: i32 = 48;
const DEFAULT_MARGIN_BOTTOM: i32 = 72;
const DEFAULT_FONT_SIZE: f32 = 12.0;
const DEFAULT_OPACITY: f32 = 0.35;
const DEFAULT_MAX_LINES: usize = 200;
/// Used when `cosmic-randr` cannot report a current mode.
const FALLBACK_WIDTH: u32 = 1920;

/// Vertical edge on the left side of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    Top,
    #[default]
    Bottom,
}

impl Position {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}

/// Panel width: the whole active output, half of it, or an explicit pixel size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidthSpec {
    #[default]
    Full,
    Half,
    Pixels(u32),
}

impl WidthSpec {
    fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("full") || value.eq_ignore_ascii_case("screen") || value == "0"
        {
            return Some(Self::Full);
        }
        if value.eq_ignore_ascii_case("half") {
            return Some(Self::Half);
        }
        value
            .parse::<u32>()
            .ok()
            .map(|px| Self::Pixels(px.clamp(120, 8192)))
    }

    /// `gutter` is the margin left free on each side; it only applies to
    /// output-relative widths, since explicit pixel sizes are taken as given.
    pub fn resolve(self, gutter: u32) -> u32 {
        match self {
            Self::Full => active_output_width()
                .unwrap_or(FALLBACK_WIDTH)
                .saturating_sub(gutter * 2)
                .clamp(120, 8192),
            Self::Half => (active_output_width().unwrap_or(FALLBACK_WIDTH) / 2)
                .saturating_sub(gutter)
                .clamp(120, 8192),
            Self::Pixels(px) => px,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub width: WidthSpec,
    pub height: u32,
    pub position: Position,
    pub margin_left: i32,
    pub margin_top: i32,
    pub margin_bottom: i32,
    pub font_size: f32,
    pub opacity: f32,
    pub max_lines: usize,
    /// Extra arguments appended to the default `journalctl` invocation.
    pub journal_args: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: WidthSpec::default(),
            height: DEFAULT_HEIGHT,
            position: Position::default(),
            margin_left: DEFAULT_MARGIN_LEFT,
            margin_top: DEFAULT_MARGIN_TOP,
            margin_bottom: DEFAULT_MARGIN_BOTTOM,
            font_size: DEFAULT_FONT_SIZE,
            opacity: DEFAULT_OPACITY,
            max_lines: DEFAULT_MAX_LINES,
            journal_args: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut config = Self::default();
        let path = dirs_config_path();
        let Ok(contents) = std::fs::read_to_string(path) else {
            return config;
        };

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            match key {
                "width" => {
                    if let Some(spec) = WidthSpec::parse(value) {
                        config.width = spec;
                    }
                }
                "height" => {
                    if let Ok(v) = value.parse::<u32>() {
                        config.height = v.clamp(80, 4096);
                    }
                }
                "position" => {
                    if let Some(pos) = Position::parse(value) {
                        config.position = pos;
                    }
                }
                "margin_left" => {
                    if let Ok(v) = value.parse::<i32>() {
                        config.margin_left = v.max(0);
                    }
                }
                "margin_top" => {
                    if let Ok(v) = value.parse::<i32>() {
                        config.margin_top = v.max(0);
                    }
                }
                "margin_bottom" => {
                    if let Ok(v) = value.parse::<i32>() {
                        config.margin_bottom = v.max(0);
                    }
                }
                "font_size" => {
                    if let Ok(v) = value.parse::<f32>() {
                        config.font_size = v.clamp(8.0, 32.0);
                    }
                }
                "opacity" => {
                    if let Ok(v) = value.parse::<f32>() {
                        config.opacity = v.clamp(0.0, 1.0);
                    }
                }
                "max_lines" => {
                    if let Ok(v) = value.parse::<usize>() {
                        config.max_lines = v.clamp(20, 5000);
                    }
                }
                "journal_args" => {
                    config.journal_args = value
                        .split_whitespace()
                        .map(str::to_string)
                        .collect();
                }
                _ => {}
            }
        }

        config
    }
}

pub fn dirs_config_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_default();
    base.join("cosmic-desk-log").join("config")
}

/// Logical width of the current mode from `cosmic-randr list`.
fn active_output_width() -> Option<u32> {
    let output = std::process::Command::new("cosmic-randr")
        .arg("list")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let plain = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    for line in plain.lines() {
        if !line.contains("(current)") {
            continue;
        }
        if let Some((w, _)) = parse_mode_size(line) {
            return Some(w);
        }
    }
    None
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(n) = chars.next() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn parse_mode_size(line: &str) -> Option<(u32, u32)> {
    let trimmed = line.trim();
    let x = trimmed.find('x')?;
    let start = trimmed[..x]
        .rfind(|c: char| !c.is_ascii_digit())
        .map(|i| i + 1)
        .unwrap_or(0);
    let width: u32 = trimmed[start..x].parse().ok()?;
    let rest = &trimmed[x + 1..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let height: u32 = rest[..end].parse().ok()?;
    Some((width, height))
}
