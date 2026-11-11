//! Manufacturing data of a design: the tool library and the setups with their operations.
//! Toolpaths are not stored; they are worked out from this and the evaluated model, so they
//! follow parameter changes.

use serde::{Deserialize, Serialize};
use solidcore_record::Vec3;
use solidcraft_cam::{Side, Tool};

use crate::{Expr, ProfileSel};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Manufacture {
    /// The tool library, by tool number.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
    /// The machines the setups run on, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub machines: Vec<Machine>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub setups: Vec<Setup>,
}

/// Most machines a design keeps.
pub const MAX_MACHINES: usize = 256;
/// Most axes in a machine's chain.
pub const MAX_MACHINE_AXES: usize = 9;
/// Longest machine name, in bytes.
pub const MAX_MACHINE_NAME: usize = 200;

impl Manufacture {
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.machines.is_empty() && self.setups.is_empty()
    }
    pub fn tool(&self, number: u32) -> Option<&Tool> {
        self.tools.iter().find(|t| t.number == number)
    }
    pub fn machine(&self, name: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.name == name)
    }

    /// Checks the machines and the setups' references to them: every machine is sound, no two
    /// share a name, and every setup's machine is one of them. A design from elsewhere is hostile.
    pub fn check_machines(&self) -> std::result::Result<(), String> {
        if self.machines.len() > MAX_MACHINES {
            return Err(format!("more than {MAX_MACHINES} machines"));
        }
        for (i, machine) in self.machines.iter().enumerate() {
            machine.check()?;
            if self.machines[..i].iter().any(|m| m.name == machine.name) {
                return Err(format!("two machines are named \"{}\"", machine.name));
            }
        }
        for setup in &self.setups {
            if let Some(name) = &setup.machine
                && self.machine(name).is_none()
            {
                return Err(format!("setup \"{}\" runs on machine \"{name}\", which the design does not have", setup.name));
            }
        }
        Ok(())
    }
}

/// A machine tool: its axes as a chain from the table to the tool, and the post its programs
/// are written with unless a setup names its own. A lathe spindle will be another optional field
/// here; older designs then load without it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub name: String,
    /// The axes in order from the table to the tool.
    pub axes: Vec<MachineAxis>,
    /// The built-in post processor a setup on this machine takes when it names none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<String>,
}

impl Machine {
    /// The axis with this name.
    pub fn axis(&self, name: AxisName) -> Option<&MachineAxis> {
        self.axes.iter().find(|a| a.name() == name)
    }

    /// The A angle, in degrees, that points a setup's tool axis (`tool_axis`, in the part's
    /// coordinates at A 0) down the machine's tool: on the table A turns the part until the axis
    /// is machine +Z; on the head A turns the tool from +Z to the axis. An error when the machine
    /// has no rotary A, the axis is not square to A, or the angle is outside A's limits.
    pub fn a_angle(&self, tool_axis: Vec3) -> std::result::Result<f64, String> {
        let name = &self.name;
        let Some(MachineAxis::Rotary { axis, on, limits, .. }) = self.axis(AxisName::A) else {
            return Err(format!("machine \"{name}\" has no rotary A axis"));
        };
        let up = Vec3::new(0.0, 0.0, 1.0);
        let (from, to) = match on {
            Carrier::Table => (tool_axis, up),
            Carrier::Head => (up, tool_axis),
        };
        solidcraft_cam::rotary_angle(*axis, from, to, *limits).map_err(|e| format!("machine \"{name}\", axis A: {e}"))
    }

    /// Checks that the machine can be used: a name, at least one axis and no more than
    /// [`MAX_MACHINE_AXES`], no axis named twice, every axis sound, and every place a rotary
    /// axis waits at within the travel of the linear axes it names.
    pub fn check(&self) -> std::result::Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("a machine has no name".into());
        }
        if self.name.len() > MAX_MACHINE_NAME {
            return Err(format!("a machine name is longer than {MAX_MACHINE_NAME} bytes"));
        }
        let name = &self.name;
        if self.axes.is_empty() {
            return Err(format!("machine \"{name}\" has no axes"));
        }
        if self.axes.len() > MAX_MACHINE_AXES {
            return Err(format!("machine \"{name}\" has more than {MAX_MACHINE_AXES} axes"));
        }
        for (i, axis) in self.axes.iter().enumerate() {
            axis.check().map_err(|e| format!("machine \"{name}\", axis {}: {e}", axis.name().label()))?;
            if self.axes[..i].iter().any(|a| a.name() == axis.name()) {
                return Err(format!("machine \"{name}\" has two {} axes", axis.name().label()));
            }
        }
        for axis in &self.axes {
            if let MachineAxis::Rotary { name: turning, turn_at: Some(at), .. } = axis {
                for (axis_along, v) in at.named() {
                    let (turning, along) = (turning.label(), axis_along.label());
                    let Some(MachineAxis::Linear { min, max, .. }) = self.axis(axis_along) else {
                        return Err(format!("machine \"{name}\", axis {turning}: it waits at {along}, and the machine has no linear {along} axis"));
                    };
                    if !v.is_finite() || v < *min || v > *max {
                        return Err(format!("machine \"{name}\", axis {turning}: it waits at {along} {v}, outside {along}'s travel {min} to {max}"));
                    }
                }
            }
        }
        if self.post.as_deref().is_some_and(|p| p.trim().is_empty()) {
            return Err(format!("machine \"{name}\" has an empty post"));
        }
        Ok(())
    }
}

