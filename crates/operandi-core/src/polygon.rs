//! M17: independently implemented boundary-region edits. Face IDs and corner
//! ownership survive source replacement; triangulation never enters topology.
use crate::{Mesh, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// Per-operation allocation guards, not reference-product limits. Existing mesh
/// limits still apply. Bounds are checked before allocating generated geometry.
const GENERATED_VERTICES: usize = 1_000_000;
const GENERATED_FACES: usize = 1_000_000;
const SELECTED_CORNERS: usize = 100_000;

#[derive(Clone, Copy)]
pub enum PolygonEdit {
    Bevel {
        shift: f64,
        inset: f64,
        grouped: bool,
    },
    Extrude {
        extent: DVec3,
        sides: u32,
    },
}
#[derive(Clone, Copy)]
pub(crate) struct Boundary {
    pub(crate) face: usize,
    pub(crate) corner: usize,
    pub(crate) a: u32,
    pub(crate) b: u32,
}
pub(crate) struct Region {
    pub(crate) faces: Vec<usize>,
    pub(crate) vertices: BTreeSet<u32>,
    pub(crate) boundary: Vec<Boundary>,
}

impl Mesh {
    pub(crate) fn polygon_regions(&self, selected: &[u32], grouped: bool) -> Result<Vec<Region>> {
        self.validate()?;
        self.require_surface_elements()?;
        if selected.is_empty() {
            return Err("Select one or more polygons explicitly in Polygon mode".into());
        }
        let selection: BTreeSet<_> = selected.iter().copied().collect();
        if selection.len() != selected.len() {
            return Err("Duplicate selected polygon".into());
        }
        let mut corners = 0usize;
        let mut edges: BTreeMap<(u32, u32), Vec<(usize, usize)>> = BTreeMap::new();
        for &fi in selected {
            let f = self
                .faces
                .get(fi as usize)
                .ok_or("Invalid selected polygon")?;
            corners = corners
                .checked_add(f.vertices.len())
                .ok_or("Corner budget")?;
            if corners > SELECTED_CORNERS {
                return Err("Selected region exceeds 100,000-corner operation budget".into());
            }
            self.surface_triangles(fi as usize)
                .map_err(|e| format!("Polygon {fi}: {e}"))?;
            for (ci, &a) in f.vertices.iter().enumerate() {
                let b = f.vertices[(ci + 1) % f.vertices.len()];
                edges
                    .entry((a.min(b), a.max(b)))
                    .or_default()
                    .push((fi as usize, ci));
            }
        }
        let mut adjacency: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for uses in edges.values() {
            if uses.len() == 2 {
                adjacency.entry(uses[0].0).or_default().push(uses[1].0);
                adjacency.entry(uses[1].0).or_default().push(uses[0].0);
            }
        }
        let mut visited = BTreeSet::new();
        let mut regions = Vec::new();
        let mut region_vertices: BTreeSet<u32> = BTreeSet::new();
        for &start in selected {
            if !visited.insert(start as usize) {
                continue;
            }
            let mut members = BTreeSet::from([start as usize]);
            let mut stack = vec![start as usize];
            if grouped {
                while let Some(f) = stack.pop() {
                    for &next in adjacency.get(&f).into_iter().flatten() {
                        if visited.insert(next) {
                            members.insert(next);
                            stack.push(next);
                        }
                    }
                }
            }
            // Keep the ordered source selection, independently of graph traversal.
            let faces: Vec<_> = selected
                .iter()
                .map(|&f| f as usize)
                .filter(|f| members.contains(f))
                .collect();
            let vertices: BTreeSet<_> = faces
                .iter()
                .flat_map(|&f| self.faces[f].vertices.iter().copied())
                .collect();
            if grouped && vertices.iter().any(|v| region_vertices.contains(v)) {
                return Err(
                    "Selected islands touch only at a vertex; choose separate valid regions".into(),
                );
            }
            region_vertices.extend(&vertices);
            let mut boundary = Vec::new();
            for &fi in &faces {
                let f = &self.faces[fi];
                for (ci, &a) in f.vertices.iter().enumerate() {
                    let b = f.vertices[(ci + 1) % f.vertices.len()];
                    if edges[&(a.min(b), a.max(b))]
                        .iter()
                        .filter(|(f, _)| members.contains(f))
                        .count()
                        == 1
                    {
                        boundary.push(Boundary {
                            face: fi,
                            corner: ci,
                            a,
                            b,
                        });
                    }
                }
            }
            let mut incoming = BTreeSet::new();
            let mut outgoing = BTreeSet::new();
            for e in &boundary {
                if !incoming.insert(e.b) || !outgoing.insert(e.a) {
                    return Err(
                        "Branched/nonmanifold selected boundary; select a simple region".into(),
                    );
                }
            }
            if incoming != outgoing || boundary.is_empty() {
                return Err("Selected region needs closed, unbranched boundary loops".into());
            }
            regions.push(Region {
                faces,
                vertices,
                boundary,
            });
        }
        Ok(regions)
    }

    pub fn validate_polygon_selection(&self, selected: &[u32], grouped: bool) -> Result<()> {
        self.require_surface_elements()?;
        self.polygon_regions(selected, grouped).map(|_| ())
    }

    pub fn edit_polygons(&self, selected: &[u32], edit: PolygonEdit) -> Result<Self> {
        self.require_surface_elements()?;
        if let PolygonEdit::Bevel {
            shift,
            inset,
            grouped,
        } = edit
        {
            return crate::bevel_geometry(
                self,
                selected,
                shift,
                inset,
                grouped,
                &crate::BevelOptions::default(),
                None,
            )
            .map(|r| r.mesh);
        }

        if self.attributes.has_typed_data() {
            return Err("This tool cannot yet remap typed maps/named sets; source retained".into());
        }
        self.require_disjoint_exception_component(selected)?;
        if !self.attributes.point_uvs.is_empty()
            || !self.attributes.corner_weights.is_empty()
            || !self.attributes.corner_colors.is_empty()
        {
            return Err("Bevel on extended Extrude attributes is outside its current remapping domain; source retained".into());
        }
        if !self.attributes.relative_morphs.is_empty()
            || !self.attributes.smoothing_labels.is_empty()
        {
            return Err("Bevel/Extrude remapping of relative morphs/smoothing groups is not implemented; source retained".into());
        }

        let PolygonEdit::Extrude { extent, sides } = edit else {
            return Err("Invalid legacy polygon operation".into());
        };
        if !extent.is_finite() || sides == 0 {
            return Err("Extent must be finite; Sides must be a positive integer".into());
        }
        let neutral = extent == DVec3::ZERO;
        let sides = sides as usize;
        let regions = self.polygon_regions(selected, true)?;
        let mut generated_vertices = 0usize;
        let mut generated_faces = 0usize;
        for region in &regions {
            generated_faces = generated_faces
                .checked_add(
                    region
                        .boundary
                        .len()
                        .checked_mul(sides)
                        .ok_or("Wall budget")?,
                )
                .ok_or("Wall budget")?;
            generated_vertices = generated_vertices
                .checked_add(region.vertices.len())
                .and_then(|n| n.checked_add(region.boundary.len().checked_mul(sides - 1)?))
                .ok_or("Vertex budget")?;
        }
        if generated_faces
            .saturating_mul(4)
            .saturating_mul(self.uv_maps.len() + 1)
            > 8_000_000
        {
            return Err("Generated corner attributes exceed 8,000,000-entry budget; reduce Sides/selection/maps".into());
        }
        if generated_vertices > GENERATED_VERTICES
            || generated_faces > GENERATED_FACES
            || self.vertices.len().saturating_add(generated_vertices) > 5_000_000
            || self.faces.len().saturating_add(generated_faces) > 5_000_000
        {
            return Err("Polygon operation exceeds 1,000,000 generated vertices/faces or mesh budget; reduce Sides/selection".into());
        }
        if neutral {
            return Ok(self.clone());
        }
        let mut out = self.clone();
        for region in regions {
            let boundary_vertices: BTreeSet<_> = region.boundary.iter().map(|e| e.a).collect();
            let interior_vertices: BTreeSet<_> = region
                .vertices
                .difference(&boundary_vertices)
                .copied()
                .collect();
            if !interior_vertices.is_empty() {
                let members: BTreeSet<_> = region.faces.iter().copied().collect();
                if self.faces.iter().enumerate().any(|(fi, f)| {
                    !members.contains(&fi)
                        && f.vertices.iter().any(|v| interior_vertices.contains(v))
                }) {
                    return Err(
                        "Nonmanifold selected interior vertex touches an unselected face".into(),
                    );
                }
            }
            let mut cap_positions = BTreeMap::new();
            for &v in &region.vertices {
                cap_positions.insert(v, self.vertices[v as usize] + extent);
            }
            let mut previous: BTreeMap<u32, u32> =
                region.vertices.iter().map(|&v| (v, v)).collect();
            for level in 1..=sides {
                let final_level = level == sides;
                let vertices: BTreeSet<_> = if final_level {
                    region.vertices.clone()
                } else {
                    region.boundary.iter().map(|e| e.a).collect()
                };
                let mut current = BTreeMap::new();
                for v in vertices {
                    let source = self.vertices[v as usize];
                    let position = if final_level {
                        cap_positions[&v]
                    } else {
                        source + (cap_positions[&v] - source) * (level as f64 / sides as f64)
                    };
                    let interior = final_level && interior_vertices.contains(&v);
                    let id = if interior {
                        // All faces incident to an interior vertex must belong to
                        // this region. Reuse its identity rather than leaving a
                        // persistent orphan/ghost pick at the old position.
                        out.vertices[v as usize] = position;
                        v
                    } else {
                        let id = u32::try_from(out.vertices.len())
                            .map_err(|_| "Vertex identity budget")?;
                        out.vertices.push(position);
                        id
                    };
                    current.insert(v, id);
                }
                for e in &region.boundary {
                    let source = &self.faces[e.face];
                    let i = e.corner;
                    let j = (i + 1) % source.vertices.len();
                    let mut wall = source.clone();
                    wall.vertices =
                        vec![previous[&e.a], previous[&e.b], current[&e.b], current[&e.a]];
                    wall.uv = source.uv.as_ref().map(|u| vec![u[i], u[j], u[j], u[i]]);
                    out.faces.push(wall);
                    for values in out.uv_maps.values_mut() {
                        let u = &values[e.face];
                        let wall = vec![u[i], u[j], u[j], u[i]];
                        values.push(wall);
                    }
                }
                if final_level {
                    for &fi in &region.faces {
                        out.faces[fi].vertices =
                            self.faces[fi].vertices.iter().map(|v| current[v]).collect();
                        out.surface_triangles(fi)
                            .map_err(|e| format!("Invalid cap: {e}; reduce displacement"))?;
                    }
                }
                previous = current;
            }
        }
        // Walls must have area; degenerate tangential extrusion is refused.
        for fi in self.faces.len()..out.faces.len() {
            out.surface_triangles(fi)
                .map_err(|e| format!("Invalid wall: {e}; change displacement"))?;
        }
        out.validate()?;
        Ok(out)
    }
}

pub(crate) fn validate_cap_edges(
    mesh: &Mesh,
    region: &Region,
    cap: &BTreeMap<u32, DVec3>,
    normal: DVec3,
    span: f64,
) -> Result<()> {
    let cap: BTreeMap<_, _> = cap
        .iter()
        .map(|(&v, &p)| (v, p - normal * p.dot(normal)))
        .collect();
    let mut edges = BTreeSet::new();
    for &f in &region.faces {
        let vs = &mesh.faces[f].vertices;
        for i in 0..vs.len() {
            let (a, b) = (vs[i], vs[(i + 1) % vs.len()]);
            edges.insert((a.min(b), a.max(b)));
        }
    }
    // Bound the quadratic intersection guard as a technical operation budget.
    if edges.len().saturating_mul(edges.len()) > 16_000_000 {
        return Err(
            "Bevel boundary validation exceeds 16 million edge-pair budget; reduce selection"
                .into(),
        );
    }
    let edges: Vec<_> = edges.into_iter().collect();
    let eps = span * span * 1e-12;
    for (i, &(a, b)) in edges.iter().enumerate() {
        for &(c, d) in &edges[i + 1..] {
            if [a, b].contains(&c) || [a, b].contains(&d) {
                continue;
            }
            let (pa, pb, pc, pd) = (cap[&a], cap[&b], cap[&c], cap[&d]);
            let cross = |a: DVec3, b: DVec3, c: DVec3| (b - a).cross(c - a).dot(normal);
            if cross(pa, pb, pc) * cross(pa, pb, pd) <= eps * eps
                && cross(pc, pd, pa) * cross(pc, pd, pb) <= eps * eps
                && pa
                    .min(pb)
                    .cmple(pc.max(pd) + DVec3::splat(span * 1e-12))
                    .all()
                && pc
                    .min(pd)
                    .cmple(pa.max(pb) + DVec3::splat(span * 1e-12))
                    .all()
            {
                return Err(
                    "Inset crosses cap edges or hole boundaries; reduce its magnitude (M17-Q04)"
                        .into(),
                );
            }
        }
    }
    Ok(())
}
