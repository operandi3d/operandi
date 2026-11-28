//! The Transform pipe in UV space (specifications/uv-2026-11-16,
//! BRIEF-TRANSFORM-PIPE-IN-UV and ADDENDUM-GENERAL-TOOLS-IN-UV, UVT-130..138).
//!
//! One change to the pipe instead of a second set of UV tools: when a tool
//! runs in a UV view, the editor swaps the tool's start scene for a *UV
//! proxy* ([`UvProxy`]) for the length of each preview, runs the tool's
//! ordinary 3D code on it, and writes the proxy's moved points back as U
//! and V of the current map. On the proxy:
//!
//! - every UV position (a mesh point's UV in one island) is its own point at
//!   (U, V, 0), so a seam is two points (UVT-42) and a UV position moves
//!   only with its own polygons;
//! - the polygons are the visible, fully mapped polygons with the same
//!   corners, so Selection falloff steps over UV edges and "connected"
//!   means the UV island (UVT-04, UVT-131/132);
//! - the item sits at the origin with no transform, so falloffs, ranges and
//!   action centres measure in U and V (UVT-132/133);
//! - per-3D-point vertex-map weights are copied to each UV position, so a
//!   Vertex Map falloff reads them as in 3D (UVT-132).
//!
//! Only U and V of the plan's map are written back; 3D positions, polygons
//! and other maps never change (UVT-06). Hidden polygons are not in the
//! proxy, so they're never touched (UVT-02).
//!
//! Build guesses, each behind one constant below so it is one line to change:
//! [`RANGE_UNITS`] (UVT-Q11), [`CONNECTIVITY`] (UVT-Q12) and
//! [`ROTATE_COUNTER_CLOCKWISE`] (rotate direction).
use crate::topology::edge_key;
use crate::{Mesh, Mode, Result, Scene, Selection};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

/// What Element Move's Range (and the right-drag that sets it) measures in
/// a UV view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeUnits {
    /// UV units: Range 0.1 reaches a tenth of the 0–1 tile.
    Uv,
    /// Screen pixels of the UV view, whatever its zoom.
    Pixels,
}
/// UVT-Q11 (build guess): Range in a UV view is in UV units.
pub const RANGE_UNITS: RangeUnits = RangeUnits::Uv;

/// What "connected" means for Use Connectivity / Rigid Connections in a UV
/// view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UvConnectivity {
    /// The clicked UV island (polygons joined through shared UV positions).
    Island,
    /// The clicked 3D piece (polygons joined through shared 3D points).
    Mesh,
}
/// UVT-Q12 (build guess): connectivity in a UV view is the UV island.
pub const CONNECTIVITY: UvConnectivity = UvConnectivity::Island;

/// Rotate direction in a UV view (build guess): a positive angle turns
/// counter-clockwise with U to the right and V up (UVE-1 Q6). `false` turns
/// it clockwise instead.
pub const ROTATE_COUNTER_CLOCKWISE: bool = true;

/// The tools that run on the UV proxy (UVT-130): the Transform tools and
/// every preset built on them (Soft Move, Shear, Taper, Bulge/Pole, Twist,
/// Swirl, Flex, Soft Drag), Element Move (Tweak), Smooth, the Deform
/// finish point tools (Jitter, Quantize, Radial Align, Linear Align:
/// UVT-40, UVT-44), and Edge Slide, which then slides UVs along their UV
/// edges (UVT-137, started in a UV view).
pub fn runs_on_uvs(tool: crate::Tool) -> bool {
    use crate::Tool;
    matches!(
        tool,
        Tool::Move
            | Tool::Rotate
            | Tool::Scale
            | Tool::Transform
            | Tool::Tweak
            | Tool::Smooth
            | Tool::Jitter
            | Tool::Quantize
            | Tool::RadialAlign
            | Tool::LinearAlign
            | Tool::EdgeSlide
            | Tool::UvBoxTransform
    )
}

/// Screen-space falloff stamps (Screen, Lasso, Airbrush: FO-Q08) drawn in a
/// UV view weigh the UV proxy's points, not the mesh's, so they are kept
/// under their layer plus this offset in `Editor::falloff_stamps`. A UV
/// preview reads them as the layer's stamps; a 3D one never sees them.
pub const UV_STAMP_LAYER: usize = 1 << 20;

