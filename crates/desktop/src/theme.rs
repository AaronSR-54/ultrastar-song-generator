//! Visual language of the desktop app.
//!
//! One palette, one type scale and a handful of custom widgets keep every
//! screen coherent: warm-neutral dark surfaces, a single blue accent, generous
//! spacing and instant feedback on press.

use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin,
    Pos2, Rect, RichText, Shadow, Stroke, StrokeKind, TextStyle, Ui, Vec2,
};

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

pub const BG: Color32 = Color32::from_rgb(0x0F, 0x0F, 0x12);
pub const SURFACE: Color32 = Color32::from_rgb(0x18, 0x18, 0x1D);
pub const SURFACE_2: Color32 = Color32::from_rgb(0x22, 0x22, 0x29);
pub const SURFACE_3: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x33);
pub const SURFACE_4: Color32 = Color32::from_rgb(0x34, 0x34, 0x3D);
pub const FIELD: Color32 = Color32::from_rgb(0x11, 0x11, 0x15);
pub const BORDER: Color32 = Color32::from_rgb(0x2C, 0x2C, 0x34);
pub const BORDER_HOVER: Color32 = Color32::from_rgb(0x3E, 0x3E, 0x49);
pub const TEXT: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF7);
pub const TEXT_2: Color32 = Color32::from_rgb(0x9B, 0x9B, 0xA6);
pub const TEXT_3: Color32 = Color32::from_rgb(0x6D, 0x6D, 0x78);
pub const ACCENT: Color32 = Color32::from_rgb(0x0A, 0x84, 0xFF);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x33, 0x99, 0xFF);
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x00, 0x6A, 0xE0);
pub const WAVE: Color32 = Color32::from_rgb(0x4F, 0xA8, 0xFF);
pub const SUCCESS: Color32 = Color32::from_rgb(0x30, 0xD1, 0x58);
pub const WARNING: Color32 = Color32::from_rgb(0xFF, 0xD6, 0x0A);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x45, 0x3A);
pub const ORANGE: Color32 = Color32::from_rgb(0xFF, 0x9F, 0x0A);
pub const TRACK: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x32);

// ---------------------------------------------------------------------------
// Typography
// ---------------------------------------------------------------------------

const SEMIBOLD_FAMILY: &str = "usdx-semibold";

/// Heavier face used for titles, buttons and numbers.
pub fn semibold() -> FontFamily {
    FontFamily::Name(SEMIBOLD_FAMILY.into())
}

pub fn display(size: f32, text: impl Into<String>) -> RichText {
    RichText::new(text).font(FontId::new(size, semibold()))
}

pub fn title(text: impl Into<String>) -> RichText {
    RichText::new(text).font(FontId::new(17.0, semibold()))
}

pub fn strong(text: impl Into<String>) -> RichText {
    RichText::new(text).font(FontId::new(14.5, semibold()))
}

pub fn secondary(text: impl Into<String>) -> RichText {
    RichText::new(text).color(TEXT_2)
}

pub fn caption(text: impl Into<String>) -> RichText {
    RichText::new(text).size(12.5).color(TEXT_3)
}

/// Installs fonts, palette and spacing. Call once when the app starts.
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_theme(egui::ThemePreference::Dark);

    let visuals = visuals();
    let spacing = spacing();
    ctx.all_styles_mut(move |style| {
        style.visuals = visuals.clone();
        style.spacing = spacing.clone();
        style.text_styles = text_styles();
        style.animation_time = 0.18;
    });
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    if let Some(set) = system_fonts() {
        fonts.font_data.insert(
            "usdx-regular".into(),
            Arc::new(FontData::from_owned(set.regular)),
        );
        if let Some(semibold) = set.semibold {
            fonts.font_data.insert(
                "usdx-semibold".into(),
                Arc::new(FontData::from_owned(semibold)),
            );
        }

        let fallback = fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();

        let mut proportional = vec!["usdx-regular".to_owned()];
        proportional.extend(fallback.iter().cloned());
        fonts
            .families
            .insert(FontFamily::Proportional, proportional);

        let heavy_key = if fonts.font_data.contains_key("usdx-semibold") {
            "usdx-semibold"
        } else {
            "usdx-regular"
        };
        let mut heavy = vec![heavy_key.to_owned()];
        heavy.extend(fallback);
        fonts.families.insert(semibold(), heavy);
    }
    ctx.set_fonts(fonts);
}

struct FontSet {
    regular: Vec<u8>,
    semibold: Option<Vec<u8>>,
}

fn read_font(candidates: &[std::path::PathBuf]) -> Option<Vec<u8>> {
    candidates
        .iter()
        .find_map(|path| std::fs::read(path).ok().filter(|bytes| bytes.len() > 1024))
}

