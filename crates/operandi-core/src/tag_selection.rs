//! Polygon tag selection modes (colour-nav note 04): Material, Part,
//! Smoothing Group and Selection Set modes. A click selects every polygon
//! that carries the same tag of the current kind (a whole tagged region).
//!
//! The result is stored as an ordinary polygon selection plus the active
//! region list (PT-B3), so polygon tools run on it unchanged and Shift/Ctrl
//! add or remove whole regions. The mode itself is Polygon mode with
//! `Selection::tag` set; any other mode change clears it.
use crate::polygon_tags::{PolygonTagKind, polygon_tags};
use crate::{Mesh, Mode, Result, Scene};
use std::collections::BTreeSet;

/// One tagged region: a tag name, or `None` for the polygons with no tag of
/// that kind (PT-B2).
pub type TagRegion = Option<String>;

/// The active tag selection: which kind of tag clicks select by, and the
/// regions selected so far, in the order they were added.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TagSelection {
    pub kind: PolygonTagKind,
    pub regions: Vec<TagRegion>,
}

impl PolygonTagKind {
    /// Whether this kind can be chosen yet. Smoothing Group is listed but
    /// greyed out (colour-nav decision): Operandi has no smoothing group
    /// tags to author yet.
    pub fn available(self) -> bool {
        self != Self::SmoothingGroup
    }
    /// Why an unavailable kind is greyed out.
    pub fn unavailable_reason(self) -> Option<&'static str> {
        (!self.available()).then_some("no smoothing group tags yet")
    }
}

/// The regions `face` belongs to: each of its tags (PT-B1: every set it is
/// in), or the untagged region.
pub fn face_regions(mesh: &Mesh, face: usize, kind: PolygonTagKind) -> Vec<TagRegion> {
    let tags = polygon_tags(mesh, face, kind);
    if tags.is_empty() {
        vec![None]
    } else {
        tags.into_iter().map(|t| Some(t.to_owned())).collect()
    }
}

/// Every polygon of `mesh` in `region` (line polygons only when tagged).
pub fn region_faces(mesh: &Mesh, kind: PolygonTagKind, region: &TagRegion) -> Vec<u32> {
    (0..mesh.faces.len())
        .filter(|&f| match region {
            Some(name) => polygon_tags(mesh, f, kind).contains(&name.as_str()),
            None => !mesh.is_polyline(f) && polygon_tags(mesh, f, kind).is_empty(),
        })
        .map(|f| f as u32)
        .collect()
}

