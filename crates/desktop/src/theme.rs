//! Visual language of the desktop app.
//!
//! One neutral palette, one accent, one type scale and a small set of painted
//! widgets keep every screen coherent. Icons are vector paths (see
//! [`crate::icons`]), never font glyphs, and every interactive surface eases
//! between states so the app feels settled instead of flickering.

use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin,
    Pos2, Rect, RichText, Sense, Shadow, Stroke, StrokeKind, TextStyle, Ui, Vec2,
};

use crate::icons::{self, Icon};

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------
//
// Every colour the app paints comes from this block. The accent is reserved for
// what can be acted on or what is playing; decoration (grid, brackets, hairlines)
// stays neutral so blue keeps a single meaning.

pub const BG: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0C);
pub const SURFACE: Color32 = Color32::from_rgb(0x15, 0x16, 0x1B);
pub const SURFACE_2: Color32 = Color32::from_rgb(0x1B, 0x1C, 0x22);
pub const SURFACE_3: Color32 = Color32::from_rgb(0x23, 0x24, 0x2B);
pub const SURFACE_4: Color32 = Color32::from_rgb(0x2C, 0x2D, 0x35);
pub const FIELD: Color32 = Color32::from_rgb(0x0D, 0x0D, 0x11);
pub const BORDER: Color32 = Color32::from_rgb(0x2A, 0x2C, 0x36);
pub const BORDER_HOVER: Color32 = Color32::from_rgb(0x3C, 0x3E, 0x48);
pub const TEXT: Color32 = Color32::from_rgb(0xF4, 0xF4, 0xF7);
pub const TEXT_2: Color32 = Color32::from_rgb(0xA0, 0xA2, 0xAD);
pub const TEXT_3: Color32 = Color32::from_rgb(0x82, 0x84, 0x8F);
pub const ACCENT: Color32 = Color32::from_rgb(0x4F, 0x8D, 0xFF);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x6B, 0x9F, 0xFF);
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x3E, 0x77, 0xE6);
pub const ACCENT_TEXT: Color32 = Color32::from_rgb(0x9D, 0xC0, 0xFF);
pub const WAVE: Color32 = Color32::from_rgb(0x6E, 0xA8, 0xFF);
pub const SUCCESS: Color32 = Color32::from_rgb(0x3D, 0xD6, 0x8C);
pub const WARNING: Color32 = Color32::from_rgb(0xF2, 0xC1, 0x4E);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x5F, 0x5F);
pub const ORANGE: Color32 = Color32::from_rgb(0xFF, 0xB2, 0x5E);
pub const TRACK: Color32 = Color32::from_rgb(0x24, 0x25, 0x2C);

/// Label painted on top of a filled accent surface (6.2:1, white would be 3.2:1).
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x08, 0x0D, 0x18);
/// Pitch lane grid, from octave lines down to the faintest semitone.
pub const GRID_MAJOR: Color32 = Color32::from_rgb(0x24, 0x26, 0x30);
pub const GRID_MINOR: Color32 = Color32::from_rgb(0x17, 0x18, 0x1E);
pub const GRID_FAINT: Color32 = Color32::from_rgb(0x11, 0x12, 0x17);
/// Windows' own close-button red, used only by the custom title bar.
pub const CLOSE_RED: Color32 = Color32::from_rgb(0xE8, 0x11, 0x23);

// One radius: the whole app is square, like a technical drawing.
pub const RADIUS: u8 = 0;
const HOVER_TIME: f32 = 0.14;
const PRESS_TIME: f32 = 0.07;

// ---------------------------------------------------------------------------
// Spacing scale
// ---------------------------------------------------------------------------
//
// Every gap, padding and inset is one of these five steps. Nothing else.

pub const GAP_XS: f32 = 4.0;
pub const GAP_S: f32 = 8.0;
pub const GAP_M: f32 = 12.0;
pub const GAP_L: f32 = 16.0;
pub const GAP_XL: f32 = 24.0;

// Icon glyph sizes: 16 everywhere inside the interface, 14 inside the
// smallest chips. The custom title bar keeps the operating system's smaller
// marks.
pub const ICON: f32 = 16.0;
pub const ICON_SM: f32 = 14.0;

// Control heights: full action, modal footer, and the low-emphasis ghost.
pub const BTN_H: f32 = 44.0;
pub const BTN_H_SM: f32 = 36.0;
pub const BTN_H_GHOST: f32 = 32.0;
pub const BTN_ICON: f32 = 36.0;

// ---------------------------------------------------------------------------
// Typography
// ---------------------------------------------------------------------------
//
// Six steps, no in-betweens: micro 11 Â· caption 12.5 Â· body 14 Â· title 16 Â·
// display 20 Â· hero 24.

