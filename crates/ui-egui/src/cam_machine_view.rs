//! A picture of the machine a machine form describes, drawn in the form beside nothing else: a
//! block on the table, an arrow along each linear axis, each rotary axis as a line with a ring
//! whose arrow turns the positive way, and the tool. Not to scale: it shows which way each axis
//! goes, and the form's sliders turn the block (or the tool) and move the tool to try it.
//! Built as lines in its own space, so tests read the drawing rather than pixels.

use solidcore_record::Vec3;
use solidcraft_engine::doc::manufacture::{AxisName, Carrier, MachineAxis};
use solidcraft_engine::geom::prelude::*;

/// What a line or a label of the drawing shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// A linear axis's arrow, positive along it.
    Linear,
    /// A rotary axis: its centre line, its ring and the arrow on the ring.
    Rotary,
    /// The block on the table, turned by the rotary axes the table carries.
    Stock,
    /// The tool, moved by the linear axes and turned by the rotary axes the head carries.
    Tool,
}

/// The drawing, in the picture's own space: Z up, the block's centre at the origin.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawing {
    pub lines: Vec<(Vec3, Vec3, Part)>,
    pub labels: Vec<(Vec3, &'static str, Part)>,
}

/// A rotary axis's ring about `axis` through `centre`: its edges, and the arrow on it as a
/// segment from a point of the ring the way positive turning (the right-hand rule) goes there.
pub struct Ring {
    pub edges: Vec<(Vec3, Vec3)>,
    pub arrow: (Vec3, Vec3),
}

/// Sides of a ring.
const RING_SIDES: usize = 48;
/// The block's half size.
const BLOCK: Vec3 = Vec3::new(30.0, 20.0, 15.0);
/// A table ring's radius (the next one out is this much wider) and how far an axis line reaches.
const RING_RADIUS: f64 = 45.0;
const RING_STEP: f64 = 12.0;
const AXIS_REACH: f64 = 80.0;
/// The tool: its length, a head ring's radius, and where its tip is with every linear axis in
/// the middle of its travel; a linear axis moves it this far either side.
const TOOL_LENGTH: f64 = 50.0;
const HEAD_RING: f64 = 18.0;
const TOOL_HOME: Vec3 = Vec3::new(0.0, 0.0, 45.0);
const TOOL_REACH: f64 = 25.0;
/// Where the linear axes' arrows start, and how long they are.
const ARROWS_AT: Vec3 = Vec3::new(-75.0, -60.0, -45.0);
const ARROW_LENGTH: f64 = 40.0;

/// The ring of a rotary axis about `axis` (any length) through `centre`; `None` for no direction.
pub fn ring(centre: Vec3, axis: Vec3, radius: f64) -> Option<Ring> {
    let k = axis.normalized()?;
    let u = k.any_perp();
    let v = k.cross(u);
    let at = |t: f64| centre + (u * t.cos() + v * t.sin()) * radius;
    let step = std::f64::consts::TAU / RING_SIDES as f64;
    let edges = (0..RING_SIDES).map(|i| (at(i as f64 * step), at((i + 1) as f64 * step))).collect();
    // From the ring at t = 0 a little way on: along k × u = v, the right-hand way about k.
    let arrow = (at(0.0), at(0.0) + v * (radius * 0.6));
    Some(Ring { edges, arrow })
}

/// `p` turned by `degrees` about the line along `axis` (any length) through `centre`, the
/// right-hand way.
pub fn turned(p: Vec3, centre: Vec3, axis: Vec3, degrees: f64) -> Vec3 {
    let Some(k) = axis.normalized() else { return p };
    let (s, c) = degrees.to_radians().sin_cos();
    let w = p - centre;
    centre + w * c + k.cross(w) * s + k * (k.dot(w) * (1.0 - c))
}

/// Two short strokes at `to` making an arrowhead for the segment `from`–`to`.
fn head(from: Vec3, to: Vec3, part: Part, lines: &mut Vec<(Vec3, Vec3, Part)>) {
    let Some(d) = (to - from).normalized() else { return };
    let side = d.any_perp();
    let size = (to - from).len().min(30.0) * 0.25;
    for s in [side, side * -1.0] {
        lines.push((to, to - d * size + s * (size * 0.5), part));
    }
}

/// The unit direction of a linear axis's name.
fn along(name: AxisName) -> Vec3 {
    match name {
        AxisName::X => Vec3::X,
        AxisName::Y => Vec3::Y,
        _ => Vec3::Z,
    }
}

/// A rotary axis's ring, centre line and their arrows, named, into `d`.
fn rotary(d: &mut Drawing, name: AxisName, centre: Vec3, axis: Vec3, radius: f64, reach: f64) {
    let Some(k) = axis.normalized() else { return };
    d.lines.push((centre - k * reach, centre + k * reach, Part::Rotary));
    head(centre, centre + k * reach, Part::Rotary, &mut d.lines);
    if let Some(r) = ring(centre, k, radius) {
        d.lines.extend(r.edges.into_iter().map(|(a, b)| (a, b, Part::Rotary)));
        d.lines.push((r.arrow.0, r.arrow.1, Part::Rotary));
        head(r.arrow.0, r.arrow.1, Part::Rotary, &mut d.lines);
    }
    d.labels.push((centre + k * (reach + 12.0), name.label(), Part::Rotary));
}

