//! LXO scene import beyond phase 1: files as Modo 16/17 saves them for
//! real scenes (curves and other polygon types beside faces, edge weights
//! on edges no polygon has, zero-area polygons), and the scene's
//! animation and rig.
//!
//! Every file here is written by this test from the format's layout; no
//! saved Modo scenes are used.
use operandi_core::lxo;

// --- writing --------------------------------------------------------------

fn text(out: &mut Vec<u8>, s: &str) {
    let start = out.len();
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    if (out.len() - start) % 2 == 1 {
        out.push(0);
    }
}
fn u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn f32(out: &mut Vec<u8>, v: f64) {
    out.extend_from_slice(&(v as f32).to_be_bytes());
}
fn vx(out: &mut Vec<u8>, v: u32) {
    if v < 0xff00 {
        u16(out, v as u16);
    } else {
        u32(out, v | 0xff00_0000);
    }
}

/// A Modo 17-style scene file, chunk by chunk.
#[derive(Default)]
struct File {
    body: Vec<u8>,
}

impl File {
    fn new(tags: &[&str], names: &[&str]) -> File {
        let mut f = File::default();
        let mut d = Vec::new();
        u32(&mut d, 16);
        u32(&mut d, 0);
        text(&mut d, "Modo by The Foundry");
        f.chunk(b"VRSN", &d);
        let mut d = Vec::new();
        for v in [17, 4001, 1, 123456] {
            u32(&mut d, v);
        }
        text(&mut d, "Modo 17.1v1");
        f.chunk(b"APPV", &d);
        let mut d = Vec::new();
        u32(&mut d, 2);
        f.chunk(b"ENCO", &d);
        let mut d = Vec::new();
        for t in tags {
            text(&mut d, t);
        }
        f.chunk(b"TAGS", &d);
        let mut d = Vec::new();
        u32(&mut d, names.len() as u32);
        for n in names {
            text(&mut d, n);
        }
        f.chunk(b"CHNM", &d);
        f
    }
    fn chunk(&mut self, id: &[u8; 4], data: &[u8]) {
        self.body.extend_from_slice(id);
        u32(&mut self.body, data.len() as u32);
        self.body.extend_from_slice(data);
        if data.len() % 2 == 1 {
            self.body.push(0);
        }
    }
    /// A mesh layer owned by mesh ITEM `item`.
    fn layer(&mut self, index: u16, name: &str, item: u32) {
        let mut d = Vec::new();
        u16(&mut d, index);
        u16(&mut d, 0);
        for _ in 0..3 {
            f32(&mut d, 0.);
        }
        text(&mut d, name);
        u16(&mut d, 0xffff);
        f32(&mut d, 0.);
        f32(&mut d, 0.);
        d.extend_from_slice(&[0; 36]);
        u32(&mut d, item);
        self.chunk(b"LAYR", &d);
    }
    /// Points as Modo shows them (stored with Z negated).
    fn points(&mut self, points: &[[f64; 3]]) {
        let mut d = Vec::new();
        for p in points {
            f32(&mut d, p[0]);
            f32(&mut d, p[1]);
            f32(&mut d, -p[2]);
        }
        self.chunk(b"PNTS", &d);
    }
    /// Polygons of `kind`, corners in Modo's stored order.
    fn polygons(&mut self, kind: &[u8; 4], polygons: &[&[u32]]) {
        let mut d = kind.to_vec();
        for p in polygons {
            u16(&mut d, p.len() as u16);
            for &v in *p {
                vx(&mut d, v);
            }
        }
        self.chunk(b"POLS", &d);
    }
    fn raw_polygons(&mut self, kind: &[u8; 4], payload: &[u8]) {
        let mut d = kind.to_vec();
        d.extend_from_slice(payload);
        self.chunk(b"POLS", &d);
    }
    fn edge_weights(&mut self, edges: &[(u32, u32, f64)]) {
        let mut d = b"SUBV".to_vec();
        u16(&mut d, 1);
        text(&mut d, "Subdivision");
        for &(a, b, w) in edges {
            vx(&mut d, a);
            vx(&mut d, b);
            f32(&mut d, w);
        }
        self.chunk(b"VMED", &d);
    }
    /// An ITEM with its sub-records.
    fn item(&mut self, kind: &str, name: &str, id: u32, subs: &[([u8; 4], Vec<u8>)]) {
        let mut d = Vec::new();
        text(&mut d, kind);
        text(&mut d, name);
        u32(&mut d, id);
        for (sid, sd) in subs {
            d.extend_from_slice(sid);
            u16(&mut d, sd.len() as u16);
            d.extend_from_slice(sd);
            if sd.len() % 2 == 1 {
                d.push(0);
            }
        }
        self.chunk(b"ITEM", &d);
    }
    fn bytes(&self) -> Vec<u8> {
        let mut out = b"FORM".to_vec();
        u32(&mut out, self.body.len() as u32 + 4);
        out.extend_from_slice(b"LXOB");
        out.extend_from_slice(&self.body);
        out
    }
}

