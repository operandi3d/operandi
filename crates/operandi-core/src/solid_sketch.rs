//! Solid Sketch (spec `solid-sketch` v0.3, sections 3, 6 and 8): a tree of
//! nodes, each a soft box, joined into one closed quad cage.
//!
//! The cage follows the spec's loop model: every node is a box with two
//! four-sided loops across its run, at ±Size X from its centre (Size is a
//! half-size), linked boxes are joined by a 4-quad tube between facing
//! faces, and free ends keep their cap. So a chain of n well-spaced nodes has
//! 8n − 2 quads and each branch adds 8 (spec 6, test 11b).
//!
//! Provisional ([OP] in the spec, open questions logged as SS-Q* in
//! `docs/SPECIFICATION_QUESTIONS.md`):
//! - Node frame: X along the run from the parent (the root: towards its
//!   first child, else the work plane's X), Y carried along from the parent
//!   by the smallest turn, then the node's own Rotate and Twist.
//! - A link uses the parent's free face that best faces the child (Q2). A
//!   node with no free face left (more than five links) gets a separate
//!   closed box instead of splitting a face (SS-Q02).
//! - Auto Merge: when a link's two facing faces overlap (the child's face
//!   is behind the parent's along the run), the two loops become one, which
//!   drops a loop and its 4 tube quads (Q8b, SS-Q03).
//! - Negative is stored but builds nothing different yet (Q9).
//!
//! Everything here is in the mesh item's local space and has no UI.
use crate::{Face, Mesh, PolygonRepresentation, Result};
use glam::{DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// Smallest half-size a node keeps, so its box never collapses (spec 8.6).
pub const MIN_SIZE: f64 = 1e-4;
/// Size of a node made by a plain click: 100 mm half-size (spec 5.1).
pub const CLICK_SIZE: f64 = 0.1;
/// Most nodes one sketch keeps (a guard, not a spec value).
pub const MAX_NODES: usize = 4096;

/// One control point of the sketch (spec 3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchNode {
    pub center: DVec3,
    /// Half-size along the node's own X (run), Y and Z.
    pub size: DVec3,
    /// Spin of the cross-section round the run, radians.
    pub twist: f64,
    /// The node's own turn from the rotate rings (spec 5.2).
    #[serde(default = "identity")]
    pub rotation: DQuat,
    /// Parent index; always lower than this node's own index. `None` only
    /// for the root.
    pub parent: Option<usize>,
    /// Symmetry twin (SS-SYM-01, `solid_sketch_symmetry.rs`): the node that
    /// mirrors this one across [`SolidSketch::mirror`]; the two point at
    /// each other. The lower index leads: the other's frame is its mirror.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub twin: Option<usize>,
    /// A seam node (SS-SYM-02): on the mirror plane, no twin, its box kept
    /// mirror-symmetric.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub seam: bool,
}
fn identity() -> DQuat {
    DQuat::IDENTITY
}
impl SketchNode {
    pub fn new(center: DVec3, size: f64, parent: Option<usize>) -> Self {
        Self {
            center,
            size: DVec3::splat(size.max(MIN_SIZE)),
            twist: 0.,
            rotation: DQuat::IDENTITY,
            parent,
            twin: None,
            seam: false,
        }
    }
}

/// Subdiv Mode (`subdiv`): none gives the cage as plain polygons, Level 1
/// the same cage marked as subdivision polygons, Level 2 the cage with one
/// Catmull-Clark step baked in, also marked (spec 4, 6 rule 4, Q13).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubdivMode {
    None,
    Level1,
    #[default]
    Level2,
}
impl SubdivMode {
    pub const ALL: [SubdivMode; 3] = [SubdivMode::None, SubdivMode::Level1, SubdivMode::Level2];
    pub fn token(self) -> &'static str {
        match self {
            SubdivMode::None => "none",
            SubdivMode::Level1 => "level1",
            SubdivMode::Level2 => "level2",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SubdivMode::None => "none",
            SubdivMode::Level1 => "Level 1",
            SubdivMode::Level2 => "Level 2",
        }
    }
    pub fn index(self) -> u8 {
        self as u8
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "none" | "0" => Some(SubdivMode::None),
            "level1" | "1" => Some(SubdivMode::Level1),
            "level2" | "2" => Some(SubdivMode::Level2),
            _ => None,
        }
    }
}

