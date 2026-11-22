//! Shape-specific Cube bounds. Preview transactions, targeting and history belong
//! to Editor; adding a primitive must supply its own parameters and generator.
use crate::Result;
use glam::DVec3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxHandleMode {
    FromCenter,
    #[default]
    Resize,
    MoveAll,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxBounds {
    pub min: DVec3,
    pub max: DVec3,
}
impl BoxBounds {
    pub fn new(position: DVec3, size: DVec3) -> Result<Self> {
        // PRIM-C-09: a negative size is treated as positive.
        let size = size.abs();
        if !position.is_finite()
            || position.abs().max_element() > 1e9
            || !size.is_finite()
            || size.max_element() > 1e8
        {
            return Err("Cube requires finite dimensions within the size budget".into());
        }
        Ok(Self {
            min: position - size * 0.5,
            max: position + size * 0.5,
        })
    }
    pub fn position(self) -> DVec3 {
        (self.min + self.max) * 0.5
    }
    pub fn size(self) -> DVec3 {
        self.max - self.min
    }
    /// One selected boundary; all other axes remain unchanged. Crossed bounds
    /// are kept as they are (negative size). The caller owns pointer
    /// interpretation.
    pub fn edit_side(
        self,
        axis: usize,
        maximum: bool,
        coordinate: f64,
        mode: BoxHandleMode,
    ) -> Result<Self> {
        if axis > 2 || !coordinate.is_finite() {
            return Err("Invalid Cube side coordinate".into());
        }
        let mut next = self;
        let old = if maximum {
            self.max[axis]
        } else {
            self.min[axis]
        };
        let delta = coordinate - old;
        match mode {
            BoxHandleMode::MoveAll => {
                next.min[axis] += delta;
                next.max[axis] += delta;
            }
            BoxHandleMode::Resize => {
                if maximum {
                    next.max[axis] = coordinate;
                } else {
                    next.min[axis] = coordinate;
                }
            }
            BoxHandleMode::FromCenter => {
                if maximum {
                    next.max[axis] += delta;
                    next.min[axis] -= delta;
                } else {
                    next.min[axis] += delta;
                    next.max[axis] -= delta;
                }
            }
        }
        // No clamping (unit-primitive-2026-11-05 rule 5): Min past Max is
        // accepted and gives a negative Size; `new` reads a negative size as
        // positive, so the geometry is the sorted box with outward normals.
        Self::new(next.position(), next.size())?;
        Ok(next)
    }
    /// Viewport hauling of one side: like [`Self::edit_side`], but dragging a
    /// side past the opposite one builds the box on the other side instead of
    /// refusing it: a box can be drawn toward + or − on any axis.
    /// The faces stay outward-facing because the bounds are reordered.
    pub fn haul_side(
        self,
        axis: usize,
        maximum: bool,
        coordinate: f64,
        mode: BoxHandleMode,
    ) -> Result<Self> {
        if mode == BoxHandleMode::MoveAll {
            return self.edit_side(axis, maximum, coordinate, mode);
        }
        if axis > 2 || !coordinate.is_finite() {
            return Err("Invalid Cube side coordinate".into());
        }
        let mut next = self;
        let fixed = if maximum {
            self.min[axis]
        } else {
            self.max[axis]
        };
        let (lo, hi) = if mode == BoxHandleMode::FromCenter {
            let center = (self.min[axis] + self.max[axis]) * 0.5;
            let half = (coordinate - center).abs();
            (center - half, center + half)
        } else {
            (fixed.min(coordinate), fixed.max(coordinate))
        };
        next.min[axis] = lo;
        next.max[axis] = hi;
        Self::new(next.position(), next.size())?;
        Ok(next)
    }
}

/// Approved BOX-Q01 product topology: one quad at zero depth, empty for two
/// zero axes; a shared eight-vertex box once every axis has positive extent.
pub fn cube_geometry(position: DVec3, size: DVec3) -> Result<crate::Mesh> {
    BoxBounds::new(position, size)?;
    // PRIM-C-09: a negative size builds the same cube as the positive one.
    let size = size.abs();
    let zero: Vec<_> = (0..3).filter(|&axis| size[axis] == 0.).collect();
    if zero.is_empty() {
        return crate::Mesh::box_mesh(position, size, 0);
    }
    if zero.len() > 1 {
        return Ok(crate::Mesh::default());
    }
    let normal = zero[0];
    let u = (normal + 1) % 3;
    let v = (normal + 2) % 3;
    let mut vertices = Vec::new();
    for (a, b) in [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)] {
        let mut point = position;
        point[u] += a * size[u] * 0.5;
        point[v] += b * size[v] * 0.5;
        vertices.push(point);
    }
    let mesh = crate::Mesh {
        vertices,
        faces: vec![crate::Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    };
    mesh.validate()?;
    Ok(mesh)
}
/// Shared primitive insertion service. Extend existing named sparse maps with
/// absent new corners, retaining every prior corner and seam exactly.
/// New polygons take the user's Default Material, and the primitive UV map
/// the Default Texture Map name.
pub fn append_geometry(target: &mut crate::Mesh, mut generated: crate::Mesh) -> Result<()> {
    let material = crate::user_defaults::new_polygon_material();
    if !material.is_empty() {
        for face in &mut generated.faces {
            if face.material.is_empty() {
                face.material = material.clone();
            }
        }
    }
    let texture_map = crate::user_defaults::current().texture_map;
    if texture_map != crate::PRIMITIVE_UV_MAP
        && let Some(map) = generated.uv_maps.remove(crate::PRIMITIVE_UV_MAP)
    {
        generated.uv_maps.insert(texture_map, map);
    }
    crate::append_mesh(target, generated)
}

/// Shared-grid Cube surface with a technical preview budget, rather than a
/// permanent arbitrary per-axis segment cap. At 250k faces the owned face,
/// corner and snapshot allocations remain bounded for interactive replacement.
pub fn segmented_cube(position: DVec3, size: DVec3, segments: [u32; 3]) -> Result<crate::Mesh> {
    let bounds = BoxBounds::new(position, size)?;
    if segments.contains(&0) {
        return Err("Segments must be positive integers".into());
    }
    let zero: Vec<_> = (0..3).filter(|&i| size[i] == 0.).collect();
    if zero.len() > 1 {
        return Ok(crate::Mesh::default());
    }
    let [a, b, c] = segments.map(u128::from);
    let faces = if let Some(&normal) = zero.first() {
        u128::from(segments[(normal + 1) % 3]) * u128::from(segments[(normal + 2) % 3])
    } else {
        2 * (a * b + b * c + a * c)
    };
    if faces > 250_000 {
        return Err(
            "Cube preview exceeds the 250,000-face interactive allocation budget; reduce Segments"
                .into(),
        );
    }
    if segments == [1; 3] {
        return cube_geometry(position, size);
    }
    let mut mesh = crate::Mesh::default();
    let mut indices = std::collections::BTreeMap::<[u32; 3], u32>::new();
    let planes: Vec<_> = if let Some(&normal) = zero.first() {
        vec![(normal, true)]
    } else {
        (0..3).flat_map(|n| [(n, false), (n, true)]).collect()
    };
    for (normal, maximum) in planes {
        let u = (normal + 1) % 3;
        let v = (normal + 2) % 3;
        for j in 0..segments[v] {
            for i in 0..segments[u] {
                let mut corners = Vec::with_capacity(4);
                for (x, y) in [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)] {
                    let mut key = [0; 3];
                    key[normal] = if maximum && zero.is_empty() {
                        segments[normal]
                    } else {
                        0
                    };
                    key[u] = x;
                    key[v] = y;
                    let index = *indices.entry(key).or_insert_with(|| {
                        let index = mesh.vertices.len() as u32;
                        mesh.vertices.push(
                            bounds.min
                                + size
                                    * DVec3::new(
                                        key[0] as f64 / segments[0] as f64,
                                        key[1] as f64 / segments[1] as f64,
                                        key[2] as f64 / segments[2] as f64,
                                    ),
                        );
                        index
                    });
                    corners.push(index);
                }
                if !maximum {
                    corners.reverse();
                }
                mesh.faces.push(crate::Face {
                    vertices: corners,
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                });
            }
        }
    }
    mesh.validate()?;
    Ok(mesh)
}