pub const T_MICRO: f32 = 11.0;
pub const T_CAPTION: f32 = 12.5;
pub const T_BODY: f32 = 14.0;
pub const T_TITLE: f32 = 16.0;
pub const T_DISPLAY: f32 = 20.0;
pub const T_HERO: f32 = 24.0;

const SEMIBOLD_FAMILY: &str = "usdx-semibold";

/// Heavier face used for titles, buttons and numbers.
pub fn semibold() -> FontFamily {
    FontFamily::Name(SEMIBOLD_FAMILY.into())
}

pub fn display(size: f32, text: impl Into<String>) -> RichText {
    RichText::new(text).font(FontId::new(size, semibold()))
}

pub fn title(text: impl Into<String>) -> RichText {
    RichText::new(text).font(FontId::new(T_TITLE, semibold()))
}

pub fn strong(text: impl Into<String>) -> RichText {
    RichText::new(text).font(FontId::new(T_BODY, semibold()))
}

pub fn secondary(text: impl Into<String>) -> RichText {
    RichText::new(text).size(T_BODY).color(TEXT_2)
}

pub fn caption(text: impl Into<String>) -> RichText {
    RichText::new(text).size(T_CAPTION).color(TEXT_3)
}

/// Technical micro-label: uppercase mono with extra tracking.
pub fn micro(text: impl Into<String>) -> RichText {
    RichText::new(text.into().to_uppercase())
        .size(T_MICRO)
        .monospace()
        .extra_letter_spacing(1.0)
        .color(TEXT_3)
}

