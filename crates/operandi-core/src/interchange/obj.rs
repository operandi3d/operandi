//! Wavefront OBJ: a public, line-based text format. One item per `o` object
//! (or per `g` group when a file has no objects), polygons kept as n-gons,
//! `usemtl` names as polygon materials, `s` groups as smooth shading and `vt`
//! coordinates as the Texture UV map. Normals (`vn`) are recomputed from the
//! geometry rather than stored, and lines and points are skipped.
use super::{RawFace, RawMesh, WorldMesh, num};
use crate::Result;
use glam::{DVec2, DVec3};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

pub(super) fn read(
    bytes: &[u8],
    stem: &str,
    groups_as_meshes: bool,
    notes: &mut Vec<String>,
) -> Result<Vec<RawMesh>> {
    let text = String::from_utf8_lossy(bytes);
    // Join "\" continuations so every statement is one logical line.
    let text = text.replace("\\\r\n", " ").replace("\\\n", " ");
    // Import Groups as Separate Meshes splits on `g` even when the file has
    // objects; otherwise groups split only a file without any.
    let split_on_groups = groups_as_meshes
        || !text
            .lines()
            .any(|l| l.trim_start().starts_with("o ") || l.trim() == "o");
    let mut positions: Vec<DVec3> = Vec::new();
    let mut texcoords: Vec<DVec2> = Vec::new();
    let mut meshes: Vec<RawMesh> = Vec::new();
    let mut current = RawMesh {
        name: stem.into(),
        ..Default::default()
    };
    let mut material = String::new();
    let mut smooth = false;
    let mut lines_skipped = 0usize;
    for (number, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut words = line.split_whitespace();
        let Some(keyword) = words.next() else {
            continue;
        };
        let at = |e: String| format!("OBJ line {}: {e}", number + 1);
        match keyword {
            "v" => positions.push(parse_vec3(&mut words).map_err(at)?),
            "vt" => {
                let u = parse_f64(words.next()).map_err(at)?;
                let v = words
                    .next()
                    .map_or(Ok(0.0), |w| parse_f64(Some(w)))
                    .map_err(at)?;
                texcoords.push(DVec2::new(u, v));
            }
            "f" | "fo" => {
                let mut face = RawFace {
                    material: material.clone(),
                    smooth,
                    ..Default::default()
                };
                let mut uv = Vec::new();
                let mut all_uv = true;
                for corner in words {
                    let mut parts = corner.split('/');
                    let p = resolve(parts.next(), positions.len()).map_err(at)?;
                    let t = match parts.next() {
                        Some(s) if !s.is_empty() => {
                            Some(resolve(Some(s), texcoords.len()).map_err(at)?)
                        }
                        _ => None,
                    };
                    // File-wide index for now; `localize` renumbers per item.
                    face.vertices.push(p as u32);
                    match t {
                        Some(t) => uv.push(texcoords[t]),
                        None => all_uv = false,
                    }
                }
                if all_uv && !uv.is_empty() {
                    face.uv = Some(uv);
                }
                current.faces.push(face);
            }
            "o" | "g" => {
                if keyword == "o" || split_on_groups {
                    let name = line[keyword.len()..].trim();
                    let finished = std::mem::take(&mut current);
                    if !finished.faces.is_empty() {
                        meshes.push(localize(finished, &positions));
                    }
                    current.name = if name.is_empty() {
                        stem.into()
                    } else {
                        name.into()
                    };
                }
            }
            "usemtl" => material = line["usemtl".len()..].trim().to_owned(),
            "s" => smooth = !matches!(words.next(), Some("off") | Some("0") | None),
            "l" | "p" => lines_skipped += 1,
            // Normals, materials libraries, curves and unknown statements
            // carry nothing Operandi stores.
            _ => {}
        }
    }
    if !current.faces.is_empty() {
        meshes.push(localize(current, &positions));
    }
    if lines_skipped > 0 {
        notes.push(format!("Skipped {lines_skipped} OBJ line/point element(s)"));
    }
    Ok(meshes)
}

/// Give an item only the points its polygons use, kept in file order.
fn localize(mut mesh: RawMesh, positions: &[DVec3]) -> RawMesh {
    let used: BTreeSet<u32> = mesh
        .faces
        .iter()
        .flat_map(|f| f.vertices.iter().copied())
        .collect();
    let local: HashMap<u32, u32> = used
        .iter()
        .enumerate()
        .map(|(i, &p)| (p, i as u32))
        .collect();
    mesh.points = used.iter().map(|&p| positions[p as usize]).collect();
    for f in &mut mesh.faces {
        for v in &mut f.vertices {
            *v = local[v];
        }
    }
    mesh
}

fn parse_f64(word: Option<&str>) -> std::result::Result<f64, String> {
    let w = word.ok_or("missing number")?;
    let v: f64 = w.parse().map_err(|_| format!("'{w}' is not a number"))?;
    if !v.is_finite() {
        return Err(format!("'{w}' is not finite"));
    }
    Ok(v)
}