/// The outline of a polygon selection: edges used by exactly one selected
/// polygon (the region's border, open borders included). Reads only the
/// selected polygons, so it stays cheap on big meshes.
pub fn outline_edges(mesh: &Mesh, faces: &[u32]) -> Vec<(u32, u32)> {
    let mut uses: crate::fxhash::FxHashMap<(u32, u32), u32> = Default::default();
    for &f in faces {
        let Some(face) = mesh.faces.get(f as usize) else {
            continue;
        };
        if mesh.is_polyline(f as usize) {
            continue;
        }
        let v = &face.vertices;
        for i in 0..v.len() {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let mut out: Vec<(u32, u32)> = uses
        .into_iter()
        .filter(|&(_, n)| n == 1)
        .map(|(e, _)| e)
        .collect();
    out.sort_unstable();
    out
}

impl Scene {
    /// Whether a tag selection mode is on.
    pub fn tag_mode(&self) -> Option<PolygonTagKind> {
        (self.selection.mode == Mode::Polygon)
            .then(|| self.selection.tag.as_ref().map(|t| t.kind))
            .flatten()
    }

    /// Enter the tag mode of `kind`. The polygon selection stays (it still
    /// implies polygons); with `convert` (Alt+click on the mode button) the
    /// current selection grows to the whole regions it touches.
    pub fn enter_tag_mode(&mut self, kind: PolygonTagKind, convert: bool) -> Result<()> {
        if !kind.available() {
            return Err(format!(
                "{} mode isn't available: {}",
                kind.label(),
                kind.unavailable_reason().unwrap_or_default()
            ));
        }
        let touched = if convert {
            self.touched_faces()
        } else {
            Vec::new()
        };
        self.selection.mode = Mode::Polygon;
        self.selection.tag = Some(TagSelection {
            kind,
            regions: Vec::new(),
        });
        if convert {
            self.selection.faces.clear();
            let Some(item) = self.selection.item else {
                return Ok(());
            };
            for f in touched {
                self.select_tag_region(item, f, false, false)?;
            }
        }
        Ok(())
    }

    /// Change which tag kind clicks select by. The mode and the current
    /// polygon selection stay; the region list starts again (provisional,
    /// PT-Q3).
    pub fn set_tag_kind(&mut self, kind: PolygonTagKind) {
        if let Some(tag) = self.selection.tag.as_mut()
            && tag.kind != kind
        {
            tag.kind = kind;
            tag.regions.clear();
        }
    }

    /// The polygons the current selection touches, in any mode: selected
    /// polygons; polygons using a selected point or edge; every polygon in
    /// Item mode.
    fn touched_faces(&self) -> Vec<u32> {
        let s = &self.selection;
        let Some(item) = self.active() else {
            return Vec::new();
        };
        let mesh = &item.mesh;
        let all = || (0..mesh.faces.len() as u32).collect::<Vec<_>>();
        match s.mode {
            Mode::Polygon => s.faces.clone(),
            Mode::Item => all(),
            Mode::Vertex => {
                let points: BTreeSet<u32> = s.vertices.iter().copied().collect();
                all()
                    .into_iter()
                    .filter(|&f| {
                        mesh.faces[f as usize]
                            .vertices
                            .iter()
                            .any(|v| points.contains(v))
                    })
                    .collect()
            }
            Mode::Edge => {
                let edges: BTreeSet<(u32, u32)> =
                    s.edges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
                all()
                    .into_iter()
                    .filter(|&f| {
                        let v = &mesh.faces[f as usize].vertices;
                        (0..v.len()).any(|i| {
                            let (a, b) = (v[i], v[(i + 1) % v.len()]);
                            edges.contains(&(a.min(b), a.max(b)))
                        })
                    })
                    .collect()
            }
        }
    }

    /// A click on polygon `face` of `item` in a tag mode: select (`replace`),
    /// add, or with `remove` deselect every region the polygon belongs to.
    pub(crate) fn select_tag_region(
        &mut self,
        item: u64,
        face: u32,
        replace: bool,
        remove: bool,
    ) -> Result<()> {
        let Some(kind) = self.selection.tag.as_ref().map(|t| t.kind) else {
            return Err("No tag selection mode is on".into());
        };
        if remove && self.selection.item != Some(item) {
            return Ok(());
        }
        self.select_item(Some(item));
        let mesh = &self
            .items
            .iter()
            .find(|i| i.id == item)
            .ok_or("Missing item")?
            .mesh;
        let regions = face_regions(mesh, face as usize, kind);
        let tag = self.selection.tag.get_or_insert_with(Default::default);
        if replace {
            tag.regions.clear();
            self.selection.faces.clear();
        }
        if remove {
            tag.regions.retain(|r| !regions.contains(r));
            let mut out: BTreeSet<u32> = BTreeSet::new();
            for r in &regions {
                out.extend(region_faces(mesh, kind, r));
            }
            self.selection.faces.retain(|f| !out.contains(f));
            // A polygon in a removed set and a kept one stays selected.
            let kept: Vec<TagRegion> = tag.regions.clone();
            let mut have: BTreeSet<u32> = self.selection.faces.iter().copied().collect();
            for r in &kept {
                for f in region_faces(mesh, kind, r) {
                    if have.insert(f) {
                        self.selection.faces.push(f);
                    }
                }
            }
        } else {
            let mut have: BTreeSet<u32> = self.selection.faces.iter().copied().collect();
            for r in regions {
                for f in region_faces(mesh, kind, &r) {
                    if have.insert(f) {
                        self.selection.faces.push(f);
                    }
                }
                if !tag.regions.contains(&r) {
                    tag.regions.push(r);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Editor};
    use glam::DVec3;

    /// A six-polygon box: polygons 0-1 "Glass", 2-3 "Metal", 4-5 untagged;
    /// set "Top" holds 1 and 2, set "Side" holds 2.
    fn editor() -> (Editor, u64) {
        let mut e = Editor::default();
        let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        for (f, name) in ["Glass", "Glass", "Metal", "Metal", "Default", ""]
            .iter()
            .enumerate()
        {
            mesh.faces[f].material = (*name).into();
        }
        mesh.faces[0].part = Some("Lid".into());
        mesh.attributes
            .polygon_sets
            .insert("Top".into(), [1, 2].into_iter().collect());
        mesh.attributes
            .polygon_sets
            .insert("Side".into(), [2].into_iter().collect());
        e.scene.new_mesh().unwrap();
        let item = e.scene.items.last_mut().unwrap();
        item.mesh = mesh.into();
        let id = item.id;
        e.scene.select_item(Some(id));
        (e, id)
    }

    fn click(e: &mut Editor, item: u64, face: u32, shift: bool, ctrl: bool) {
        e.dispatch(Command::Select {
            item,
            hit: crate::Component::Polygon(face),
            replace: !shift && !ctrl,
            remove: ctrl,
        })
        .unwrap();
    }

    fn faces(e: &Editor) -> Vec<u32> {
        let mut f = e.scene.selection.faces.clone();
        f.sort_unstable();
        f
    }

    #[test]
    fn a_click_selects_the_whole_material_region() {
        let (mut e, id) = editor();
        e.dispatch(Command::EnterTagMode { convert: false })
            .unwrap();
        assert_eq!(e.scene.tag_mode(), Some(PolygonTagKind::Material));
        click(&mut e, id, 1, false, false);
        assert_eq!(faces(&e), vec![0, 1]);
        // Shift adds a region, Ctrl removes one.
        click(&mut e, id, 3, true, false);
        assert_eq!(faces(&e), vec![0, 1, 2, 3]);
        click(&mut e, id, 0, false, true);
        assert_eq!(faces(&e), vec![2, 3]);
        // Each click is one selection undo step.
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(faces(&e), vec![0, 1, 2, 3]);
    }

    #[test]
    fn untagged_polygons_are_one_region() {
        let (mut e, id) = editor();
        e.dispatch(Command::EnterTagMode { convert: false })
            .unwrap();
        click(&mut e, id, 5, false, false);
        // "Default" and blank both mean untagged (PT-B2).
        assert_eq!(faces(&e), vec![4, 5]);
        assert_eq!(e.scene.selection.tag.as_ref().unwrap().regions, vec![None]);
    }

    #[test]
    fn a_polygon_in_several_sets_selects_their_union() {
        let (mut e, id) = editor();
        e.dispatch(Command::SetPolygonTagKind(PolygonTagKind::SelectionSet))
            .unwrap();
        e.dispatch(Command::EnterTagMode { convert: false })
            .unwrap();
        click(&mut e, id, 2, false, false);
        assert_eq!(faces(&e), vec![1, 2]);
        // Removing "Side" keeps polygon 2: it is still in "Top".
        let tag = e.scene.selection.tag.clone().unwrap();
        assert_eq!(tag.regions.len(), 2);
        e.scene.selection.tag.as_mut().unwrap().regions = vec![Some("Top".into())];
        e.scene.select_tag_region(id, 1, false, false).unwrap();
        assert_eq!(faces(&e), vec![1, 2]);
    }

    #[test]
    fn smoothing_group_mode_is_refused_and_other_modes_leave_tag_mode() {
        let (mut e, id) = editor();
        assert!(
            e.dispatch(Command::SetPolygonTagKind(PolygonTagKind::SmoothingGroup))
                .is_err()
        );
        e.dispatch(Command::EnterTagMode { convert: false })
            .unwrap();
        click(&mut e, id, 0, false, false);
        e.dispatch(Command::Mode(Mode::Item)).unwrap();
        assert_eq!(e.scene.tag_mode(), None);
        // The polygons stay selected for Polygon mode.
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        assert_eq!(faces(&e), vec![0, 1]);
        assert_eq!(e.scene.tag_mode(), None);
    }

    #[test]
    fn alt_click_converts_the_selection_to_regions() {
        let (mut e, id) = editor();
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        click(&mut e, id, 2, false, false);
        e.dispatch(Command::EnterTagMode { convert: true }).unwrap();
        assert_eq!(faces(&e), vec![2, 3]);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(faces(&e), vec![2]);
    }

    #[test]
    fn changing_kind_in_the_mode_changes_what_clicks_select() {
        let (mut e, id) = editor();
        e.dispatch(Command::EnterTagMode { convert: false })
            .unwrap();
        e.dispatch(Command::SetPolygonTagKind(PolygonTagKind::Part))
            .unwrap();
        assert_eq!(e.scene.tag_mode(), Some(PolygonTagKind::Part));
        click(&mut e, id, 0, false, false);
        assert_eq!(faces(&e), vec![0]);
        click(&mut e, id, 3, false, false);
        assert_eq!(faces(&e), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn the_outline_is_the_region_border() {
        let (e, _) = editor();
        let mesh = &e.scene.items[0].mesh;
        // One quad: its four edges. Two adjacent quads: the shared edge goes.
        assert_eq!(outline_edges(mesh, &[0]).len(), 4);
        let all: Vec<u32> = (0..6).collect();
        assert!(outline_edges(mesh, &all).is_empty());
    }
}
