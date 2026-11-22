//! Patches sample to exactly `modo_surface`'s polygons.
use super::*;
use crate::subdivision::tests::{cage, oracles};
use crate::subdivision::{SubdivisionSettings, display_surface};
use crate::{Face, MeshCrease};

/// Every sample of every patch equals the CPU surface's point, polygon by
/// polygon (the Face polygons come first there, then the patches' L × L
/// quads in patch order).
fn same_as_cpu(mesh: &Mesh, level: u8) -> std::result::Result<(), String> {
    let settings = SubdivisionSettings {
        subdivision_level: level,
        ..Default::default()
    };
    let (surface, sources) = display_surface(mesh, &settings).map_err(|e| e.to_string())?;
    let net = build(mesh, level).ok_or("not eligible")?;
    let l = level as usize;
    let first = sources
        .iter()
        .position(|&s| !net.blocks[s].is_empty())
        .ok_or("no patches")?;
    if surface.faces.len() - first != net.patches.len() * l * l {
        return Err(format!(
            "{} pieces, {} patches",
            surface.faces.len() - first,
            net.patches.len()
        ));
    }
    let scale = mesh
        .vertices
        .iter()
        .map(|v| v.abs().max_element())
        .fold(1.0, f64::max);
    for (k, face) in surface.faces[first..].iter().enumerate() {
        let (p, rest) = (k / (l * l), k % (l * l));
        let (i, j) = (rest / l, rest % l);
        if net.owner[p] as usize != sources[first + k] {
            return Err(format!(
                "piece {k}: owner {} not {}",
                net.owner[p],
                sources[first + k]
            ));
        }
        let at = |a: usize, b: usize| net.evaluate(p, a as f64 / l as f64, b as f64 / l as f64);
        let expected = [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)];
        for (c, &v) in face.vertices.iter().enumerate() {
            let got = surface.vertices[v as usize];
            if (got - expected[c]).abs().max_element() > 2e-6 * scale {
                return Err(format!("piece {k} corner {c}: {got} vs {}", expected[c]));
            }
        }
    }
    Ok(())
}

#[test]
fn patches_match_every_subdivision_oracle() {
    let mut checked = 0;
    let failures: Vec<String> = oracles()
        .iter()
        .filter(|(id, _)| id.starts_with("SDV-O-SUBD-"))
        .filter_map(|(id, o)| {
            let (mesh, settings) = cage(&o["input"]);
            if !eligible(&mesh, settings.subdivision_level) {
                return None;
            }
            checked += 1;
            same_as_cpu(&mesh, settings.subdivision_level)
                .err()
                .map(|e| format!("{id}: {e}"))
        })
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(checked > 3, "only {checked} oracles checked");
}

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// An n × n grid of quads, bumped so no two points line up, with a
/// pentagon and a triangle stitched onto one side.
fn bumpy_grid(n: u32) -> Mesh {
    let mut mesh = Mesh::default();
    for y in 0..=n {
        for x in 0..=n {
            let h = ((x * 7 + y * 13) % 5) as f64 * 0.1;
            mesh.vertices.push(DVec3::new(
                x as f64,
                y as f64,
                h + 0.01 * x as f64 * y as f64,
            ));
        }
    }
    let at = |x: u32, y: u32| y * (n + 1) + x;
    for y in 0..n {
        for x in 0..n {
            mesh.faces.push(face(vec![
                at(x, y),
                at(x + 1, y),
                at(x + 1, y + 1),
                at(x, y + 1),
            ]));
        }
    }
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend([
        DVec3::new(0.5, -1.0, 0.2),
        DVec3::new(1.5, -1.2, 0.0),
        DVec3::new(2.5, -0.8, 0.3),
    ]);
    mesh.faces
        .push(face(vec![at(1, 0), at(0, 0), base, base + 1, at(2, 0)]));
    mesh.faces.push(face(vec![at(2, 0), base + 1, base + 2]));
    mesh.attributes.representations = vec![PolygonRepresentation::Subdivs; mesh.faces.len()];
    mesh
}

#[test]
fn patches_match_open_grids_with_ngons_at_levels_1_to_4() {
    let mesh = bumpy_grid(4);
    for level in 1..=4 {
        same_as_cpu(&mesh, level).unwrap_or_else(|e| panic!("level {level}: {e}"));
    }
}

