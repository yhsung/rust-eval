use eframe::egui::{self, Align2, Color32, FontId, Rect, Stroke, StrokeKind, vec2};
use rust_modem_safety_island::{EventKind, RaceStep, StateKind};

use super::{INK, MUTED, REJECTED, RULE, SIGNAL, TEXT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Edge {
    Start,
    Setup,
    Expiry,
    LateSetup,
    LateExpiry,
}

pub(super) fn highlighted_path(trace: &[RaceStep; 2]) -> [Edge; 3] {
    [
        Edge::Start,
        match trace[0].event {
            EventKind::RrcSetup => Edge::Setup,
            EventKind::T300Expiry => Edge::Expiry,
            other => unreachable!("unexpected first race event: {other:?}"),
        },
        match trace[1].event {
            EventKind::RrcSetup => Edge::LateSetup,
            EventKind::T300Expiry => Edge::LateExpiry,
            other => unreachable!("unexpected second race event: {other:?}"),
        },
    ]
}

impl Edge {
    fn diagram(self) -> (&'static [[f32; 2]], &'static str, [f32; 2]) {
        match self {
            Self::Start => (&[[158., 98.], [307., 98.]], "RRCSetupRequest", [232., 83.]),
            Self::Setup => (&[[413., 98.], [562., 98.]], "RRCSetup", [488., 83.]),
            Self::Expiry => (
                &[[338., 119.], [338., 158.], [125., 158.], [125., 119.]],
                "T300Expiry",
                [232., 144.],
            ),
            Self::LateSetup => (
                &[[85., 77.], [85., 44.], [125., 44.], [125., 77.]],
                "Late RRCSetup",
                [105., 29.],
            ),
            Self::LateExpiry => (
                &[[595., 77.], [595., 44.], [635., 44.], [635., 77.]],
                "Late T300Expiry",
                [615., 29.],
            ),
        }
    }

    fn destination(self) -> StateKind {
        match self {
            Self::Start => StateKind::Connecting,
            Self::Setup | Self::LateExpiry => StateKind::Connected,
            Self::Expiry | Self::LateSetup => StateKind::Idle,
        }
    }
}

pub(super) fn show(ui: &mut egui::Ui, visited: &[Edge]) {
    let scale = (ui.available_width() / 720.0).min(1.0);
    let (response, painter) = ui.allocate_painter(
        vec2(ui.available_width(), 224.0 * scale),
        egui::Sense::hover(),
    );
    let origin = response.rect.min + vec2((response.rect.width() - 720.0 * scale) * 0.5, 0.0);
    let point = |[x, y]: [f32; 2]| origin + vec2(x, y) * scale;
    painter.rect_filled(response.rect, 7.0, INK);
    let grid = Stroke::new(1.0_f32, RULE.gamma_multiply(0.36));
    for x in (40..720).step_by(40) {
        let x = x as f32;
        painter.line_segment([point([x, 8.]), point([x, 216.])], grid);
    }
    for y in (24..224).step_by(24) {
        let y = y as f32;
        painter.line_segment([point([8., y]), point([712., y])], grid);
    }
    for edge in [
        Edge::Start,
        Edge::Setup,
        Edge::Expiry,
        Edge::LateSetup,
        Edge::LateExpiry,
    ] {
        let highlighted = visited.contains(&edge);
        let color = if !highlighted {
            MUTED
        } else if matches!(edge, Edge::LateSetup | Edge::LateExpiry) {
            REJECTED
        } else {
            SIGNAL
        };
        let width: f32 = if highlighted { 2.8 } else { 1.2 };
        let stroke = Stroke::new(width, color);
        let (route, label, at) = edge.diagram();
        for pair in route.windows(2) {
            painter.line_segment([point(pair[0]), point(pair[1])], stroke);
        }
        let tip = point(route[route.len() - 1]);
        let axis = (tip - point(route[route.len() - 2])).normalized();
        let side = vec2(-axis.y, axis.x);
        for sign in [-1.0, 1.0] {
            painter.line_segment(
                [tip, tip - (axis * 8.0 + side * 4.0 * sign) * scale],
                stroke,
            );
        }
        painter.text(
            point(at),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(14.0 * scale),
            color,
        );
    }
    let current = visited
        .last()
        .map_or(StateKind::Idle, |edge| edge.destination());
    for (x, state, name) in [
        (105., StateKind::Idle, "Idle"),
        (360., StateKind::Connecting, "Connecting"),
        (615., StateKind::Connected, "Connected"),
    ] {
        let center = point([x, 98.]);
        let rect = Rect::from_center_size(center, vec2(106., 42.) * scale);
        let (width, border, fill): (f32, _, _) = if current == state {
            (2.0, SIGNAL, Color32::from_rgb(25, 62, 58))
        } else {
            (1.0, RULE, Color32::from_rgb(22, 39, 43))
        };
        painter.rect(
            rect,
            7.0,
            fill,
            Stroke::new(width, border),
            StrokeKind::Inside,
        );
        painter.text(
            center,
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(16.0 * scale),
            TEXT,
        );
    }
}
