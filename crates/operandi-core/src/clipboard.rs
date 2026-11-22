//! Create panel Edit group (BEHAVIOR-CLIPBOARD.md): Cut, Copy and Paste of
//! the selected vertices, edges or polygons, and the one-shot paste the
//! Paste Tool makes.
//!
//! - One app-wide clipboard (CLP-01, CLP-12: not the system clipboard). It
//!   holds the copied geometry in world space, so a paste keeps the world
//!   position (CLP-08), and the selection mode it was copied in.
//! - Vertices copy as loose points; edges as two-point lines (CLP-04);
//!   polygons with their UVs, material, part and polygon type (CLP-08).
//! - Cut removes what Copy takes (CLP-09): vertices leave their polygons
//!   (which lose that corner; fewer than three corners and the polygon
//!   goes), edges take the polygons that use them, polygons go. Points no
//!   longer used by anything go too; points that were loose stay.
//! - Paste adds a copy to the selected mesh (a new mesh when none is
//!   selected; provisional, CLP-11), not welded to anything (CLP-10), and
//!   selects only the pasted elements, in the mode they were copied in.
//! - Cut and Paste are one undo step each; Copy changes nothing (CLP-14).
//! - Nothing selected: Cut and Copy say so and do nothing (provisional).
use crate::{Editor, Face, Mesh, Mode, PolygonRepresentation, Result};
use glam::{DMat4, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// What Cut or Copy last took, in world space.
#[derive(Clone, Debug, PartialEq)]
pub struct Clipboard {
    pub mode: Mode,
    pub mesh: Mesh,
}

/// Where and how big the Paste Tool puts a copy (PST-04): the copy's
/// bounding-box centre lands on `center` (world space, PST-Q05 decided),
/// scaled per axis by `size` (1 = 100%) about that centre, then moved by
/// `bias`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PastePlacement {
    pub center: DVec3,
    pub size: DVec3,
    pub bias: DVec3,
}
impl Default for PastePlacement {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            size: DVec3::ONE,
            bias: DVec3::ZERO,
        }
    }
}

/// One polygon of a [`subset`]: the source polygon and which of its
/// corners stay, in order.
pub(crate) struct KeptFace {
    pub source: usize,
    pub corners: Vec<usize>,
}