/// Corners at the same mesh point whose UVs are within this of each other
/// are one UV position (as the UV view's seam test and `uv_edit::plan`).
const SAME_UV: f64 = 1e-6;

fn uv_key(p: DVec2) -> (i64, i64) {
    let q = |x: f64| (x / SAME_UV).round() as i64;
    (q(p.x), q(p.y))
}

/// Uses of one authored owner that always receive the same UV edit.
#[derive(Clone, Debug)]
struct RecordUse {
    proxy: Option<u32>,
    corners: Vec<(usize, usize)>,
}

#[derive(Debug)]
struct EditedAttributes {
    owners: Vec<(usize, usize, u32)>,
    attributes: crate::Shared<crate::MeshAttributes>,
}

/// The UV proxy of one mesh's map: the scene a UV tool runs on.
#[derive(Clone, Debug)]
pub struct UvProxy {
    /// The map the tool edits.
    map: String,
    /// The real item.
    pub item: u64,
    /// The tool's start scene with the item's mesh swapped for the proxy
    /// mesh, its transform cleared and the selection put in proxy indices.
    /// Other items are left out, so nothing else can move.
    pub scene: Scene,
    /// For each real polygon, the proxy point of each corner (empty for a
    /// polygon that isn't in the proxy).
    corners: Vec<Vec<u32>>,
    /// The real mesh point of each proxy point.
    pub points: Vec<u32>,
    /// The real polygon of each proxy polygon.
    pub faces: Vec<u32>,
    /// The real mesh the proxy was made from (to tell when it is stale).
    source: crate::MeshSlot,
    tear_off: bool,
    shared_records: Vec<Vec<RecordUse>>,
    spare_records: Vec<u32>,
    edited_attributes: Arc<Mutex<Option<EditedAttributes>>>,
}