#[test]
fn patches_match_with_creases_soft_hard_and_negative() {
    let mut mesh = bumpy_grid(5);
    let at = |x: u32, y: u32| y * 6 + x;
    for (a, b, w) in [
        (at(1, 1), at(2, 1), 0.05),
        (at(2, 1), at(3, 1), 0.15),
        (at(3, 1), at(4, 1), 1.0),
        (at(2, 2), at(2, 3), -0.5),
        (at(2, 3), at(2, 4), -1.0),
        (at(3, 3), at(3, 4), 0.5),
        (at(3, 3), at(4, 3), 0.5),
        (at(3, 3), at(3, 2), 0.5),
    ] {
        mesh.attributes.creases.push(MeshCrease {
            a: a.min(b),
            b: a.max(b),
            weight: w,
        });
    }
    for level in 1..=3 {
        same_as_cpu(&mesh, level).unwrap_or_else(|e| panic!("level {level}: {e}"));
    }
}

#[test]
fn face_polygons_and_lines_stay_out_of_the_patches() {
    let mut mesh = bumpy_grid(3);
    mesh.attributes.representations[4] = PolygonRepresentation::Face;
    // Lines are never smooth (toggle refuses them).
    mesh.faces.push(face(vec![0, 15]));
    mesh.attributes
        .representations
        .push(PolygonRepresentation::Face);
    same_as_cpu(&mesh, 2).unwrap();
    let net = build(&mesh, 2).unwrap();
    assert!(net.blocks[4].is_empty());
    assert!(net.blocks[mesh.faces.len() - 1].is_empty());
    assert_eq!(patch_count(&mesh), net.patches.len() as u64);
}

#[test]
fn catmull_clark_polygons_or_level_0_are_not_eligible() {
    let mut mesh = bumpy_grid(2);
    assert!(!eligible(&mesh, 0));
    assert!(eligible(&mesh, 1));
    mesh.attributes.representations[0] = PolygonRepresentation::CatmullClark;
    assert!(!eligible(&mesh, 2));
    assert!(build(&mesh, 2).is_none());
}

#[test]
fn wire_curves_follow_the_traced_cage_edges() {
    // One record per cage edge; its two halves meet the surface's samples
    // on that edge.
    let mesh = bumpy_grid(2);
    let net = build(&mesh, 2).unwrap();
    assert_eq!(net.wire.len(), mesh.edges().len());
    let cp = |i: u32| {
        let c = net.controls[i as usize];
        DVec3::new(c[0] as f64, c[1] as f64, c[2] as f64)
    };
    let bez = |a: DVec3, m: DVec3, b: DVec3, t: f64| {
        (1. - t) * (1. - t) * a + 2. * t * (1. - t) * m + t * t * b
    };
    // Patch 0's side 0 runs from its corner sample along a cage edge.
    let (start, end) = (net.evaluate(0, 0.0, 0.0), net.evaluate(0, 1.0, 0.0));
    let mid = net.evaluate(0, 0.5, 0.0);
    let on_side = |w: &[u32; 6]| -> Option<DVec3> {
        let near = |a: DVec3, b: DVec3| (a - b).length() < 1e-6;
        if near(cp(w[0]), start) && near(cp(w[2]), end) {
            Some(bez(cp(w[0]), cp(w[1]), cp(w[2]), 0.5))
        } else if near(cp(w[4]), start) && near(cp(w[2]), end) {
            Some(bez(cp(w[4]), cp(w[3]), cp(w[2]), 0.5))
        } else {
            None
        }
    };
    let on = net
        .wire
        .iter()
        .find_map(on_side)
        .expect("a wire record along patch 0's side 0");
    assert!((on - mid).length() < 1e-6);
}

/// A quad sphere: a cube of 6 × n² quads pushed out to radius 1.
pub(crate) fn quad_sphere(n: u32) -> Mesh {
    let mut mesh = Mesh::default();
    let mut index: crate::fxhash::FxHashMap<(i64, i64, i64), u32> = Default::default();
    let mut point = |p: DVec3, mesh: &mut Mesh| -> u32 {
        let k = (
            (p.x * 1e6).round() as i64,
            (p.y * 1e6).round() as i64,
            (p.z * 1e6).round() as i64,
        );
        *index.entry(k).or_insert_with(|| {
            mesh.vertices.push(p.normalize());
            mesh.vertices.len() as u32 - 1
        })
    };
    let faces: [(DVec3, DVec3, DVec3); 6] = [
        (DVec3::X, DVec3::Y, DVec3::Z),
        (-DVec3::X, DVec3::Z, DVec3::Y),
        (DVec3::Y, DVec3::Z, DVec3::X),
        (-DVec3::Y, DVec3::X, DVec3::Z),
        (DVec3::Z, DVec3::X, DVec3::Y),
        (-DVec3::Z, DVec3::Y, DVec3::X),
    ];
    for (normal, u, v) in faces {
        let at = |i: u32, j: u32| {
            normal + u * (2.0 * i as f64 / n as f64 - 1.0) + v * (2.0 * j as f64 / n as f64 - 1.0)
        };
        let mut ids = vec![0u32; ((n + 1) * (n + 1)) as usize];
        for j in 0..=n {
            for i in 0..=n {
                ids[(j * (n + 1) + i) as usize] = point(at(i, j), &mut mesh);
            }
        }
        let id = |i: u32, j: u32| ids[(j * (n + 1) + i) as usize];
        for j in 0..n {
            for i in 0..n {
                mesh.faces.push(face(vec![
                    id(i, j),
                    id(i + 1, j),
                    id(i + 1, j + 1),
                    id(i, j + 1),
                ]));
            }
        }
    }
    mesh.attributes.representations = vec![PolygonRepresentation::Subdivs; mesh.faces.len()];
    mesh
}