/// A new mesh made of some of `source`'s polygons (or fewer of their
/// corners) and the points they use, plus `extra_points`. Per-polygon,
/// per-corner and per-point data follow their element; data of dropped
/// elements goes. Returns the mesh and each source point's new index.
pub(crate) fn subset(
    source: &Mesh,
    faces: &[KeptFace],
    extra_points: &BTreeSet<u32>,
) -> Result<(Mesh, Vec<Option<u32>>)> {
    let mut keep = vec![false; source.vertices.len()];
    for f in faces {
        for &c in &f.corners {
            keep[source.faces[f.source].vertices[c] as usize] = true;
        }
    }
    for &v in extra_points {
        if let Some(k) = keep.get_mut(v as usize) {
            *k = true;
        }
    }
    let mut point = vec![None; source.vertices.len()];
    let mut out = Mesh::default();
    for (v, _) in keep.iter().enumerate().filter(|(_, k)| **k) {
        point[v] = Some(out.vertices.len() as u32);
        out.vertices.push(source.vertices[v]);
    }
    let id = |v: u32| point.get(v as usize).copied().flatten();
    // Source polygon -> new polygon, only when all its corners stayed (so
    // whole-polygon records like sets stay meaningful).
    let mut whole: BTreeMap<usize, u32> = BTreeMap::new();
    for (i, f) in faces.iter().enumerate() {
        let src = &source.faces[f.source];
        out.faces.push(Face {
            vertices: f
                .corners
                .iter()
                .map(|&c| id(src.vertices[c]).expect("kept"))
                .collect(),
            uv: src
                .uv
                .as_ref()
                .map(|uv| f.corners.iter().map(|&c| uv[c]).collect()),
            ..src.clone()
        });
        if f.corners.len() == src.vertices.len() {
            whole.insert(f.source, i as u32);
        }
    }
    fn corners<T: Clone>(rows: &[Vec<T>], faces: &[KeptFace]) -> Vec<Vec<T>> {
        faces
            .iter()
            .map(|f| {
                let row = &rows[f.source];
                f.corners.iter().map(|&c| row[c].clone()).collect()
            })
            .collect()
    }
    fn points<T: Clone>(
        m: &BTreeMap<String, BTreeMap<u32, T>>,
        id: &impl Fn(u32) -> Option<u32>,
    ) -> BTreeMap<String, BTreeMap<u32, T>> {
        m.iter()
            .map(|(k, map)| {
                (
                    k.clone(),
                    map.iter()
                        .filter_map(|(v, x)| id(*v).map(|i| (i, x.clone())))
                        .collect(),
                )
            })
            .collect()
    }
    out.uv_maps = source
        .uv_maps
        .iter()
        .map(|(k, rows)| (k.clone(), corners(rows, faces)))
        .collect();
    // Polyline spans aren't surface edges; filtering on all spans is fine.
    let edges: BTreeSet<(u32, u32)> = (0..out.faces.len())
        .flat_map(|f| out.element_spans(f).collect::<Vec<_>>())
        .map(|(a, b)| (a.min(b), a.max(b)))
        .collect();
    let (a, at) = (&source.attributes, &mut out.attributes);
    at.typed_maps = a
        .typed_maps
        .iter()
        .map(|(k, m)| {
            let mut m = m.clone();
            m.points = m
                .points
                .iter()
                .filter_map(|(v, x)| id(*v).map(|i| (i, x.clone())))
                .collect();
            if !m.corners.is_empty() {
                m.corners = corners(&m.corners, faces);
            }
            (k.clone(), m)
        })
        .collect();
    at.vertex_sets = a
        .vertex_sets
        .iter()
        .map(|(k, s)| (k.clone(), s.iter().filter_map(|&v| id(v)).collect()))
        .collect();
    at.polygon_sets = a
        .polygon_sets
        .iter()
        .map(|(k, s)| {
            (
                k.clone(),
                s.iter()
                    .filter_map(|f| whole.get(&(*f as usize)).copied())
                    .collect(),
            )
        })
        .collect();
    at.set_groups = a.set_groups.clone();
    at.point_uvs = points(&a.point_uvs, &id);
    at.point_weights = points(&a.point_weights, &id);
    at.point_colors = points(&a.point_colors, &id);
    at.relative_morphs = points(&a.relative_morphs, &id);
    at.corner_weights = a
        .corner_weights
        .iter()
        .map(|(k, rows)| (k.clone(), corners(rows, faces)))
        .collect();
    at.corner_colors = a
        .corner_colors
        .iter()
        .map(|(k, rows)| (k.clone(), corners(rows, faces)))
        .collect();
    at.corner_normals = a
        .corner_normals
        .iter()
        .map(|(k, rows)| (k.clone(), corners(rows, faces)))
        .collect();
    at.uv_records = a
        .uv_records
        .iter()
        .map(|(k, rows)| (k.clone(), corners(rows, faces)))
        .collect();
    at.normal_map_order = a.normal_map_order.clone();
    if !a.representations.is_empty() {
        at.representations = faces.iter().map(|f| a.representations[f.source]).collect();
    }
    if !a.smoothing_labels.is_empty() {
        at.smoothing_labels = faces
            .iter()
            .map(|f| a.smoothing_labels[f.source].clone())
            .collect();
    }
    at.degenerate_faces = a
        .degenerate_faces
        .iter()
        .filter_map(|f| whole.get(&(*f as usize)).copied())
        .collect();
    at.creases = a
        .creases
        .iter()
        .filter_map(|c| {
            let (x, y) = (id(c.a)?, id(c.b)?);
            let (x, y) = (x.min(y), x.max(y));
            edges.contains(&(x, y)).then(|| crate::MeshCrease {
                a: x,
                b: y,
                ..c.clone()
            })
        })
        .collect();
    at.edge_sets = a
        .edge_sets
        .iter()
        .map(|(k, s)| {
            (
                k.clone(),
                s.iter()
                    .filter_map(|&(x, y)| {
                        let (x, y) = (id(x)?, id(y)?);
                        let e = (x.min(y), x.max(y));
                        edges.contains(&e).then_some(e)
                    })
                    .collect(),
            )
        })
        .collect();
    out.validate()?;
    Ok((out, point))
}

pub(crate) fn whole_faces(mesh: &Mesh, keep: impl Fn(usize) -> bool) -> Vec<KeptFace> {
    (0..mesh.faces.len())
        .filter(|&f| keep(f))
        .map(|f| KeptFace {
            source: f,
            corners: (0..mesh.faces[f].vertices.len()).collect(),
        })
        .collect()
}

fn loose_points(mesh: &Mesh) -> BTreeSet<u32> {
    mesh.used_points()
        .iter()
        .enumerate()
        .filter(|(_, used)| !**used)
        .map(|(v, _)| v as u32)
        .collect()
}

fn transformed(mut mesh: Mesh, m: DMat4) -> Mesh {
    for v in &mut mesh.vertices {
        *v = m.transform_point3(*v);
    }
    mesh
}

