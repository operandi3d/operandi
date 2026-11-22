//! Falloff (specifications/falloff-2026-11-01): one active weight field that
//! scales each point's share of a transform or a Bevel. Weights are sampled
//! at world positions. Tool context, not scene data.
use crate::{Mesh, Mode, Result, Scene, Selection};
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// FAL-L-03 shapes, as w(t) for t in 0…1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum FalloffShape {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    Smooth,
    /// Fitted to five measured (In, Out) pairs; inferred elsewhere, and which
    /// stored value the UI calls In is inferred too (FAL-Q02).
    Custom {
        ease_in: f64,
        ease_out: f64,
    },
}
impl FalloffShape {
    pub fn weight(self, t: f64) -> f64 {
        let t = t.clamp(0., 1.);
        let s = 1. - t;
        match self {
            Self::Linear => s,
            Self::EaseIn => 1. - t * t,
            Self::EaseOut => s * s,
            Self::Smooth => 1. - (3. * t * t - 2. * t * t * t),
            Self::Custom { ease_in, ease_out } => {
                s * s * s + s * s * t * (2. - ease_out) + s * t * t * (1. + ease_in)
            }
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::EaseIn => "Ease-In",
            Self::EaseOut => "Ease-Out",
            Self::Smooth => "Smooth",
            Self::Custom { .. } => "Custom",
        }
    }
    /// FAL-M-03: Custom starts at In 0, Out 1.
    pub const CUSTOM: Self = Self::Custom {
        ease_in: 0.,
        ease_out: 1.,
    };
    pub(crate) fn validate(self) -> Result<()> {
        if let Self::Custom { ease_in, ease_out } = self
            && !(ease_in.is_finite() && ease_out.is_finite())
        {
            return Err("Custom falloff In and Out must be finite".into());
        }
        Ok(())
    }
}

/// FAL-L-04.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FalloffSymmetric {
    #[default]
    None,
    /// Mirrored about End: past End the weight rises again.
    Start,
    /// Mirrored about Start: before Start the weight falls again.
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearFalloff {
    pub start: DVec3,
    pub end: DVec3,
    #[serde(default)]
    pub shape: FalloffShape,
    #[serde(default)]
    pub symmetric: FalloffSymmetric,
    /// FO-35 Use World Transforms: on, Start and End are world positions
    /// and weights read world positions; off (factory), both are in each
    /// mesh's own space, so copies of a mesh deform alike.
    #[serde(default)]
    pub world: bool,
}
impl Default for LinearFalloff {
    fn default() -> Self {
        Self {
            // FO-40: factory Start 0,0,0 and End 0,1,0 (measured).
            start: DVec3::ZERO,
            end: DVec3::Y,
            shape: FalloffShape::Linear,
            symmetric: FalloffSymmetric::None,
            world: false,
        }
    }
}
impl LinearFalloff {
    pub fn validate(self) -> Result<()> {
        let direction = self.end - self.start;
        if !direction.is_finite()
            || !direction.length_squared().is_finite()
            || !self.start.is_finite()
            || !self.end.is_finite()
            || (self.end - self.start).length_squared() < 1e-20
        {
            return Err("Linear falloff requires two distinct finite world points".into());
        }
        self.shape.validate()
    }
    /// FAL-L-01/02/04: weight from the projection onto Start→End only.
    pub fn weight(self, point: DVec3) -> f64 {
        let d = self.end - self.start;
        let mut t = (point - self.start).dot(d) / d.length_squared();
        match self.symmetric {
            FalloffSymmetric::Start if t > 1. => t = 2. - t,
            FalloffSymmetric::End if t < 0. => t = -t,
            _ => {}
        }
        self.shape.weight(t)
    }
    /// FAL-M-05.
    pub fn reversed(self) -> Self {
        Self {
            start: self.end,
            end: self.start,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialFalloff {
    pub center: DVec3,
    /// The ellipsoid's radii along X, Y, Z (FAL-R-01).
    pub size: DVec3,
    #[serde(default)]
    pub shape: FalloffShape,
    /// FO-35, as for Linear.
    #[serde(default)]
    pub world: bool,
}
impl Default for RadialFalloff {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            size: DVec3::ONE,
            shape: FalloffShape::Linear,
            world: false,
        }
    }
}
impl RadialFalloff {
    pub fn validate(self) -> Result<()> {
        if !self.center.is_finite() || !self.size.is_finite() {
            return Err("Radial falloff needs a finite Center and Size".into());
        }
        self.shape.validate()
    }
    /// FAL-R-01; a zero Size axis doesn't limit the weight (FAL-B2-05).
    pub fn weight(self, point: DVec3) -> f64 {
        let mut sum = 0.;
        for axis in 0..3 {
            let offset = point[axis] - self.center[axis];
            let size = self.size[axis].abs();
            // FAL-B2-05: a zero Size axis is ignored.
            if size != 0. {
                sum += (offset / size).powi(2);
            }
        }
        let d = sum.sqrt();
        if d >= 1. { 0. } else { self.shape.weight(d) }
    }
}

/// FO-42 Cylinder: Radial with the Axis component ignored, so the field is
/// an endless tube along Axis.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CylinderFalloff {
    pub center: DVec3,
    /// The tube's radii; the Axis component is unused by the weight.
    pub size: DVec3,
    /// 0, 1, 2 for X, Y, Z.
    pub axis: usize,
    pub shape: FalloffShape,
    pub world: bool,
}
impl Default for CylinderFalloff {
    /// FO-42 (measured): Center 0,0,0, Size 1,1,1, Axis Y, Linear, World
    /// off.
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            size: DVec3::ONE,
            axis: 1,
            shape: FalloffShape::Linear,
            world: false,
        }
    }
}
impl CylinderFalloff {
    pub fn validate(self) -> Result<()> {
        if !self.center.is_finite() || !self.size.is_finite() {
            return Err("Cylinder falloff needs a finite Center and Size".into());
        }
        if self.axis > 2 {
            return Err("Cylinder Axis must be X, Y or Z".into());
        }
        self.shape.validate()
    }
    /// d = length of the two non-Axis components of (p − Center) / Size;
    /// Shape(d), 0 from d = 1 on. A zero Size axis is ignored, as Radial's.
    pub fn weight(self, point: DVec3) -> f64 {
        let mut sum = 0.;
        for axis in (0..3).filter(|&a| a != self.axis) {
            let size = self.size[axis].abs();
            if size != 0. {
                sum += ((point[axis] - self.center[axis]) / size).powi(2);
            }
        }
        let d = sum.sqrt();
        if d >= 1. { 0. } else { self.shape.weight(d) }
    }
}