impl UvProxy {
    /// The proxy of `base`'s active mesh on `map`. `tear_off` (UVT-41):
    /// the selected polygons' UV positions are split from their unselected
    /// neighbours first, so the piece comes away cleanly.
    pub fn build(base: &Scene, map: &str, tear_off: bool) -> Result<Self> {
        let item = base.active().ok_or("Select a mesh")?;
        let mesh = &item.mesh;
        let rows = mesh
            .uv_maps
            .get(map)
            .ok_or_else(|| format!("The mesh has no UV map called {map}"))?;
        let hidden = crate::Protection::of(mesh, &item.elements)
            .map(|p| p.face_hidden)
            .unwrap_or_default();
        let selection = &base.selection;
        let torn: BTreeSet<u32> = if tear_off && selection.mode == Mode::Polygon {
            selection.faces.iter().copied().collect()
        } else {
            BTreeSet::new()
        };
        let mut proxy = Mesh::default();
        let mut corners: Vec<Vec<u32>> = vec![Vec::new(); mesh.faces.len()];
        let mut points: Vec<u32> = Vec::new();
        let mut faces: Vec<u32> = Vec::new();
        let mut proxy_faces: Vec<crate::Face> = Vec::new();
        let mut position: HashMap<(u32, (i64, i64), bool), u32> = HashMap::new();
        for (f, face) in mesh.faces.iter().enumerate() {
            let usable = !hidden.get(f).copied().unwrap_or(false)
                && !mesh.is_polyline(f)
                && rows.get(f).is_some_and(|r| {
                    r.len() == face.vertices.len() && r.iter().all(Option::is_some)
                });
            if !usable {
                continue;
            }
            let side = torn.contains(&(f as u32));
            let mut ids = Vec::with_capacity(face.vertices.len());
            for (c, &p) in face.vertices.iter().enumerate() {
                let uv = rows[f][c].expect("usable corners are mapped");
                let next = points.len() as u32;
                let id = *position.entry((p, uv_key(uv), side)).or_insert(next);
                if id == next {
                    points.push(p);
                    proxy.vertices.push(DVec3::new(uv.x, uv.y, 0.));
                }
                ids.push(id);
            }
            proxy_faces.push(crate::Face {
                vertices: ids.clone(),
                uv: None,
                ..face.clone()
            });
            corners[f] = ids;
            faces.push(f as u32);
        }
        proxy.faces = proxy_faces.into();
        // UV layouts may hold zero-area or overlapping polygons; the proxy
        // is never kept, so its own check lets them through (the real mesh
        // is checked after the write-back).
        let mut attributes = crate::MeshAttributes {
            degenerate_faces: (0..faces.len() as u32).collect(),
            extrude_edge_uses: (*proxy.exceptional_edge_uses()).clone(),
            ..Default::default()
        };
        // A Vertex Map falloff reads each UV position's 3D point's weight.
        if !mesh.attributes.point_weights.is_empty() {
            for (name, weights) in &mesh.attributes.point_weights {
                let copied = points
                    .iter()
                    .enumerate()
                    .filter_map(|(k, p)| weights.get(p).map(|&w| (k as u32, w)))
                    .collect();
                attributes.point_weights.insert(name.clone(), copied);
            }
        }
        proxy.attributes = attributes.into();
        let mut record_uses = BTreeMap::<u32, Vec<RecordUse>>::new();
        if let Some(records) = mesh.attributes.uv_records.get(map) {
            for (f, ids) in records.iter().enumerate() {
                for (c, &record) in ids.iter().enumerate() {
                    let Some(record) = record else { continue };
                    let proxy = corners.get(f).and_then(|row| row.get(c)).copied();
                    let uses = record_uses.entry(record).or_default();
                    if let Some(group) = uses.iter_mut().find(|group| group.proxy == proxy) {
                        group.corners.push((f, c));
                    } else {
                        uses.push(RecordUse {
                            proxy,
                            corners: vec![(f, c)],
                        });
                    }
                }
            }
        }
        let splits: usize = record_uses.values().map(|uses| uses.len() - 1).sum();
        let spare_records = (0..=u32::MAX)
            .filter(|id| !record_uses.contains_key(id))
            .take(splits)
            .collect();
        let shared_records = record_uses
            .into_values()
            .filter(|uses| uses.len() > 1)
            .collect();
        let proxy_selection = map_selection(selection, mesh, &corners, &points)?;
        let mut scene = base.clone();
        scene.items.retain(|i| i.id == item.id);
        let it = scene.items.first_mut().expect("the active item");
        it.mesh = proxy.into();
        it.transform = Default::default();
        it.parent = None;
        it.subdivision = None;
        it.elements = Default::default();
        scene.selection = proxy_selection;
        Ok(Self {
            map: map.to_owned(),
            item: item.id,
            scene,
            corners,
            points,
            faces,
            source: item.mesh.clone(),
            tear_off,
            shared_records,
            spare_records,
            edited_attributes: Default::default(),
        })
    }

    /// Whether this proxy was made from `base`'s active mesh with these
    /// settings (a new drag starts from a new scene).
    pub fn fits(&self, base: &Scene, map: &str, tear_off: bool) -> bool {
        self.map == map
            && self.tear_off == tear_off
            && base
                .active()
                .is_some_and(|i| i.id == self.item && crate::MeshSlot::same(&i.mesh, &self.source))
    }

    /// For each polygon, its corners' proxy point indices.
    pub fn corners(&self) -> &[Vec<u32>] {
        &self.corners
    }

    /// The proxy mesh.
    pub fn mesh(&self) -> &Mesh {
        &self.scene.items[0].mesh
    }

