//! Vertex Map Texture and Variation Texture (gradient family v1 §3–4).
//!
//! Both read per-polygon-corner data the renderer exports as an RGBA
//! corner attribute (`attribute_name`): a weight map as (w, w, w, 1), a
//! colour map as its RGBA, a variation number as (r, r, r, 1). A mesh
//! without the map exports (0, 0, 0, 0), so alpha 0 means "this layer does
//! nothing here".
use crate::gradient::{Curve, Gradient};
use crate::mesh::Mesh;
use serde::{Deserialize, Serialize};

/// Which kind of vertex map a Vertex Map Texture reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VertexMapKind {
    Weight,
    /// RGB or RGBA.
    Color,
}

/// A vertex map by name and kind (the menu lists the scene's maps).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VertexMapRef {
    pub name: String,
    pub kind: VertexMapKind,
}

/// Variation Source (§4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariationSource {
    /// Each connected island of polygons.
    MeshPart,
    /// Each item.
    Item,
    /// [measured] the starting value. Operandi has no particles: r = 0.
    #[default]
    Particle,
    TextureParticle,
}
impl VariationSource {
    pub const ALL: [VariationSource; 4] = [
        VariationSource::MeshPart,
        VariationSource::Item,
        VariationSource::Particle,
        VariationSource::TextureParticle,
    ];
    pub fn label(self) -> &'static str {
        match self {
            VariationSource::MeshPart => "Mesh Part",
            VariationSource::Item => "Item",
            VariationSource::Particle => "Particle",
            VariationSource::TextureParticle => "Texture Particle",
        }
    }
    /// Sources that render as r = 0 until Operandi has particles.
    pub fn is_stored_only(self) -> bool {
        matches!(
            self,
            VariationSource::Particle | VariationSource::TextureParticle
        )
    }
}

/// A Variation Texture: its source and the Gradient's Value / Color curves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variation {
    pub source: VariationSource,
    pub value: Curve<f64>,
    pub color: Curve<[f64; 4]>,
}
impl Default for Variation {
    /// Particle, curves as a new Gradient's.
    fn default() -> Self {
        let g = Gradient::default();
        Self {
            source: VariationSource::Particle,
            value: g.value,
            color: g.color,
        }
    }
}
impl Variation {
    pub fn valid(&self) -> bool {
        self.value.valid()
            && self.color.valid()
            && !self.value.keys.is_empty()
            && !self.color.keys.is_empty()
    }
}

/// The renderer's attribute for a vertex map.
pub fn attribute_name(map: &VertexMapRef) -> String {
    match map.kind {
        VertexMapKind::Weight => format!("weight:{}", map.name),
        VertexMapKind::Color => format!("color:{}", map.name),
    }
}
/// The renderer's attribute for a variation source.
pub fn variation_attribute(source: VariationSource) -> Option<&'static str> {
    match source {
        VariationSource::MeshPart => Some("operandi:variation-part"),
        VariationSource::Item => Some("operandi:variation-item"),
        _ => None,
    }
}