// Which tools a falloff scales is each tool's card (`ToolCard::layers`,
// tool_card.rs); the refusal message is `tool_card::falloff_refusal`.

/// FAL-M-01: one falloff at a time; more can be added on top with a Mix
/// Mode (VMF-10, `Editor::added_falloffs`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Falloff {
    Linear(LinearFalloff),
    Radial(RadialFalloff),
    /// FO-42.
    Cylinder(CylinderFalloff),
    /// VMF-01: the selected weight map's values.
    VertexMap(VertexMapFalloff),
    /// VMF-12: 1 − w of the falloffs before it, w clamped to 0…1.
    Invert,
    /// SPEC-SELECTION-FALLOFF-FLEX §1: weights from the component selection.
    Selection(SelectionFalloff),
    /// FO-47: the selection at full weight, fading out over Radius beyond.
    SoftSelection(crate::falloff_reach::SoftSelectionFalloff),
    /// FO-45: the element picked under the pointer, with Range around it.
    Element(crate::falloff_reach::ElementFalloff),
    /// FO-48: the selection plus neighbouring polygons facing within Angle.
    Coplanar(crate::falloff_reach::CoplanarFalloff),
    /// FO-43: a circle on screen, stamped when drawn (FO-Q08).
    Screen(crate::falloff_screen::ScreenFalloff),
    /// FO-52: a shape drawn on screen, stamped when drawn.
    Lasso(crate::falloff_screen::LassoFalloff),
    /// FO-44: weights painted by dragging.
    Airbrush(crate::falloff_screen::AirbrushFalloff),
    /// FO-50: smooth 3D noise.
    Noise(crate::falloff_pattern::NoiseFalloff),
    /// FO-54: a picture's brightness, stamped by the app.
    Image(crate::falloff_pattern::ImageFalloff),
    /// FO-53: a tube around a curve clicked in the view
    /// (`Editor::falloff_path`).
    Path(crate::falloff_path::PathFalloff),
    /// FO-51: how sharply the surface bends.
    Curvature(crate::falloff_path::CurvatureFalloff),
}

