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
        // Modo writes the item assignment table; Operandi's own export
        // profile doesn't, so files here are read by the Modo rules.
        f.chunk(b"IASS", &[]);
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
    /// An ACTN chunk: `name` is also its kind; `subs` its records.
    fn action(&mut self, name: &str, id: u32, subs: &[([u8; 4], Vec<u8>)]) {
        let mut d = Vec::new();
        text(&mut d, name);
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
        self.chunk(b"ACTN", &d);
    }
    /// The file, ending with an empty setup action as every Modo file
    /// has one.
    fn bytes(&self) -> Vec<u8> {
        let mut end = File::default();
        end.action("setup", 2, &[]);
        let mut out = b"FORM".to_vec();
        u32(&mut out, (self.body.len() + end.body.len()) as u32 + 4);
        out.extend_from_slice(b"LXOB");
        out.extend_from_slice(&self.body);
        out.extend_from_slice(&end.body);
        out
    }
}

fn link(graph: &str, target: u32) -> ([u8; 4], Vec<u8>) {
    link_at(graph, target, 0)
}
fn link_at(graph: &str, target: u32, ordinal: u32) -> ([u8; 4], Vec<u8>) {
    let mut d = Vec::new();
    text(&mut d, graph);
    u32(&mut d, target);
    u32(&mut d, ordinal);
    (*b"LINK", d)
}

/// A CHNV vector channel (`pos` with X, Y, Z).
fn vector(base: &str, v: [f64; 3]) -> ([u8; 4], Vec<u8>) {
    let mut d = Vec::new();
    text(&mut d, base);
    u16(&mut d, 2);
    u16(&mut d, 3);
    for (c, x) in ["X", "Y", "Z"].iter().zip(v) {
        text(&mut d, c);
        f32(&mut d, x);
    }
    (*b"CHNV", d)
}

/// A string CHAN; `index` is the name's place in CHNM.
fn string_channel(index: u32, value: &str) -> ([u8; 4], Vec<u8>) {
    let mut d = Vec::new();
    vx(&mut d, index);
    u16(&mut d, 3);
    text(&mut d, value);
    (*b"CHAN", d)
}

/// Channel names every hierarchy file declares, in CHNM order.
const NAMES: [&str; 3] = ["type", "order", "visible"];
const TYPE: u32 = 0;
const ORDER: u32 = 1;
const VISIBLE: u32 = 2;

/// A core transform item of `kind` moving `target`, at stack place `ordinal`.
fn transform(
    f: &mut File,
    id: u32,
    kind: &str,
    target: u32,
    ordinal: u32,
    v: [f64; 3],
    extra: &[([u8; 4], Vec<u8>)],
) {
    let base = match kind {
        "translation" => "pos",
        "rotation" => "rot",
        _ => "scl",
    };
    let mut subs = vec![
        string_channel(TYPE, "core"),
        vector(base, v),
        link_at("xfrmCore", target, ordinal),
    ];
    subs.extend(extra.iter().cloned());
    f.item(kind, "", id, &subs);
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

// --- hierarchy --------------------------------------------------------------

#[test]
fn meshes_under_locators_keep_their_place() {
    // Locator (moved to x 10) > Group Locator (rotated 90° about Y) > Mesh.
    let mut f = File::new(&["Default"], &NAMES);
    f.layer(0, "Mesh", 3);
    f.points(&QUAD);
    f.polygons(b"FACE", &[&[0, 3, 2, 1]]);
    f.item("locator", "Rig", 1, &[]);
    f.item("groupLocator", "Group", 2, &[link("parent", 1)]);
    f.item("mesh", "Mesh", 3, &[link("parent", 2)]);
    transform(&mut f, 10, "translation", 1, 2, [10., 0., 0.], &[]);
    let quarter = std::f64::consts::FRAC_PI_2;
    transform(
        &mut f,
        11,
        "rotation",
        2,
        1,
        [0., quarter, 0.],
        &[string_channel(ORDER, "xyz")],
    );
    let c = import(&f.bytes()).unwrap();
    let s = &c.scene;
    assert_eq!(s.items.len(), 3);
    let by = |n: &str| s.items.iter().find(|i| i.name == n).unwrap();
    let (rig, group, mesh) = (by("Rig"), by("Group"), by("Mesh"));
    assert!(rig.is_group_locator() && group.is_group_locator());
    assert_eq!(group.parent, Some(rig.id));
    assert_eq!(mesh.parent, Some(group.id));
    assert_eq!(
        group.transform.rotation_order(),
        operandi_core::scene::RotationOrder::Xyz
    );
    // The mesh's +X corner (1, 0, 0) turns to -Z, then moves to x 10.
    let w = s
        .world_matrix(mesh.id)
        .unwrap()
        .transform_point3(glam::DVec3::X);
    assert!((w - glam::DVec3::new(10., 0., -1.)).length() < 1e-6, "{w}");
    assert_eq!(c.report.meshes, 1);
    assert!(
        c.report
            .notes
            .iter()
            .any(|n| n == "2 locators and group locators imported as Group Locators")
    );
}

#[test]
fn the_core_transform_wins_over_a_zeroed_one() {
    // "Zero transforms" leaves a second rotation in the stack; the core
    // one carries the item's rotation, whatever order the items come in.
    let mut f = File::new(&["Default"], &NAMES);
    f.item("locator", "Loc", 1, &[]);
    f.item(
        "rotation",
        "Rotation Zero",
        2,
        &[
            string_channel(TYPE, "21"),
            vector("rot", [0.5, 0., 0.]),
            link_at("xfrmCore", 1, 2),
        ],
    );
    transform(&mut f, 3, "rotation", 1, 1, [0.25, 0., 0.], &[]);
    let c = import(&f.bytes()).unwrap();
    assert_eq!(c.scene.items[0].transform.rotation.x, 0.25);
    assert_eq!(
        c.report.omissions["extra rotation transforms not combined"],
        1
    );
}

#[test]
fn all_off_hides_the_item_and_its_children() {
    let mut f = File::new(&["Default"], &NAMES);
    f.item(
        "groupLocator",
        "Hidden",
        1,
        &[string_channel(VISIBLE, "allOff")],
    );
    f.item("locator", "Shown", 2, &[string_channel(VISIBLE, "default")]);
    let c = import(&f.bytes()).unwrap();
    let hidden = &c.scene.items[0];
    assert!(!hidden.visible && hidden.hides_children);
    assert!(c.scene.items[1].visible);
}

#[test]
fn a_mesh_instance_shows_its_source_mesh() {
    let mut f = File::new(&["Default"], &NAMES);
    f.layer(0, "Mesh", 1);
    f.points(&QUAD);
    f.polygons(b"FACE", &[&[0, 3, 2, 1]]);
    f.item("mesh", "Mesh", 1, &[]);
    f.item("meshInst", "Copy", 2, &[link("source", 1)]);
    transform(&mut f, 10, "translation", 2, 0, [0., 5., 0.], &[]);
    let c = import(&f.bytes()).unwrap();
    let copy = c.scene.items.iter().find(|i| i.name == "Copy").unwrap();
    assert_eq!(copy.instance_of, Some(c.scene.items[0].id));
    assert_eq!(copy.mesh.faces.len(), 1);
    assert_eq!(copy.transform.translation.y, 5.);
    // Counted once, as the source mesh.
    assert_eq!(c.report.meshes, 1);
}