/// The name of a machine axis: X, Y and Z move along, A, B and C turn about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxisName {
    X,
    Y,
    Z,
    A,
    B,
    C,
}

impl AxisName {
    pub fn label(self) -> &'static str {
        match self {
            AxisName::X => "X",
            AxisName::Y => "Y",
            AxisName::Z => "Z",
            AxisName::A => "A",
            AxisName::B => "B",
            AxisName::C => "C",
        }
    }
    /// Whether the axis turns (A, B, C) rather than moves along (X, Y, Z).
    pub fn is_rotary(self) -> bool {
        matches!(self, AxisName::A | AxisName::B | AxisName::C)
    }
}

/// What carries a rotary axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Carrier {
    /// The table: the part turns.
    Table,
    /// The head: the tool turns.
    Head,
}

/// One axis of a machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MachineAxis {
    /// Moves along its name's direction from `min` to `max` (mm, machine coordinates).
    Linear { name: AxisName, min: f64, max: f64 },
    /// Turns about `axis` (a direction in machine coordinates) through `pivot` (mm), carried by
    /// the table or the head, between `limits` (degrees) or round and round without them.
    Rotary {
        name: AxisName,
        axis: Vec3,
        on: Carrier,
        pivot: Vec3,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limits: Option<[f64; 2]>,
        /// Where the tool waits while this axis turns (none: Z to the top of its travel).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        turn_at: Option<TurnAt>,
    },
}

/// Where the tool waits while a rotary axis turns, in machine coordinates (mm), per linear
/// axis: Z first, then the others, so it never moves sideways low down. A Z left out is the top
/// of Z's travel; an X or Y left out stays where it is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TurnAt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
}

impl TurnAt {
    /// The axes it names, with their values.
    pub fn named(&self) -> impl Iterator<Item = (AxisName, f64)> {
        [(AxisName::X, self.x), (AxisName::Y, self.y), (AxisName::Z, self.z)].into_iter().filter_map(|(n, v)| v.map(|v| (n, v)))
    }
}

impl MachineAxis {
    pub fn name(&self) -> AxisName {
        match self {
            MachineAxis::Linear { name, .. } | MachineAxis::Rotary { name, .. } => *name,
        }
    }

    fn check(&self) -> std::result::Result<(), String> {
        match self {
            MachineAxis::Linear { name, min, max } => {
                if name.is_rotary() {
                    return Err("a linear axis must be X, Y or Z".into());
                }
                if !min.is_finite() || !max.is_finite() || min >= max {
                    return Err("its travel needs finite limits, the lower below the upper".into());
                }
            }
            MachineAxis::Rotary { name, axis, pivot, limits, .. } => {
                if !name.is_rotary() {
                    return Err("a rotary axis must be A, B or C".into());
                }
                let length = (axis.x * axis.x + axis.y * axis.y + axis.z * axis.z).sqrt();
                if !axis.is_finite() || !length.is_finite() || length < 1e-9 {
                    return Err("it needs a direction to turn about".into());
                }
                if !pivot.is_finite() {
                    return Err("its pivot must be a finite point".into());
                }
                if let Some([low, high]) = limits
                    && (!low.is_finite() || !high.is_finite() || low >= high)
                {
                    return Err("its limits must be finite, the lower below the upper".into());
                }
            }
        }
        Ok(())
    }
}