    /// A copy of the start mesh with U and V of every proxy point
    /// that `moved` moved written into the map's corners. Only those
    /// corners change: a UV position that didn't move keeps its exact
    /// value, and corners of polygons outside the proxy never change.
    fn write_back(&self, moved: &Mesh) -> Result<crate::MeshSlot> {
        self.source.validate()?;
        let start = self.mesh();
        if moved.vertices.len() != start.vertices.len() {
            return Err("A UV tool can't change the number of UV points".into());
        }
        let changed: Vec<bool> = start
            .vertices
            .iter()
            .zip(&moved.vertices)
            .map(|(a, b)| a.x.to_bits() != b.x.to_bits() || a.y.to_bits() != b.y.to_bits())
            .collect();
        if !changed.iter().any(|&c| c) {
            return Ok(self.source.clone());
        }
        if moved.vertices.iter().any(|p| !p.is_finite()) {
            return Err("UV values must be numbers".into());
        }
        let mut real = self.source.to_owned_value();
        let mut rows = real
            .uv_maps
            .get(&self.map)
            .ok_or_else(|| format!("The mesh has no UV map called {}", self.map))?
            .clone();
        for (f, ids) in self.corners.iter().enumerate() {
            for (c, &id) in ids.iter().enumerate() {
                if changed[id as usize] {
                    let p = moved.vertices[id as usize];
                    rows[f][c] = Some(DVec2::new(p.x, p.y));
                }
            }
        }
        // Only owners used by different proxy points (or by corners
        // outside the proxy) can diverge. Those groups were found once
        // when making the proxy. Keep the original ID on unchanged uses.
        let mut available = self.spare_records.iter().copied();
        let mut owners = Vec::new();
        for uses in &self.shared_records {
            let is_moved = |group: &RecordUse| group.proxy.is_some_and(|p| changed[p as usize]);
            if !uses.iter().any(is_moved) {
                continue;
            }
            let original = uses
                .iter()
                .find(|group| !is_moved(group))
                .unwrap_or(&uses[0]);
            let value = |group: &RecordUse| {
                let (f, c) = group.corners[0];
                rows[f][c]
            };
            let mut split = Vec::new();
            for group in uses.iter().filter(|group| value(group) != value(original)) {
                let uv = value(group);
                let id = if let Some(&(_, id)) = split.iter().find(|&&(v, _)| v == uv) {
                    id
                } else {
                    let id = available
                        .next()
                        .ok_or("UV record identity budget exhausted")?;
                    split.push((uv, id));
                    id
                };
                owners.extend(group.corners.iter().map(|&(f, c)| (f, c, id)));
            }
        }
        real.uv_maps.insert(self.map.clone(), rows);
        if !owners.is_empty() || real.attributes.point_uvs.contains_key(&self.map) {
            // The same split usually lasts the whole drag. Reuse its
            // records instead of copying the authored map each frame.
            let mut cached = self
                .edited_attributes
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(edit) = cached.as_ref().filter(|edit| edit.owners == owners) {
                real.attributes = edit.attributes.clone();
            } else {
                if !owners.is_empty() {
                    let ids = real
                        .attributes
                        .uv_records
                        .get_mut(&self.map)
                        .expect("authored UV map");
                    for &(f, c, id) in &owners {
                        ids[f][c] = Some(id);
                    }
                }
                real.attributes.point_uvs.remove(&self.map);
                *cached = Some(EditedAttributes {
                    owners,
                    attributes: real.attributes.clone(),
                });
            }
        }
        // This is the validated source with the same row shape and only
        // finite coordinates overwritten. Every authored owner still names
        // one point and UV value, because conflicting uses got fresh IDs.
        let result = crate::MeshSlot::from(real);
        result.shared().record_checked();
        Ok(result)
    }

    /// UVT-Q12 alternative ([`UvConnectivity::Mesh`]): the proxy points
    /// whose 3D points are joined to `seeds`' 3D points through the real
    /// mesh.
    pub fn connected_in_3d(&self, seeds: &[u32]) -> Vec<bool> {
        let seeds_3d: Vec<u32> = seeds.iter().map(|&s| self.points[s as usize]).collect();
        let joined = crate::tweak::connected_points_of(&self.source, &seeds_3d);
        self.points.iter().map(|&p| joined[p as usize]).collect()
    }
}