#[cfg(target_os = "windows")]
fn system_fonts() -> Option<FontSet> {
    let root = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("C:\\Windows"))
        .join("Fonts");
    let regular = read_font(&[root.join("segoeui.ttf")])?;
    let semibold = read_font(&[root.join("seguisb.ttf"), root.join("segoeuib.ttf")]);
    Some(FontSet { regular, semibold })
}

#[cfg(target_os = "macos")]
fn system_fonts() -> Option<FontSet> {
    let regular = read_font(&[
        "/System/Library/Fonts/SFNS.ttf".into(),
        "/System/Library/Fonts/Supplemental/Arial.ttf".into(),
    ])?;
    Some(FontSet {
        regular,
        semibold: None,
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn system_fonts() -> Option<FontSet> {
    let regular = read_font(&[
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".into(),
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf".into(),
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf".into(),
    ])?;
    let semibold = read_font(&[
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf".into(),
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf".into(),
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf".into(),
    ]);
    Some(FontSet { regular, semibold })
}

fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    [
        (TextStyle::Small, FontId::proportional(12.5)),
        (TextStyle::Body, FontId::proportional(14.5)),
        (TextStyle::Button, FontId::new(14.5, semibold())),
        (TextStyle::Heading, FontId::new(24.0, semibold())),
        (TextStyle::Monospace, FontId::monospace(12.5)),
    ]
    .into()
}

fn spacing() -> egui::style::Spacing {
    egui::style::Spacing {
        item_spacing: Vec2::new(10.0, 10.0),
        button_padding: Vec2::new(14.0, 8.0),
        interact_size: Vec2::new(36.0, 34.0),
        window_margin: Margin::same(24),
        menu_margin: Margin::same(12),
        indent: 18.0,
        slider_width: 150.0,
        combo_width: 160.0,
        text_edit_width: 260.0,
        icon_width: 18.0,
        icon_width_inner: 10.0,
        icon_spacing: 4.0,
        ..Default::default()
    }
}

fn visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.dark_mode = true;
    visuals.override_text_color = Some(TEXT);
    visuals.weak_text_color = Some(TEXT_2);
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = CornerRadius::same(20);
    visuals.window_shadow = Shadow {
        offset: [0, 14],
        blur: 40,
        spread: 0,
        color: Color32::from_black_alpha(150),
    };
    visuals.popup_shadow = visuals.window_shadow;
    visuals.menu_corner_radius = CornerRadius::same(12);
    visuals.extreme_bg_color = FIELD;
    visuals.text_edit_bg_color = Some(FIELD);
    visuals.faint_bg_color = Color32::from_rgb(0x15, 0x15, 0x19);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = WARNING;
    visuals.error_fg_color = DANGER;
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(10, 132, 255, 90);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    visuals.slider_trailing_fill = true;
    visuals.disabled_alpha = 0.42;
    visuals.striped = false;

    visuals.widgets.noninteractive =
        widget(Color32::TRANSPARENT, Color32::TRANSPARENT, BORDER, TEXT, 8);
    visuals.widgets.inactive = widget(SURFACE_2, SURFACE_2, BORDER, TEXT, 10);
    visuals.widgets.hovered = widget(SURFACE_3, SURFACE_3, BORDER_HOVER, TEXT, 10);
    visuals.widgets.active = widget(SURFACE_4, SURFACE_4, BORDER_HOVER, TEXT, 10);
    visuals.widgets.open = widget(SURFACE_3, SURFACE_3, BORDER_HOVER, TEXT, 10);
    visuals
}

fn widget(
    bg_fill: Color32,
    weak_bg_fill: Color32,
    stroke: Color32,
    text: Color32,
    radius: u8,
) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill,
        weak_bg_fill,
        bg_stroke: if stroke == Color32::TRANSPARENT {
            Stroke::NONE
        } else {
            Stroke::new(1.0, stroke)
        },
        corner_radius: CornerRadius::same(radius),
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    }
}

// ---------------------------------------------------------------------------
// Building blocks
// ---------------------------------------------------------------------------

/// Soft elevated surface used by every screen.
pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(22)
        .shadow(Shadow {
            offset: [0, 16],
            blur: 44,
            spread: 0,
            color: Color32::from_black_alpha(120),
        })
        .inner_margin(Margin::same(30))
        .show(ui, add_contents)
        .inner
}

/// Low-contrast inset used for paths, errors and secondary controls.
pub fn inset<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(FIELD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, add_contents)
        .inner
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
}