/// A stable number in [0, 1) for `seed` (SplitMix64).
pub fn random01(seed: u64) -> f64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Per polygon: the index of its connected island (polygons sharing a
/// point are connected).
pub fn islands(mesh: &Mesh) -> Vec<u32> {
    let n = mesh.vertices.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for f in &mesh.faces {
        let Some(&first) = f.vertices.first() else {
            continue;
        };
        for &v in &f.vertices[1..] {
            let (a, b) = (
                find(&mut parent, first as usize),
                find(&mut parent, v as usize),
            );
            if a != b {
                parent[a] = b;
            }
        }
    }
    let mut ids = std::collections::HashMap::new();
    mesh.faces
        .iter()
        .map(|f| {
            let root = f
                .vertices
                .first()
                .map_or(usize::MAX, |&v| find(&mut parent, v as usize));
            let next = ids.len() as u32;
            *ids.entry(root).or_insert(next)
        })
        .collect()
}

/// Exports one corner attribute of `item`'s cage polygon `face` at its
/// corner `corner`: the RGBA the renderer reads (see the module notes).
/// `islands` comes from [`islands`] when a part variation is wanted.
pub fn corner_value(
    name: &str,
    item: u64,
    mesh: &Mesh,
    face: usize,
    corner: usize,
    islands: Option<&[u32]>,
) -> [f64; 4] {
    let point = mesh
        .faces
        .get(face)
        .and_then(|f| f.vertices.get(corner))
        .copied();
    if name == "operandi:variation-item" {
        let r = random01(item);
        return [r, r, r, 1.];
    }
    if name == "operandi:variation-part" {
        let island = islands.and_then(|i| i.get(face)).copied().unwrap_or(0);
        let r = random01(item.wrapping_mul(0x1000_0000_01B3) ^ u64::from(island));
        return [r, r, r, 1.];
    }
    let a = &mesh.attributes;
    if let Some(map) = name.strip_prefix("weight:") {
        // The polygon's own corner value where the map is discontinuous.
        let own = a
            .corner_weights
            .get(map)
            .and_then(|c| c.get(face))
            .and_then(|c| c.get(corner))
            .copied()
            .flatten();
        let at_point = point.and_then(|p| a.point_weights.get(map).and_then(|w| w.get(&p)));
        if own.is_none() && at_point.is_none() && !has_weight_map(mesh, map) {
            return [0.; 4];
        }
        // GQ-5: clamped to 0..1.
        let w = own.or(at_point.copied()).unwrap_or(0.).clamp(0., 1.);
        return [w, w, w, 1.];
    }
    if let Some(map) = name.strip_prefix("color:") {
        let own = a
            .corner_colors
            .get(map)
            .and_then(|c| c.get(face))
            .and_then(|c| c.get(corner))
            .copied()
            .flatten();
        let at_point = point.and_then(|p| a.point_colors.get(map).and_then(|w| w.get(&p)));
        if own.is_none() && at_point.is_none() && !has_color_map(mesh, map) {
            return [0.; 4];
        }
        // A point without a value: black, opaque.
        let c = own
            .or(at_point.copied())
            .unwrap_or(glam::DVec4::new(0., 0., 0., 1.));
        return c.to_array();
    }
    [0.; 4]
}

fn has_weight_map(mesh: &Mesh, name: &str) -> bool {
    mesh.attributes.point_weights.contains_key(name)
        || mesh.attributes.corner_weights.contains_key(name)
}
fn has_color_map(mesh: &Mesh, name: &str) -> bool {
    mesh.attributes.point_colors.contains_key(name)
        || mesh.attributes.corner_colors.contains_key(name)
}

impl crate::Scene {
    /// The weight and colour maps on the scene's meshes, by name.
    pub fn vertex_maps(&self) -> Vec<VertexMapRef> {
        let mut out = std::collections::BTreeSet::new();
        for item in self.items.iter().filter(|i| i.is_mesh()) {
            let a = &item.mesh.attributes;
            for name in a.point_weights.keys().chain(a.corner_weights.keys()) {
                out.insert(VertexMapRef {
                    name: name.clone(),
                    kind: VertexMapKind::Weight,
                });
            }
            for name in a.point_colors.keys().chain(a.corner_colors.keys()) {
                out.insert(VertexMapRef {
                    name: name.clone(),
                    kind: VertexMapKind::Color,
                });
            }
        }
        out.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    fn two_boxes() -> Mesh {
        let mut a = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let b = Mesh::box_mesh(DVec3::new(3., 0., 0.), DVec3::ONE, 1).unwrap();
        let offset = a.vertices.len() as u32;
        a.vertices.extend(b.vertices);
        for mut f in b.faces {
            for v in &mut f.vertices {
                *v += offset;
            }
            a.faces.push(f);
        }
        a
    }

    #[test]
    fn islands_split_unconnected_polygons() {
        let m = two_boxes();
        let i = islands(&m);
        assert_eq!(i.len(), 12);
        assert!(i[..6].iter().all(|&x| x == i[0]));
        assert!(i[6..].iter().all(|&x| x == i[6]));
        assert_ne!(i[0], i[6]);
        // Different islands draw different numbers, the same every time.
        let v = |f| corner_value("operandi:variation-part", 7, &m, f, 0, Some(&i));
        assert_ne!(v(0), v(6));
        assert_eq!(v(0), v(5));
        assert_eq!(v(0), v(0));
    }

    #[test]
    fn random_numbers_are_in_range_and_stable() {
        for s in 0..1000 {
            let r = random01(s);
            assert!((0.0..1.0).contains(&r));
            assert_eq!(r, random01(s));
        }
        assert_ne!(random01(1), random01(2));
    }

    #[test]
    fn weights_clamp_corners_win_and_missing_maps_are_transparent() {
        let mut m = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let p = m.faces[0].vertices[0];
        m.attributes
            .point_weights
            .entry("w".into())
            .or_default()
            .insert(p, 1.7);
        assert_eq!(
            corner_value("weight:w", 1, &m, 0, 0, None),
            [1., 1., 1., 1.]
        );
        // A point the map doesn't list: 0, still covered.
        assert_eq!(
            corner_value("weight:w", 1, &m, 0, 1, None),
            [0., 0., 0., 1.]
        );
        // A discontinuous corner value overrides the point's.
        let mut corners = vec![vec![None; 4]; m.faces.len()];
        corners[0][0] = Some(0.25);
        m.attributes.corner_weights.insert("w".into(), corners);
        assert_eq!(corner_value("weight:w", 1, &m, 0, 0, None)[0], 0.25);
        // No such map on this mesh: alpha 0.
        assert_eq!(corner_value("weight:none", 1, &m, 0, 0, None), [0.; 4]);
        assert_eq!(corner_value("color:none", 1, &m, 0, 0, None), [0.; 4]);
    }
}