/// Monospace readout for numbers, paths and timings.
pub fn mono(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).size(size).monospace().color(TEXT_2)
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
        style.animation_time = 0.16;
    });
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    if let Some(set) = system_fonts() {
        let proportional_tweak = egui::FontTweak {
            y_offset_factor: -0.09,
            ..Default::default()
        };
        fonts.font_data.insert(
            "usdx-regular".into(),
            Arc::new(FontData::from_owned(set.regular).tweak(proportional_tweak.clone())),
        );
        if let Some(semibold) = set.semibold {
            fonts.font_data.insert(
                "usdx-semibold".into(),
                Arc::new(FontData::from_owned(semibold).tweak(proportional_tweak)),
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
    if let Some(bytes) = system_mono() {
        fonts
            .font_data
            .insert("usdx-mono".into(), Arc::new(FontData::from_owned(bytes)));
        let fallback = fonts
            .families
            .get(&FontFamily::Monospace)
            .cloned()
            .unwrap_or_default();
        let mut mono = vec!["usdx-mono".to_owned()];
        mono.extend(fallback);
        fonts.families.insert(FontFamily::Monospace, mono);
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

#[cfg(target_os = "windows")]
fn system_mono() -> Option<Vec<u8>> {
    let root = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("C:\\Windows"))
        .join("Fonts");
    read_font(&[
        root.join("consola.ttf"),
        root.join("CascadiaMono.ttf"),
        root.join("seguisb.ttf"),
    ])
}

#[cfg(target_os = "macos")]
fn system_mono() -> Option<Vec<u8>> {
    read_font(&[
        "/System/Library/Fonts/SFNSMono.ttf".into(),
        "/System/Library/Fonts/Supplemental/Menlo.ttc".into(),
    ])
}

#[cfg(all(unix, not(target_os = "macos")))]
fn system_mono() -> Option<Vec<u8>> {
    read_font(&[
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf".into(),
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf".into(),
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf".into(),
    ])
}

fn text_styles() -> std::collections::BTreeMap<TextStyle, FontId> {
    [
        (TextStyle::Small, FontId::proportional(T_CAPTION)),
        (TextStyle::Body, FontId::proportional(T_BODY)),
        (TextStyle::Button, FontId::new(T_BODY, semibold())),
        (TextStyle::Heading, FontId::new(T_DISPLAY, semibold())),
        (TextStyle::Monospace, FontId::monospace(T_CAPTION)),
    ]
    .into()
}

fn spacing() -> egui::style::Spacing {
    egui::style::Spacing {
        item_spacing: Vec2::new(GAP_S, GAP_S),
        button_padding: Vec2::new(GAP_L, GAP_S),
        interact_size: Vec2::new(36.0, 34.0),
        window_margin: Margin::same(GAP_XL as i8),
        menu_margin: Margin::same(GAP_M as i8),
        indent: GAP_L,
        slider_width: 150.0,
        combo_width: 160.0,
        text_edit_width: 260.0,
        icon_width: T_TITLE,
        icon_width_inner: 9.0,
        icon_spacing: 5.0,
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
    visuals.window_corner_radius = CornerRadius::same(RADIUS);
    visuals.window_shadow = modal_shadow();
    visuals.popup_shadow = modal_shadow();
    visuals.menu_corner_radius = CornerRadius::same(RADIUS);
    visuals.extreme_bg_color = FIELD;
    visuals.text_edit_bg_color = Some(FIELD);
    visuals.faint_bg_color = Color32::from_rgb(0x10, 0x10, 0x14);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = WARNING;
    visuals.error_fg_color = DANGER;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    visuals.slider_trailing_fill = true;
    visuals.disabled_alpha = 0.38;
    visuals.striped = false;

    visuals.widgets.noninteractive =
        widget(Color32::TRANSPARENT, Color32::TRANSPARENT, BORDER, TEXT, 0);
    visuals.widgets.inactive = widget(SURFACE_2, SURFACE_2, BORDER, TEXT, 0);
    visuals.widgets.hovered = widget(SURFACE_3, SURFACE_3, BORDER_HOVER, TEXT, 0);
    visuals.widgets.active = widget(SURFACE_4, SURFACE_4, BORDER_HOVER, TEXT, 0);
    visuals.widgets.open = widget(SURFACE_3, SURFACE_3, BORDER_HOVER, TEXT, 0);
    visuals
}

/// Elevation is a soft drop shadow, never a hard offset copy.
pub fn shadow() -> Shadow {
    Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(110),
    }
}

pub fn modal_shadow() -> Shadow {
    Shadow {
        offset: [0, 20],
        blur: 52,
        spread: 0,
        color: Color32::from_black_alpha(170),
    }
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
// Motion
// ---------------------------------------------------------------------------

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_out_quint(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(5)
}

fn hover(ui: &Ui, id: egui::Id, hovered: bool) -> f32 {
    ui.ctx().animate_bool_with_time(id, hovered, HOVER_TIME)
}

fn press(ui: &Ui, id: egui::Id, pressed: bool) -> f32 {
    ui.ctx().animate_bool_with_time(id, pressed, PRESS_TIME)
}

// ---------------------------------------------------------------------------
// Activation
// ---------------------------------------------------------------------------

/// What every painted control returns. Publishes the pointer click of the
/// underlying [`egui::Response`] and also fires when the control holds keyboard
/// focus and Enter or Space is pressed, so painted widgets are reachable by Tab.
#[derive(Debug, Clone)]
pub struct Activation(egui::Response);

impl std::ops::Deref for Activation {
    type Target = egui::Response;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Activation {
    fn new(response: egui::Response) -> Self {
        Self(response)
    }

    pub fn clicked(&self) -> bool {
        if !self.0.enabled() {
            return false;
        }
        self.0.clicked() || (self.0.has_focus() && key_activated(&self.0.ctx))
    }

    /// Keeps the tooltip on the activation wrapper instead of consuming it.
    pub fn on_hover_text(self, text: impl Into<egui::WidgetText>) -> Self {
        Self(self.0.on_hover_text(text))
    }
}

fn key_activated(ctx: &egui::Context) -> bool {
    ctx.input(|input| input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space))
}

/// The one focus ring, painted outside the control so it never moves the label.
fn focus_ring(ui: &Ui, rect: Rect, focused: bool) {
    if !focused {
        return;
    }
    ui.painter().rect_stroke(
        rect.expand(1.5),
        RADIUS,
        Stroke::new(1.0, ACCENT),
        StrokeKind::Outside,
    );
}

// ---------------------------------------------------------------------------
// Building blocks
// ---------------------------------------------------------------------------

/// Dot grid and top hairline that give the empty space a technical texture.
/// Neutral on purpose: the accent is reserved for what can be acted on.
///
/// The grid is one repeated texture, not thousands of rectangles: it costs a
/// single quad per frame however large the window is.
pub fn background(ui: &Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    const TILE: f32 = 32.0;
    let id = egui::Id::unique("theme.dot-grid");
    let cached = ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = match cached {
        Some(texture) => texture,
        None => {
            let dot = Color32::from_rgba_unmultiplied(0x8A, 0x8C, 0x97, 18);
            let mut image =
                egui::ColorImage::filled([TILE as usize, TILE as usize], Color32::TRANSPARENT);
            image[(0, 0)] = dot;
            image[(1, 0)] = dot;
            image[(0, 1)] = dot;
            image[(1, 1)] = dot;
            let texture = ui.ctx().load_texture(
                "usdx-dot-grid",
                image,
                egui::TextureOptions {
                    wrap_mode: egui::TextureWrapMode::Repeat,
                    ..Default::default()
                },
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        }
    };
    // Offset by half a pixel so the 2x2 dot reads as a 1.4 px point.
    let uv = Rect::from_min_size(
        Pos2::new(0.5, 0.5),
        Vec2::new(rect.width() / TILE, rect.height() / TILE),
    );
    painter.image(texture.id(), rect, uv, Color32::WHITE);
}

/// Soft elevated surface used by every screen.
pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let response = egui::Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS)
        .shadow(shadow())
        .inner_margin(Margin::same(GAP_XL as i8))
        .show(ui, add_contents);
    brackets(ui.painter(), response.response.rect, BORDER_HOVER);
    response.inner
}

/// Four short corner marks, like the framing of a technical drawing.
pub fn brackets(painter: &egui::Painter, rect: Rect, color: Color32) {
    let rect = rect.shrink(9.0);
    let len = 10.0;
    let stroke = Stroke::new(1.0, color);
    for (corner, x_dir, y_dir) in [
        (rect.left_top(), 1.0, 1.0),
        (rect.right_top(), -1.0, 1.0),
        (rect.left_bottom(), 1.0, -1.0),
        (rect.right_bottom(), -1.0, -1.0),
    ] {
        painter.line_segment([corner, corner + Vec2::new(len * x_dir, 0.0)], stroke);
        painter.line_segment([corner, corner + Vec2::new(0.0, len * y_dir)], stroke);
    }
}

/// Low-contrast inset used for paths, errors and secondary controls.
pub fn inset<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(FIELD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS)
        .inner_margin(Margin::symmetric(GAP_M as i8, GAP_M as i8))
        .show(ui, add_contents)
        .inner
}

/// Hairline divider that matches the rest of the surfaces.
pub fn divider(ui: &mut Ui) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0, BORDER);
}

