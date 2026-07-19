mod config;
mod journal;

use std::collections::VecDeque;

use cosmic::app::{Core, Settings, Task};
use cosmic::cctk::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::{
    IcedMargin, SctkLayerSurfaceSettings,
};
use cosmic::iced::widget::scrollable::{self, RelativeOffset};
use cosmic::iced::window::Id;
use cosmic::iced::{Color, Font, Length, Limits, Subscription};
use cosmic::surface::action::{app_layer_shell, LiveSettings};
use cosmic::widget::{column, container, text};
use cosmic::{Application, Element};

use config::{Config, Position};
use journal::{LogLine, Severity};

fn main() -> cosmic::iced::Result {
    let settings = Settings::default()
        .no_main_window(true)
        .transparent(true)
        .exit_on_close(false)
        .is_daemon(true)
        .client_decorations(false)
        .default_text_size(12.0);

    cosmic::app::run::<DeskLog>(settings, ())
}

struct DeskLog {
    core: Core,
    config: Config,
    lines: VecDeque<LogLine>,
    surface_id: Id,
    scroll_id: cosmic::widget::Id,
}

#[derive(Debug, Clone)]
enum Message {
    Line(LogLine),
}

impl Application for DeskLog {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "io.github.sbj_ee.CosmicDeskLog";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let config = Config::load();
        let surface_id = Id::unique();
        let scroll_id = cosmic::widget::Id::unique();

        let app = Self {
            core,
            config: config.clone(),
            lines: VecDeque::with_capacity(config.max_lines),
            surface_id,
            scroll_id,
        };

        let width = config.width.resolve();
        let height = config.height;
        let margin_left = config.margin_left;
        let margin_top = config.margin_top;
        let margin_bottom = config.margin_bottom;
        let position = config.position;
        let id = surface_id;

        let (anchor, margin) = match position {
            Position::Top => (
                Anchor::TOP.union(Anchor::LEFT),
                IcedMargin {
                    top: margin_top,
                    right: 0,
                    bottom: 0,
                    left: margin_left,
                },
            ),
            Position::Bottom => (
                Anchor::BOTTOM.union(Anchor::LEFT),
                IcedMargin {
                    top: 0,
                    right: 0,
                    bottom: margin_bottom,
                    left: margin_left,
                },
            ),
        };

        let open = Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Surface(
            app_layer_shell::<DeskLog>(
                |_| LiveSettings::default(),
                move |_app| SctkLayerSurfaceSettings {
                    id,
                    layer: Layer::Bottom,
                    keyboard_interactivity: KeyboardInteractivity::None,
                    // Empty input region: clicks pass through to the desktop.
                    input_zone: Some(Vec::new()),
                    anchor,
                    namespace: "cosmic-desk-log".into(),
                    margin,
                    size: Some((Some(width), Some(height))),
                    exclusive_zone: 0,
                    size_limits: Limits::NONE
                        .min_width(1.0)
                        .min_height(1.0)
                        .max_width(width as f32)
                        .max_height(height as f32),
                    ..Default::default()
                },
                None,
            ),
        )));

        (app, open)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Line(line) => {
                journal::push_line(&mut self.lines, line, self.config.max_lines);
                scrollable::snap_to(self.scroll_id.clone(), RelativeOffset::END.into())
            }
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::run(journal_stream).map(Message::Line)
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // No XDG main window; content lives on the layer surface.
        text("").into()
    }

    fn view_window(&self, id: Id) -> Element<'_, Self::Message> {
        if id != self.surface_id {
            return text("").into();
        }

        let font_size = self.config.font_size;
        let opacity = self.config.opacity;
        let lines = column::with_capacity(self.lines.len())
            .spacing(1)
            .width(Length::Fill)
            .extend(self.lines.iter().map(|line| {
                text(line.text.as_str())
                    .size(font_size)
                    .font(Font::MONOSPACE)
                    .class(cosmic::theme::Text::Color(severity_color(line.severity)))
                    .into()
            }));

        let body = if self.lines.is_empty() {
            Element::from(
                text("waiting for journal…")
                    .size(font_size)
                    .font(Font::MONOSPACE)
                    .class(cosmic::theme::Text::Color(Color::from_rgba(
                        0.75, 0.78, 0.82, 0.7,
                    ))),
            )
        } else {
            // Click-through surface: keep auto-scroll, hide the dead scrollbar.
            scrollable::Scrollable::new(lines)
                .id(self.scroll_id.clone())
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::hidden(),
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([8, 10])
            .align_x(Horizontal::Left)
            .class(cosmic::style::Container::custom(move |_theme| {
                cosmic::widget::container::Style {
                    // Near-transparent tint so the wallpaper shows through.
                    background: Some(cosmic::iced::Background::Color(Color::from_rgba(
                        0.04, 0.05, 0.07, opacity,
                    ))),
                    text_color: Some(Color::from_rgb(0.82, 0.86, 0.90)),
                    border: cosmic::iced::Border {
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    shadow: cosmic::iced::Shadow::default(),
                    icon_color: None,
                    snap: true,
                }
            }))
            .into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: Color::from_rgb(0.82, 0.86, 0.90),
            icon_color: Color::from_rgb(0.82, 0.86, 0.90),
        })
    }
}

fn severity_color(severity: Severity) -> Color {
    match severity {
        Severity::Error => Color::from_rgb(0.95, 0.45, 0.42),
        Severity::Warning => Color::from_rgb(0.95, 0.78, 0.35),
        Severity::Info => Color::from_rgba(0.82, 0.86, 0.90, 0.92),
        Severity::Debug => Color::from_rgba(0.55, 0.62, 0.70, 0.75),
    }
}

fn journal_stream() -> impl cosmic::iced::futures::Stream<Item = LogLine> {
    journal::follow(Config::load())
}