/// The real selection in proxy indices: polygons keep their polygon (by
/// position in the proxy), points become all their UV positions, edges all
/// their UV edges. Item mode, or nothing selected, is nothing selected
/// (all visible polygons, UVT-02). A selection with nothing left in the
/// proxy (all hidden or unmapped) is an error, not "everything".
fn map_selection(
    s: &Selection,
    mesh: &Mesh,
    corners: &[Vec<u32>],
    points: &[u32],
) -> Result<Selection> {
    let mut out = Selection {
        mode: if s.mode == Mode::Item {
            Mode::Polygon
        } else {
            s.mode
        },
        item: s.item,
        ..Default::default()
    };
    let mut proxy_face = vec![u32::MAX; corners.len()];
    let mut next = 0u32;
    for (f, ids) in corners.iter().enumerate() {
        if !ids.is_empty() {
            proxy_face[f] = next;
            next += 1;
        }
    }
    let had = match s.mode {
        Mode::Polygon => !s.faces.is_empty(),
        Mode::Vertex => !s.vertices.is_empty(),
        Mode::Edge => !s.edges.is_empty(),
        Mode::Item => false,
    };
    match s.mode {
        Mode::Polygon => {
            out.faces = s
                .faces
                .iter()
                .filter_map(|&f| proxy_face.get(f as usize).copied())
                .filter(|&f| f != u32::MAX)
                .collect();
        }
        Mode::Vertex => {
            let chosen: BTreeSet<u32> = s.vertices.iter().copied().collect();
            out.vertices = points
                .iter()
                .enumerate()
                .filter(|(_, p)| chosen.contains(p))
                .map(|(k, _)| k as u32)
                .collect();
        }
        Mode::Edge => {
            let chosen: BTreeSet<(u32, u32)> =
                s.edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
            let mut edges = BTreeSet::new();
            for (f, ids) in corners.iter().enumerate() {
                let v = &mesh.faces[f].vertices;
                for i in 0..ids.len() {
                    let j = (i + 1) % ids.len();
                    if chosen.contains(&edge_key(v[i], v[j])) {
                        edges.insert(edge_key(ids[i], ids[j]));
                    }
                }
            }
            out.edges = edges.into_iter().collect();
        }
        Mode::Item => {}
    }
    let left = !(out.faces.is_empty() && out.vertices.is_empty() && out.edges.is_empty());
    if had && !left {
        return Err("The selection has no visible UVs in this map".into());
    }
    if points.is_empty() {
        return Err("No visible polygons have UVs in this map".into());
    }
    Ok(out)
}

/// The action frame of a UV tool (UVT-133), worked out on the proxy:
/// Selection, Automatic, Border and None the UV selection's centre, Local
/// each island about its own (the tool does that), Element and Screen the
/// clicked point when it was clicked in the UV view, Origin UV (0, 0).
/// Axes are always U, V and the view axis.
pub fn action_frame(
    context: crate::ActionContext,
    tool: crate::Tool,
    proxy: &UvProxy,
) -> Result<crate::ActionFrame> {
    use crate::CenterMode;
    let center = match context.center {
        CenterMode::Origin => DVec3::ZERO,
        CenterMode::Element(p) => p.point() * DVec3::new(1., 1., 0.),
        _ => {
            let fallback = crate::ActionContext {
                center: CenterMode::Automatic,
                axis: crate::AxisOrientation::default(),
            };
            let context = match context.center {
                // Screen and Work Plane fall back to the selection centre
                // (UVT-133, build guess); Local's handle sits there too.
                CenterMode::Screen(_) | CenterMode::Local => fallback,
                _ => crate::ActionContext {
                    axis: crate::AxisOrientation::default(),
                    ..context
                },
            };
            crate::Editor::extrude_frame(context, tool, &proxy.scene)
                .or_else(|_| crate::Editor::extrude_frame(fallback, tool, &proxy.scene))?
                .center
        }
    };
    Ok(crate::ActionFrame {
        center: DVec3::new(center.x, center.y, 0.),
        basis: glam::DMat3::IDENTITY,
    })
}

impl PartialEq for UvProxy {
    /// Two proxies are equal when they were made from the same mesh with the
    /// same settings (the proxy is a cache of that).
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
            && self.item == other.item
            && self.tear_off == other.tear_off
            && crate::MeshSlot::same(&self.source, &other.source)
    }
}

/// UVT-135: only U, V and the turn about the view axis drive a UV tool. The
/// Z fields are greyed in the panel; whatever they hold, the run ignores
/// them, so a stray value can't tilt the flat proxy.
fn flatten(tool: crate::Tool, p: &mut crate::Parameters) {
    use crate::Tool;
    let sign = if ROTATE_COUNTER_CLOCKWISE { 1. } else { -1. };
    match tool {
        Tool::Move | Tool::Tweak => p.delta.z = 0.,
        Tool::Rotate => p.delta = DVec3::new(0., 0., p.delta.z * sign),
        Tool::Scale => p.delta.z = 1.,
        Tool::Transform => {
            p.delta.z = 0.;
            p.rotation_delta = DVec3::new(0., 0., p.rotation_delta.z * sign);
            p.scale_delta.z = 1.;
        }
        // UVT-135: Jitter's Range Z and Quantize's Step Z are greyed; the
        // run leaves the view axis alone whatever they hold.
        Tool::Jitter => {
            p.jitter.range.z = 0.;
            p.jitter.enable[2] = false;
            p.jitter.planar = true;
        }
        Tool::Quantize => p.quantize.step.z = 0.,
        _ => {}
    }
}