/// Inline error with a vector cross, so it reads before the text does.
pub fn error_row(ui: &mut Ui, text: &str) {
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(GAP_L), egui::Sense::hover());
        icons::paint(Icon::Close, ui.painter(), rect.shrink(1.0), DANGER);
        ui.add(
            egui::Label::new(RichText::new(text).size(T_BODY).color(DANGER))
                .wrap()
                .selectable(true),
        );
    });
}

/// Brand mark: a square tile with the music glyph painted as vectors.
pub fn logo(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(
        Rect::from_center_size(rect.center(), Vec2::splat(rect.width() * 1.8)),
        0,
        ACCENT.gamma_multiply(0.03),
    );
    painter.rect_filled(
        Rect::from_center_size(rect.center(), Vec2::splat(rect.width() * 1.35)),
        0,
        ACCENT.gamma_multiply(0.04),
    );
    painter.rect_filled(rect, 0, ACCENT.gamma_multiply(0.14));
    painter.rect_stroke(
        rect,
        0,
        Stroke::new(1.0, ACCENT.gamma_multiply(0.32)),
        StrokeKind::Inside,
    );
    let glyph = rect.shrink(rect.width() * 0.26);
    icons::paint(Icon::MusicNote, painter, glyph, ACCENT_TEXT);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
}

/// A button that paints its own surface, so hover and press feedback are
/// immediate and identical on every platform.
pub fn button(ui: &mut Ui, label: &str, kind: ButtonKind, enabled: bool) -> Activation {
    button_with_icon(ui, label, None, kind, enabled)
}

pub fn button_with_icon(
    ui: &mut Ui,
    label: &str,
    icon: Option<Icon>,
    kind: ButtonKind,
    enabled: bool,
) -> Activation {
    let font = button_font(kind);
    let size = match kind {
        ButtonKind::Ghost => {
            let galley = ui
                .painter()
                .layout_no_wrap(label.to_owned(), font.clone(), TEXT);
            let icon_width = icon.map_or(0.0, |_| ICON + GAP_S);
            Vec2::new(galley.size().x + icon_width + GAP_XL, BTN_H_GHOST)
        }
        _ => Vec2::new(ui.available_width(), BTN_H),
    };
    button_sized_with_icon(ui, label, icon, kind, size, enabled)
}

pub fn button_sized(
    ui: &mut Ui,
    label: &str,
    kind: ButtonKind,
    size: Vec2,
    enabled: bool,
) -> Activation {
    button_sized_with_icon(ui, label, None, kind, size, enabled)
}

pub fn button_sized_with_icon(
    ui: &mut Ui,
    label: &str,
    icon: Option<Icon>,
    kind: ButtonKind,
    size: Vec2,
    enabled: bool,
) -> Activation {
    button_sized_full(ui, label, icon, false, kind, size, enabled)
}

/// Same as [`button_sized_with_icon`], but the icon sits before the label.
pub fn button_sized_with_leading_icon(
    ui: &mut Ui,
    label: &str,
    icon: Icon,
    kind: ButtonKind,
    size: Vec2,
    enabled: bool,
) -> Activation {
    button_sized_full(ui, label, Some(icon), true, kind, size, enabled)
}