/// Level 2 (spec 6 rule 4): one Catmull-Clark step of the cage, written
/// out and marked as subdivision polygons. Reuses the subdivision module's
/// Freeze at level 1; the cage is closed, so the boundary rule never applies.
fn bake_one_step(cage: &Mesh) -> Result<Mesh> {
    let settings = crate::subdivision::SubdivisionSettings {
        subdivision_level: 1,
        catmull_clark_level: 1,
        ..Default::default()
    };
    let mut mesh = crate::subdivision::freeze(cage, &settings)?;
    mesh.attributes.representations = vec![PolygonRepresentation::Subdivs; mesh.faces.len()];
    Ok(mesh)
}

/// Edit Mode (`emod`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditMode {
    #[default]
    CreateMoveScale,
    Destroy,
}
impl EditMode {
    pub fn token(self) -> &'static str {
        match self {
            EditMode::CreateMoveScale => "createMoveScale",
            EditMode::Destroy => "destroy",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            EditMode::CreateMoveScale => "Create / Move / Scale",
            EditMode::Destroy => "Destroy",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "createMoveScale" | "0" => Some(EditMode::CreateMoveScale),
            "destroy" | "1" => Some(EditMode::Destroy),
            _ => None,
        }
    }
}

/// UV projection (`uvProj`, hidden, unused: Q15). Only the default is named.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UvProjection {
    #[default]
    Default,
    Other(i64),
}
impl UvProjection {
    pub fn token(self) -> String {
        match self {
            UvProjection::Default => "default".into(),
            UvProjection::Other(n) => n.to_string(),
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "default" | "0" => Some(UvProjection::Default),
            n => n.parse().ok().map(UvProjection::Other),
        }
    }
}

/// Hidden arguments: accepted and stored so scripts don't break, unused
/// (spec 4, Q15).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HiddenArgs {
    /// Percent.
    pub tension: f64,
    pub material: String,
    pub part: String,
    pub uvmap: String,
    pub uv_projection: UvProjection,
    pub cen_u: f64,
    pub cen_v: f64,
    /// Radians.
    pub rot_u: f64,
}

/// Per-tool settings of a Solid Sketch session (spec 4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SketchOptions {
    pub edit_mode: EditMode,
    pub subdiv: SubdivMode,
    pub rotate_handles: bool,
    pub auto_align: bool,
    pub auto_merge: bool,
    /// Stored, not built yet (Q9).
    pub negative: bool,
    pub hidden: HiddenArgs,
}
impl Default for SketchOptions {
    fn default() -> Self {
        Self {
            edit_mode: EditMode::CreateMoveScale,
            subdiv: SubdivMode::Level2,
            rotate_handles: false,
            // Q18: off, as the tool's built-in defaults.
            auto_align: false,
            auto_merge: false,
            negative: false,
            hidden: HiddenArgs::default(),
        }
    }
}
impl SketchOptions {
    /// The `SS Default` preset the button and menu run (spec 2).
    pub fn ss_default() -> Self {
        Self {
            subdiv: SubdivMode::Level1,
            edit_mode: EditMode::CreateMoveScale,
            ..Self::default()
        }
    }
}

/// The node tree plus the work-plane normal the root's frame is built from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolidSketch {
    pub nodes: Vec<SketchNode>,
    /// Work-plane normal, local space; the root's Y leans this way.
    pub up: DVec3,
    /// The plane the sketch's twins mirror across (SS-SYM-08), local space;
    /// `None` for a sketch made without Symmetry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<crate::solid_sketch_symmetry::SketchMirror>,
}
impl Default for SolidSketch {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            up: DVec3::Y,
            mirror: None,
        }
    }
}

/// A node's axes in local space, after Rotate and Twist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeFrame {
    pub x: DVec3,
    pub y: DVec3,
    pub z: DVec3,
}

