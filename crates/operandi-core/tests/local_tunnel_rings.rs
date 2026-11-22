//! Action Center Local on a loop around a hole (forum report "funky gizmo",
//! FG-Q02, Jason 2026-11-22): the polygons around a tunnel ring face every
//! way, so their normals mostly cancel. Below `LOCAL_CANCELLED` the piece
//! has no common normal and takes its axes from its shape: the ring's
//! plane normal is the tunnel's direction, whatever the segments' spacing.
//! Patches that face one way keep their averaged normal.
use glam::{DMat3, DMat4, DVec3};
use operandi_core::*;
use std::collections::BTreeSet;
use std::f64::consts::TAU;

fn quad(v: [u32; 4]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A tunnel along Z (radius 1, centred at y = 3): three rings of points at
/// `angles`, z = 0, 0.5, 1, joined by quads facing the axis. With `open`,
/// the last span is left out (an arc, not a ring).
fn tunnel(angles: &[f64], open: bool) -> Mesh {
    let n = angles.len() as u32;
    let mut vertices = Vec::new();
    for ring in 0..3 {
        for &a in angles {
            vertices.push(DVec3::new(a.cos(), 3. + a.sin(), ring as f64 * 0.5));
        }
    }
    let spans = if open { n - 1 } else { n };
    let mut faces = Vec::new();
    for ring in 0..2 {
        for side in 0..spans {
            let (a, b) = (ring * n + side, ring * n + (side + 1) % n);
            faces.push(quad([b, a, a + n, b + n]));
        }
    }
    Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
}

/// The Local frame of the middle ring's edges (spans `0..spans`).
fn middle_ring_frame(mesh: &Mesh, n: u32, spans: u32) -> DMat3 {
    let edges: Vec<(u32, u32)> = (0..spans).map(|s| (n + s, n + (s + 1) % n)).collect();
    let selection = Selection {
        mode: Mode::Edge,
        edges,
        ..Default::default()
    };
    let moved: BTreeSet<u32> = (n..2 * n).collect();
    let pieces = local_piece_frames(mesh, &moved, &selection);
    pieces[&n].frame(DMat4::IDENTITY, DMat3::IDENTITY).basis
}

fn even(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / n as f64 * TAU).collect()
}

/// An arch: 12 segments bunched toward the start, so the polygons' unit
/// normals leave a 0.30 leftover (the case that spun the gizmo).
fn arch() -> Vec<f64> {
    (0..12).map(|i| (i as f64 / 12.).powi(2) * TAU).collect()
}

#[test]
fn a_tunnel_ring_takes_the_tunnel_direction_however_its_segments_are_spaced() {
    let expected = middle_ring_frame(&tunnel(&even(12), false), 12, 12);
    // The ring's plane normal is the tunnel's direction (Z), on Z.
    assert!(
        expected.z_axis.abs().abs_diff_eq(DVec3::Z, 1e-9),
        "{expected}"
    );
    let frame = middle_ring_frame(&tunnel(&arch(), false), 12, 12);
    assert!(frame.z_axis.abs().abs_diff_eq(DVec3::Z, 1e-9), "{frame}");
    // The leftover of the normals no longer turns it: the same frame as
    // the bare ring of points, which has no polygons to vote (before the
    // cutoff it was turned 12° about the tunnel).
    let mut bare = tunnel(&arch(), false);
    bare.faces = Vec::new().into();
    let shape = middle_ring_frame(&bare, 12, 12);
    assert!(frame.abs_diff_eq(shape, 1e-9), "{frame} vs {shape}");
}

#[test]
fn polygons_facing_one_way_keep_their_averaged_normal() {
    // Half a cylinder (unit normals average 0.64): the normal is the
    // vote's, pointing into the arc's middle (-Y here, toward the axis
    // from the arc around +Y).
    let angles: Vec<f64> = (0..7).map(|i| i as f64 / 6. * TAU / 2.).collect();
    let mesh = tunnel(&angles, true);
    let frame = middle_ring_frame(&mesh, 7, 6);
    let mut sum = DVec3::ZERO;
    for f in 0..mesh.faces.len() {
        sum += mesh.normal(f).normalize();
    }
    let vote = sum.normalize();
    assert!(sum.length() / mesh.faces.len() as f64 > LOCAL_CANCELLED);
    let carried = [frame.x_axis, frame.y_axis, frame.z_axis]
        .into_iter()
        .any(|c| c.abs_diff_eq(vote, 1e-9));
    assert!(carried, "normal {vote} not an axis of {frame}");
}