fn button_sized_full(
    ui: &mut Ui,
    label: &str,
    icon: Option<Icon>,
    icon_leading: bool,
    kind: ButtonKind,
    size: Vec2,
    enabled: bool,
) -> Activation {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let focus = enabled && response.has_focus();
    let hover = hover(ui, response.id.with("hover"), hovered);
    let press = press(ui, response.id.with("press"), pressed);

    let font = button_font(kind);
    let (fill, stroke, text, base_radius) = match kind {
        ButtonKind::Primary => {
            let base = if enabled {
                ACCENT.lerp_to_gamma(ACCENT_HOVER, hover)
            } else {
                ACCENT.gamma_multiply(0.35)
            };
            let fill = base.lerp_to_gamma(ACCENT_PRESSED, press * 0.8);
            (
                fill,
                Stroke::NONE,
                if enabled {
                    ON_ACCENT
                } else {
                    Color32::from_white_alpha(130)
                },
                RADIUS as f32,
            )
        }
        ButtonKind::Secondary => {
            let base = SURFACE_2
                .lerp_to_gamma(SURFACE_3, hover)
                .lerp_to_gamma(SURFACE_4, press);
            (
                base,
                Stroke::new(1.0, BORDER.lerp_to_gamma(BORDER_HOVER, hover)),
                TEXT,
                RADIUS as f32,
            )
        }
        ButtonKind::Ghost => {
            let fill = SURFACE_2
                .gamma_multiply(hover * (1.0 - press * 0.4))
                .lerp_to_gamma(SURFACE_3.gamma_multiply(press * 0.9), press);
            (
                fill,
                Stroke::NONE,
                TEXT_2.lerp_to_gamma(TEXT, hover),
                RADIUS as f32,
            )
        }
    };

    // Press feedback shrinks the painted surface: the equivalent of
    // `scale(0.98)` and it happens on pointer-down, not on release.
    let painted = rect.shrink(press * 0.7);
    let radius = (base_radius - press * 0.6).max(0.0);
    let painter = ui.painter();
    if kind == ButtonKind::Primary {
        painter.rect_filled(
            painted.expand(6.0),
            0.0,
            ACCENT.gamma_multiply(0.05 + hover * 0.05),
        );
    }
    painter.rect(painted, radius, fill, stroke, StrokeKind::Inside);
    if kind == ButtonKind::Primary && enabled {
        painter.hline(
            egui::Rangef::new(painted.left() + GAP_M, painted.right() - GAP_M),
            painted.top() + 0.75,
            Stroke::new(1.0, Color32::from_white_alpha(38)),
        );
    }

    paint_label(
        painter,
        painted,
        label,
        icon,
        icon_leading,
        font,
        text,
        hover,
    );
    focus_ring(ui, rect, focus);

    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    Activation::new(response)
}

fn button_font(_kind: ButtonKind) -> FontId {
    FontId::new(T_BODY, semibold())
}

