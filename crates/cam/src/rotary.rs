//! Turns of a rotary axis: how far an axis must turn to bring one direction to another.

use solidcore_record::Vec3;
use solidcraft_geom::prelude::*;

use crate::{Result, invalid};

/// How far off square to a rotary axis a direction may lean (the sine of the angle) and still
/// count as turning in the axis's plane.
const SQUARE: f64 = 1e-9;

/// The turn about `about`, in degrees by the right-hand rule, that takes direction `from` to
/// direction `to`. Without `limits` the axis turns freely and the turn is from −180 to 180;
/// with them it is the turn, or that turn a whole round on either way, that falls inside.
/// Both directions must be square to `about`; a turn can't bring a direction off its plane in.
pub fn rotary_angle(about: Vec3, from: Vec3, to: Vec3, limits: Option<[f64; 2]>) -> Result<f64> {
    let Some(a) = about.normalized().filter(|a| a.is_finite()) else {
        return invalid("the rotary axis has no direction");
    };
    let (Some(f), Some(t)) = (from.normalized().filter(|v| v.is_finite()), to.normalized().filter(|v| v.is_finite())) else {
        return invalid("a direction to turn has no length");
    };
    if f.dot(a).abs() > SQUARE || t.dot(a).abs() > SQUARE {
        return invalid("the direction is not square to the rotary axis, so no turn of it brings the direction there");
    }
    let turn = a.dot(f.cross(t)).atan2(f.dot(t)).to_degrees();
    let turn = if turn <= -180.0 + 1e-9 { turn + 360.0 } else { turn };
    let Some([low, high]) = limits else {
        return Ok(turn);
    };
    let eps = 1e-9;
    [turn, turn - 360.0, turn + 360.0]
        .into_iter()
        .find(|t| *t >= low - eps && *t <= high + eps)
        .map_or_else(|| invalid(format!("the turn of {turn:.3}° is outside the axis's limits {low}° to {high}°")), Ok)
}