/// The Selection falloff (SPEC-SELECTION-FALLOFF-FLEX §1, `falloff.selection`):
/// 0 on the selection's border, rising to 1 over `steps` rings inward;
/// points outside the selection weigh 0.
///
/// Provisional readings (SF-Q1..Q5 in docs/SPECIFICATION_QUESTIONS.md):
/// points carry the weight, ring r weighs `shape(r / steps)`; rings are
/// counted along edges; Polygon mode's border is the points shared with an
/// unselected polygon, Vertex and Edge modes' the selected points with an
/// unselected neighbour; a selection with no border weighs 1 everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SelectionFalloff {
    pub steps: u32,
    pub shape: FalloffShape,
}
impl Default for SelectionFalloff {
    /// The registry's defaults: Steps 2, Shape Smooth (SF-Q5).
    fn default() -> Self {
        Self {
            steps: 2,
            shape: FalloffShape::Smooth,
        }
    }
}
/// A point outside the selection (`selection_rings`).
pub const RING_OUTSIDE: u32 = u32::MAX;
/// A selected point no border reaches: full weight.
pub const RING_INNER: u32 = u32::MAX - 1;
pub const MAX_SELECTION_STEPS: u32 = 1000;
impl SelectionFalloff {
    pub fn validate(self) -> Result<()> {
        if !(1..=MAX_SELECTION_STEPS).contains(&self.steps) {
            return Err(format!(
                "Selection falloff Steps must be a whole number from 1 to {MAX_SELECTION_STEPS}"
            ));
        }
        self.shape.validate()
    }
    /// The weight of a point `ring` rings in from the selection's border.
    pub fn weight(self, ring: u32) -> f64 {
        match ring {
            RING_OUTSIDE => 0.,
            RING_INNER => 1.,
            r => {
                let t = (r as f64 / self.steps.max(1) as f64).min(1.);
                // The shape curves run from full (t = 0) to none (t = 1);
                // here full is inside, so the ramp is read from the inside.
                self.shape.weight(1. - t)
            }
        }
    }
}

/// How many rings in from the selection's border each point of `mesh`
/// lies, counted along edges: 0 on the border, [`RING_INNER`] for a
/// selected point no border reaches, [`RING_OUTSIDE`] for unselected
/// points. Works near the selection: only selected points and their
/// polygons are read (Mesh::point_faces is the shared per-topology memo).
pub fn selection_rings(mesh: &Mesh, selection: &Selection) -> Vec<u32> {
    let n = mesh.vertices.len();
    let faces = mesh.point_faces();
    let polygon_mode = selection.mode == Mode::Polygon && !selection.faces.is_empty();
    let mut selected_face = crate::fxhash::FxHashMap::default();
    let mut points: Vec<u32> = Vec::new();
    match selection.mode {
        Mode::Item => return vec![RING_INNER; n],
        Mode::Vertex => points.extend(selection.vertices.iter().copied()),
        Mode::Edge => points.extend(selection.edges.iter().flat_map(|&(a, b)| [a, b])),
        Mode::Polygon => {
            for &f in &selection.faces {
                if let Some(face) = mesh.faces.get(f as usize) {
                    selected_face.insert(f, ());
                    points.extend(face.vertices.iter().copied());
                }
            }
        }
    }
    points.retain(|&v| (v as usize) < n);
    if points.is_empty() {
        // Nothing selected: the tool works on the whole mesh, which has no
        // selection border.
        return vec![RING_INNER; n];
    }
    let mut ring = vec![RING_OUTSIDE; n];
    for &v in &points {
        ring[v as usize] = RING_INNER;
    }
    // The points next to `v` along polygon sides; with a polygon selection
    // only the selected polygons' sides count.
    let neighbours = |v: u32, out: &mut Vec<u32>| {
        out.clear();
        for &f in faces.of(v) {
            if polygon_mode && !selected_face.contains_key(&f) {
                continue;
            }
            let corners = &mesh.faces[f as usize].vertices;
            let len = corners.len();
            if len < 2 {
                continue;
            }
            let closed = !mesh.is_polyline(f as usize)
                || matches!(
                    mesh.attributes.representations.get(f as usize),
                    Some(crate::PolygonRepresentation::Polyline { closed: true, .. })
                );
            for (i, &c) in corners.iter().enumerate() {
                if c != v {
                    continue;
                }
                if i + 1 < len || closed {
                    out.push(corners[(i + 1) % len]);
                }
                if i > 0 || closed {
                    out.push(corners[(i + len - 1) % len]);
                }
            }
        }
    };
    let mut queue = std::collections::VecDeque::new();
    let mut around = Vec::new();
    points.sort_unstable();
    points.dedup();
    for &v in &points {
        let border = if polygon_mode {
            faces.of(v).iter().any(|f| !selected_face.contains_key(f))
        } else {
            neighbours(v, &mut around);
            around.iter().any(|&u| ring[u as usize] == RING_OUTSIDE)
        };
        if border {
            ring[v as usize] = 0;
            queue.push_back(v);
        }
    }
    while let Some(v) = queue.pop_front() {
        let next = ring[v as usize] + 1;
        neighbours(v, &mut around);
        for &u in &around {
            if ring[u as usize] == RING_INNER {
                ring[u as usize] = next;
                queue.push_back(u);
            }
        }
    }
    ring
}

