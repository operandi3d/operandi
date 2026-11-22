// Oracle correspondence for Bevel weld acceptance (BV-T09, BV-R13/R21).
//
// Included by the private-draft tests in `src/bevel_weld_tests.rs` and the
// published-output tests in `tests/bevel_amendment.rs`, so both compare a
// result against ORACLES.json by exactly one rule. It expects `Mesh` in scope.
//
// The admitted comparison permits one consistent vertex/face permutation and a
// cyclic rotation of each oriented face, with per-corner rows rotated alongside.
// It forbids reversed winding and arbitrary nearest matching at coincident
// welded corners. The search therefore assigns whole oriented faces, requires a
// single vertex bijection across all of them, and refuses an ambiguous result
// instead of choosing one. Cap selection, tags and corner rows are separate
// checks against the same correspondence; none of them is used to derive it.

const ORACLE_TOLERANCE: f64 = 2e-6;

#[derive(Clone, Debug, PartialEq)]
struct OracleCorrespondence {
    /// Actual vertex -> expected vertex.
    vertex: Vec<usize>,
    /// Actual face -> expected face.
    face: Vec<usize>,
    /// Actual corner `c` of face `f` is expected corner `(c + rotation[f]) % n`.
    rotation: Vec<usize>,
}

/// Complete effective corner rows of one authored map: face -> corner -> components.
/// An absent record is an error here; these rows are complete-map oracles.
#[allow(dead_code)] // Used by the private-draft includer only.
fn actual_corner_rows(
    actual: &Mesh,
    kind: &str,
    name: &str,
) -> std::result::Result<Vec<Vec<Vec<f64>>>, String> {
    fn rows<T>(
        map: Option<&Vec<Vec<Option<T>>>>,
        components: impl Fn(&T) -> Vec<f64>,
    ) -> std::result::Result<Vec<Vec<Vec<f64>>>, String> {
        map.ok_or("map missing")?
            .iter()
            .map(|row| {
                row.iter()
                    .map(|v| v.as_ref().map(&components).ok_or("absent corner record"))
                    .collect::<std::result::Result<Vec<_>, _>>()
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
    let a = &actual.attributes;
    match kind {
        "txuv" | "texture" => rows(actual.uv_maps.get(name), |v| v.to_array().to_vec()),
        "weight" => rows(a.corner_weights.get(name), |v| vec![*v]),
        "rgba" => rows(a.corner_colors.get(name), |v| v.to_array().to_vec()),
        "normal" => rows(a.corner_normals.get(name), |v| v.to_array().to_vec()),
        _ => Err(format!("unadmitted map kind {kind}")),
    }
}

/// Compare complete corner rows with each face's rotation applied.
#[allow(dead_code)] // Used by the private-draft includer only.
fn check_corner_rows(
    rows: &[Vec<Vec<f64>>],
    correspondence: &OracleCorrespondence,
    expected: &serde_json::Value,
    label: &str,
) -> std::result::Result<(), String> {
    if rows.len() != correspondence.face.len() {
        return Err(format!("{label}: {} rows", rows.len()));
    }
    for (f, row) in rows.iter().enumerate() {
        let (g, r, n) = (
            correspondence.face[f],
            correspondence.rotation[f],
            row.len(),
        );
        let target = expected[g].as_array().ok_or("expected row")?;
        let dimension = row.first().map_or(0, Vec::len);
        if target.len() != n * dimension {
            return Err(format!("{label} face {f}->{g}: row length"));
        }
        for (c, values) in row.iter().enumerate() {
            for (k, value) in values.iter().enumerate() {
                let want = target[((c + r) % n) * dimension + k].as_f64().unwrap();
                if (value - want).abs() >= ORACLE_TOLERANCE {
                    return Err(format!(
                        "{label} face {f}->{g} corner {c}: component {k} {value} != {want}"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Consistent relabeling for positive controls: `vertex_order[new] = old`,
/// `face_order[new] = old`, and each new face starts at old corner `start[new]`.
/// Only containers these weld fixtures use are moved; any other populated
/// container would make the control incomplete, so it is rejected. Exceptional
/// edge incidences are index-derived from the faces and are rebuilt.
#[allow(dead_code)] // Used by the private-draft includer only.
fn relabel_for_control(
    mesh: &Mesh,
    selected: &[u32],
    vertex_order: &[usize],
    face_order: &[usize],
    start: &[usize],
) -> (Mesh, Vec<u32>) {
    let a = &mesh.attributes;
    assert!(
        a.typed_maps.is_empty()
            && a.point_uvs.is_empty()
            && a.uv_records.is_empty()
            && a.point_weights.is_empty()
            && a.point_colors.is_empty()
            && a.creases.is_empty()
            && a.representations.is_empty()
            && a.smoothing_labels.is_empty()
            && a.vertex_sets.is_empty()
            && a.polygon_sets.is_empty()
            && a.relative_morphs.is_empty()
            && a.degenerate_faces.is_empty(),
        "control relabeling does not cover this fixture"
    );
    let mut new_vertex = vec![0_u32; vertex_order.len()];
    for (new, &old) in vertex_order.iter().enumerate() {
        new_vertex[old] = new as u32;
    }
    let mut new_face = vec![0_u32; face_order.len()];
    for (new, &old) in face_order.iter().enumerate() {
        new_face[old] = new as u32;
    }
    fn rotate<T: Clone>(rows: &[Vec<T>], order: &[usize], start: &[usize]) -> Vec<Vec<T>> {
        order
            .iter()
            .zip(start)
            .map(|(&old, &s)| {
                let row = &rows[old];
                (0..row.len())
                    .map(|c| row[(c + s) % row.len()].clone())
                    .collect()
            })
            .collect()
    }
    let mut out = mesh.clone();
    out.vertices = vertex_order.iter().map(|&v| mesh.vertices[v]).collect();
    out.faces = face_order
        .iter()
        .zip(start)
        .map(|(&old, &s)| {
            let mut face = mesh.faces[old].clone();
            let n = face.vertices.len();
            face.vertices = (0..n)
                .map(|c| new_vertex[mesh.faces[old].vertices[(c + s) % n] as usize])
                .collect();
            face
        })
        .collect();
    for (name, rows) in &mesh.uv_maps {
        out.uv_maps
            .insert(name.clone(), rotate(rows, face_order, start));
    }
    for (name, rows) in &a.corner_weights {
        out.attributes
            .corner_weights
            .insert(name.clone(), rotate(rows, face_order, start));
    }
    for (name, rows) in &a.corner_colors {
        out.attributes
            .corner_colors
            .insert(name.clone(), rotate(rows, face_order, start));
    }
    for (name, rows) in &a.corner_normals {
        out.attributes
            .corner_normals
            .insert(name.clone(), rotate(rows, face_order, start));
    }
    // Same rule `Mesh::validate` checks: an edge used more than twice, or twice
    // in one direction, is recorded with its exact (face, a, b) uses.
    let mut uses: std::collections::BTreeMap<(u32, u32), Vec<[u32; 3]>> = Default::default();
    for (f, face) in out.faces.iter().enumerate() {
        for c in 0..face.vertices.len() {
            let (a, b) = (
                face.vertices[c],
                face.vertices[(c + 1) % face.vertices.len()],
            );
            uses.entry((a.min(b), a.max(b)))
                .or_default()
                .push([f as u32, a, b]);
        }
    }
    out.attributes.extrude_edge_uses = uses
        .into_iter()
        .filter(|(_, u)| u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]))
        .map(|((a, b), u)| (format!("{a}:{b}"), u))
        .collect();
    assert_eq!(
        out.attributes.extrude_edge_uses.len(),
        a.extrude_edge_uses.len(),
        "relabeling changed exceptional incidences"
    );
    let selected = selected.iter().map(|&f| new_face[f as usize]).collect();
    (out, selected)
}