/// The six faces of a node box by corner index (bit 0 = +X, 1 = +Y, 2 = +Z),
/// wound outward: −X, +X, −Y, +Y, −Z, +Z.
const BOX_FACES: [[usize; 4]; 6] = [
    [0, 4, 6, 2],
    [1, 3, 7, 5],
    [0, 1, 5, 4],
    [2, 6, 7, 3],
    [0, 2, 3, 1],
    [4, 5, 7, 6],
];

fn perpendicular(v: DVec3, x: DVec3) -> DVec3 {
    let p = v - x * v.dot(x);
    if p.length_squared() > 1e-18 {
        p.normalize()
    } else {
        x.any_orthonormal_vector()
    }
}
fn direction(from: DVec3, to: DVec3) -> Option<DVec3> {
    let d = to - from;
    (d.length_squared() > 1e-18).then(|| d.normalize())
}

impl SolidSketch {
    pub fn new(up: DVec3) -> Self {
        Self {
            nodes: Vec::new(),
            up: up.try_normalize().unwrap_or(DVec3::Y),
            mirror: None,
        }
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Checks the stored tree: finite values, parents before children.
    pub fn validate(&self) -> Result<()> {
        if self.nodes.len() > MAX_NODES {
            return Err(format!("Solid Sketch keeps at most {MAX_NODES} nodes"));
        }
        if !self.up.is_finite() || self.up.length_squared() < 1e-18 {
            return Err("Solid Sketch needs a work plane".into());
        }
        for (i, n) in self.nodes.iter().enumerate() {
            let finite = n.center.is_finite()
                && n.size.is_finite()
                && n.twist.is_finite()
                && n.rotation.is_finite()
                && n.center.abs().max_element() <= 1e8
                && n.size.max_element() <= 1e8;
            if !finite {
                return Err(format!("Solid Sketch node {i} has an invalid value"));
            }
            if let Some(t) = n.twin
                && (t == i
                    || self.nodes.get(t).is_none_or(|m| m.twin != Some(i))
                    || self.mirror.is_none())
            {
                return Err(format!("Solid Sketch node {i} has a broken symmetry twin"));
            }
            match n.parent {
                None if i != 0 => return Err(format!("Solid Sketch node {i} has no parent")),
                Some(p) if p >= i => {
                    return Err(format!("Solid Sketch node {i} must come after its parent"));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Each node's children, in index order.
    pub fn children(&self) -> Vec<Vec<usize>> {
        let mut out = vec![Vec::new(); self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            if let Some(p) = n.parent {
                out[p].push(i);
            }
        }
        out
    }

    /// `index` and every node below it, in index order.
    pub fn branch(&self, index: usize) -> Vec<usize> {
        let mut inside = vec![false; self.nodes.len()];
        let mut out = Vec::new();
        for i in index..self.nodes.len() {
            let below = i == index || self.nodes[i].parent.is_some_and(|p| inside[p]);
            if below {
                inside[i] = true;
                out.push(i);
            }
        }
        out
    }

    /// Node axes (spec 6 point 1, Auto Align point 5).
    pub fn frames(&self, auto_align: bool) -> Vec<NodeFrame> {
        let children = self.children();
        // Frames before Twist, which children are carried from.
        let mut untwisted: Vec<NodeFrame> = Vec::with_capacity(self.nodes.len());
        let mut out = Vec::with_capacity(self.nodes.len());
        for (i, node) in self.nodes.iter().enumerate() {
            let out_dir = children[i]
                .first()
                .and_then(|&c| direction(node.center, self.nodes[c].center));
            let inverse = node.rotation.inverse();
            let (x, y) = match node.parent {
                None => {
                    // The root faces its first child, so its own Rotate
                    // only spins it about that run (the child already
                    // swung round).
                    let x = out_dir.map_or(plane_x(self.up), |d| inverse * d);
                    (x, perpendicular(inverse * self.up, x))
                }
                Some(p) => {
                    let parent = untwisted[p];
                    let in_dir = direction(self.nodes[p].center, node.center).unwrap_or(parent.x);
                    let x = match out_dir {
                        Some(d) if auto_align => (in_dir + d).try_normalize().unwrap_or(in_dir),
                        _ => in_dir,
                    };
                    let carried = DQuat::from_rotation_arc(parent.x, x) * parent.y;
                    (x, perpendicular(carried, x))
                }
            };
            let (x, y) = (node.rotation * x, node.rotation * y);
            let z = x.cross(y);
            let spin = DQuat::from_axis_angle(x, node.twist);
            let mut frame = NodeFrame {
                x,
                y: spin * y,
                z: spin * z,
            };
            let mut plain = NodeFrame { x, y, z };
            // Symmetry (SS-SYM-01/02): a twin's frame is its leader's,
            // mirrored; a seam node's box is kept mirror-symmetric.
            if let Some(m) = self.mirror {
                match node.twin {
                    Some(t) if t < i => {
                        frame = m.mirror_frame(out[t]);
                        plain = m.mirror_frame(untwisted[t]);
                    }
                    _ if node.seam => {
                        frame = m.seam_frame(frame);
                        plain = frame;
                    }
                    _ => {}
                }
            }
            untwisted.push(plain);
            out.push(frame);
        }
        out
    }

    /// The eight corners of node `index` (corner bit 0 = +X, 1 = +Y, 2 = +Z).
    fn corners(&self, index: usize, frame: &NodeFrame) -> [DVec3; 8] {
        let n = &self.nodes[index];
        let s = n.size.max(DVec3::splat(MIN_SIZE));
        std::array::from_fn(|k| {
            let sign = |bit: usize| if k & bit != 0 { 1. } else { -1. };
            n.center
                + frame.x * (s.x * sign(1))
                + frame.y * (s.y * sign(2))
                + frame.z * (s.z * sign(4))
        })
    }

    /// The sketch's output mesh (spec 6 rule 4). Subdiv Mode none: the cage
    /// as plain quads. Level 1: the same quads marked as subdivision
    /// polygons. Level 2: one Catmull-Clark step baked into the cage (4× the
    /// quads), marked as subdivision polygons too.
    pub fn cage(&self, options: &SketchOptions) -> Result<Mesh> {
        self.validate()?;
        if self.nodes.is_empty() {
            return Ok(Mesh::default());
        }
        let mut mesh = None;
        if options.auto_merge {
            let merged = self.build(options, true)?;
            if merged.validate().is_ok() {
                mesh = Some(merged);
            }
        }
        let mesh = match mesh {
            Some(m) => m,
            None => {
                let m = self.build(options, false)?;
                m.validate()?;
                m
            }
        };
        if options.subdiv == SubdivMode::Level2 {
            return bake_one_step(&mesh);
        }
        Ok(mesh)
    }

    fn build(&self, options: &SketchOptions, merge: bool) -> Result<Mesh> {
        let frames = self.frames(options.auto_align);
        let count = self.nodes.len();
        let mut points: Vec<DVec3> = Vec::with_capacity(count * 8);
        for (i, frame) in frames.iter().enumerate() {
            points.extend(self.corners(i, frame));
        }
        let corner = |node: usize, k: usize| (node * 8 + k) as u32;
        let normal = |node: usize, face: usize| {
            let f = frames[node];
            [-f.x, f.x, -f.y, f.y, -f.z, f.z][face]
        };
        let face_center = |points: &[DVec3], node: usize, face: usize| {
            BOX_FACES[face]
                .iter()
                .map(|&k| points[node * 8 + k])
                .sum::<DVec3>()
                / 4.
        };
        let mut used = vec![[false; 6]; count];
        // The faces each link took, by child: (parent face, child face).
        let mut chosen: Vec<Option<(usize, usize)>> = vec![None; count];
        // The free face of `node` whose outward normal is `v` (frames are
        // exact mirrors, so the match is within rounding).
        let face_along = |node: usize, v: DVec3, used: &[[bool; 6]]| {
            (0..6).find(|&f| !used[node][f] && normal(node, f).dot(v) > 1. - 1e-6)
        };
        let mut bridges: Vec<[u32; 4]> = Vec::new();
        // Pairs of points that become one (Auto Merge).
        let mut same: Vec<u32> = (0..points.len() as u32).collect();
        for child in 0..count {
            let Some(parent) = self.nodes[child].parent else {
                continue;
            };
            let dir = direction(self.nodes[parent].center, self.nodes[child].center)
                .unwrap_or(frames[parent].x);
            let best = |node: usize, toward: DVec3, used: &[[bool; 6]]| {
                (0..6).filter(|&f| !used[node][f]).max_by(|&a, &b| {
                    normal(node, a)
                        .dot(toward)
                        .total_cmp(&normal(node, b).dot(toward))
                })
            };
            // Symmetry (SS-SYM-03/05): a twin's link takes the mirror
            // images of the faces its leader's link took, so the cage is
            // symmetric in its polygons, not only its points.
            let mirrored = self.mirror.and_then(|m| {
                let lead = self.nodes[child].twin.filter(|&t| t < child)?;
                let lead_parent = self.nodes[lead].parent?;
                if self.nodes[lead_parent].twin.unwrap_or(lead_parent) != parent {
                    return None;
                }
                let (lpf, lcf) = chosen[lead]?;
                Some((
                    face_along(parent, m.vector(normal(lead_parent, lpf)), &used)?,
                    face_along(child, m.vector(normal(lead, lcf)), &used)?,
                ))
            });
            // A leader under a seam node shares that box with its twin, so
            // it leaves through a face on its own side. Otherwise the face
            // that best faces both (a hip's bottom, for two legs) went to
            // the leader and the twin was pushed onto another face, fusing
            // the legs into one lopsided blob (Jason, 2026-11-14).
            let own_side = self.mirror.and_then(|m| {
                let twin = self.nodes[child].twin.filter(|&t| t > child)?;
                if !self.nodes[parent].seam || self.nodes[twin].parent != Some(parent) {
                    return None;
                }
                let at = self.nodes[child].center;
                let side = m.normal * (m.normal.dot(at) - m.offset).signum();
                (0..6)
                    .filter(|&f| !used[parent][f] && normal(parent, f).dot(side) > 0.5)
                    .max_by(|&a, &b| {
                        normal(parent, a)
                            .dot(dir)
                            .total_cmp(&normal(parent, b).dot(dir))
                    })
            });
            let pick = mirrored.or_else(|| {
                let pf = own_side.or_else(|| best(parent, dir, &used))?;
                Some((pf, best(child, -dir, &used)?))
            });
            let Some((pf, cf)) = pick else {
                // No free face on the parent: a separate box (SS-Q02).
                continue;
            };
            chosen[child] = Some((pf, cf));
            used[parent][pf] = true;
            used[child][cf] = true;
            let a: [u32; 4] = BOX_FACES[pf].map(|k| corner(parent, k));
            let b: [u32; 4] = BOX_FACES[cf].map(|k| corner(child, k));
            // Match the child's loop, reversed, to the parent's by the
            // least travel, so the tube never twists (spec 8.8).
            let x_at = |shift: usize, i: usize| b[(shift + 4 - i) % 4];
            let shift = (0..4)
                .min_by(|&s, &t| {
                    let cost = |s: usize| {
                        (0..4)
                            .map(|i| {
                                points[a[i] as usize].distance_squared(points[x_at(s, i) as usize])
                            })
                            .sum::<f64>()
                    };
                    cost(s).total_cmp(&cost(t))
                })
                .unwrap_or(0);
            let gap = (face_center(&points, child, cf) - face_center(&points, parent, pf)).dot(dir);
            if merge && gap < 0. {
                for (i, &ai) in a.iter().enumerate() {
                    let (p, q) = (find(&same, ai), find(&same, x_at(shift, i)));
                    if p != q {
                        points[p as usize] = (points[p as usize] + points[q as usize]) / 2.;
                        same[q as usize] = p;
                    }
                }
            } else {
                for i in 0..4 {
                    let j = (i + 1) % 4;
                    // Same winding as the two faces it replaces.
                    bridges.push([x_at(shift, j), x_at(shift, i), a[i], a[j]]);
                }
            }
        }
        let mut quads: Vec<[u32; 4]> = Vec::with_capacity(count * 8);
        for (node, faces) in used.iter().enumerate() {
            for (f, &taken) in faces.iter().enumerate() {
                if !taken {
                    quads.push(BOX_FACES[f].map(|k| corner(node, k)));
                }
            }
        }
        quads.extend(bridges);
        // Resolve merges and keep only used points, in first-use order.
        let mut remap = vec![u32::MAX; points.len()];
        let mut vertices = Vec::new();
        let faces = quads
            .iter()
            .map(|q| {
                let vertices_of = q.map(|v| {
                    let r = find(&same, v) as usize;
                    if remap[r] == u32::MAX {
                        remap[r] = vertices.len() as u32;
                        vertices.push(points[r]);
                    }
                    remap[r]
                });
                Face {
                    vertices: vertices_of.to_vec(),
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                }
            })
            .collect::<Vec<_>>();
        let mut mesh = Mesh {
            vertices,
            faces: faces.into(),
            ..Default::default()
        };
        if options.subdiv != SubdivMode::None {
            mesh.attributes.representations =
                vec![PolygonRepresentation::Subdivs; mesh.faces.len()];
        }
        Ok(mesh)
    }

    // ---- Edits (spec 5 and 8). Each keeps the tree valid. ----

    /// Adds a node under `parent` (or as the root) and returns its index.
    pub fn add(&mut self, center: DVec3, size: f64, parent: Option<usize>) -> Result<usize> {
        if self.nodes.len() >= MAX_NODES {
            return Err(format!("Solid Sketch keeps at most {MAX_NODES} nodes"));
        }
        let parent = match parent {
            Some(p) if p < self.nodes.len() => Some(p),
            Some(_) => return Err("The parent node doesn't exist".into()),
            None if self.nodes.is_empty() => None,
            None => Some(self.nodes.len() - 1),
        };
        check_point(center)?;
        self.nodes.push(SketchNode::new(center, size, parent));
        Ok(self.nodes.len() - 1)
    }

    /// Ctrl-insert (spec 5.1, test 11c): a node on the link into `child`,
    /// at `center`; it takes `child`'s index and later nodes move up one.
    pub fn insert(&mut self, child: usize, center: DVec3, size: f64) -> Result<usize> {
        check_point(center)?;
        if self.nodes.len() >= MAX_NODES {
            return Err(format!("Solid Sketch keeps at most {MAX_NODES} nodes"));
        }
        let parent = self
            .nodes
            .get(child)
            .ok_or("That node doesn't exist")?
            .parent
            .ok_or("The root has no link to insert on")?;
        for n in &mut self.nodes {
            if let Some(p) = &mut n.parent
                && *p >= child
            {
                *p += 1;
            }
            if let Some(t) = &mut n.twin
                && *t >= child
            {
                *t += 1;
            }
        }
        self.nodes
            .insert(child, SketchNode::new(center, size, Some(parent)));
        self.nodes[child + 1].parent = Some(child);
        Ok(child)
    }

    /// Deletes one node; its children join its parent (spec 8.3). Deleting
    /// the root makes its first child the root. Returns the node to make
    /// current, if any.
    pub fn delete(&mut self, index: usize) -> Result<Option<usize>> {
        let node = self.nodes.get(index).ok_or("That node doesn't exist")?;
        let parent = node.parent;
        let children: Vec<usize> = self.children()[index].clone();
        let heir = parent.or(children.first().copied());
        for &c in &children {
            self.nodes[c].parent = if Some(c) == heir { None } else { heir };
        }
        self.remove_indices(&[index]);
        Ok(match (parent, heir) {
            (Some(p), _) => Some(p),
            (None, Some(h)) => Some(h - 1),
            _ => None,
        }
        .filter(|_| !self.nodes.is_empty()))
    }

    /// Deletes a node and its whole branch (spec 5.1 Shift + middle click).
    pub fn delete_branch(&mut self, index: usize) -> Result<Option<usize>> {
        let parent = self
            .nodes
            .get(index)
            .ok_or("That node doesn't exist")?
            .parent;
        let branch = self.branch(index);
        self.remove_indices(&branch);
        Ok(parent.filter(|_| !self.nodes.is_empty()))
    }

    pub(crate) fn remove_indices(&mut self, sorted: &[usize]) {
        let mut gone = vec![false; self.nodes.len()];
        for &i in sorted {
            gone[i] = true;
        }
        let mut new_index = vec![usize::MAX; self.nodes.len()];
        let mut next = 0;
        for i in 0..self.nodes.len() {
            if !gone[i] {
                new_index[i] = next;
                next += 1;
            }
        }
        let old = std::mem::take(&mut self.nodes);
        self.nodes = old
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !gone[*i])
            .map(|(_, mut n)| {
                n.parent = n.parent.map(|p| new_index[p]);
                // A twin whose partner went becomes an ordinary node.
                n.twin = n.twin.map(|t| new_index[t]).filter(|&t| t != usize::MAX);
                n
            })
            .collect();
    }

    /// Moves one node (children stay put) or its whole branch by `delta`.
    pub fn translate(&mut self, index: usize, delta: DVec3, branch: bool) -> Result<()> {
        check_point(delta)?;
        let targets = if branch {
            self.branch(index)
        } else {
            vec![index]
        };
        if index >= self.nodes.len() {
            return Err("That node doesn't exist".into());
        }
        for i in targets {
            let moved = self.nodes[i].center + delta;
            check_point(moved)?;
            self.nodes[i].center = moved;
        }
        Ok(())
    }

    /// Scales a node's sizes, or its branch's sizes and spacing about the
    /// node's centre (spec 5.1 right drag, Shift + right drag).
    pub fn scale(&mut self, index: usize, factor: f64, branch: bool) -> Result<()> {
        if !factor.is_finite() || factor <= 0. {
            return Err("Scale must be above 0".into());
        }
        let pivot = self
            .nodes
            .get(index)
            .ok_or("That node doesn't exist")?
            .center;
        let targets = if branch {
            self.branch(index)
        } else {
            vec![index]
        };
        for i in targets {
            let n = &mut self.nodes[i];
            n.size = (n.size * factor).max(DVec3::splat(MIN_SIZE));
            if i != index {
                n.center = pivot + (n.center - pivot) * factor;
            }
        }
        Ok(())
    }

    /// Turns a node and swings its whole branch round the node's centre
    /// (spec 5.2 rotate rings, test 7).
    pub fn rotate(&mut self, index: usize, turn: DQuat) -> Result<()> {
        if !turn.is_finite() {
            return Err("Rotation must be finite".into());
        }
        let turn = turn.normalize();
        let pivot = self
            .nodes
            .get(index)
            .ok_or("That node doesn't exist")?
            .center;
        for i in self.branch(index) {
            let n = &mut self.nodes[i];
            if i == index {
                n.rotation = (turn * n.rotation).normalize();
            } else {
                n.center = pivot + turn * (n.center - pivot);
            }
        }
        Ok(())
    }

    /// Nearest point on the link into `child` to `point`, and its distance.
    pub fn link_point(&self, child: usize, point: DVec3) -> Option<DVec3> {
        let p = self.nodes[self.nodes.get(child)?.parent?].center;
        let c = self.nodes[child].center;
        let d = c - p;
        let t = if d.length_squared() > 1e-18 {
            ((point - p).dot(d) / d.length_squared()).clamp(0., 1.)
        } else {
            0.
        };
        Some(p + d * t)
    }
}

/// Union-find root of point `v` (Auto Merge).
fn find(same: &[u32], mut v: u32) -> u32 {
    while same[v as usize] != v {
        v = same[v as usize];
    }
    v
}

fn check_point(p: DVec3) -> Result<()> {
    if p.is_finite() && p.abs().max_element() <= 1e8 {
        Ok(())
    } else {
        Err("Solid Sketch values must be finite and in range".into())
    }
}

/// The work plane's own X for a lone node: world X unless the plane faces
/// along X, then world Z.
fn plane_x(up: DVec3) -> DVec3 {
    let x = if up.normalize_or(DVec3::Y).x.abs() > 0.9 {
        DVec3::Z
    } else {
        DVec3::X
    };
    perpendicular(x, up.normalize_or(DVec3::Y))
}

#[cfg(test)]
mod tests;