thread_local! {
    /// The last ring result: a drag reads the same selection on the same
    /// topology every frame (Fast drags: computed once per drag).
    static RINGS: std::cell::RefCell<Option<RingMemo>> = const { std::cell::RefCell::new(None) };
}
struct RingMemo {
    faces: crate::shared::WeakShared<Vec<crate::Face>>,
    points: usize,
    selection: Selection,
    rings: std::rc::Rc<Vec<u32>>,
}

/// Item `id`'s own component selection (it may be one of several active
/// meshes).
pub(crate) fn item_selection(scene: &Scene, id: u64) -> Selection {
    if scene.selection.item == Some(id) {
        let mut s = scene.selection.clone();
        s.others = Default::default();
        s
    } else {
        scene.selection.sole(id)
    }
}

/// [`selection_rings`] of item `id` in `scene` (its own components when it
/// is one of several active meshes), remembered while the topology and
/// selection stay the same.
pub fn item_selection_rings(scene: &Scene, id: u64) -> Result<std::rc::Rc<Vec<u32>>> {
    let item = scene
        .items
        .iter()
        .find(|i| i.id == id)
        .ok_or("Missing falloff item")?;
    let selection = item_selection(scene, id);
    let mesh = &item.mesh;
    let hit = RINGS.with_borrow(|m| {
        m.as_ref()
            .filter(|m| {
                m.faces.is(&mesh.faces)
                    && m.points == mesh.vertices.len()
                    && m.selection.mode == selection.mode
                    && m.selection.vertices == selection.vertices
                    && m.selection.edges == selection.edges
                    && m.selection.faces == selection.faces
            })
            .map(|m| m.rings.clone())
    });
    if let Some(rings) = hit {
        return Ok(rings);
    }
    let rings = std::rc::Rc::new(selection_rings(mesh, &selection));
    RINGS.set(Some(RingMemo {
        faces: mesh.faces.downgrade(),
        points: mesh.vertices.len(),
        selection,
        rings: rings.clone(),
    }));
    Ok(rings)
}

/// Per-point values some falloffs read besides the position: the selected
/// weight map (Vertex Map) and the selection rings (Selection).
#[derive(Clone, Copy, Debug, Default)]
pub struct PointInputs<'a> {
    pub map: Option<&'a std::collections::BTreeMap<u32, f64>>,
    pub rings: Option<&'a [u32]>,
    /// Per-layer point weights worked out ahead (Soft Selection, Element,
    /// Coplanar), indexed like the stack: 0 the first layer, then the
    /// added ones (`ActiveFalloff::prepare`).
    pub layers: &'a [Option<std::sync::Arc<Vec<f64>>>],
}

/// What a stack reads per point of one item, worked out once
/// (`ActiveFalloff::prepare`); `inputs` lends it to `weight_at`.
#[derive(Clone, Debug, Default)]
pub struct Prepared {
    pub map: Option<std::collections::BTreeMap<u32, f64>>,
    pub rings: Option<std::rc::Rc<Vec<u32>>>,
    pub layers: Vec<Option<std::sync::Arc<Vec<f64>>>>,
}
impl Prepared {
    pub fn inputs(&self) -> PointInputs<'_> {
        PointInputs {
            map: self.map.as_ref(),
            rings: self.rings.as_deref().map(Vec::as_slice),
            layers: &self.layers,
        }
    }
}

/// VMF-01 Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VertexMapMode {
    #[default]
    Magnitude,
    Component,
}

/// The Vertex Map falloff (VMF-01). It has no map field: it reads the
/// selected weight map (VMF-05).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VertexMapFalloff {
    pub mode: VertexMapMode,
    /// Component index; a weight map has one value, so every index reads
    /// it (VMF-07).
    pub index: u32,
}