/// One clamping of the part: which way the tool points, where the work origin is, the stock
/// the part is cut from, and what is cut in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Setup {
    pub id: u64,
    pub name: String,
    /// The tool axis: the setup's +Z as a direction in the world (the tool points along −Z).
    #[serde(default = "world_up")]
    pub axis: Vec3,
    /// The setup's +X as a direction in the world, across the tool axis (none: worked out from
    /// the axis). Turned over from below, world +X turns the part about X, −X about Y.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x_axis: Option<Vec3>,
    pub origin: SetupOrigin,
    #[serde(default)]
    pub stock: StockDef,
    /// What the stock is made of: operations take their tool's cutting data for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    /// The built-in post processor programs are written with.
    pub post: String,
    /// The machine it runs on, by name in [`Manufacture::machines`] (none: a generic three-axis
    /// mill, as before machines).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    /// The work offset its programs run in: 54 to 59 (`G54` to `G59`).
    #[serde(default = "g54", skip_serializing_if = "is_g54")]
    pub work_offset: u8,
    /// An earlier setup whose result is this one's stock (the part turned over, say): its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stock_from: Option<u64>,
    /// The part stays clamped as it was for `stock_from`'s setup and is only turned with the A
    /// axis: no new clamping between them (false: it is taken out and clamped again).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub same_clamping: bool,
    /// What holds the stock that no tool may touch (clamps, vices), in the setup's work coordinates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clamps: Vec<Clamp>,
    /// The bodies that are the part, by name (empty: every body). Others (fixtures, a modelled
    /// stock) are neither cut round nor checked against.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bodies: Vec<String>,
    /// The bodies that hold the stock (a vise, its jaws, a plate), by name: never the part,
    /// and no tool or holder may come into them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixtures: Vec<String>,
    /// Copies of the part side by side on the stock, each cut by every operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copies: Option<Copies>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<CamOperation>,
}

/// Copies of the part in rows and columns along the setup's X and Y, `dx` and `dy` apart (mm):
/// `nx` × `ny` in all, the first where the part is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Copies {
    pub nx: u32,
    pub ny: u32,
    pub dx: f64,
    pub dy: f64,
}

impl Copies {
    /// Where each copy is, from the first, in rows along X.
    pub fn offsets(&self) -> Vec<(f64, f64)> {
        (0..self.ny).flat_map(|j| (0..self.nx).map(move |i| (f64::from(i) * self.dx, f64::from(j) * self.dy))).collect()
    }
}

/// A box no tool may enter: from `min` to `max` across, in the setup's work coordinates, up to
/// `top` (it stands on whatever is below).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clamp {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub min: [f64; 2],
    pub max: [f64; 2],
    pub top: f64,
}

fn g54() -> u8 {
    54
}

fn is_g54(offset: &u8) -> bool {
    *offset == 54
}

fn world_up() -> Vec3 {
    Vec3::new(0.0, 0.0, 1.0)
}

/// Where a setup's work origin is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SetupOrigin {
    /// A fixed point in the world.
    Point { at: Vec3 },
    /// A point of the box around the stock or the part, in the setup's own axes: a corner, the
    /// middle of an edge or a face, or the centre. It follows the design.
    Box { of: BoxOf, x: Anchor, y: Anchor, z: Anchor },
    /// A point of a body's edge: the end nearest `at`, its middle, or the centre of its circle,
    /// across the tool axis; its height from `z`. It follows the design: the edge is found by
    /// its persistent name `edge` on `body`, else as the piece of it nearest `at`.
    Edge {
        body: String,
        edge: String,
        at: Vec3,
        on: EdgePoint,
        #[serde(default)]
        z: OriginZ,
    },
    /// The work origin of the setup whose clamping it stays in (`Setup::same_clamping`): the
    /// part only turns with A about it, so both share one work offset. That setup's origin
    /// must be on the A axis.
    Clamping,
}

/// Which point of an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgePoint {
    /// The end nearest the point it was picked at (a corner).
    Vertex,
    /// Halfway along it.
    Middle,
    /// The centre of the circle or arc it is.
    Center,
}

/// Where a work origin on an edge is along the tool axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OriginZ {
    /// The edge's point itself.
    Point,
    /// The bottom, middle or top of the box round the stock or the part.
    Box { of: BoxOf, z: Anchor },
}

impl Default for OriginZ {
    fn default() -> Self {
        OriginZ::Box { of: BoxOf::Stock, z: Anchor::Max }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoxOf {
    Stock,
    Part,
}

/// Where along one axis of a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    Min,
    Center,
    Max,
}

/// The stock, in the setup's axes: by its [`StockShape`], the box around the bodies grown by
/// these offsets (length expressions), a box of a set size, or a round bar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StockDef {
    /// Added on each of the four sides.
    pub side: Expr,
    /// Added above the part (towards the tool).
    pub top: Expr,
    /// Added below the part.
    pub bottom: Expr,
    #[serde(default, skip_serializing_if = "StockShape::is_offset")]
    pub shape: StockShape,
}