/// A button that paints its own surface, so hover and press feedback are
/// immediate and identical on every platform.
pub fn button(ui: &mut Ui, label: &str, kind: ButtonKind, enabled: bool) -> egui::Response {
    let font = match kind {
        ButtonKind::Primary => FontId::new(15.0, semibold()),
        _ => FontId::proportional(14.5),
    };
    let size = match kind {
        ButtonKind::Ghost => {
            let galley = ui
                .painter()
                .layout_no_wrap(label.to_owned(), font.clone(), TEXT);
            Vec2::new(galley.size().x + 24.0, 32.0)
        }
        _ => Vec2::new(ui.available_width(), 46.0),
    };
    button_sized(ui, label, kind, size, enabled)
}

pub fn button_sized(
    ui: &mut Ui,
    label: &str,
    kind: ButtonKind,
    size: Vec2,
    enabled: bool,
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();

    let font = match kind {
        ButtonKind::Primary => FontId::new(15.0, semibold()),
        _ => FontId::proportional(14.5),
    };
    let (fill, stroke, text, radius) = match kind {
        ButtonKind::Primary => {
            let fill = if !enabled {
                ACCENT.gamma_multiply(0.35)
            } else if pressed {
                ACCENT_PRESSED
            } else if hovered {
                ACCENT_HOVER
            } else {
                ACCENT
            };
            (
                fill,
                Stroke::NONE,
                if enabled {
                    Color32::WHITE
                } else {
                    Color32::from_white_alpha(140)
                },
                12,
            )
        }
        ButtonKind::Secondary => {
            let fill = if pressed {
                SURFACE_4
            } else if hovered {
                SURFACE_3
            } else {
                SURFACE_2
            };
            (fill, Stroke::new(1.0, BORDER), TEXT, 12)
        }
        ButtonKind::Ghost => {
            let fill = if pressed {
                SURFACE_3
            } else if hovered {
                SURFACE_2
            } else {
                Color32::TRANSPARENT
            };
            (fill, Stroke::NONE, if hovered { TEXT } else { TEXT_2 }, 10)
        }
    };

    // Press feedback shrinks the painted surface by one point: the equivalent
    // of `scale(0.97)` and it happens on pointer-down, not on release.
    let painted = if pressed { rect.shrink(1.0) } else { rect };
    let painter = ui.painter();
    painter.rect(painted, radius, fill, stroke, StrokeKind::Inside);
    painter.text(painted.center(), Align2::CENTER_CENTER, label, font, text);

    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

pub fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    button(ui, label, ButtonKind::Primary, true)
}

pub fn secondary_button(ui: &mut Ui, label: &str) -> egui::Response {
    button(ui, label, ButtonKind::Secondary, true)
}

pub fn ghost_button(ui: &mut Ui, label: &str) -> egui::Response {
    button(ui, label, ButtonKind::Ghost, true)
}

pub fn icon_button(ui: &mut Ui, glyph: &str, tooltip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(38.0), egui::Sense::click());
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let fill = if pressed {
        SURFACE_3
    } else if hovered {
        SURFACE_2
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 10, fill);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::new(17.0, semibold()),
        if hovered { TEXT } else { TEXT_2 },
    );
    response
        .on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Compact rounded choice used for stems and tempo candidates.
pub fn pill(ui: &mut Ui, label: &str, selected: bool) -> egui::Response {
    let font = FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), TEXT);
    let size = Vec2::new(galley.size().x + 22.0, 30.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let fill = if selected {
        ACCENT.gamma_multiply(if pressed { 0.75 } else { 1.0 })
    } else if pressed {
        SURFACE_4
    } else if hovered {
        SURFACE_3
    } else {
        SURFACE_2
    };
    let text = if selected {
        Color32::WHITE
    } else if hovered {
        TEXT
    } else {
        TEXT_2
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        15,
        fill,
        if selected {
            Stroke::NONE
        } else {
            Stroke::new(1.0, BORDER)
        },
        StrokeKind::Inside,
    );
    painter.text(rect.center(), Align2::CENTER_CENTER, label, font, text);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Full-width text field with a focus ring.
pub fn text_field(ui: &mut Ui, text: &mut String, hint: &str) -> egui::Response {
    text_field_sized(ui, text, hint, ui.available_width(), 15.5, 14.0)
}

pub fn text_field_sized(
    ui: &mut Ui,
    text: &mut String,
    hint: &str,
    width: f32,
    font_size: f32,
    padding: f32,
) -> egui::Response {
    let frame = egui::Frame::NONE
        .fill(FIELD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12);
    let response = ui.add(
        egui::TextEdit::singleline(text)
            .frame(frame)
            .margin(Margin::symmetric(14, padding as i8))
            .font(FontId::proportional(font_size))
            .hint_text(hint.to_owned())
            .desired_width(width),
    );
    let painter = ui.painter();
    if response.has_focus() {
        painter.rect_stroke(
            response.rect,
            12,
            Stroke::new(2.0, ACCENT.gamma_multiply(0.75)),
            StrokeKind::Outside,
        );
    } else if response.hovered() {
        painter.rect_stroke(
            response.rect,
            12,
            Stroke::new(1.0, BORDER_HOVER),
            StrokeKind::Inside,
        );
    }
    response
}

/// Rounded progress track.
pub fn progress_bar(ui: &mut Ui, fraction: f32) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4, TRACK);
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        let width = (rect.width() * fraction).max(8.0);
        let fill = Rect::from_min_size(rect.min, Vec2::new(width, rect.height()));
        painter.rect_filled(fill, 4, ACCENT);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Pending,
    Running,
    Done,
    Failed,
}