/// VMF-11: how an added falloff combines with the result of the ones
/// before it. Unclamped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MixMode {
    #[default]
    Multiply,
    Add,
    Subtract,
    Max,
    Min,
}

impl MixMode {
    pub const ALL: [MixMode; 5] = [
        Self::Multiply,
        Self::Add,
        Self::Subtract,
        Self::Max,
        Self::Min,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Multiply => "Multiply",
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Max => "Max",
            Self::Min => "Min",
        }
    }
    /// `before` combined with `this`.
    pub fn mix(self, before: f64, this: f64) -> f64 {
        match self {
            Self::Multiply => before * this,
            Self::Add => before + this,
            Self::Subtract => before - this,
            Self::Max => before.max(this),
            Self::Min => before.min(this),
        }
    }
}

/// The whole falloff for one evaluation: the active falloff, the ones
/// added on top (each with its Mix Mode), the selected weight map's name,
/// which Vertex Map falloffs read (VM-02), and the Element falloff's pick
/// with the Symmetry it mirrors through (FO-45).
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveFalloff {
    pub first: Falloff,
    pub added: Vec<(Falloff, MixMode)>,
    pub weight_map: Option<String>,
    pub element: Option<crate::TweakGrab>,
    pub symmetry: crate::Symmetry,
    /// Screen, Lasso and Airbrush weights per (layer, item), worked out
    /// when drawn or painted (`Editor::falloff_stamps`).
    pub stamps: std::collections::BTreeMap<(usize, u64), std::sync::Arc<Vec<f64>>>,
    /// The Path falloff's control points (FO-53).
    pub path: Vec<DVec3>,
}