impl crate::Editor {
    /// The proxy for the live tool's UV space, reusing the one the
    /// parameters carry while it still fits the tool's base scene.
    fn uv_proxy(&self, space: &crate::uv_edit::UvToolSpace) -> Result<std::sync::Arc<UvProxy>> {
        let live = self.session.live.as_ref().ok_or("No active tool")?;
        match &space.proxy {
            Some(proxy) if proxy.fits(&live.base, &space.map, space.tear_off) => Ok(proxy.clone()),
            _ => Ok(std::sync::Arc::new(UvProxy::build(
                &live.base,
                &space.map,
                space.tear_off,
            )?)),
        }
    }

    /// One preview of a tool in UV space (UVT-130): the tool's own 3D code
    /// runs on the UV proxy with 3D Symmetry off (UVT-136; the UV Symmetry
    /// layer mirrors the edit instead, uv_symmetry.rs), and the moved
    /// proxy points are written back as U and V of the map.
    pub(crate) fn preview_uv(&mut self, mut p: crate::Parameters, apply: bool) -> Result<()> {
        let live = self.session.live.as_ref().ok_or("No active tool")?;
        let tool = live.tool;
        let space = p.uv.as_mut().ok_or("Not a UV tool")?;
        let proxy = self.uv_proxy(space)?;
        space.proxy = Some(proxy.clone());
        // UVT-Q05 (provisional): Snapping lands the moved centre on the grid.
        if let (Some(step), Some(frame)) = (space.grid_snap.filter(|s| *s > 0.), live.action_frame)
            && matches!(tool, crate::Tool::Move | crate::Tool::Transform)
            && p.delta.truncate() != DVec2::ZERO
        {
            let c = frame.center.truncate();
            let landed = ((c + p.delta.truncate()) / step).round() * step;
            p.delta.x = landed.x - c.x;
            p.delta.y = landed.y - c.y;
        }
        let mut run = p.clone();
        flatten(tool, &mut run);
        let live = self.session.live.as_mut().expect("checked above");
        let real_base = std::mem::replace(&mut live.base, proxy.scene.clone());
        let symmetry = std::mem::take(&mut self.symmetry);
        // Screen falloffs drawn in the UV view weigh proxy points.
        let uv_stamps = self
            .falloff_stamps
            .iter()
            .filter(|((layer, _), _)| *layer >= UV_STAMP_LAYER)
            .map(|(&(layer, id), w)| ((layer - UV_STAMP_LAYER, id), w.clone()))
            .collect();
        let stamps = std::mem::replace(&mut self.falloff_stamps, uv_stamps);
        let result = self.preview_one(run, apply);
        self.falloff_stamps = stamps;
        self.symmetry = symmetry;
        let live = self.session.live.as_mut().expect("the tool stays live");
        live.base = real_base;
        result?;
        // Keep the values as entered (not flattened); keep what the run
        // worked out once per drag (Element Move's plan).
        let ran = std::mem::replace(&mut live.params, p);
        live.params.tweak_plan = ran.tweak_plan;
        let moved = std::mem::replace(&mut self.scene, live.base.clone());
        let mut moved = (*moved.items.first().ok_or("Select a mesh")?.mesh).clone();
        // UVT-70: the UV Symmetry layer copies each edit to its partner.
        if self.uv_symmetry.axis.is_some() {
            crate::uv_symmetry::mirror_edit(
                &proxy.mesh().vertices,
                &mut moved.vertices,
                &self.uv_symmetry,
            );
        }
        let item = self.scene.item_mut(proxy.item).ok_or("Select a mesh")?;
        item.mesh = proxy.write_back(&moved)?;
        self.scene.validate()
    }