#[test]
fn closed_quad_sphere_matches() {
    same_as_cpu(&quad_sphere(4), 2).unwrap();
}

/// Build time and size for the reference sphere (OPERANDI_SPHERE_N, default
/// 866: 6 × 866² ≈ 4.5M quads) at level 2.
#[test]
#[ignore]
fn patch_build_speed() {
    let n = std::env::var("OPERANDI_SPHERE_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(866);
    let t = std::time::Instant::now();
    let mesh = quad_sphere(n);
    eprintln!("sphere: {} quads in {:?}", mesh.faces.len(), t.elapsed());
    let mut mesh = mesh;
    let (mut state, net) = PatchState::new(&mesh, 2).unwrap();
    for count in [1usize, 100, 10_000] {
        let moved: Vec<u32> = (0..count as u32).map(|i| 1000 + i).collect();
        for &v in &moved {
            mesh.vertices[v as usize] *= 1.01;
        }
        let t = std::time::Instant::now();
        let updated = state.update(&net, &mesh, &moved);
        let changed = (0..updated.controls.chunks().len())
            .filter(|&k| !updated.controls.same_chunk(&net.controls, k))
            .count();
        eprintln!(
            "update {count} points: {:?}, {changed} control chunks changed",
            t.elapsed()
        );
    }
    for _ in 0..2 {
        let t = std::time::Instant::now();
        let net = build(&mesh, 2).unwrap();
        eprintln!(
            "build: {:?}, {} patches, {} triangles, {} MB",
            t.elapsed(),
            net.patches.len(),
            net.triangles(),
            net.bytes() >> 20
        );
    }
}

#[test]
fn clusters_hold_every_sample_and_normal_of_their_patches() {
    let mut mesh = bumpy_grid(6);
    mesh.attributes.creases.push(MeshCrease {
        a: 8,
        b: 9,
        weight: 0.7,
    });
    let net = build(&mesh, 3).unwrap();
    let per = fill_per_cluster(3) as usize;
    assert_eq!(net.fill_clusters.len(), net.patches.len().div_ceil(per));
    assert!(!net.closed);
    for (p, _) in net.patches.iter().enumerate() {
        let c = &net.fill_clusters[p / per];
        let centre = DVec3::new(c.centre[0] as f64, c.centre[1] as f64, c.centre[2] as f64);
        let axis = DVec3::new(c.axis[0] as f64, c.axis[1] as f64, c.axis[2] as f64);
        for i in 0..=8 {
            for j in 0..=8 {
                let (u, v) = (i as f64 / 8., j as f64 / 8.);
                assert!(net.evaluate(p, u, v).distance(centre) <= c.radius as f64 + 1e-9);
                let e = 1e-5;
                let du = net.evaluate(p, (u + e).min(1.), v) - net.evaluate(p, (u - e).max(0.), v);
                let dv = net.evaluate(p, u, (v + e).min(1.)) - net.evaluate(p, u, (v - e).max(0.));
                let n = du.cross(dv);
                if c.cos > 0.0 && n.length() > 1e-12 {
                    assert!(
                        n.normalize().dot(axis) >= c.cos as f64 - 1e-3,
                        "patch {p} ({u}, {v})"
                    );
                }
            }
        }
    }
    assert!(build(&quad_sphere(3), 2).unwrap().closed);
}

#[test]
fn tab_refuses_a_surface_bigger_than_free_memory() {
    use crate::subdivision::{Toggle, display_bytes};
    let mut e = crate::Editor::default();
    e.dispatch(crate::Command::NewMesh).unwrap();
    let mut mesh = quad_sphere(20);
    mesh.attributes.representations.clear();
    e.scene.active_mut().unwrap().mesh = mesh.clone().into();
    // 2400 quads at level 2: stored, 16 pieces each.
    let needs = {
        let mut smooth = mesh.clone();
        smooth.attributes.representations =
            vec![PolygonRepresentation::Subdivs; smooth.faces.len()];
        display_bytes(&smooth, &SubdivisionSettings::default())
    };
    assert_eq!(
        needs,
        2400 * 16 * crate::subdivision::STORED_BYTES_PER_PIECE
    );
    e.free_memory = crate::FreeMemory(Some(|| 1_000_000));
    let error = e.toggle_subdivision(Toggle::Subdivision).unwrap_err();
    assert!(error.contains("needs about"), "{error}");
    assert!(!crate::subdivision::has_smooth(
        &e.scene.active().unwrap().mesh
    ));
    e.free_memory = crate::FreeMemory(Some(|| 1 << 40));
    e.toggle_subdivision(Toggle::Subdivision).unwrap();
    assert!(crate::subdivision::has_smooth(
        &e.scene.active().unwrap().mesh
    ));
    // Raising the level is checked the same way.
    e.free_memory = crate::FreeMemory(Some(|| 1_000_000));
    let higher = SubdivisionSettings {
        subdivision_level: 4,
        ..Default::default()
    };
    assert!(e.set_subdivision(higher).is_err());
}

#[test]
fn big_surfaces_count_patch_memory() {
    use crate::subdivision::{PATCH_MIN_PIECES, display_bytes, display_pieces, draws_as_patches};
    let sphere = quad_sphere(150); // 135k quads, 2.16M pieces at level 2
    let settings = SubdivisionSettings::default();
    assert!(display_pieces(&sphere, &settings) >= PATCH_MIN_PIECES);
    assert!(draws_as_patches(&sphere, &settings, PATCH_MIN_PIECES));
    assert_eq!(
        display_bytes(&sphere, &settings),
        4 * 135_000 * 2 * BYTES_PER_PATCH
    );
    assert!(!draws_as_patches(
        &quad_sphere(4),
        &settings,
        PATCH_MIN_PIECES
    ));
}

/// Moving points and updating the net near them gives exactly the net a
/// full build of the moved mesh gives, sharing every untouched chunk.
#[test]
fn drag_updates_match_a_full_build() {
    let mut mesh = bumpy_grid(9);
    let at = |x: u32, y: u32| y * 10 + x;
    for (a, b, w) in [
        (at(2, 2), at(3, 2), 0.4),
        (at(3, 2), at(4, 2), -0.6),
        (at(5, 5), at(5, 6), 1.0),
    ] {
        mesh.attributes.creases.push(MeshCrease {
            a: a.min(b),
            b: a.max(b),
            weight: w,
        });
    }
    let (mut state, mut net) = PatchState::new(&mesh, 2).unwrap();
    // An inner point, a crease end, a border point, the n-gon's points.
    let moves: [&[u32]; 4] = [
        &[at(4, 4)],
        &[at(3, 2), at(5, 6)],
        &[at(0, 3), at(9, 9)],
        &[100, 101, 102],
    ];
    for moved in moves {
        for &v in moved {
            mesh.vertices[v as usize] += DVec3::new(0.13, -0.07, 0.21);
        }
        let updated = state.update(&net, &mesh, moved);
        let full = build(&mesh, 2).unwrap();
        assert_eq!(updated.controls.len(), full.controls.len());
        for i in 0..full.controls.len() {
            assert_eq!(
                updated.controls[i], full.controls[i],
                "control {i} after moving {moved:?}"
            );
        }
        for k in 0..full.fill_clusters.len() {
            assert_eq!(
                updated.fill_clusters[k], full.fill_clusters[k],
                "fill cluster {k}"
            );
        }
        for k in 0..full.wire_clusters.len() {
            assert_eq!(
                updated.wire_clusters[k], full.wire_clusters[k],
                "wire cluster {k}"
            );
        }
        for r in 0..full.wire_sides.len() {
            assert_eq!(updated.wire_sides[r], full.wire_sides[r], "sides {r}");
        }
        net = updated;
    }
}

#[test]
fn chunks_share_what_a_change_leaves() {
    let a = Chunks::from_vec((0..(3 * CHUNK + 5) as u32).collect());
    let mut b = a.clone();
    b.set(CHUNK + 1, 7);
    assert_eq!(b[CHUNK + 1], 7);
    assert_eq!(a[CHUNK + 1], (CHUNK + 1) as u32);
    assert!(a.same_chunk(&b, 0) && !a.same_chunk(&b, 1) && a.same_chunk(&b, 3));
    assert_eq!(a.chunks().len(), 4);
}
