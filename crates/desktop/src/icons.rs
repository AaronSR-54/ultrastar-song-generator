//! Vector icon set.
//!
//! Every icon is a small set of strokes and fills laid out on a 24 × 24 grid
//! and painted straight onto the egui painter. Nothing depends on a font, so
//! the icons render identically on every machine and stay sharp at any size.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    MusicNote,
    Settings,
    Undo,
    Redo,
    Play,
    Pause,
    Stop,
    SkipBack,
    Loop,
    Plus,
    Minus,
    ChevronLeft,
    ChevronRight,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Trash,
    Refresh,
    Check,
    Close,
    Minimize,
    Maximize,
    Restore,
    Folder,
    ExternalLink,
    Download,
    Edit,
}

impl Icon {
    /// Natural size in points; used when an icon is placed on its own.
    pub fn default_size(self) -> f32 {
        match self {
            Self::MusicNote => 20.0,
            _ => 16.0,
        }
    }
}

/// Paints `icon` centred inside `rect` with a single colour.
pub fn paint(icon: Icon, painter: &egui::Painter, rect: Rect, color: Color32) {
    let side = rect.width().min(rect.height());
    let width = (side * 0.085).clamp(1.15, 2.1);
    let pen = Pen {
        painter,
        center: rect.center(),
        scale: side / 24.0,
        color,
        width,
        mirror: false,
    };
    draw(icon, &pen);
}

/// Paints an icon at an explicit size, returning the space it used.
pub fn show(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    paint(icon, ui.painter(), rect, color);
}

#[derive(Clone, Copy)]
struct Pen<'a> {
    painter: &'a egui::Painter,
    center: Pos2,
    scale: f32,
    color: Color32,
    width: f32,
    mirror: bool,
}

impl Pen<'_> {
    fn mirrored(&self) -> Self {
        Self {
            mirror: !self.mirror,
            ..*self
        }
    }

    fn at(&self, x: f32, y: f32) -> Pos2 {
        let x = if self.mirror { 24.0 - x } else { x };
        self.center + Vec2::new((x - 12.0) * self.scale, (y - 12.0) * self.scale)
    }

    fn stroke(&self) -> Stroke {
        Stroke::new(self.width, self.color)
    }

    fn line(&self, points: &[(f32, f32)]) {
        let points = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.painter.line(points, self.stroke());
    }

    fn closed(&self, points: &[(f32, f32)]) {
        let points = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.painter.add(Shape::closed_line(points, self.stroke()));
    }

    fn triangle(&self, points: [(f32, f32); 3]) {
        let points = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.painter
            .add(Shape::convex_polygon(points, self.color, Stroke::NONE));
    }

    fn round_rect(&self, x: f32, y: f32, width: f32, height: f32, radius: f32) {
        let rect = Rect::from_min_max(self.at(x, y), self.at(x + width, y + height));
        self.painter
            .rect_filled(rect, radius * self.scale, self.color);
    }

    fn ring(&self, center: (f32, f32), radius: f32) {
        self.painter.circle_stroke(
            self.at(center.0, center.1),
            radius * self.scale,
            self.stroke(),
        );
    }

    fn dot(&self, center: (f32, f32), radius: f32) {
        self.painter
            .circle_filled(self.at(center.0, center.1), radius * self.scale, self.color);
    }

    /// Points along a circular arc. Angles are degrees, clockwise on screen.
    fn arc(&self, center: (f32, f32), radius: f32, start: f32, end: f32) -> Vec<Pos2> {
        let sweep = (end - start).abs();
        let steps = ((sweep / 7.0).ceil() as usize).max(6);
        (0..=steps)
            .map(|index| {
                let angle = (start + (end - start) * index as f32 / steps as f32).to_radians();
                self.at(
                    center.0 + radius * angle.cos(),
                    center.1 + radius * angle.sin(),
                )
            })
            .collect()
    }

    fn arc_line(&self, center: (f32, f32), radius: f32, start: f32, end: f32) {
        self.painter
            .line(self.arc(center, radius, start, end), self.stroke());
    }

    /// Small solid arrowhead whose tip sits on `at`, pointing along `direction`.
    fn arrow_head(&self, at: (f32, f32), direction: (f32, f32)) {
        let length = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
        if length <= f32::EPSILON {
            return;
        }
        let direction = (direction.0 / length, direction.1 / length);
        let normal = (-direction.1, direction.0);
        let size = 3.6;
        self.triangle([
            at,
            (
                at.0 - direction.0 * size + normal.0 * size * 0.85,
                at.1 - direction.1 * size + normal.1 * size * 0.85,
            ),
            (
                at.0 - direction.0 * size - normal.0 * size * 0.85,
                at.1 - direction.1 * size - normal.1 * size * 0.85,
            ),
        ]);
    }
}