/// The picture of `axes` (chain order from the table to the tool), posed at `at` (per axis: mm
/// along a linear one, degrees about a rotary one).
pub fn drawing(axes: &[MachineAxis], at: &[f64]) -> Drawing {
    let mut d = Drawing::default();
    let pose = |k: usize| at.get(k).copied().filter(|v| v.is_finite());
    // The arrows of the linear axes, from one corner, and where they put the tool's tip.
    let mut tip = TOOL_HOME;
    for (k, axis) in axes.iter().enumerate() {
        let MachineAxis::Linear { name, min, max } = axis else { continue };
        let e = along(*name);
        let to = ARROWS_AT + e * ARROW_LENGTH;
        d.lines.push((ARROWS_AT, to, Part::Linear));
        head(ARROWS_AT, to, Part::Linear, &mut d.lines);
        d.labels.push((to + e * 10.0, name.label(), Part::Linear));
        if max > min {
            let share = pose(k).map_or(0.5, |v| ((v - min) / (max - min)).clamp(0.0, 1.0));
            tip += e * ((share - 0.5) * 2.0 * TOOL_REACH);
        }
    }
    // The table's rotary axes through the block, each ring a little wider; the block turned by
    // them, the one nearest the tool first, as the one nearer the table carries it.
    let mut turns = Vec::new();
    let mut radius = RING_RADIUS;
    for (k, axis) in axes.iter().enumerate() {
        if let MachineAxis::Rotary { name, axis, on: Carrier::Table, .. } = axis {
            rotary(&mut d, *name, Vec3::ZERO, *axis, radius, AXIS_REACH);
            turns.push((*axis, pose(k).unwrap_or(0.0)));
            radius += RING_STEP;
        }
    }
    let block = |p: Vec3| turns.iter().rev().fold(p, |p, (axis, a)| turned(p, Vec3::ZERO, *axis, *a));
    let corner = |i: usize| {
        let s = |bit: usize| if i & bit == 0 { -1.0 } else { 1.0 };
        block(Vec3::new(BLOCK.x * s(1), BLOCK.y * s(2), BLOCK.z * s(4)))
    };
    for i in 0..8usize {
        for bit in [1usize, 2, 4] {
            if i & bit == 0 {
                d.lines.push((corner(i), corner(i | bit), Part::Stock));
            }
        }
    }
    // The block's top marked, so a turn shows which side came up.
    d.lines.push((block(Vec3::new(0.0, 0.0, BLOCK.z)), block(Vec3::new(0.0, BLOCK.y, BLOCK.z)), Part::Stock));
    // The tool up from its tip, turned about its tip by the head's rotary axes.
    let mut up = Vec3::Z * TOOL_LENGTH;
    for (k, axis) in axes.iter().enumerate().rev() {
        if let MachineAxis::Rotary { name, axis, on: Carrier::Head, .. } = axis {
            rotary(&mut d, *name, tip + Vec3::Z * (TOOL_LENGTH * 0.6), *axis, HEAD_RING, HEAD_RING * 1.8);
            up = turned(up, Vec3::ZERO, *axis, pose(k).unwrap_or(0.0));
        }
    }
    d.lines.push((tip, tip + up, Part::Tool));
    let side = up.any_perp() * 5.0;
    d.lines.push((tip - side, tip + side, Part::Tool));
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ring's arrow turns the right-hand way: the arrow's direction is `axis × (from − centre)`.
    #[test]
    fn a_rings_arrow_turns_the_right_hand_way_about_its_axis() {
        for axis in [Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)] {
            let centre = Vec3::new(5.0, -723.0, -98.0);
            let r = ring(centre, axis, 40.0).unwrap();
            let (from, to) = r.arrow;
            let want = axis.cross(from - centre).normalized().unwrap();
            let got = (to - from).normalized().unwrap();
            assert!(got.dist(want) < 1e-9, "about {axis:?}: {got:?} not {want:?}");
            assert!(((from - centre).len() - 40.0).abs() < 1e-9 && (from - centre).dot(axis).abs() < 1e-9, "on the ring");
            assert_eq!(r.edges.len(), RING_SIDES);
        }
    }

    /// Turning by the right-hand rule: about −X, −90° brings +Y up; about +Y, 90° takes +Z to +X.
    #[test]
    fn turning_follows_the_right_hand_rule() {
        let up = turned(Vec3::Y, Vec3::ZERO, Vec3::new(-1.0, 0.0, 0.0), -90.0);
        assert!(up.dist(Vec3::Z) < 1e-9, "{up:?}");
        let x = turned(Vec3::Z, Vec3::ZERO, Vec3::Y, 90.0);
        assert!(x.dist(Vec3::X) < 1e-9, "{x:?}");
    }

    /// The tool goes the positive way of a linear axis as its slider goes up, and stays in the
    /// picture past the travel's ends.
    #[test]
    fn a_linear_slider_moves_the_tool_the_positive_way() {
        let x = [MachineAxis::Linear { name: AxisName::X, min: -672.0, max: 0.0 }];
        let tip = |at: f64| drawing(&x, &[at]).lines.iter().find(|l| l.2 == Part::Tool).map(|l| l.0).unwrap();
        assert!(tip(-600.0).x < tip(-336.0).x && tip(-336.0).x < tip(0.0).x);
        assert_eq!(tip(-336.0), TOOL_HOME);
        assert_eq!(tip(500.0), tip(0.0));
    }
}