impl ActiveFalloff {
    /// A stack of `first` alone, with no weight map, pick or Symmetry.
    pub fn new(first: Falloff) -> Self {
        Self {
            first,
            added: Vec::new(),
            weight_map: None,
            element: None,
            symmetry: crate::Symmetry::default(),
            stamps: Default::default(),
            path: Vec::new(),
        }
    }
    fn layers(&self) -> impl Iterator<Item = Falloff> + '_ {
        std::iter::once(self.first).chain(self.added.iter().map(|&(f, _)| f))
    }
    /// Whether any layer reads the weight map (then weights need the point
    /// index, not just a position).
    pub fn reads_map(&self) -> bool {
        self.layers().any(|f| matches!(f, Falloff::VertexMap(_)))
    }
    /// Whether any layer's weights are worked out per point from the
    /// selection or a pick (Selection, Soft Selection, Element, Coplanar):
    /// then weights need the point index, not just a position.
    pub fn reads_selection(&self) -> bool {
        self.layers().any(|f| f.reads_points())
    }
    /// Whether a layer reaches points outside the selection (FO-47: Soft
    /// Selection, Coplanar, Element), so a transform moves those too.
    pub fn reaches(&self) -> bool {
        self.layers().any(|f| f.reaches())
    }
    /// The weight of point `point` of a mesh at world position `world`;
    /// `inputs` holds that mesh's selected weight map, selection rings and
    /// per-layer weights, where the stack reads them.
    pub fn weight_at(&self, world: DVec3, point: u32, inputs: PointInputs<'_>) -> f64 {
        self.weight_at_local(world, world, point, inputs)
    }
    /// [`Self::weight_at`] with the point's position in its own mesh's
    /// space too: geometric falloffs with Use World Transforms off read
    /// that one (FO-35).
    pub fn weight_at_local(
        &self,
        world: DVec3,
        local: DVec3,
        point: u32,
        inputs: PointInputs<'_>,
    ) -> f64 {
        let at = |on: bool| if on { world } else { local };
        let layer = |i: usize, f: Falloff| -> f64 {
            match f {
                Falloff::Linear(f) => f.weight(at(f.world)),
                Falloff::Radial(f) => f.weight(at(f.world)),
                Falloff::Cylinder(f) => f.weight(at(f.world)),
                Falloff::Noise(f) => f.weight(at(f.world)),
                // VMF-06: no map selected weighs 1; VMF-04: no value is 0.
                // FO-49: colour and morph maps come worked out per point.
                Falloff::VertexMap(_)
                    if let Some(w) = inputs.layers.get(i).and_then(Option::as_ref) =>
                {
                    w.get(point as usize).copied().unwrap_or(0.)
                }
                Falloff::VertexMap(_) => match (&self.weight_map, inputs.map) {
                    (None, _) => 1.,
                    (Some(_), Some(m)) => m.get(&point).copied().unwrap_or(0.),
                    (Some(_), None) => 0.,
                },
                Falloff::Selection(s) => s.weight(inputs.rings.map_or(RING_INNER, |r| {
                    r.get(point as usize).copied().unwrap_or(RING_OUTSIDE)
                })),
                Falloff::SoftSelection(_)
                | Falloff::Element(_)
                | Falloff::Coplanar(_)
                | Falloff::Screen(_)
                | Falloff::Lasso(_)
                | Falloff::Airbrush(_)
                | Falloff::Image(_)
                | Falloff::Path(_)
                | Falloff::Curvature(_) => inputs
                    .layers
                    .get(i)
                    .and_then(Option::as_ref)
                    .and_then(|w| w.get(point as usize).copied())
                    .unwrap_or(0.),
                Falloff::Invert => 1.,
            }
        };
        let invert = |w: f64| 1. - w.clamp(0., 1.);
        let mut w = match self.first {
            // An Invert on its own inverts "no falloff" (1).
            Falloff::Invert => invert(1.),
            f => layer(0, f),
        };
        for (i, &(f, mix)) in self.added.iter().enumerate() {
            w = match f {
                Falloff::Invert => invert(w),
                f => mix.mix(w, layer(i + 1, f)),
            };
        }
        // FO-34: unclamped (VMF-11). FO-32: the first layer's Mix Mode has
        // no effect (measured), so it isn't read here.
        w
    }
    /// Item `id`'s selection rings when the stack reads the selection.
    pub fn rings(&self, scene: &Scene, id: u64) -> Result<Option<std::rc::Rc<Vec<u32>>>> {
        if self.layers().any(|f| matches!(f, Falloff::Selection(_))) {
            item_selection_rings(scene, id).map(Some)
        } else {
            Ok(None)
        }
    }
    /// Everything the stack reads per point of item `id` in `scene`: the
    /// selected weight map, the selection rings and the per-layer weights
    /// (remembered while the mesh and selection stay the same).
    pub fn prepare(&self, scene: &Scene, id: u64) -> Result<Prepared> {
        let item = scene.item(id).ok_or("Missing falloff item")?;
        let map = self
            .weight_map
            .as_ref()
            .filter(|_| self.reads_map())
            .and_then(|n| item.mesh.attributes.point_weights.get(n).cloned());
        let rings = self.rings(scene, id)?;
        let n = item.mesh.vertices.len();
        let mut layers = Vec::new();
        let per_point = |f: Falloff| {
            f.stamped()
                || f.reaches()
                || matches!(f, Falloff::Path(_) | Falloff::Curvature(_))
                || matches!(f, Falloff::VertexMap(_))
                    && self.weight_map.as_deref().is_some_and(|m| {
                        crate::falloff_pattern::map_kind(&item.mesh, m)
                            .is_some_and(|k| k != crate::falloff_pattern::MapKind::Weight)
                    })
        };
        if self.layers().any(per_point) {
            let world = scene.world_matrix(id)?;
            let selection = item_selection(scene, id);
            use crate::falloff_reach::{ReachKey, reach_weights};
            for (i, f) in self.layers().enumerate() {
                let key = match f {
                    Falloff::SoftSelection(f) => Some(ReachKey::Soft(f)),
                    Falloff::Coplanar(f) => Some(ReachKey::Coplanar(f)),
                    Falloff::Element(f) => Some(ReachKey::Element(
                        f,
                        self.element.filter(|g| g.item == id),
                        self.symmetry,
                    )),
                    Falloff::Path(f) => Some(ReachKey::Path(f, self.path.clone())),
                    Falloff::Curvature(f) => Some(ReachKey::Curvature(f)),
                    _ => None,
                };
                layers.push(match (key, f) {
                    (Some(k), _) => Some(reach_weights(&item.mesh, world, &selection, k)),
                    (None, Falloff::VertexMap(v)) => self
                        .weight_map
                        .as_deref()
                        .and_then(|m| crate::falloff_pattern::vertex_map_values(&item.mesh, m, v))
                        .map(std::sync::Arc::new),
                    (None, f) if f.stamped() => {
                        self.stamps.get(&(i, id)).filter(|w| w.len() == n).cloned()
                    }
                    _ => None,
                });
            }
        }
        Ok(Prepared { map, rings, layers })
    }
    /// The points of item `id` a reaching layer (Soft Selection, Coplanar,
    /// Element) gives weight, which a transform moves besides the
    /// selection's. Empty when no layer reaches.
    pub fn reach(&self, scene: &Scene, id: u64) -> Result<Vec<u32>> {
        if !self.reaches() {
            return Ok(Vec::new());
        }
        let prepared = self.prepare(scene, id)?;
        let mut on = vec![false; scene.item(id).map_or(0, |i| i.mesh.vertices.len())];
        let reaching = prepared
            .layers
            .iter()
            .zip(self.layers())
            .filter(|(_, f)| f.reaches())
            .filter_map(|(w, _)| w.as_ref());
        for w in reaching {
            for (v, &w) in w.iter().enumerate() {
                if w != 0. {
                    on[v] = true;
                }
            }
        }
        Ok((0..on.len() as u32).filter(|&v| on[v as usize]).collect())
    }
    /// Every point's weight on item `id` (world positions and the selection
    /// from `scene`).
    pub fn weights(&self, scene: &Scene, id: u64) -> Result<Vec<f64>> {
        let world = scene.world_matrix(id)?;
        let item = scene.item(id).ok_or("Missing falloff item")?;
        let prepared = self.prepare(scene, id)?;
        let inputs = prepared.inputs();
        Ok(item
            .mesh
            .vertices
            .iter()
            .enumerate()
            .map(|(v, &p)| self.weight_at_local(world.transform_point3(p), p, v as u32, inputs))
            .collect())
    }
}
impl From<LinearFalloff> for Falloff {
    fn from(f: LinearFalloff) -> Self {
        Self::Linear(f)
    }
}
impl From<RadialFalloff> for Falloff {
    fn from(f: RadialFalloff) -> Self {
        Self::Radial(f)
    }
}
impl From<CylinderFalloff> for Falloff {
    fn from(f: CylinderFalloff) -> Self {
        Self::Cylinder(f)
    }
}
impl Falloff {
    pub fn validate(self) -> Result<()> {
        match self {
            Self::Linear(f) => f.validate(),
            Self::Radial(f) => f.validate(),
            Self::Cylinder(f) => f.validate(),
            Self::VertexMap(f) => {
                if f.mode == VertexMapMode::Magnitude && f.index != 0 {
                    // VMF-01: Index is refused in Magnitude mode.
                    return Err("Index only applies in Component mode".into());
                }
                Ok(())
            }
            Self::Invert => Ok(()),
            Self::Selection(f) => f.validate(),
            Self::SoftSelection(f) => f.validate(),
            Self::Element(f) => f.validate(),
            Self::Coplanar(f) => f.validate(),
            Self::Screen(f) => f.validate(),
            Self::Lasso(f) => f.validate(),
            Self::Airbrush(f) => f.validate(),
            Self::Noise(f) => f.validate(),
            Self::Image(f) => f.validate(),
            Self::Path(f) => f.validate(),
            Self::Curvature(f) => f.validate(),
        }
    }
    /// The weight from position alone. Vertex Map and Invert need the
    /// point and the stack (`ActiveFalloff::weight_at`), and so do the
    /// selection- and pick-driven falloffs; here they give 1.
    pub fn weight(self, point: DVec3) -> f64 {
        match self {
            Self::Linear(f) => f.weight(point),
            Self::Radial(f) => f.weight(point),
            Self::Cylinder(f) => f.weight(point),
            Self::Noise(f) => f.weight(point),
            Self::Image(_)
            | Self::Path(_)
            | Self::Curvature(_)
            | Self::VertexMap(_)
            | Self::Invert
            | Self::Selection(_)
            | Self::SoftSelection(_)
            | Self::Element(_)
            | Self::Coplanar(_)
            | Self::Screen(_)
            | Self::Lasso(_)
            | Self::Airbrush(_) => 1.,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Linear(_) => "Linear",
            Self::Radial(_) => "Radial",
            Self::Cylinder(_) => "Cylinder",
            Self::VertexMap(_) => "Vertex Map",
            Self::Invert => "Invert",
            Self::Selection(_) => "Selection",
            Self::SoftSelection(_) => "Soft Selection",
            Self::Element(_) => "Element",
            Self::Coplanar(_) => "Coplanar",
            Self::Screen(_) => "Screen",
            Self::Lasso(_) => "Lasso",
            Self::Airbrush(_) => "Airbrush",
            Self::Noise(_) => "Noise",
            Self::Image(_) => "Image",
            Self::Path(_) => "Path",
            Self::Curvature(_) => "Curvature",
        }
    }
    /// Its weights are stamped from the view when drawn or painted
    /// (Screen, Lasso, Airbrush; FO-Q08).
    pub fn stamped(self) -> bool {
        matches!(
            self,
            Self::Screen(_) | Self::Lasso(_) | Self::Airbrush(_) | Self::Image(_)
        )
    }
    /// Its weights are worked out per point from the selection or a pick.
    pub fn reads_points(self) -> bool {
        matches!(
            self,
            Self::Selection(_)
                | Self::SoftSelection(_)
                | Self::Element(_)
                | Self::Coplanar(_)
                | Self::Screen(_)
                | Self::Lasso(_)
                | Self::Airbrush(_)
                | Self::Image(_)
                | Self::Path(_)
                | Self::Curvature(_)
        )
    }
    /// It gives weight to points outside the selection (FO-47, FO-48,
    /// FO-45), which a transform then moves.
    pub fn reaches(self) -> bool {
        matches!(
            self,
            Self::SoftSelection(_) | Self::Element(_) | Self::Coplanar(_)
        )
    }
    /// Whether it has viewport handles (VMF-02: Vertex Map has none).
    pub fn has_handles(self) -> bool {
        matches!(self, Self::Linear(_) | Self::Radial(_) | Self::Cylinder(_))
    }
    /// FO-35: whether its positions are world positions (Use World
    /// Transforms on). Falloffs with no positions say true.
    pub fn uses_world(self) -> bool {
        match self {
            Self::Linear(f) => f.world,
            Self::Radial(f) => f.world,
            Self::Cylinder(f) => f.world,
            Self::Noise(f) => f.world,
            Self::Path(f) => f.world,
            _ => true,
        }
    }
}