fn parse_vec3<'a>(words: &mut impl Iterator<Item = &'a str>) -> std::result::Result<DVec3, String> {
    Ok(DVec3::new(
        parse_f64(words.next())?,
        parse_f64(words.next())?,
        parse_f64(words.next())?,
    ))
}

/// OBJ indices are 1-based; negative ones count back from the latest entry.
fn resolve(word: Option<&str>, count: usize) -> std::result::Result<usize, String> {
    let w = word.filter(|w| !w.is_empty()).ok_or("missing index")?;
    let i: i64 = w.parse().map_err(|_| format!("'{w}' is not an index"))?;
    let resolved = if i > 0 { i - 1 } else { count as i64 + i };
    if i == 0 || resolved < 0 || resolved as usize >= count {
        return Err(format!("index {i} is out of range"));
    }
    Ok(resolved as usize)
}

/// The OBJ text, and the material library when `mtl` names one (Save
/// Material File): a `newmtl` per material used, with its colour.
pub(super) fn write(
    meshes: &[WorldMesh],
    as_groups: bool,
    mtl: Option<&str>,
    notes: &mut Vec<String>,
) -> (Vec<u8>, Option<Vec<u8>>) {
    let mut out = String::from("# Exported by Operandi\n");
    let used: BTreeSet<&str> = meshes
        .iter()
        .flat_map(|m| m.mesh.faces.iter().map(|f| f.material.as_str()))
        .filter(|m| !m.is_empty())
        .collect();
    let library = mtl.filter(|_| !used.is_empty()).map(|name| {
        let _ = writeln!(out, "mtllib {}", one_line(name));
        let stored = meshes.first().map(|m| &m.materials);
        let mut lib = String::from("# Exported by Operandi\n");
        for material in &used {
            let m = stored
                .and_then(|s| s.get(*material))
                .cloned()
                .unwrap_or_default();
            let [r, g, b] = m.diffuse_color();
            let _ = writeln!(lib, "newmtl {}", one_line(material));
            let _ = writeln!(lib, "Kd {} {} {}", num(r), num(g), num(b));
        }
        lib.into_bytes()
    });
    let (mut point_base, mut uv_base) = (1usize, 1usize);
    let mut lines = 0;
    for m in meshes.iter().filter(|m| !m.mesh.faces.is_empty()) {
        let _ = writeln!(
            out,
            "{} {}",
            if as_groups { "g" } else { "o" },
            one_line(&m.name)
        );
        for p in &m.mesh.vertices {
            let _ = writeln!(out, "v {} {} {}", num(p.x), num(p.y), num(p.z));
        }
        // One vt per polygon corner that has a UV.
        let mut corner_uv: Vec<Vec<Option<usize>>> = Vec::new();
        let mut written = 0;
        for (f, face) in m.mesh.faces.iter().enumerate() {
            let uv = m.uvs.as_ref().and_then(|u| u.get(f));
            let complete = !m.polyline[f] && uv.is_some_and(|u| u.iter().all(Option::is_some));
            corner_uv.push(
                (0..face.vertices.len())
                    .map(|c| {
                        complete.then(|| {
                            let t = uv.unwrap()[c].unwrap();
                            let _ = writeln!(out, "vt {} {}", num(t.x), num(t.y));
                            written += 1;
                            uv_base + written - 1
                        })
                    })
                    .collect(),
            );
        }
        let mut material = None;
        let mut smooth = None;
        for (f, face) in m.mesh.faces.iter().enumerate() {
            if m.polyline[f] {
                let ids: Vec<String> = face
                    .vertices
                    .iter()
                    .map(|v| (point_base + *v as usize).to_string())
                    .collect();
                let _ = writeln!(out, "l {}", ids.join(" "));
                lines += 1;
                continue;
            }
            if material != Some(&face.material) && !face.material.is_empty() {
                let _ = writeln!(out, "usemtl {}", one_line(&face.material));
            }
            material = Some(&face.material);
            if smooth != Some(face.smooth) {
                out.push_str(if face.smooth { "s 1\n" } else { "s off\n" });
                smooth = Some(face.smooth);
            }
            out.push('f');
            for (c, v) in face.vertices.iter().enumerate() {
                let _ = match corner_uv[f][c] {
                    Some(t) => write!(out, " {}/{t}", point_base + *v as usize),
                    None => write!(out, " {}", point_base + *v as usize),
                };
            }
            out.push('\n');
        }
        point_base += m.mesh.vertices.len();
        uv_base += written;
    }
    if lines > 0 {
        notes.push(format!("{lines} polyline(s) written as OBJ lines"));
    }
    (out.into_bytes(), library)
}

/// Names go on one line; newlines would start a new statement.
fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}
