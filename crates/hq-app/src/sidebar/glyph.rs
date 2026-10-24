//! Status glyphs and agent marks, painted as shapes on a 24-unit design grid, so status
//! reads by shape as well as colour.

use std::f32::consts::PI;

use gpui::{
    AnyElement, Bounds, IntoElement, ParentElement, PathBuilder, Pixels, Point, Styled, Window,
    canvas, div, point, px,
};
use hq_signals::event::Agent;

use hq_protocol::usage::Rings;

use super::model::Glyph;
use crate::terminal::view::hsla;
use crate::theme::Chrome;
use crate::theme::Paint;

/// The square every glyph is centred in.
pub const CELL: f32 = 22.0;

/// Maps the 24-unit design grid into `size` pixels centred in `bounds`.
#[derive(Clone, Copy)]
struct Grid {
    origin: Point<Pixels>,
    unit: f32,
}

impl Grid {
    fn new(bounds: Bounds<Pixels>, size: f32) -> Grid {
        let left = f32::from(bounds.size.width) - size;
        let top = f32::from(bounds.size.height) - size;
        Grid {
            origin: point(
                bounds.origin.x + px(left / 2.0),
                bounds.origin.y + px(top / 2.0),
            ),
            unit: size / 24.0,
        }
    }

    fn at(self, x: f32, y: f32) -> Point<Pixels> {
        point(
            self.origin.x + px(x * self.unit),
            self.origin.y + px(y * self.unit),
        )
    }

    fn width(self, units: f32) -> Pixels {
        px(units * self.unit)
    }

    fn radius(self, units: f32) -> Point<Pixels> {
        point(px(units * self.unit), px(units * self.unit))
    }
}

fn circle(builder: &mut PathBuilder, grid: Grid, cx: f32, cy: f32, r: f32) {
    builder.move_to(grid.at(cx + r, cy));
    builder.arc_to(grid.radius(r), px(0.), false, true, grid.at(cx - r, cy));
    builder.arc_to(grid.radius(r), px(0.), false, true, grid.at(cx + r, cy));
    builder.close();
}

fn polyline(builder: &mut PathBuilder, grid: Grid, points: &[(f32, f32)]) {
    let mut points = points.iter();
    if let Some(&(x, y)) = points.next() {
        builder.move_to(grid.at(x, y));
    }
    for &(x, y) in points {
        builder.line_to(grid.at(x, y));
    }
}

fn paint(window: &mut Window, builder: PathBuilder, color: impl Into<Paint>) {
    // A shape that fails to build is skipped; the row's text still says the status.
    if let Ok(path) = builder.build() {
        window.paint_path(path, hsla(color));
    }
}

/// Paints a status glyph into `bounds`; working paints nothing.
fn paint_status(glyph: Glyph, bounds: Bounds<Pixels>, chrome: Chrome, window: &mut Window) {
    match glyph {
        Glyph::NeedsYou => {
            let grid = Grid::new(bounds, 20.0);
            let mut ring = PathBuilder::stroke(grid.width(2.4));
            circle(&mut ring, grid, 12.0, 12.0, 9.0);
            paint(window, ring, chrome.glyph_amber);
            let mut dot = PathBuilder::fill();
            circle(&mut dot, grid, 12.0, 12.0, 4.2);
            paint(window, dot, chrome.glyph_amber);
        }
        Glyph::IdleUnread => {
            paint_bubble(bounds, chrome.text, true, window);
            let grid = Grid::new(bounds, 18.0);
            let mut dot = PathBuilder::fill();
            circle(&mut dot, grid, 20.0, 4.0, 3.4);
            paint(window, dot, chrome.accent);
        }
        Glyph::Idle => paint_bubble(bounds, chrome.muted, false, window),
        Glyph::Inactive => paint_moon(bounds, chrome.muted, window),
        // The slot stays empty, so ages line up with the other rows.
        Glyph::Elsewhere | Glyph::Working | Glyph::Starting => {}
    }
}

/// The outline of a speech bubble on the 24-unit grid.
const BUBBLE: [(f32, f32); 12] = [
    (5.0, 5.5),
    (19.0, 5.5),
    (20.5, 7.0),
    (20.5, 15.0),
    (19.0, 16.5),
    (11.0, 16.5),
    (7.0, 19.5),
    (7.0, 16.5),
    (5.0, 16.5),
    (3.5, 15.0),
    (3.5, 7.0),
    (5.0, 5.5),
];

/// A speech bubble: the agent waits for the owner's next message (the owner's choice of
/// idle glyph). `filled` draws a reply not read yet.
fn paint_bubble(
    bounds: Bounds<Pixels>,
    color: impl Into<Paint> + Copy,
    filled: bool,
    window: &mut Window,
) {
    let grid = Grid::new(bounds, 18.0);
    let mut bubble = if filled {
        PathBuilder::fill()
    } else {
        PathBuilder::stroke(grid.width(2.2))
    };
    polyline(&mut bubble, grid, &BUBBLE);
    if filled {
        bubble.close();
    }
    paint(window, bubble, color);
}

