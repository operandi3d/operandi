//! Shared polygon-tag helpers (spec set-part-2026-11-04, SP-05 and SP-18 to
//! SP-20). A polygon carries four kinds of tag: Material, Part, Smoothing
//! Group and Selection Set. Material and Part are single-valued and live on
//! `Face`; Smoothing Group and Selection Set live in `MeshAttributes`. This
//! module only reads them, so Set Part, Set Material and the Select tab's
//! Polygon Tag dropdown can share one "which polygons" rule and one name list.
use crate::{Mesh, Scene};
use std::collections::BTreeSet;

/// The four polygon tag kinds, in the Select tab's dropdown order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PolygonTagKind {
    #[default]
    Material,
    Part,
    SmoothingGroup,
    SelectionSet,
}

impl PolygonTagKind {
    pub const ALL: [Self; 4] = [
        Self::Material,
        Self::Part,
        Self::SmoothingGroup,
        Self::SelectionSet,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Material => "Material",
            Self::Part => "Part",
            Self::SmoothingGroup => "Smoothing Group",
            Self::SelectionSet => "Selection Set",
        }
    }
    /// Shown by a name list when the scene has no tag of this kind.
    pub fn none_message(self) -> &'static str {
        match self {
            Self::Material => "No materials exist.",
            Self::Part => "No parts exist.",
            Self::SmoothingGroup => "No smoothing groups exist.",
            Self::SelectionSet => "No selection sets exist.",
        }
    }
}

/// Every tag name of `kind` that `face` of `mesh` carries. Untagged polygons
/// give nothing (SP-Q06, provisional: they are hidden from name lists).
pub fn polygon_tags(mesh: &Mesh, face: usize, kind: PolygonTagKind) -> Vec<&str> {
    let Some(f) = mesh.faces.get(face) else {
        return Vec::new();
    };
    match kind {
        PolygonTagKind::Material => {
            // Blank and "Default" both mean the default material, which is
            // not a tag a name list offers (matches Set Material's model).
            let name = f.material.as_str();
            (!name.trim().is_empty() && name != "Default")
                .then_some(name)
                .into_iter()
                .collect()
        }
        PolygonTagKind::Part => f.part.as_deref().into_iter().collect(),
        PolygonTagKind::SmoothingGroup => mesh
            .attributes
            .smoothing_labels
            .get(face)
            .and_then(Option::as_deref)
            .into_iter()
            .collect(),
        PolygonTagKind::SelectionSet => mesh
            .attributes
            .polygon_sets
            .iter()
            .filter(|(_, faces)| faces.contains(&(face as u32)))
            .map(|(name, _)| name.as_str())
            .collect(),
    }
}

/// Polygons of `mesh` carrying the tag `name` of `kind`, in index order.
pub fn faces_with_tag(mesh: &Mesh, kind: PolygonTagKind, name: &str) -> Vec<u32> {
    (0..mesh.faces.len())
        .filter(|&f| polygon_tags(mesh, f, kind).contains(&name))
        .map(|f| f as u32)
        .collect()
}

/// SP-05: the polygons a tag command writes. Selected polygons when there
/// are any, otherwise every surface polygon of the mesh. Line polygons
/// (curves) are left out of the "all" case until curve tagging is built
/// (SP-07); an explicit selection is checked by the caller.
/// The polygons Set Material and Set Part tag in `selection`'s mode
/// (set-material-2026-11-05 and set-part-observed-2026-11-05): the selected
/// polygons in Polygon mode, or every surface polygon with nothing selected;
/// every polygon in Item mode; in Vertex or Edge mode none when something is
/// selected (decided Q-SM-01, Q-PT-02) and every polygon otherwise.
pub fn tag_target_faces_in(mesh: &Mesh, selection: &crate::Selection) -> Vec<u32> {
    match selection.mode {
        crate::Mode::Polygon => tag_target_faces(mesh, &selection.faces),
        crate::Mode::Item => tag_target_faces(mesh, &[]),
        crate::Mode::Vertex if !selection.vertices.is_empty() => Vec::new(),
        crate::Mode::Edge if !selection.edges.is_empty() => Vec::new(),
        _ => tag_target_faces(mesh, &[]),
    }
}

