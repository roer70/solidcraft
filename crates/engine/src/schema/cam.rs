//! Contracts and result types for the commands in `cmd/cam.rs` that have them: the machines a
//! design keeps (`cam.machine.*`). The handlers read these same types, so the contract is exactly
//! what they take; like them, a contract refuses unknown keys.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use solidcore_record::Vec3;
use solidcraft_doc::manufacture::{AxisName, Carrier, Machine, MachineAxis};

/// The name of a machine axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AxisNameParam {
    X,
    Y,
    Z,
    A,
    B,
    C,
}

/// What carries a rotary axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CarrierParam {
    /// The table: the part turns.
    Table,
    /// The head: the tool turns.
    Head,
}

/// One axis of a machine: `linear` (X, Y or Z) or `rotary` (A, B or C).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MachineAxisParam {
    /// Moves along its name's direction.
    Linear {
        /// X, Y or Z.
        name: AxisNameParam,
        /// Lower end of its travel, mm in machine coordinates.
        min: f64,
        /// Upper end of its travel, mm in machine coordinates; above `min`.
        max: f64,
    },
    /// Turns about a direction through a pivot.
    Rotary {
        /// A, B or C.
        name: AxisNameParam,
        /// The direction it turns about, [x, y, z] in machine coordinates (not zero).
        axis: [f64; 3],
        /// What carries it: the table or the head.
        on: CarrierParam,
        /// A point on the line it turns about, [x, y, z] mm in machine coordinates. Default: the
        /// machine origin.
        #[serde(default)]
        pivot: [f64; 3],
        /// Its limits in degrees, [lower, upper], the lower below the upper. Default: none, it
        /// turns round and round.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limits: Option<[f64; 2]>,
    },
}

impl From<AxisNameParam> for AxisName {
    fn from(n: AxisNameParam) -> Self {
        match n {
            AxisNameParam::X => AxisName::X,
            AxisNameParam::Y => AxisName::Y,
            AxisNameParam::Z => AxisName::Z,
            AxisNameParam::A => AxisName::A,
            AxisNameParam::B => AxisName::B,
            AxisNameParam::C => AxisName::C,
        }
    }
}

impl From<AxisName> for AxisNameParam {
    fn from(n: AxisName) -> Self {
        match n {
            AxisName::X => AxisNameParam::X,
            AxisName::Y => AxisNameParam::Y,
            AxisName::Z => AxisNameParam::Z,
            AxisName::A => AxisNameParam::A,
            AxisName::B => AxisNameParam::B,
            AxisName::C => AxisNameParam::C,
        }
    }
}

impl From<MachineAxisParam> for MachineAxis {
    fn from(a: MachineAxisParam) -> Self {
        let v = |[x, y, z]: [f64; 3]| Vec3::new(x, y, z);
        match a {
            MachineAxisParam::Linear { name, min, max } => MachineAxis::Linear { name: name.into(), min, max },
            MachineAxisParam::Rotary { name, axis, on, pivot, limits } => MachineAxis::Rotary {
                name: name.into(),
                axis: v(axis),
                on: match on {
                    CarrierParam::Table => Carrier::Table,
                    CarrierParam::Head => Carrier::Head,
                },
                pivot: v(pivot),
                limits,
            },
        }
    }
}

impl From<&MachineAxis> for MachineAxisParam {
    fn from(a: &MachineAxis) -> Self {
        let v = |p: Vec3| [p.x, p.y, p.z];
        match a {
            MachineAxis::Linear { name, min, max } => MachineAxisParam::Linear { name: (*name).into(), min: *min, max: *max },
            MachineAxis::Rotary { name, axis, on, pivot, limits } => MachineAxisParam::Rotary {
                name: (*name).into(),
                axis: v(*axis),
                on: match on {
                    Carrier::Table => CarrierParam::Table,
                    Carrier::Head => CarrierParam::Head,
                },
                pivot: v(*pivot),
                limits: *limits,
            },
        }
    }
}

/// Add a machine to the design, for setups to run on.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineAddParams {
    /// The machine's name, unique in the design (ignoring case), at most 200 bytes.
    pub name: String,
    /// Its axes in order from the table to the tool: 1 to 9, no name twice.
    pub axes: Vec<MachineAxisParam>,
    /// The built-in post a setup on it takes when the setup is given none: rs274, grbl, grblhal,
    /// lnc or lnc_atc. Default: none (setups keep their own, rs274 unless given).
    #[serde(default)]
    pub post: Option<String>,
}

/// Change a machine of the design. What is left out stays as it is.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineEditParams {
    /// The machine's name.
    pub machine: String,
    /// A new name, unique in the design (ignoring case); setups on it follow.
    #[serde(default)]
    pub name: Option<String>,
    /// Its axes in order from the table to the tool, replacing all it has: 1 to 9, no name twice.
    #[serde(default)]
    pub axes: Option<Vec<MachineAxisParam>>,
    /// Its default post, as in `cam.machine.add`. `null` clears it.
    #[serde(default, deserialize_with = "some_or_null")]
    #[schemars(with = "Option<String>")]
    pub post: Option<Option<String>>,
}

/// `Some(None)` for an explicit `null`, so it can be told from a key left out (`None`).
fn some_or_null<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

/// Remove a machine from the design. Refused while a setup runs on it.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineRemoveParams {
    /// The machine's name.
    pub machine: String,
}

/// List the design's machines.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineListParams {}

/// What `cam.machine.add` and `cam.machine.edit` return.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MachineResult {
    /// The machine's name.
    pub machine: String,
    /// In the warn parameter-check mode: what the call's parameters got wrong.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

/// What `cam.machine.remove` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MachineRemoved {
    /// The removed machine's name.
    pub removed: String,
    /// In the warn parameter-check mode: what the call's parameters got wrong.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

/// One machine in `cam.machine.list`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MachineEntry {
    pub name: String,
    /// Its axes in order from the table to the tool.
    pub axes: Vec<MachineAxisParam>,
    /// Its default post, if it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<String>,
    /// The names of the setups that run on it.
    pub setups: Vec<String>,
}

impl MachineEntry {
    pub fn new(machine: &Machine, setups: Vec<String>) -> Self {
        MachineEntry { name: machine.name.clone(), axes: machine.axes.iter().map(Into::into).collect(), post: machine.post.clone(), setups }
    }
}

/// What `cam.machine.list` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MachineList {
    /// The design's machines, in the order they were added.
    pub machines: Vec<MachineEntry>,
    /// In the warn parameter-check mode: what the call's parameters got wrong.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

#[cfg(test)]
#[path = "cam_tests.rs"]
mod tests;