/// A crescent moon: the session sleeps, its agent stopped (the owner's choice of inactive
/// glyph). The outer edge is a circle of radius 8 around (12, 12); the inner edge one of
/// the same radius around (18, 12), so the horns meet where x = 15.
fn paint_moon(bounds: Bounds<Pixels>, color: impl Into<Paint> + Copy, window: &mut Window) {
    let grid = Grid::new(bounds, 18.0);
    let horn = (64.0_f32 - 9.0).sqrt();
    let (top, bottom) = ((15.0, 12.0 - horn), (15.0, 12.0 + horn));
    let mut moon = PathBuilder::fill();
    moon.move_to(grid.at(top.0, top.1));
    moon.arc_to(
        grid.radius(8.0),
        px(0.),
        true,
        false,
        grid.at(bottom.0, bottom.1),
    );
    moon.arc_to(grid.radius(8.0), px(0.), false, true, grid.at(top.0, top.1));
    moon.close();
    paint(window, moon, color);
}

/// The status glyph for a row. Working has none: the row's breathing "working" word
/// marks it (owner, 2026-10-21; fix-sidebar-motion-redraw), and the empty cell keeps
/// rows aligned.
pub fn status(glyph: Glyph, chrome: Chrome) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| paint_status(glyph, bounds, chrome, window),
    )
    .size(px(CELL))
    .flex_none()
    .into_any_element()
}

fn paint_agent(
    agent: Agent,
    bounds: Bounds<Pixels>,
    size: f32,
    color: impl Into<Paint> + Copy,
    window: &mut Window,
) {
    let grid = Grid::new(bounds, size);
    let mut mark = PathBuilder::stroke(grid.width(2.2));
    match agent {
        Agent::Claude => {
            polyline(&mut mark, grid, &[(12.0, 3.0), (12.0, 21.0)]);
            polyline(&mut mark, grid, &[(4.2, 7.5), (19.8, 16.5)]);
            polyline(&mut mark, grid, &[(4.2, 16.5), (19.8, 7.5)]);
        }
        Agent::Codex => {
            polyline(
                &mut mark,
                grid,
                &[
                    (12.0, 3.0),
                    (20.0, 7.5),
                    (20.0, 16.5),
                    (12.0, 21.0),
                    (4.0, 16.5),
                    (4.0, 7.5),
                ],
            );
            mark.close();
        }
        Agent::Agy => {
            polyline(&mut mark, grid, &[(12.0, 4.0), (21.0, 20.0), (3.0, 20.0)]);
            mark.close();
        }
    }
    paint(window, mark, color);
}

/// Headquarter's drawn mark for an agent, in a `size` pixel cell: an asterisk for
/// Claude, a hexagon for Codex, a triangle for agy. Stands in for the vendor's mark
/// (`crate::marks`).
pub fn agent(agent: Agent, size: f32, color: Paint) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| paint_agent(agent, bounds, size * 13.0 / 18.0, color, window),
    )
    .size(px(size))
    .flex_none()
    .into_any_element()
}

/// One ring of `r` grid units: the track, then `share` (0 to 1) from the top clockwise.
fn paint_ring(
    window: &mut Window,
    grid: Grid,
    r: f32,
    share: Option<f32>,
    color: Paint,
    track: Paint,
) {
    let (cx, cy, width) = (12.0, 12.0, 2.3);
    let mut ring = PathBuilder::stroke(grid.width(width));
    circle(&mut ring, grid, cx, cy, r);
    paint(window, ring, track);
    let Some(share) = share.map(|share| share.clamp(0.0, 1.0)) else {
        return;
    };
    if share >= 0.999 {
        let mut full = PathBuilder::stroke(grid.width(width));
        circle(&mut full, grid, cx, cy, r);
        paint(window, full, color);
    } else if share > 0.0 {
        let start = -PI / 2.0;
        let end = start + share * 2.0 * PI;
        let on = |angle: f32| grid.at(cx + r * angle.cos(), cy + r * angle.sin());
        let mut arc = PathBuilder::stroke(grid.width(width));
        arc.move_to(on(start));
        arc.arc_to(grid.radius(r), px(0.), share > 0.5, true, on(end));
        paint(window, arc, color);
    }
}

/// Usage rings (owner, 2026-10-22): the outer ring for the agent's longest window (the
/// week), the inner one for its shortest (5 hours), each in its own colour, with the
/// agent's mark inside: `mark` when given, else the drawn one. With nothing current to
/// show, one dotted ring.
pub fn usage_ring(
    agent: Agent,
    rings: Rings,
    outer_color: Paint,
    inner_color: Paint,
    chrome: Chrome,
    mark: Option<AnyElement>,
) -> AnyElement {
    let drawn = mark.is_none();
    let ring = canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let grid = Grid::new(bounds, 30.0);
            if rings.is_empty() {
                let mut ring = PathBuilder::stroke(grid.width(2.3))
                    .dash_array(&[grid.width(2.0), grid.width(2.6)]);
                circle(&mut ring, grid, 12.0, 12.0, 10.0);
                paint(window, ring, chrome.muted);
            } else {
                let share = |percent: Option<f64>| percent.map(|percent| (percent / 100.0) as f32);
                paint_ring(
                    window,
                    grid,
                    10.0,
                    share(rings.outer),
                    outer_color,
                    chrome.track,
                );
                if rings.inner.is_some() {
                    paint_ring(
                        window,
                        grid,
                        6.6,
                        share(rings.inner),
                        inner_color,
                        chrome.track,
                    );
                }
            }
            if drawn {
                let size = if rings.inner.is_some() { 8.0 } else { 12.0 };
                paint_agent(agent, bounds, size, chrome.muted, window);
            }
        },
    )
    .size(px(36.))
    .flex_none();
    match mark {
        None => ring.into_any_element(),
        Some(mark) => div()
            .relative()
            .size(px(36.))
            .flex_none()
            .child(ring)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size(px(36.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(mark),
            )
            .into_any_element(),
    }
}