/// Stage marker: ring, spinner, check or cross.
pub fn stage_glyph(ui: &Ui, center: Pos2, radius: f32, glyph: Glyph, time: f64) {
    let painter = ui.painter();
    match glyph {
        Glyph::Pending => {
            painter.circle_stroke(center, radius, Stroke::new(1.4, BORDER_HOVER));
        }
        Glyph::Done => {
            painter.circle_filled(center, radius, SUCCESS);
            check_mark(painter, center, radius * 0.62, Color32::WHITE);
        }
        Glyph::Failed => {
            painter.circle_filled(center, radius, DANGER);
            let shift = radius * 0.34;
            painter.line_segment(
                [
                    center + Vec2::new(-shift, -shift),
                    center + Vec2::new(shift, shift),
                ],
                Stroke::new(1.8, Color32::WHITE),
            );
            painter.line_segment(
                [
                    center + Vec2::new(shift, -shift),
                    center + Vec2::new(-shift, shift),
                ],
                Stroke::new(1.8, Color32::WHITE),
            );
        }
        Glyph::Running => {
            let phase = (time * 3.4) as f32;
            let count = 20;
            for index in 0..count {
                let t = index as f32 / count as f32;
                let angle = phase + t * std::f32::consts::TAU;
                let alpha = (35.0 + 220.0 * t).clamp(0.0, 255.0) as u8;
                let point = center + Vec2::new(angle.cos(), angle.sin()) * (radius - 1.2);
                painter.circle_filled(
                    point,
                    1.6,
                    Color32::from_rgba_unmultiplied(10, 132, 255, alpha),
                );
            }
        }
    }
}

pub fn check_mark(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let point = |x: f32, y: f32| center + Vec2::new(x, y) * radius;
    painter.add(egui::Shape::line(
        vec![point(-0.75, 0.02), point(-0.18, 0.58), point(0.78, -0.52)],
        Stroke::new(2.2, color),
    ));
}

/// Tiny spring that always animates from its current value, so it can be
/// re-targeted mid-motion without a jump.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    value: f32,
    velocity: f32,
}

impl Spring {
    pub const fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn reset(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
    }

    /// `response` is the time scale in seconds, `damping` 1.0 is critically
    /// damped and values below 1.0 overshoot slightly.
    pub fn update(&mut self, target: f32, dt: f32, response: f32, damping: f32) -> f32 {
        let dt = dt.clamp(0.0, 1.0 / 20.0);
        let omega = std::f32::consts::TAU / response.max(0.05);
        let stiffness = omega * omega;
        let friction = 2.0 * damping * omega;
        let steps = 4;
        let step = dt / steps as f32;
        for _ in 0..steps {
            let acceleration = -stiffness * (self.value - target) - friction * self.velocity;
            self.velocity += acceleration * step;
            self.value += self.velocity * step;
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_settles_on_its_target() {
        let mut spring = Spring::new(0.0);
        for _ in 0..240 {
            spring.update(1.0, 1.0 / 60.0, 0.35, 1.0);
        }
        assert!((spring.value() - 1.0).abs() < 0.001, "{spring:?}");
    }

    #[test]
    fn spring_is_interruptible() {
        let mut spring = Spring::new(0.0);
        for _ in 0..10 {
            spring.update(1.0, 1.0 / 60.0, 0.35, 1.0);
        }
        let before = spring.value();
        spring.update(0.0, 1.0 / 60.0, 0.35, 1.0);
        // Re-targeting must continue from the presentation value, not jump.
        assert!(
            (spring.value() - before).abs() < 0.1,
            "jumped from {before}"
        );
        for _ in 0..300 {
            spring.update(0.0, 1.0 / 60.0, 0.35, 1.0);
        }
        assert!(spring.value().abs() < 0.01, "{spring:?}");
    }
}