impl Default for StockDef {
    fn default() -> Self {
        StockDef { side: "0 mm".into(), top: "0 mm".into(), bottom: "0 mm".into(), shape: StockShape::Offset }
    }
}

/// What shape a setup's stock has.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StockShape {
    /// The box around the part grown by the offsets.
    #[default]
    Offset,
    /// A box of this size along the setup's X, Y and Z (length expressions), centred on the part
    /// across and its top `top` above the part's.
    Box { x: Expr, y: Expr, z: Expr },
    /// A round bar along the tool axis this wide (a length expression), centred on the part
    /// across and reaching `top` above and `bottom` below it.
    Round { diameter: Expr },
}

impl StockShape {
    pub fn is_offset(&self) -> bool {
        *self == StockShape::Offset
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CamOperation {
    pub id: u64,
    pub name: String,
    /// Tool number in the library.
    pub tool: u32,
    pub spindle_rpm: f64,
    #[serde(default)]
    pub coolant: bool,
    /// Feed rates (mm/min).
    pub cut_feed: f64,
    pub plunge_feed: f64,
    /// Heights in the setup's Z, as length expressions. One left out follows the geometry:
    /// the sketch plane or picked face, and the picked face's body (see the engine's `cam`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearance: Option<Expr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retract: Option<Expr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<Expr>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<Expr>,
    /// Depth per pass, as a length expression.
    pub stepdown: Expr,
    /// How many depth passes, instead of the stepdown (each then the depth over this).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passes: Option<u32>,
    /// Where to cut.
    pub geometry: CamGeometry,
    pub kind: CamOperationKind,
    /// What particular faces of the part are to a 3D operation; every other face is machined.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub face_roles: Vec<FaceRole>,
}

/// A face of the part and what it is to an operation, other than machined.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaceRole {
    pub face: FaceRef,
    pub role: Role,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Role {
    /// Left out of the part the operation sees: the tool may cut through it.
    Ignore,
    /// Kept at least `clearance` away from (mm).
    Avoid { clearance: f64 },
    /// This much left on it instead of the operation's own allowance (mm).
    Leave { stock_to_leave: f64 },
}

/// What an operation's regions come from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CamGeometry {
    /// Profiles of a sketch.
    Sketch { sketch: u64, profiles: ProfileSel },
    /// The outlines of planar body faces (their inner loops are islands, or holes to drill).
    Faces { faces: Vec<FaceRef> },
    /// The stock's top, seen along the tool: what a facing operation covers.
    Stock,
    /// The box round the bodies, seen along the tool.
    Part,
}

/// A body face: the one with the persistent name `name` on `body`, else the piece of it nearest `at`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaceRef {
    pub body: String,
    pub name: String,
    pub at: Vec3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CamOperationKind {
    /// Faces the box around its boundary (profiles, faces, the stock or the part).
    Face {
        stepover: Expr,
        /// The tool stays inside the boundary; otherwise it cuts over it.
        #[serde(default)]
        inside: bool,
        /// How far inside or over the boundary (a length expression); left out: over by the tool's radius, inside by nothing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        margin: Option<Expr>,
    },
    Contour {
        side: Side,
        climb: bool,
        stock_to_leave: Expr,
        /// Steepest descent into each depth pass (an angle expression; 0 plunges).
        #[serde(default = "default_ramp")]
        ramp_angle: Expr,
        /// Tabs left standing to hold a part being cut out.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tabs: Option<TabsDef>,
        /// Allowance the depth passes leave on the wall for one finishing pass at full depth (a length expression).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finish: Option<Expr>,
        /// Radius of the quarter-circle lead-in and lead-out onto the wall (a length expression; none: it ramps along the loop).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lead: Option<Expr>,
        /// Dogbones in the sharp inside corners the round tool cannot reach.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        dogbones: bool,
        /// An onion skin left on the bottom (a length expression), taken off in a last light pass when `skin_last`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skin: Option<Expr>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        skin_last: bool,
    },
    Pocket {
        stepover: Expr,
        climb: bool,
        stock_to_leave: Expr,
        #[serde(default = "default_ramp")]
        ramp_angle: Expr,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finish: Option<Expr>,
        /// Rest machining: the larger tool (its number) that cleared the pocket before; only what it left is cut.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rest_from: Option<u32>,
        /// Dogbones in the pocket's sharp corners.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        dogbones: bool,
    },
    /// Clears a pocket at a steady load on the tool: a cut `stepover_percent` of its diameter
    /// wide at most, entering on a helix (`helix_angle`, an angle expression; its diameter
    /// `helix_max` to `helix_min` percent of the tool's).
    Adaptive {
        stepover_percent: f64,
        climb: bool,
        stock_to_leave: Expr,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        finish: Option<Expr>,
        helix_angle: Expr,
        helix_max: f64,
        helix_min: f64,
        /// Rest machining: the larger tool (its number) that cleared the pocket before.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rest_from: Option<u32>,
        /// How fast a step lighter than the stepover may go, in percent of the feed (none: 100,
        /// never faster than the feed).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        feed_up: Option<f64>,
        /// Roughing in 3D: level by level down to the model's shape, inside the geometry (the
        /// stock when none), rather than a pocket of the geometry's profiles.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        model: bool,
        /// Roughing in 3D: levels in between on slopes, this far apart at most (a length
        /// expression; none: a quarter of the stepdown; 0: none).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step_up: Option<Expr>,
    },
    /// Chamfers top edges with a pointed tool: `width` across the top, its point `tip_offset`
    /// deeper than the chamfer (length expressions).
    Chamfer {
        side: Side,
        climb: bool,
        width: Expr,
        tip_offset: Expr,
        #[serde(default = "default_ramp")]
        ramp_angle: Expr,
    },
    /// Letters and shapes V-carved with a V-bit: rings inside the outlines, each where the tool's
    /// cone just touches the walls, deeper the farther from them.
    VCarve {
        /// Distance between the rings (a length expression).
        stepover: Expr,
        /// Deepest the point goes below the top (a length expression; none: as deep as the shape asks).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_depth: Option<Expr>,
        climb: bool,
    },
    /// Finishes the model's shape in 3D with passes side by side `stepover` apart (a length
    /// expression), running `angle` degrees from X, following the model with `stock_to_leave`
    /// left on it, back and forth when `zigzag`; with `rest`, only where earlier operations
    /// left material on the model.
    Parallel3d {
        stepover: Expr,
        angle: f64,
        stock_to_leave: Expr,
        zigzag: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        rest: bool,
    },
    /// Finishes the model's steep walls in 3D level by level, `step` apart (a length
    /// expression), the tool running round the model where it touches it, where the model is
    /// at least `steep` degrees steep, with `stock_to_leave` left on it; with `rest`, only where
    /// earlier operations left material on the model.
    ZLevel3d {
        step: Expr,
        stock_to_leave: Expr,
        steep: f64,
        climb: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        rest: bool,
    },
    /// A thread milled with a single-profile thread mill round each round profile: inside a
    /// hole (drawn at its core diameter) or round a boss (drawn at its major diameter).
    Thread {
        /// How far the helix rises in one turn (a length expression).
        pitch: Expr,
        /// The thread's major diameter (a length expression; none: from the drawn circle and the pitch).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diameter: Option<Expr>,
        internal: bool,
        climb: bool,
        /// Passes out to the full thread depth.
        passes: u32,
        /// A left-hand thread; otherwise right-hand.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        left_hand: bool,
    },
    /// Mills round holes larger than the tool on a helix, `pitch` (a length expression) down a turn.
    Bore { climb: bool, stock_to_leave: Expr, pitch: Expr },
    /// Drills at the centre of each profile.
    Drill {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        peck: Option<Expr>,
        #[serde(default)]
        dwell: f64,
    },
}

/// A contour's tabs: how many on each loop, how long along the part's edge and how high over
/// the bottom (length expressions).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabsDef {
    pub count: u32,
    pub width: Expr,
    pub height: Expr,
}

fn default_ramp() -> Expr {
    "3 deg".into()
}

impl CamOperationKind {
    pub fn label(&self) -> &'static str {
        match self {
            CamOperationKind::Face { .. } => "face",
            CamOperationKind::Contour { .. } => "contour",
            CamOperationKind::Pocket { .. } => "pocket",
            CamOperationKind::Adaptive { model: true, .. } => "adaptive_3d",
            CamOperationKind::Adaptive { .. } => "adaptive",
            CamOperationKind::Chamfer { .. } => "chamfer",
            CamOperationKind::VCarve { .. } => "vcarve",
            CamOperationKind::Parallel3d { .. } => "parallel_3d",
            CamOperationKind::ZLevel3d { .. } => "zlevel_3d",
            CamOperationKind::Thread { .. } => "thread",
            CamOperationKind::Bore { .. } => "bore",
            CamOperationKind::Drill { .. } => "drill",
        }
    }
}