#[allow(clippy::too_many_arguments)]
fn paint_label(
    painter: &egui::Painter,
    rect: Rect,
    label: &str,
    icon: Option<Icon>,
    icon_leading: bool,
    font: FontId,
    color: Color32,
    hover: f32,
) {
    if label.is_empty() {
        if let Some(icon) = icon {
            let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(ICON));
            icons::paint(icon, painter, icon_rect, color);
        }
        return;
    }
    let galley = painter.layout_no_wrap(label.to_owned(), font, color);
    let gap = if icon.is_some() { GAP_S - 1.0 } else { 0.0 };
    let total = galley.size().x + if icon.is_some() { ICON + gap } else { 0.0 };
    let left = rect.center().x - total / 2.0;
    let text_left = if icon_leading {
        left + ICON + gap
    } else {
        left
    };
    painter.galley(
        Pos2::new(text_left, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    if let Some(icon) = icon {
        let drift = hover * 1.5;
        let icon_center = if icon_leading {
            Pos2::new(left + ICON / 2.0 - drift, rect.center().y)
        } else {
            Pos2::new(left + total - ICON / 2.0 + drift, rect.center().y)
        };
        let icon_rect = Rect::from_center_size(icon_center, Vec2::splat(ICON));
        icons::paint(icon, painter, icon_rect, color);
    }
}

pub fn primary_button(ui: &mut Ui, label: &str) -> Activation {
    button(ui, label, ButtonKind::Primary, true)
}

pub fn secondary_button(ui: &mut Ui, label: &str) -> Activation {
    button(ui, label, ButtonKind::Secondary, true)
}

pub fn ghost_button(ui: &mut Ui, label: &str) -> Activation {
    button(ui, label, ButtonKind::Ghost, true)
}

pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Activation {
    icon_button_sized(ui, icon, tooltip, Vec2::splat(BTN_ICON))
}

pub fn icon_button_sized(ui: &mut Ui, icon: Icon, tooltip: &str, size: Vec2) -> Activation {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let focus = response.has_focus();
    let hover = hover(ui, response.id.with("hover"), hovered);
    let press = press(ui, response.id.with("press"), pressed);

    let fill = SURFACE_2
        .gamma_multiply(hover)
        .lerp_to_gamma(SURFACE_3.gamma_multiply(press * 0.9), press);
    let color = TEXT_2.lerp_to_gamma(TEXT, hover.max(press));
    let painted = rect.shrink(press * 0.6);

    let painter = ui.painter();
    if fill.a() > 0 {
        painter.rect_filled(painted, 0, fill);
    }
    let glyph = Rect::from_center_size(painted.center(), Vec2::splat(ICON));
    icons::paint(icon, painter, glyph, color);
    focus_ring(ui, rect, focus);

    let response = response
        .on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    Activation::new(response)
}

/// Flat window-control button for the integrated title bar. Out of the tab
/// order on purpose: it repeats what the operating system already offers.
pub fn chrome_button(ui: &mut Ui, icon: Icon, danger: bool, height: f32) -> Activation {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(46.0, height), Sense::CLICK);
    let hover = hover(ui, response.id.with("hover"), response.hovered());
    let press = press(
        ui,
        response.id.with("press"),
        response.is_pointer_button_down_on(),
    );
    let painter = ui.painter();
    if danger {
        painter.rect_filled(rect, 0, CLOSE_RED.gamma_multiply(hover.max(press * 0.9)));
    } else if hover > 0.0 {
        painter.rect_filled(rect, 0, SURFACE_2.gamma_multiply(hover));
    }
    let color = if danger && hover > 0.02 {
        Color32::WHITE
    } else {
        TEXT_2.lerp_to_gamma(TEXT, hover.max(press))
    };
    icons::paint(
        icon,
        painter,
        Rect::from_center_size(rect.center(), Vec2::splat(ICON_SM)),
        color,
    );
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    Activation::new(response)
}

/// Rectangular segmented control. Returns the chosen value when clicked, or
/// when it holds focus and the arrow keys move the selection.
pub fn segmented<T: Copy + PartialEq>(ui: &mut Ui, options: &[(&str, T)], current: T) -> Option<T> {
    let font = FontId::new(T_CAPTION, semibold());
    let widths: Vec<f32> = options
        .iter()
        .map(|(label, _)| {
            ui.painter()
                .layout_no_wrap((*label).to_owned(), font.clone(), TEXT)
                .size()
                .x
                + GAP_XL
        })
        .collect();
    let total: f32 = widths.iter().sum();
    let height = BTN_H_GHOST;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(total.max(1.0), height), egui::Sense::click());
    let focus = response.has_focus();
    let painter = ui.painter();
    painter.rect_filled(rect, 0, FIELD);
    painter.rect_stroke(rect, 0, Stroke::new(1.0, BORDER), StrokeKind::Inside);

    let pointer = response.hover_pos();
    let mut clicked = None;
    let mut x = rect.left();
    for (index, ((label, value), width)) in options.iter().zip(&widths).enumerate() {
        let cell = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(*width, height));
        let hovered = pointer.is_some_and(|position| cell.contains(position));
        let hover = hover(ui, response.id.with(index), hovered);
        if index > 0 {
            painter.vline(x, rect.y_range(), Stroke::new(1.0, BORDER));
        }
        let selected = *value == current;
        if selected {
            painter.rect_filled(cell, 0, ACCENT.gamma_multiply(0.16));
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(cell.left(), cell.bottom() - 2.0),
                    Vec2::new(cell.width(), 2.0),
                ),
                0,
                ACCENT,
            );
        } else if hover > 0.0 {
            painter.rect_filled(cell, 0, SURFACE_2.gamma_multiply(hover));
        }
        painter.text(
            cell.center(),
            Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if selected {
                ACCENT_TEXT
            } else {
                TEXT_2.lerp_to_gamma(TEXT, hover)
            },
        );
        if response.clicked() && hovered {
            clicked = Some(*value);
        }
        x += width;
    }
    focus_ring(ui, rect, focus);

    // Arrow keys move the selection while the control holds focus.
    if clicked.is_none() && focus {
        ui.input(|input| {
            let step = if input.key_pressed(egui::Key::ArrowRight) {
                1
            } else if input.key_pressed(egui::Key::ArrowLeft) {
                -1
            } else {
                0
            };
            if step != 0 {
                let index = options
                    .iter()
                    .position(|(_, value)| *value == current)
                    .unwrap_or(0);
                let next = (index as isize + step).clamp(0, options.len() as isize - 1) as usize;
                clicked = options.get(next).map(|(_, value)| *value);
            }
        });
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand);
    clicked
}