/// The selected elements of `mesh` as their own mesh (local space).
fn copied(mesh: &Mesh, mode: Mode, sel: &crate::scene::Selection) -> Result<Mesh> {
    match mode {
        Mode::Vertex => {
            let points: BTreeSet<u32> = sel.vertices.iter().copied().collect();
            Ok(subset(mesh, &[], &points)?.0)
        }
        Mode::Edge => {
            let mut out = Mesh::default();
            let mut at: BTreeMap<u32, u32> = BTreeMap::new();
            for &(a, b) in &sel.edges {
                let mut index = |v: u32| {
                    *at.entry(v).or_insert_with(|| {
                        out.vertices.push(mesh.vertices[v as usize]);
                        out.vertices.len() as u32 - 1
                    })
                };
                let (a, b) = (index(a), index(b));
                out.faces.push(Face {
                    vertices: vec![a, b],
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                });
                out.attributes
                    .representations
                    .push(PolygonRepresentation::Polyline {
                        closed: false,
                        curve: Default::default(),
                    });
            }
            out.validate()?;
            Ok(out)
        }
        Mode::Polygon => {
            let faces: BTreeSet<usize> = sel.faces.iter().map(|&f| f as usize).collect();
            Ok(subset(
                mesh,
                &whole_faces(mesh, |f| faces.contains(&f)),
                &BTreeSet::new(),
            )?
            .0)
        }
        Mode::Item => {
            Err("Cut and Copy work on vertices, edges or polygons; use Duplicate for items".into())
        }
    }
}

/// `mesh` with the selected elements cut away (CLP-09).
fn cut_away(mesh: &Mesh, mode: Mode, sel: &crate::scene::Selection) -> Result<Mesh> {
    let loose = loose_points(mesh);
    let faces = match mode {
        Mode::Vertex => {
            let gone: BTreeSet<u32> = sel.vertices.iter().copied().collect();
            (0..mesh.faces.len())
                .filter_map(|f| {
                    let corners: Vec<usize> = mesh.faces[f]
                        .vertices
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| !gone.contains(v))
                        .map(|(c, _)| c)
                        .collect();
                    let min = if mesh.is_polyline(f) { 2 } else { 3 };
                    (corners.len() >= min).then_some(KeptFace { source: f, corners })
                })
                .collect::<Vec<_>>()
        }
        Mode::Edge => {
            let gone: BTreeSet<(u32, u32)> = sel
                .edges
                .iter()
                .map(|&(a, b)| (a.min(b), a.max(b)))
                .collect();
            whole_faces(mesh, |f| {
                !mesh
                    .element_spans(f)
                    .any(|(a, b)| gone.contains(&(a.min(b), a.max(b))))
            })
        }
        Mode::Polygon => {
            let gone: BTreeSet<u32> = sel.faces.iter().copied().collect();
            whole_faces(mesh, |f| !gone.contains(&(f as u32)))
        }
        Mode::Item => return Err("Cut works on vertices, edges or polygons".into()),
    };
    let keep_loose: BTreeSet<u32> = match mode {
        Mode::Vertex => loose
            .into_iter()
            .filter(|v| !sel.vertices.contains(v))
            .collect(),
        _ => loose,
    };
    Ok(subset(mesh, &faces, &keep_loose)?.0)
}

fn nothing_selected(mode: Mode, sel: &crate::scene::Selection) -> bool {
    match mode {
        Mode::Vertex => sel.vertices.is_empty(),
        Mode::Edge => sel.edges.is_empty(),
        Mode::Polygon => sel.faces.is_empty(),
        Mode::Item => false,
    }
}