fn link(graph: &str, target: u32) -> ([u8; 4], Vec<u8>) {
    let mut d = Vec::new();
    text(&mut d, graph);
    u32(&mut d, target);
    u32(&mut d, 0);
    (*b"LINK", d)
}

fn import(bytes: &[u8]) -> lxo::Result<lxo::Candidate> {
    let mut go = |_: &str| true;
    lxo::parse(bytes, &mut lxo::Control::new(&mut go))
}

/// A unit quad in XY: points 0..3.
const QUAD: [[f64; 3]; 4] = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];

// --- real-scene geometry ----------------------------------------------------

#[test]
fn curve_polygons_beside_faces_are_left_out_by_name() {
    // A B-spline curve stores more than corner indices after its count, so
    // reading it as faces finds a "point index" out of range. The face
    // chunk imports; the curve chunk is named in the report.
    let mut f = File::new(&["Default"], &[]);
    f.layer(0, "Mesh", 1);
    f.points(&QUAD);
    f.polygons(b"FACE", &[&[0, 3, 2, 1]]);
    let mut curve = Vec::new();
    u16(&mut curve, 4);
    for v in [0u16, 0x1a00, 0, 1, 2, 3, 0] {
        u16(&mut curve, v);
    }
    f.raw_polygons(b"BCRV", &curve);
    f.item("mesh", "Mesh", 1, &[]);
    let c = import(&f.bytes()).unwrap();
    let mesh = &c.scene.items[0].mesh;
    assert_eq!(mesh.faces.len(), 1);
    assert_eq!(c.report.omissions["POLS.BCRV records not imported"], 1);
}

#[test]
fn edge_weights_on_edges_no_polygon_has_are_dropped_with_a_note() {
    let mut f = File::new(&["Default"], &[]);
    f.layer(0, "Mesh", 1);
    f.points(&QUAD);
    f.polygons(b"FACE", &[&[0, 3, 2, 1]]);
    // 0-1 is a polygon edge; 0-2 is a diagonal no polygon has.
    f.edge_weights(&[(0, 1, 0.5), (0, 2, 0.75)]);
    f.item("mesh", "Mesh", 1, &[]);
    let c = import(&f.bytes()).unwrap();
    let creases = &c.scene.items[0].mesh.attributes.creases;
    assert_eq!(creases.len(), 1);
    assert_eq!((creases[0].a, creases[0].b, creases[0].weight), (0, 1, 0.5));
    assert!(
        c.report
            .notes
            .iter()
            .any(|n| n == "Mesh: 1 edge weights on edges no polygon has were not imported"),
        "{:?}",
        c.report.notes
    );
}

#[test]
fn zero_area_polygons_are_kept_as_degenerate() {
    let mut f = File::new(&["Default"], &[]);
    f.layer(0, "Mesh", 1);
    let mut points = QUAD.to_vec();
    points.extend([[2., 0., 0.], [3., 0., 0.], [4., 0., 0.]]);
    f.points(&points);
    // A quad and a flat triangle of three points on a line.
    f.polygons(b"FACE", &[&[0, 3, 2, 1], &[4, 6, 5]]);
    f.item("mesh", "Mesh", 1, &[]);
    let c = import(&f.bytes()).unwrap();
    let mesh = &c.scene.items[0].mesh;
    assert_eq!(mesh.faces.len(), 2);
    assert!(mesh.attributes.degenerate_faces.contains(&1));
    assert!(
        c.report
            .notes
            .iter()
            .any(|n| n.contains("1 zero-area polygon(s) kept as degenerate")),
        "{:?}",
        c.report.notes
    );
}

#[test]
fn a_parent_link_between_meshes_still_imports() {
    // The writer's ITEM and LINK layout, checked on the phase-1 case it
    // shares with the older tests.
    let mut f = File::new(&["Default"], &[]);
    f.layer(0, "Parent", 1);
    f.points(&QUAD);
    f.polygons(b"FACE", &[&[0, 3, 2, 1]]);
    f.layer(1, "Child", 2);
    f.points(&QUAD);
    f.polygons(b"FACE", &[&[0, 3, 2, 1]]);
    f.item("mesh", "Parent", 1, &[]);
    f.item("mesh", "Child", 2, &[link("parent", 1)]);
    let c = import(&f.bytes()).unwrap();
    let child = c.scene.items.iter().find(|i| i.name == "Child").unwrap();
    assert_eq!(child.parent, Some(2));
}