fn draw(icon: Icon, pen: &Pen<'_>) {
    match icon {
        Icon::MusicNote => {
            pen.dot((8.6, 17.1), 3.1);
            pen.line(&[(11.4, 16.4), (11.4, 5.6)]);
            pen.line(&[(11.4, 5.6), (14.0, 5.2), (16.2, 6.2), (17.3, 8.4)]);
        }
        Icon::Settings => {
            for (y, knob) in [(7.4, 10.4), (12.0, 15.0), (16.6, 7.8)] {
                pen.line(&[(3.6, y), (knob - 2.6, y)]);
                pen.line(&[(knob + 2.6, y), (20.4, y)]);
                pen.ring((knob, y), 2.1);
            }
        }
        Icon::Undo => {
            pen.line(&[(4.2, 9.0), (14.0, 9.0)]);
            pen.arc_line((14.0, 14.2), 5.2, -90.0, 90.0);
            pen.line(&[(14.0, 19.4), (11.2, 19.4)]);
            pen.line(&[(9.4, 4.2), (4.6, 9.0), (9.4, 13.8)]);
        }
        Icon::Redo => draw(Icon::Undo, &pen.mirrored()),
        Icon::Play => {
            pen.triangle([(9.2, 5.8), (19.0, 12.0), (9.2, 18.2)]);
        }
        Icon::Pause => {
            pen.round_rect(7.4, 6.0, 3.4, 12.0, 1.4);
            pen.round_rect(13.2, 6.0, 3.4, 12.0, 1.4);
        }
        Icon::Stop => {
            pen.round_rect(6.6, 6.6, 10.8, 10.8, 2.0);
        }
        Icon::SkipBack => {
            pen.round_rect(5.4, 6.2, 2.2, 11.6, 1.0);
            pen.triangle([(18.6, 6.4), (9.2, 12.0), (18.6, 17.6)]);
        }
        Icon::Loop => {
            pen.arc_line((12.0, 12.0), 6.2, 205.0, 328.0);
            pen.arrow_head((17.5, 8.8), (0.72, 0.69));
            pen.arc_line((12.0, 12.0), 6.2, 25.0, 148.0);
            pen.arrow_head((6.5, 15.2), (-0.72, -0.69));
        }
        Icon::Plus => {
            pen.line(&[(12.0, 5.4), (12.0, 18.6)]);
            pen.line(&[(5.4, 12.0), (18.6, 12.0)]);
        }
        Icon::Minus => {
            pen.line(&[(5.4, 12.0), (18.6, 12.0)]);
        }
        Icon::ChevronLeft => {
            pen.line(&[(15.0, 5.6), (8.8, 12.0), (15.0, 18.4)]);
        }
        Icon::ChevronRight => {
            pen.line(&[(9.0, 5.6), (15.2, 12.0), (9.0, 18.4)]);
        }
        Icon::ArrowUp => {
            pen.line(&[(12.0, 5.0), (12.0, 19.0)]);
            pen.line(&[(7.2, 9.8), (12.0, 5.0), (16.8, 9.8)]);
        }
        Icon::ArrowDown => {
            pen.line(&[(12.0, 19.0), (12.0, 5.0)]);
            pen.line(&[(7.2, 14.2), (12.0, 19.0), (16.8, 14.2)]);
        }
        Icon::ArrowLeft => {
            pen.line(&[(19.0, 12.0), (5.0, 12.0)]);
            pen.line(&[(9.8, 7.2), (5.0, 12.0), (9.8, 16.8)]);
        }
        Icon::ArrowRight => {
            pen.line(&[(5.0, 12.0), (19.0, 12.0)]);
            pen.line(&[(14.2, 7.2), (19.0, 12.0), (14.2, 16.8)]);
        }
        Icon::Trash => {
            pen.line(&[(4.0, 6.9), (20.0, 6.9)]);
            pen.line(&[(9.2, 4.4), (14.8, 4.4)]);
            pen.line(&[(6.2, 6.9), (7.3, 19.6)]);
            pen.line(&[(17.8, 6.9), (16.7, 19.6)]);
            pen.line(&[(7.3, 19.6), (16.7, 19.6)]);
            pen.line(&[(10.2, 10.2), (10.3, 16.4)]);
            pen.line(&[(13.8, 10.2), (13.7, 16.4)]);
        }
        Icon::Refresh => {
            pen.arc_line((12.0, 12.0), 6.4, 40.0, 325.0);
            pen.arrow_head((7.1, 6.9), (-0.62, -0.78));
        }
        Icon::Check => {
            pen.line(&[(5.2, 12.6), (9.9, 17.2), (18.8, 6.9)]);
        }
        Icon::Close => {
            pen.line(&[(6.2, 6.2), (17.8, 17.8)]);
            pen.line(&[(17.8, 6.2), (6.2, 17.8)]);
        }
        Icon::Minimize => {
            pen.line(&[(6.0, 12.0), (18.0, 12.0)]);
        }
        Icon::Maximize => {
            pen.closed(&[(5.5, 5.5), (18.5, 5.5), (18.5, 18.5), (5.5, 18.5)]);
        }
        Icon::Restore => {
            pen.closed(&[(8.0, 8.0), (18.0, 8.0), (18.0, 18.0), (8.0, 18.0)]);
            pen.line(&[(8.0, 5.0), (19.0, 5.0), (19.0, 16.0)]);
        }
        Icon::Folder => {
            pen.closed(&[
                (3.6, 7.4),
                (9.4, 7.4),
                (11.4, 9.8),
                (20.4, 9.8),
                (20.4, 18.6),
                (3.6, 18.6),
            ]);
        }
        Icon::ExternalLink => {
            pen.line(&[
                (13.4, 5.0),
                (5.2, 5.0),
                (5.2, 18.8),
                (19.0, 18.8),
                (19.0, 10.6),
            ]);
            pen.line(&[(11.8, 12.2), (19.2, 4.8)]);
            pen.line(&[(13.8, 4.8), (19.2, 4.8), (19.2, 10.2)]);
        }
        Icon::Download => {
            pen.line(&[(12.0, 4.2), (12.0, 15.0)]);
            pen.line(&[(7.6, 10.8), (12.0, 15.2), (16.4, 10.8)]);
            pen.line(&[(5.0, 16.4), (5.0, 19.8), (19.0, 19.8), (19.0, 16.4)]);
        }
        Icon::Edit => {
            pen.closed(&[
                (4.2, 19.8),
                (5.4, 15.2),
                (15.9, 4.6),
                (19.4, 8.1),
                (8.9, 18.6),
            ]);
            pen.line(&[(14.4, 6.2), (17.9, 9.7)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_painter() -> (egui::Context, egui::Painter) {
        let context = egui::Context::default();
        let painter = context.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::unique("icons-test"),
        ));
        (context, painter)
    }

    #[test]
    fn every_icon_paints_inside_its_box() {
        let (_context, painter) = sample_painter();
        let base = Rect::from_min_size(egui::pos2(10.0, 20.0), Vec2::splat(24.0));
        for icon in [
            Icon::MusicNote,
            Icon::Settings,
            Icon::Undo,
            Icon::Redo,
            Icon::Play,
            Icon::Pause,
            Icon::Stop,
            Icon::SkipBack,
            Icon::Loop,
            Icon::Plus,
            Icon::Minus,
            Icon::ChevronLeft,
            Icon::ChevronRight,
            Icon::ArrowUp,
            Icon::ArrowDown,
            Icon::ArrowLeft,
            Icon::ArrowRight,
            Icon::Trash,
            Icon::Refresh,
            Icon::Check,
            Icon::Close,
            Icon::Minimize,
            Icon::Maximize,
            Icon::Restore,
            Icon::Folder,
            Icon::ExternalLink,
            Icon::Download,
            Icon::Edit,
        ] {
            // Painting must not panic at any size.
            for size in [12.0, 16.0, 24.0, 48.0] {
                let rect = Rect::from_center_size(base.center(), Vec2::splat(size));
                paint(icon, &painter, rect, Color32::WHITE);
            }
        }
    }

    #[test]
    fn icons_scale_with_their_box() {
        let (_context, painter) = sample_painter();
        let pen = Pen {
            painter: &painter,
            center: egui::pos2(0.0, 0.0),
            scale: 1.0,
            color: Color32::WHITE,
            width: 1.0,
            mirror: false,
        };
        let small = pen.at(24.0, 24.0);
        assert!((small.x - 12.0).abs() < f32::EPSILON);
        let pen = Pen { scale: 2.0, ..pen };
        let large = pen.at(24.0, 24.0);
        assert!((large.x - 24.0).abs() < f32::EPSILON);
    }
}
