//! The scan-speed dial.

use crate::{state::State, theme};
use nsh_core::{
    canvas::{CanvasTextAlign, CanvasTextBaseline, LineCap},
    geometry::px,
    prelude::*,
};

const TAU: f32 = std::f32::consts::TAU;

/// The dial, bound to the scan's speed and its ceiling.
pub fn view(s: &State) -> impl IntoElement + use<> {
    let width = signal(0.0_f32);
    let ceiling = s.speed_max;
    let needle = s.needle;
    canvas(move || draw(width.get(), 280.0, ceiling.get(), needle.get()))
        .h(px(280.0))
        .w_full()
        .test_id("speed-gauge")
        .on_layout(move |bounds: &Bounds<Points>| width.set(bounds.size.width.0))
}

fn draw(width: f32, height: f32, ceiling: f64, needle: f64) -> Painter {
    if width <= 0.0 || height <= 0.0 {
        return Painter::new();
    }
    let cx = width / 2.0;
    let cy = height * 0.58;
    let radius = (width / 2.0 - 18.0).min(height * 0.44) - 4.0;
    if radius < 12.0 {
        return Painter::new();
    }
    let a0 = 0.75 * std::f32::consts::PI;
    let a1 = 2.25 * std::f32::consts::PI;
    let ceiling = if ceiling <= 0.0 { 40.0 } else { ceiling };
    let angle = a0 + (needle.clamp(0.0, 100.0) as f32 / 100.0) * (a1 - a0);

    let mut painter = Painter::new()
        .line_cap(LineCap::Round)
        .line_width(10.0)
        .stroke_color(theme::BORDER.light)
        .begin_path()
        .arc(cx, cy, radius, a0, a1)
        .stroke();

    // The tick ring.
    for i in 0..=40 {
        let fraction = i as f32 / 40.0;
        let a = a0 + fraction * (a1 - a0);
        let major = i % 5 == 0;
        let inner = if major { radius - 12.0 } else { radius - 7.0 };
        painter = painter
            .stroke_color(if major {
                theme::MUTED.light
            } else {
                theme::BORDER.light
            })
            .line_width(if major { 2.0 } else { 1.0 })
            .begin_path()
            .move_to(cx + a.cos() * inner, cy + a.sin() * inner)
            .line_to(cx + a.cos() * radius, cy + a.sin() * radius)
            .stroke();
        if i % 10 == 0 {
            painter = painter
                .font("9px system-ui")
                .fill_color(theme::MUTED.light)
                .text_align(CanvasTextAlign::Center)
                .text_baseline(CanvasTextBaseline::Middle)
                .fill_text(
                    &format!("{:.0}", ceiling * f64::from(fraction)),
                    cx + a.cos() * (radius - 25.0),
                    cy + a.sin() * (radius - 25.0),
                );
        }
    }

    // The sweep follows the needle.
    painter = painter
        .line_width(7.0)
        .stroke_color(theme::ACCENT.light)
        .begin_path()
        .arc(cx, cy, radius, a0, angle)
        .stroke();

    // The needle and its hub.
    painter = painter
        .save()
        .translate(cx, cy)
        .rotate(angle)
        .fill_color(theme::TEXT.light)
        .begin_path()
        .move_to(-12.0, -3.0)
        .line_to(radius - 16.0, 0.0)
        .line_to(-12.0, 3.0)
        .close_path()
        .fill()
        .restore()
        .fill_color(theme::SURFACE.light)
        .stroke_color(theme::TEXT.light)
        .line_width(2.0)
        .begin_path()
        .arc(cx, cy, 7.0, 0.0, TAU)
        .fill()
        .stroke()
        .fill_color(theme::ACCENT.light)
        .begin_path()
        .arc(cx, cy, 3.0, 0.0, TAU)
        .fill();
    painter
}