pub fn tag_target_faces(mesh: &Mesh, selected: &[u32]) -> Vec<u32> {
    if !selected.is_empty() {
        return selected.to_vec();
    }
    (0..mesh.faces.len())
        .filter(|&f| !mesh.is_polyline(f))
        .map(|f| f as u32)
        .collect()
}

impl Scene {
    /// SP-11/SP-19/SP-Q07: every distinct tag name of `kind` on any mesh in
    /// the scene (hidden meshes included), sorted A to Z, each once.
    pub fn polygon_tag_names(&self, kind: PolygonTagKind) -> Vec<String> {
        let names: BTreeSet<&str> = self
            .items
            .iter()
            .flat_map(|i| (0..i.mesh.faces.len()).flat_map(move |f| polygon_tags(&i.mesh, f, kind)))
            .collect();
        names.into_iter().map(str::to_owned).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    fn quads() -> Mesh {
        let mut m = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        m.faces.truncate(2);
        m.attributes.representations.clear();
        m
    }

    #[test]
    fn tags_of_each_kind_are_read_where_they_live() {
        let mut m = quads();
        m.faces[0].part = Some("Lid".into());
        m.faces[0].material = "Red".into();
        m.faces[1].material = "Default".into();
        m.attributes.smoothing_labels = vec![Some("A".into()), None];
        m.attributes
            .polygon_sets
            .insert("Top".into(), BTreeSet::from([0, 1]));
        m.attributes
            .polygon_sets
            .insert("Front".into(), BTreeSet::from([0]));
        assert_eq!(polygon_tags(&m, 0, PolygonTagKind::Part), ["Lid"]);
        assert!(polygon_tags(&m, 1, PolygonTagKind::Part).is_empty());
        assert_eq!(polygon_tags(&m, 0, PolygonTagKind::Material), ["Red"]);
        assert!(polygon_tags(&m, 1, PolygonTagKind::Material).is_empty());
        assert_eq!(polygon_tags(&m, 0, PolygonTagKind::SmoothingGroup), ["A"]);
        assert_eq!(
            polygon_tags(&m, 0, PolygonTagKind::SelectionSet),
            ["Front", "Top"]
        );
        assert_eq!(
            faces_with_tag(&m, PolygonTagKind::SelectionSet, "Top"),
            [0, 1]
        );
        assert_eq!(faces_with_tag(&m, PolygonTagKind::Part, "Lid"), [0]);
        assert!(polygon_tags(&m, 9, PolygonTagKind::Part).is_empty());
    }

    #[test]
    fn empty_selection_targets_every_surface_polygon() {
        let m = quads();
        assert_eq!(tag_target_faces(&m, &[1]), [1]);
        assert_eq!(tag_target_faces(&m, &[]), [0, 1]);
    }

    #[test]
    fn scene_names_are_sorted_once_each_across_meshes() {
        let mut s = Scene::default();
        s.new_mesh().unwrap();
        s.new_mesh().unwrap();
        let mut a = quads();
        a.faces[0].part = Some("Lid".into());
        a.faces[1].part = Some("Body".into());
        let mut b = quads();
        b.faces[0].part = Some("Lid".into());
        s.items[0].mesh = a.into();
        s.items[1].mesh = b.into();
        s.items[1].visible = false;
        assert_eq!(s.polygon_tag_names(PolygonTagKind::Part), ["Body", "Lid"]);
        assert!(
            s.polygon_tag_names(PolygonTagKind::SmoothingGroup)
                .is_empty()
        );
        assert_eq!(PolygonTagKind::Part.none_message(), "No parts exist.");
    }
}