/// World bounding box of the selection's points, or the whole active mesh
/// with nothing selected (FAL-M-06).
pub fn selection_bounds(scene: &Scene) -> Result<(DVec3, DVec3)> {
    selection_bounds_in(scene, true)
}

/// [`selection_bounds`] in world space, or with `world` false in the active
/// mesh's own space (FO-35: a falloff without Use World Transforms fits
/// there).
pub fn selection_bounds_in(scene: &Scene, world_space: bool) -> Result<(DVec3, DVec3)> {
    let item = scene.active().ok_or("Select a mesh for Auto Size")?;
    if scene.selection.mode == Mode::Item {
        return Err("Auto Size works on a component selection".into());
    }
    let vertices = crate::transform_vertices(scene)?;
    let world = if world_space {
        scene.world_matrix(item.id)?
    } else {
        glam::DMat4::IDENTITY
    };
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for v in vertices {
        let p = world.transform_point3(item.mesh.vertices[v as usize]);
        lo = lo.min(p);
        hi = hi.max(p);
    }
    if !lo.is_finite() || !hi.is_finite() {
        return Err("Auto Size needs points with finite positions".into());
    }
    Ok((lo, hi))
}

impl LinearFalloff {
    /// FAL-M-04: Start and End at the box's minimum and maximum along `axis`,
    /// the other two coordinates at the box center. A box with no extent
    /// along `axis` leaves the falloff unchanged.
    pub fn auto_size(self, scene: &Scene, axis: usize) -> Result<Self> {
        if axis > 2 {
            return Err("Auto Size axis must be X, Y or Z".into());
        }
        let (lo, hi) = selection_bounds_in(scene, self.world)?;
        if hi[axis] - lo[axis] <= 0. {
            return Ok(self);
        }
        let center = (lo + hi) / 2.;
        let (mut start, mut end) = (center, center);
        start[axis] = lo[axis];
        end[axis] = hi[axis];
        Ok(Self { start, end, ..self })
    }
}
impl RadialFalloff {
    /// FAL-M-06: Center at the box center, Size half the box extents.
    pub fn auto_size(self, scene: &Scene) -> Result<Self> {
        let (lo, hi) = selection_bounds_in(scene, self.world)?;
        Ok(Self {
            center: (lo + hi) / 2.,
            size: (hi - lo) / 2.,
            ..self
        })
    }
}
impl CylinderFalloff {
    /// FO-04: Center at the box center, Size half the box extents.
    pub fn auto_size(self, scene: &Scene) -> Result<Self> {
        let (lo, hi) = selection_bounds_in(scene, self.world)?;
        Ok(Self {
            center: (lo + hi) / 2.,
            size: (hi - lo) / 2.,
            ..self
        })
    }
}