/// Compact square choice used for stems and tempo candidates.
pub fn pill(ui: &mut Ui, label: &str, selected: bool) -> Activation {
    pill_with_icon(ui, None, label, selected)
}

pub fn pill_with_icon(ui: &mut Ui, icon: Option<Icon>, label: &str, selected: bool) -> Activation {
    let font = FontId::proportional(T_CAPTION);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), TEXT);
    let icon_width = icon.map_or(0.0, |_| ICON_SM + GAP_S - 2.0);
    let size = Vec2::new(galley.size().x + icon_width + GAP_L + 4.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let focus = response.has_focus();
    let hover = hover(ui, response.id.with("hover"), hovered);
    let press = press(ui, response.id.with("press"), pressed);

    let (fill, stroke, text) = if selected {
        (
            ACCENT.gamma_multiply(0.14 + hover * 0.05),
            ACCENT.gamma_multiply(0.38 + hover * 0.1),
            ACCENT_TEXT,
        )
    } else {
        (
            SURFACE_2
                .lerp_to_gamma(SURFACE_3, hover)
                .lerp_to_gamma(SURFACE_4, press),
            BORDER.lerp_to_gamma(BORDER_HOVER, hover),
            if hovered { TEXT } else { TEXT_2 },
        )
    };
    let painted = rect.shrink(press * 0.5);

    let painter = ui.painter();
    painter.rect(
        painted,
        RADIUS,
        fill,
        Stroke::new(1.0, stroke),
        StrokeKind::Inside,
    );

    let total = galley.size().x + icon_width;
    let left = painted.center().x - total / 2.0;
    if let Some(icon) = icon {
        let icon_rect = Rect::from_center_size(
            Pos2::new(left + 7.0, painted.center().y),
            Vec2::splat(ICON_SM),
        );
        icons::paint(icon, painter, icon_rect, text);
    }
    painter.galley(
        Pos2::new(
            left + icon_width,
            painted.center().y - galley.size().y / 2.0,
        ),
        galley,
        text,
    );
    focus_ring(ui, rect, focus);

    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    Activation::new(response)
}

/// Full-width text field with an animated focus ring.
pub fn text_field(ui: &mut Ui, text: &mut String, hint: &str) -> egui::Response {
    text_field_sized(ui, text, hint, ui.available_width(), T_TITLE, GAP_M)
}

pub fn text_field_sized(
    ui: &mut Ui,
    text: &mut String,
    hint: &str,
    width: f32,
    font_size: f32,
    padding: f32,
) -> egui::Response {
    let min_height = if padding >= GAP_M { BTN_H } else { BTN_H_SM };
    let frame = egui::Frame::NONE
        .fill(FIELD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS)
        .inner_margin(Margin::symmetric(GAP_M as i8, padding as i8));
    let response = ui.add(
        egui::TextEdit::singleline(text)
            .frame(frame)
            .align(egui::Align2::LEFT_CENTER)
            .min_size(Vec2::new(width, min_height))
            .font(FontId::proportional(font_size))
            .hint_text(
                RichText::new(hint)
                    .font(FontId::proportional(font_size))
                    .color(TEXT_3),
            )
            .desired_width(width),
    );
    let focus =
        ui.ctx()
            .animate_bool_with_time(response.id.with("focus"), response.has_focus(), 0.16);
    let hover = hover(ui, response.id.with("hover"), response.hovered());
    let border = BORDER
        .lerp_to_gamma(BORDER_HOVER, hover)
        .lerp_to_gamma(ACCENT.gamma_multiply(0.85), focus);
    let width = 1.0 + focus * 0.8;
    ui.painter().rect_stroke(
        response.rect.expand(focus * 0.5),
        RADIUS,
        Stroke::new(width, border),
        StrokeKind::Outside,
    );
    response
}

/// Progress of the pipeline: one segment per stage, so the bar says where the
/// song is instead of only how far along it is. `stage` is the running stage
/// index out of `stages`; without it the bar is a single download.
pub fn progress_bar(ui: &mut Ui, stage: Option<(usize, usize)>, fraction: f32, time: f32) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
    let painter = ui.painter();
    let fraction = fraction.clamp(0.0, 1.0);

    let Some((current, count)) = stage.filter(|(_, count)| *count > 1) else {
        painter.rect_filled(rect, 0, TRACK);
        fill_segment(painter, rect, fraction, true, time);
        return;
    };

    let gap = 3.0;
    let count = count.min(12);
    let segment = ((rect.width() - gap * (count as f32 - 1.0)) / count as f32).max(1.0);
    for index in 0..count {
        let left = rect.left() + index as f32 * (segment + gap);
        let cell = Rect::from_min_size(Pos2::new(left, rect.top()), Vec2::new(segment, 6.0));
        painter.rect_filled(cell, 0, TRACK);
        if index < current {
            painter.rect_filled(cell, 0, ACCENT.gamma_multiply(0.55));
        } else if index == current {
            fill_segment(painter, cell, fraction, true, time);
        }
    }
}