impl Editor {
    fn clipboard_source(&self, what: &str) -> Result<(u64, Mode, Mesh)> {
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err(format!("Finish the selection gesture before {what}"));
        }
        let mode = self.scene.selection.mode;
        if mode == Mode::Item {
            return Err(format!(
                "{what} works on vertices, edges or polygons; use Duplicate for items"
            ));
        }
        let item = self
            .scene
            .active()
            .filter(|i| i.is_mesh())
            .ok_or_else(|| format!("Select a mesh before {what}"))?;
        if nothing_selected(mode, &self.scene.selection) {
            return Err(format!("Select something to {}", what.to_lowercase()));
        }
        let mesh = copied(&item.mesh, mode, &self.scene.selection)?;
        let world = self.scene.world_matrix(item.id)?;
        Ok((item.id, mode, transformed(mesh, world)))
    }

    /// Copy: the clipboard takes the selection; the scene doesn't change.
    pub fn copy_selection(&mut self) -> Result<()> {
        let (_, mode, mesh) = self.clipboard_source("Copy")?;
        self.clipboard = Some(Box::new(Clipboard { mode, mesh }));
        // Deselect Elements after Copying (Preferences > Mesh Items).
        if crate::user_defaults::current().deselect_after_copy {
            let before = self.scene.clone();
            let sel = &mut self.scene.selection;
            sel.vertices.clear();
            sel.edges.clear();
            sel.faces.clear();
            if before != self.scene {
                self.record("Copy".into(), before);
            }
        }
        Ok(())
    }

    pub(crate) fn cut_selection(&mut self) -> Result<()> {
        let (item, mode, mesh) = self.clipboard_source("Cut")?;
        let source = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let rest = cut_away(source, mode, &self.scene.selection)?;
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        self.scene.active_mut().ok_or("Select a mesh")?.mesh = rest.into();
        let sel = &mut self.scene.selection;
        sel.vertices.clear();
        sel.edges.clear();
        sel.faces.clear();
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.invalidate_mesh(&self.scene, item))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.clipboard = Some(Box::new(Clipboard { mode, mesh }));
        self.record("Cut".into(), before);
        Ok(())
    }

    /// Paste (`placement` None: where it was copied) or one Paste Tool copy.
    pub(crate) fn paste(&mut self, placement: Option<PastePlacement>) -> Result<()> {
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Paste".into());
        }
        let clip = self
            .clipboard
            .clone()
            .ok_or("Nothing to paste; Cut or Copy first")?;
        let mut mesh = clip.mesh;
        if let Some(p) = placement {
            if !(p.center.is_finite() && p.size.is_finite() && p.bias.is_finite())
                || p.size.min_element() <= 0.
            {
                return Err("Paste Tool: Size must be above 0% and values finite".into());
            }
            let (lo, hi) = mesh.vertices.iter().fold(
                (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
                |(lo, hi), v| (lo.min(*v), hi.max(*v)),
            );
            let mid = (lo + hi) / 2.;
            for v in &mut mesh.vertices {
                *v = p.center + (*v - mid) * p.size + p.bias;
            }
        }
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        let mut scene = self.scene.clone();
        let target = match scene.active().filter(|i| i.is_mesh() && scene.shows(i)) {
            Some(i) => i.id,
            None => {
                let next = scene.next_id.max(self.next_item_id);
                if next >= u64::MAX - 1 || scene.items.len() >= crate::MAX_SCENE_ITEMS {
                    return Err("Scene item/identity budget exhausted".into());
                }
                scene.next_id = next;
                scene.new_mesh()?;
                scene.active().ok_or("Missing new mesh")?.id
            }
        };
        let world = scene.world_matrix(target)?;
        if world.determinant().abs() < 1e-15 {
            return Err("The mesh's transform is flat; Paste can't place geometry in it".into());
        }
        let local = transformed(mesh, world.inverse());
        let item = scene.item_mut(target).ok_or("Missing paste target")?;
        let (points, faces) = (
            item.mesh.vertices.len() as u32,
            item.mesh.faces.len() as u32,
        );
        let (added_points, added_faces) = (local.vertices.len() as u32, local.faces.len() as u32);
        let edges: Vec<(u32, u32)> = if clip.mode == Mode::Edge {
            local
                .faces
                .iter()
                .map(|f| {
                    let (a, b) = (f.vertices[0] + points, f.vertices[1] + points);
                    (a.min(b), a.max(b))
                })
                .collect()
        } else {
            Vec::new()
        };
        crate::append_mesh(&mut item.mesh, local)?;
        // Copy and Paste preferences: whether the old selection goes first
        // and whether the pasted elements are selected (both on so far).
        let rules = crate::user_defaults::current();
        if scene.selection.mode != clip.mode || rules.deselect_before_paste {
            scene.selection.vertices.clear();
            scene.selection.edges.clear();
            scene.selection.faces.clear();
        }
        scene.selection.mode = clip.mode;
        if rules.select_pasted {
            match clip.mode {
                Mode::Vertex => scene
                    .selection
                    .vertices
                    .extend(points..points + added_points),
                Mode::Edge => scene.selection.edges.extend(edges),
                _ => scene.selection.faces.extend(faces..faces + added_faces),
            }
        }
        self.scene = scene;
        self.next_item_id = self.next_item_id.max(self.scene.next_id);
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.invalidate_mesh(&self.scene, target))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record(
            if placement.is_some() {
                "Paste Tool"
            } else {
                "Paste"
            }
            .into(),
            before,
        );
        Ok(())
    }
}
