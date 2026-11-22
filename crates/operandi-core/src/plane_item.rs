//! Plane Item (the Text button's Shift-click). Spec: missing-tools-g2
//! G2-PLANE-TEAPOT v1, MT5-01..05; the shape is unit-primitive-2026-11-05
//! case UP12.
use crate::{Face, Mesh};
use glam::{DVec2, DVec3};

/// MT5-02/03: a 1 m square in the XZ plane facing +Y, one Face polygon,
/// with a "Texture" map: u = x + 0.5, v = 0.5 − z (−Z at the top).
pub fn plane_item_geometry() -> Mesh {
    let corners = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
    let vertices: Vec<DVec3> = corners.iter().map(|&(x, z)| DVec3::new(x, 0., z)).collect();
    let polygon = vec![0, 3, 2, 1];
    let uvs = polygon
        .iter()
        .map(|&v: &u32| {
            let p = vertices[v as usize];
            Some(DVec2::new(p.x + 0.5, 0.5 - p.z))
        })
        .collect();
    let mut mesh = Mesh {
        vertices,
        faces: vec![Face {
            vertices: polygon,
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    };
    mesh.uv_maps
        .insert(crate::PRIMITIVE_UV_MAP.into(), vec![uvs]);
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MT5-02/03: the corners in polygon order carry (0,1), (0,0), (1,0),
    /// (1,1), and the polygon faces +Y.
    #[test]
    fn the_plane_faces_up_with_its_uvs_in_corner_order() {
        let m = plane_item_geometry();
        assert_eq!(m.faces[0].vertices, vec![0, 3, 2, 1]);
        assert!(m.normal(0).normalize().abs_diff_eq(DVec3::Y, 1e-12));
        let row: Vec<DVec2> = m.uv_maps["Texture"][0].iter().map(|u| u.unwrap()).collect();
        assert_eq!(
            row,
            [
                DVec2::new(0., 1.),
                DVec2::new(0., 0.),
                DVec2::new(1., 0.),
                DVec2::new(1., 1.)
            ]
        );
        m.validate().unwrap();
    }
}