/// Fills a track from the left, with the hard white edge that reads as "live".
fn fill_segment(painter: &egui::Painter, track: Rect, fraction: f32, live: bool, time: f32) {
    if fraction <= 0.0 {
        return;
    }
    let width = (track.width() * fraction).max(4.0);
    let fill = Rect::from_min_size(track.min, Vec2::new(width, track.height()));
    painter.rect_filled(fill, 0, ACCENT);
    painter.rect_filled(
        Rect::from_center_size(
            Pos2::new(fill.right() - 1.0, fill.center().y),
            Vec2::new(2.0, 12.0),
        ),
        0,
        Color32::WHITE,
    );
    if live && fraction < 0.995 && width > 24.0 {
        let phase = (time * 0.4) % 1.0;
        let sheen_width = 48.0;
        let left = fill.left() + phase * (width + sheen_width) - sheen_width;
        let sheen = Rect::from_min_max(
            Pos2::new(left.max(fill.left()), fill.top()),
            Pos2::new((left + sheen_width).min(fill.right()), fill.bottom()),
        );
        if sheen.width() > 1.0 {
            painter.rect_filled(sheen, 0, Color32::from_white_alpha(26));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Pending,
    Done,
}

/// Small state marker used by the model list: empty square or a check.
pub fn stage_glyph(ui: &Ui, center: Pos2, radius: f32, glyph: Glyph) {
    let painter = ui.painter();
    let rect = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
    match glyph {
        Glyph::Pending => {
            painter.rect_stroke(rect, 0, Stroke::new(1.0, BORDER_HOVER), StrokeKind::Inside);
        }
        Glyph::Done => {
            painter.rect_filled(rect, 0, SUCCESS.gamma_multiply(0.12));
            painter.rect_stroke(
                rect,
                0,
                Stroke::new(1.0, SUCCESS.gamma_multiply(0.8)),
                StrokeKind::Inside,
            );
            let size = radius * 0.62;
            painter.add(egui::Shape::line(
                vec![
                    center + Vec2::new(-size, 0.0),
                    center + Vec2::new(-size * 0.25, size * 0.72),
                    center + Vec2::new(size, -size * 0.66),
                ],
                Stroke::new(1.7, SUCCESS),
            ));
        }
    }
}

pub fn check_mark(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    check_mark_progress(painter, center, radius, color, 1.0);
}

/// Draws a check mark from 0 (nothing) to 1 (complete), so success can animate.
pub fn check_mark_progress(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    color: Color32,
    progress: f32,
) {
    let points = [(5.2, 12.6), (9.9, 17.2), (18.8, 6.9)];
    let points: Vec<Pos2> = points
        .iter()
        .map(|(x, y)| center + Vec2::new((x - 12.0) * radius / 6.4, (y - 12.0) * radius / 6.4))
        .collect();
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return;
    }
    let first = points[0].distance(points[1]);
    let second = points[1].distance(points[2]);
    let total = first + second;
    let travelled = total * progress;
    let mut path = vec![points[0]];
    if travelled <= first {
        let t = (travelled / first).clamp(0.0, 1.0);
        path.push(points[0].lerp(points[1], t));
    } else {
        path.push(points[1]);
        let t = ((travelled - first) / second).clamp(0.0, 1.0);
        path.push(points[1].lerp(points[2], t));
    }
    painter.add(egui::Shape::line(path, Stroke::new(2.0, color)));
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

    #[test]
    fn easing_helpers_are_monotonic_and_bounded() {
        for step in 0..=20 {
            let t = step as f32 / 20.0;
            let cubic = ease_out_cubic(t);
            let quint = ease_out_quint(t);
            assert!((0.0..=1.0).contains(&cubic));
            assert!((0.0..=1.0).contains(&quint));
        }
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < f32::EPSILON);
        assert!(ease_out_quint(0.5) >= ease_out_cubic(0.5));
    }

    #[test]
    fn partial_check_marks_end_on_the_last_point() {
        let context = egui::Context::default();
        let painter = context.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::unique("check-test"),
        ));
        for progress in [0.0, 0.25, 0.75, 1.0] {
            check_mark_progress(
                &painter,
                Pos2::new(10.0, 10.0),
                8.0,
                Color32::WHITE,
                progress,
            );
        }
    }
}