    /// The live tool's action frame in UV space, for `context`.
    pub(crate) fn uv_action_frame(
        &self,
        context: crate::ActionContext,
    ) -> Result<Option<crate::ActionFrame>> {
        let Some(live) = self.session.live.as_ref() else {
            return Ok(None);
        };
        let Some(space) = live.params.uv.as_ref() else {
            return Ok(None);
        };
        let proxy = self.uv_proxy(space)?;
        Ok(Some(action_frame(context, live.tool, &proxy)?))
    }

    /// `Command::UvToolSpace`: put the open tool in UV space on `map` (the
    /// selected map for `Some(None)`), or back in 3D. Refused once the tool
    /// has changed something, so a session is all UV or all 3D. The action
    /// frame and a preset's fitted falloff are worked out again in the new
    /// space.
    pub(crate) fn uv_tool_space(&mut self, space: Option<Option<String>>) -> Result<()> {
        let live = self.session.live.as_ref().ok_or("Start a tool first")?;
        let tool = live.tool;
        if !runs_on_uvs(tool) {
            return Err(format!("{} doesn't work on UVs yet", tool.label()));
        }
        // Smooth and the Deform finish point tools show their starting
        // result at once; switching space redoes
        // that result in the new one. Any other change means the session
        // already belongs to one space.
        let redo_start = tool.shows_result_at_once() && live.applied;
        if self.scene != live.base && !redo_start {
            return Err("Drop the tool before switching between UV and 3D".into());
        }
        let pinned = matches!(space, Some(Some(_)));
        let map = match space {
            None => None,
            Some(name) => {
                let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
                let map = match name {
                    Some(n) if mesh.uv_maps.contains_key(&n) => n,
                    Some(n) => return Err(format!("The mesh has no UV map called {n}")),
                    None => crate::uv_maps::current_map(mesh)
                        .ok_or("The mesh has no UV map yet: UV Unwrap or New Map makes one")?,
                };
                Some(map)
            }
        };
        let (frame, fit_scene) = match &map {
            Some(map) => {
                let proxy = UvProxy::build(&live.base, map, false)?;
                let frame = action_frame(self.action_context, tool, &proxy)?;
                (Some(frame), proxy.scene)
            }
            None => (
                live.action_frame
                    .is_some()
                    .then(|| Self::extrude_frame(self.action_context, tool, &live.base))
                    .transpose()?,
                live.base.clone(),
            ),
        };
        // A preset's Linear / Radial falloff is fitted to the selection in
        // the space the tool now works in (UVT-132).
        let falloff = match self.session.preset.as_ref() {
            Some(preset) if preset.previous_falloff.is_some() => {
                preset.row().fitted_falloff(&fit_scene)?
            }
            _ => None,
        };
        if let Some(f) = falloff {
            self.spatial_falloff = Some(f);
        }
        let live = self.session.live.as_mut().expect("checked above");
        live.params.uv = map.map(|map| crate::uv_edit::UvToolSpace {
            map,
            pinned,
            ..Default::default()
        });
        if live.action_frame.is_some() {
            live.action_frame = frame;
        }
        // The new space starts from no change.
        live.params.delta = if tool == crate::Tool::Scale {
            DVec3::ONE
        } else {
            DVec3::ZERO
        };
        live.params.rotation_delta = DVec3::ZERO;
        live.params.scale_delta = DVec3::ONE;
        live.params.tweak = None;
        live.params.tweak_plan = None;
        if redo_start {
            self.scene = live.base.clone();
            let p = live.params.clone();
            return self.preview(p, false);
        }
        Ok(())
    }
}

impl crate::Editor {
    /// The live tool's UV proxy, which a UV view picks Element Move's
    /// elements from (UVT-131): proxy point, edge and polygon numbers are
    /// what `TweakGrab` takes in UV space. Kept on the tool's parameters,
    /// made again when the tool's start scene changes. `None` outside UV
    /// space.
    pub fn uv_tool_proxy(&mut self) -> Option<std::sync::Arc<UvProxy>> {
        let space = self.session.live.as_ref()?.params.uv.clone()?;
        let proxy = self.uv_proxy(&space).ok()?;
        self.session.live.as_mut()?.params.uv.as_mut()?.proxy = Some(proxy.clone());
        Some(proxy)
    }
}
