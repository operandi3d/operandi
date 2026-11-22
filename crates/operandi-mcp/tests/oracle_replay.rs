//! Spec oracles replayed through the MCP bridge, the way the research side
//! checks a build: load the oracle's input mesh, make its selection, run the
//! tool or command with the oracle's settings, and compare the mesh and the
//! selection left behind.
//!
//! Each case is graded:
//! - `exact`: same points and polygons in the same order, same selection;
//! - `numbered`: same geometry and selection, but different point or
//!   polygon numbering;
//! - otherwise a short description of the first difference.
//!
//! The selection is read the way a script reads it, from the bridge's
//! `selection` echo, and every mode's list the oracle names is compared in
//! order (selection order is pick order, SEL-B2-04).
//!
//! `EXPECTED` pins today's grade for every case, so a fix shows up as a
//! failing "now better" line to move, and a regression as a failing case.
//! `cargo test -p operandi-mcp --test oracle_replay -- --nocapture` prints
//! the whole table.
use glam::DVec3;
use operandi_core::{Face, Mesh};
use operandi_mcp::Bridge;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn all_oracles() -> BTreeMap<String, Value> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specifications");
    let mut out = BTreeMap::new();
    // Folders in name order, and the first folder's case wins an id: the
    // disk's own order differs between macOS and Linux, and two folders
    // share ids. SP01-SP09 here are make-split's Split cases; spikey's
    // cases of the same ids are checked by core's spikey_clean_spec.rs.
    let mut folders: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    folders.sort();
    for folder in folders {
        // ORACLES.json, or cases.json (make-split-2026-11-05).
        for name in ["ORACLES.json", "cases.json"] {
            let Ok(text) = std::fs::read_to_string(folder.join(name)) else {
                continue;
            };
            let doc: Value = serde_json::from_str(&text).unwrap();
            let list = doc
                .get("oracles")
                .or(doc.get("cases"))
                .cloned()
                .unwrap_or(doc);
            for o in list.as_array().into_iter().flatten() {
                if let Some(id) = o["id"].as_str() {
                    out.entry(id.to_string()).or_insert_with(|| o.clone());
                }
            }
        }
    }
    out
}

fn vec3(v: &Value) -> DVec3 {
    let a: Vec<f64> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    DVec3::new(a[0], a[1], a[2])
}
fn list(v: &Value) -> Vec<u32> {
    v.as_array()
        .map(|a| a.iter().map(|x| x.as_u64().unwrap() as u32).collect())
        .unwrap_or_default()
}
fn pairs(v: &Value) -> Vec<[u32; 2]> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|p| {
                    let p = list(p);
                    [p[0], p[1]]
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The oracle's input mesh in the bridge's active item.
fn load_input(b: &mut Bridge, o: &Value) {
    let input = &o["input"];
    if !input.is_object() {
        return; // "empty mesh item at identity"
    }
    let mesh = Mesh {
        vertices: input["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(vec3)
            .collect(),
        faces: input["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Face {
                vertices: list(p),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    };
    // As opening a scene does: mixed winding and edges on three polygons
    // are recorded, not refused.
    let mut mesh = mesh;
    mesh.attributes.extrude_edge_uses = operandi_core::interchange::edge_exceptions(&mesh);
    b.editor.scene.active_mut().unwrap().mesh = mesh.into();
    if let Some(t) = input.get("item_transform").filter(|t| !t.is_null()) {
        let rot = vec3(&t["rot"]).map(f64::to_radians);
        b.call(
            "command_run",
            &json!({"command": "item.transform",
                    "args": {"translation": t["pos"], "rotation": [rot.x, rot.y, rot.z]}}),
        )
        .unwrap();
    }
}

/// The selection the oracle starts from, in whichever spelling it uses.
fn start_selection(o: &Value) -> (Vec<u32>, Vec<[u32; 2]>, Vec<u32>) {
    let s = &o["selection"];
    let pick = |a: &str, b: &str, c: &str| {
        [s.get(a), s.get(b), o.get(c)]
            .into_iter()
            .flatten()
            .find(|v| v.is_array())
            .cloned()
            .unwrap_or(Value::Null)
    };
    // The 2026-11-05 cases.json files say `vertices`, `edges`, `polygons`.
    let or = |v: Value, k: &str| if v.is_array() { v } else { s[k].clone() };
    (
        list(&or(
            pick("vertex", "sel_vertices", "selected_vertices"),
            "vertices",
        )),
        pairs(&or(pick("edge", "sel_edges", "selected_edges"), "edges")),
        list(&or(
            pick("polygon", "sel_polygons", "selected_polygons"),
            "polygons",
        )),
    )
}

fn select(b: &mut Bridge, mode: &str, args: Value) -> Result<(), String> {
    let mut args = args;
    args["mode"] = json!(mode);
    b.call("select", &args).map(|_| ())
}

fn tool(b: &mut Bridge, id: &str, params: Value) -> Result<(), String> {
    b.call("tool_run", &json!({"tool": id, "params": params}))
        .map(|_| ())
}
fn command(b: &mut Bridge, id: &str, args: Value) -> Result<(), String> {
    b.call("command_run", &json!({"command": id, "args": args}))
        .map(|_| ())
}

/// Make the oracle's starting selection, then run its operation.
fn run(b: &mut Bridge, id: &str, o: &Value) -> Result<(), String> {
    let s = &o["settings"];
    let f = |k: &str| s[k].as_f64();
    let (verts, edges, polys) = start_selection(o);
    let fam = if id.starts_with("MP") {
        "MP"
    } else if !id.contains('-') {
        // cases.json ids: two letters and a number (EE07, SS06, QD09).
        &id[..2]
    } else {
        id.split('-').next().unwrap()
    };
    // The mode the case starts in, where it says.
    let mode = o["selection"]["mode"]
        .as_str()
        .or(o["selection_mode"].as_str())
        .unwrap_or("");
    // Selection in the operation's own mode.
    let select_start = |b: &mut Bridge| -> Result<(), String> {
        if !polys.is_empty() || fam == "PE" || fam == "PB" {
            select(b, "polygon", json!({"faces": polys}))
        } else if !edges.is_empty() {
            select(b, "edge", json!({"edges": edges}))
        } else {
            select(b, "vertex", json!({"vertices": verts}))
        }
    };
    match fam {
        "PE" => {
            select_start(b)?;
            let extent = [
                f("shiftX").unwrap_or(0.),
                f("shiftY").unwrap_or(0.),
                f("shiftZ").unwrap_or(0.),
            ];
            tool(b, "poly.extrude", json!({"extent": extent}))
        }
        "PB" => {
            select_start(b)?;
            tool(
                b,
                "poly.bevel",
                json!({"inset": f("inset").unwrap_or(0.), "shift": f("shift").unwrap_or(0.)}),
            )
        }
        "PRIM" => {
            let size = [
                f("sizeX").unwrap_or(1.),
                f("sizeY").unwrap_or(1.),
                f("sizeZ").unwrap_or(1.),
            ];
            tool(b, "prim.cube", json!({"size": size, "position": [0, 0, 0]}))
        }
        "EBC" => {
            select_start(b)?;
            let chamfer_tool =
                s.get("offset").is_some() || s["tool"].as_str() == Some("Edge Chamfer");
            let width = f("offset")
                .or(f("Inset m"))
                .or(f("Inset or Offset m"))
                .unwrap_or(0.);
            let segments = s["segments"]
                .as_u64()
                .or(s["Segments"].as_u64())
                .unwrap_or(1);
            // The bridge takes depth as 0-1, as the recording does.
            let depth = f("depth").unwrap_or(1.);
            tool(
                b,
                if chamfer_tool {
                    "edge.chamfer"
                } else {
                    "edge.bevel"
                },
                json!({"chamfer": {"offset": width, "segments": segments, "depth": depth}}),
            )
        }
        "EE" | "VE" => {
            // Edge / Vertex Extrude (extrude-2026-11-05); nothing selected
            // acts on everything (EE07, VE08).
            if fam == "EE" {
                select(b, "edge", json!({"edges": edges}))?;
            } else {
                select(b, "vertex", json!({"vertices": verts}))?;
            }
            tool(
                b,
                if fam == "EE" {
                    "edge.extrude"
                } else {
                    "vert.extrude"
                },
                json!({"shift": f("Extrude").unwrap_or(0.), "inset": f("Width").unwrap_or(0.)}),
            )
        }
        "SS" | "TH" => {
            // Smooth Shift / Thicken (smooth-shift-thicken-2026-11-05).
            select(b, "polygon", json!({"faces": polys}))?;
            let flag = |k: &str| s[k].as_bool().unwrap_or(false);
            tool(
                b,
                "poly.smshift",
                json!({"smooth_shift": {
                    "offset": f("Offset").unwrap_or(0.),
                    "scale": f("Scale (%)").unwrap_or(100.) / 100.,
                    "angle": f("Max Smoothing Angle (deg)").unwrap_or(89.5).to_radians(),
                    "thicken": flag("Thicken"),
                    "sharp": flag("Sharp"),
                    "angle_weight": flag("Polygon Angle Weighting"),
                    "both_sides": flag("Both Sides"),
                }}),
            )
        }
        "SP" | "QD" | "FL" => {
            // Split, Quadruple, Flip: in the case's mode.
            match mode {
                "vertex" => select(b, "vertex", json!({"vertices": verts}))?,
                "edge" => select(b, "edge", json!({"edges": edges}))?,
                _ => select(b, "polygon", json!({"faces": polys}))?,
            }
            let name = match fam {
                "SP" => "poly.split",
                "QD" => "poly.quadruple",
                _ => "poly.flip",
            };
            command(b, name, json!({}))
        }
        "SEL" => {
            for p in list(&s["picks_in_order"]) {
                select(b, "polygon", json!({"faces": [p], "add": true}))?;
            }
            b.call("selection_op", &json!({"op": "between"}))
                .map(|_| ())
        }
        "MP" => {
            // Make Polygon (P key, Type Auto): any polygons first, then the
            // listed points or edges in order, in the case's mode.
            if !polys.is_empty() {
                select(b, "polygon", json!({"faces": polys}))?;
            }
            match o["selection_mode"].as_str() {
                Some("edge") => select(b, "edge", json!({"edges": edges}))?,
                _ => select(b, "vertex", json!({"vertices": verts}))?,
            }
            command(b, "poly.make", json!({}))
        }
        "SYM" => {
            // Picks by script (no mirror, SYM-SEL-07), then Select
            // Symmetrical.
            command(b, "symmetry.set", json!({"axis": "x"}))?;
            for p in list(&s["picks_in_order"]) {
                select(b, "vertex", json!({"vertices": [p], "add": true}))?;
            }
            command(b, "select.symmetrical", json!({}))
        }
        "CMD" => {
            select_start(b)?;
            let name = o["tool"]
                .as_str()
                .or(s["operation"].as_str())
                .unwrap_or_default();
            let flag = |k: &str| s[k].as_bool().unwrap_or(false);
            match name {
                "Polygon Align" | "Align" => command(b, "poly.align", json!({})),
                "Quadruple" => command(b, "poly.quadruple", json!({})),
                "Polygon Merge" => command(b, "poly.merge", json!({})),
                "Edge Join" => command(
                    b,
                    if flag("Average") {
                        "edge.joinAverage"
                    } else {
                        "edge.join"
                    },
                    json!({}),
                ),
                "Edge Subdivide" => {
                    let mut args = json!({"count": s["Count"].as_u64().unwrap_or(1)});
                    if let Some(p) = f("Percent") {
                        // The recording holds the panel's percent; the
                        // bridge takes the name map's 0–1.
                        args["percent"] = json!(p / 100.);
                    }
                    command(b, "edge.subdivide", args)
                }
                "Vertex Collapse" => command(b, "vert.collapse", json!({})),
                "Vertex Split" => command(b, "vert.split", json!({})),
                "Vertex Join" => command(b, "vert.join", json!({"average": flag("Average")})),
                "Vertex Align" => {
                    let axes = match s["Axis"].as_str() {
                        Some("X") => [true, false, false],
                        Some("Y") => [false, true, false],
                        Some("Z") => [false, false, true],
                        _ => [true; 3],
                    };
                    command(b, "vert.align", json!({"axes": axes}))
                }
                other => Err(format!("no driver for {other}")),
            }
        }
        _ => Err(format!("no driver for {id}")),
    }
}

/// Positions rounded for comparison.
fn key(p: DVec3) -> [i64; 3] {
    p.to_array().map(|x| (x * 1e4).round() as i64)
}
fn shape(points: &[DVec3], polygon: &[u32]) -> Vec<[i64; 3]> {
    let q: Vec<[i64; 3]> = polygon.iter().map(|&v| key(points[v as usize])).collect();
    let start = (0..q.len()).min_by_key(|&i| q[i]).unwrap_or(0);
    (0..q.len()).map(|k| q[(start + k) % q.len()]).collect()
}
fn sorted<T: Ord>(mut v: Vec<T>) -> Vec<T> {
    v.sort();
    v
}

struct State {
    points: Vec<DVec3>,
    polygons: Vec<Vec<u32>>,
    verts: Option<Vec<u32>>,
    edges: Option<Vec<[u32; 2]>>,
    polys: Option<Vec<u32>>,
}

fn expected(o: &Value) -> State {
    let x = &o["expected"];
    // cases.json keeps the selection in `selection_after`.
    let after = x.get("selection_after").unwrap_or(x);
    let get = |a: &str, b: &str, c: &str| {
        after
            .get(a)
            .or(after.get(b))
            .or(x.get("selection_after").and_then(|s| s.get(c)))
            .filter(|v| v.is_array())
    };
    // A cases.json `selection_after` that names only the mode means
    // nothing is selected (EE04, VE04).
    let none_named = after.is_object()
        && x.get("selection_after").is_some()
        && after
            .as_object()
            .unwrap()
            .keys()
            .all(|k| k == "mode" || k.starts_with("selection_mode"));
    let empty = Value::Array(Vec::new());
    let get = |a: &str, b: &str, c: &str| get(a, b, c).or(none_named.then_some(&empty));
    State {
        points: x["points"].as_array().unwrap().iter().map(vec3).collect(),
        polygons: x["polygons"].as_array().unwrap().iter().map(list).collect(),
        verts: get("selected_vertices", "sel_vertices", "vertices").map(list),
        edges: get("selected_edges", "sel_edges", "edges").map(pairs),
        polys: get("selected_polygons", "sel_polygons", "polygons").map(list),
    }
}

fn actual(b: &Bridge) -> State {
    let item = b.editor.scene.active().unwrap();
    let s = &b.summary()["selection"];
    State {
        points: item.mesh.vertices.clone(),
        polygons: item.mesh.faces.iter().map(|f| f.vertices.clone()).collect(),
        verts: Some(list(&s["vertices"])),
        edges: Some(pairs(&s["edges"])),
        polys: Some(list(&s["faces"])),
    }
}

/// Grade one result against the oracle.
fn grade(got: &State, want: &State, tolerance: f64) -> String {
    // Geometry by shape (any numbering).
    let shapes = |s: &State| sorted(s.polygons.iter().map(|p| shape(&s.points, p)).collect());
    if got.points.len() != want.points.len() || got.polygons.len() != want.polygons.len() {
        return format!(
            "geometry: {}/{} points/polygons, expected {}/{}",
            got.points.len(),
            got.polygons.len(),
            want.points.len(),
            want.polygons.len()
        );
    }
    if shapes(got) != shapes(want)
        || sorted(got.points.iter().map(|&p| key(p)).collect())
            != sorted(want.points.iter().map(|&p| key(p)).collect())
    {
        return "geometry: positions or polygon shapes differ".into();
    }
    // Selection by position (any numbering), only where the oracle says.
    let vert_keys =
        |s: &State, v: &[u32]| sorted(v.iter().map(|&i| key(s.points[i as usize])).collect());
    let edge_keys = |s: &State, e: &[[u32; 2]]| {
        sorted(
            e.iter()
                .map(|&[a, b]| sorted(vec![key(s.points[a as usize]), key(s.points[b as usize])]))
                .collect(),
        )
    };
    let poly_keys = |s: &State, p: &[u32]| {
        sorted(
            p.iter()
                .map(|&i| shape(&s.points, &s.polygons[i as usize]))
                .collect(),
        )
    };
    let gv = got.verts.as_deref().unwrap_or(&[]);
    let ge = got.edges.as_deref().unwrap_or(&[]);
    let gp = got.polys.as_deref().unwrap_or(&[]);
    let mut problems = Vec::new();
    if let Some(w) = &want.verts
        && vert_keys(got, gv) != vert_keys(want, w)
    {
        problems.push(format!("vertices {gv:?}, expected {w:?}"));
    }
    if let Some(w) = &want.edges
        && edge_keys(got, ge) != edge_keys(want, w)
    {
        problems.push(format!("edges {ge:?}, expected {w:?}"));
    }
    if let Some(w) = &want.polys
        && poly_keys(got, gp) != poly_keys(want, w)
    {
        problems.push(format!("polygons {gp:?}, expected {w:?}"));
    }
    if !problems.is_empty() {
        return format!("selection: {}", problems.join("; "));
    }
    // The same components; now the order they are listed in. Pick order
    // only means something where the numbering is the oracle's (a new
    // point's place in the list follows its number); edge lists have no
    // order rule in these specs and compare as sets.
    let same_numbering = got.points.len() == want.points.len()
        && got
            .points
            .iter()
            .zip(&want.points)
            .all(|(a, b)| (*a - *b).length() <= tolerance)
        && got.polygons == want.polygons;
    if same_numbering {
        if let Some(w) = &want.verts
            && gv != w.as_slice()
        {
            problems.push(format!("vertices {gv:?}, expected {w:?}"));
        }
        if let Some(w) = &want.polys
            && gp != w.as_slice()
        {
            problems.push(format!("polygons {gp:?}, expected {w:?}"));
        }
    }
    if !problems.is_empty() {
        return format!("selection order: {}", problems.join("; "));
    }
    // Exact numbering.
    let same_points = got
        .points
        .iter()
        .zip(&want.points)
        .all(|(a, b)| (*a - *b).length() <= tolerance);
    let norm = |e: &[[u32; 2]]| -> Vec<[u32; 2]> {
        e.iter().map(|&[a, b]| [a.min(b), a.max(b)]).collect()
    };
    let exact = same_points
        && got.polygons == want.polygons
        && want.verts.as_deref().is_none_or(|w| w == gv)
        && want.edges.as_deref().is_none_or(|w| norm(w) == norm(ge))
        && want.polys.as_deref().is_none_or(|w| w == gp);
    if exact { "exact" } else { "numbered" }.into()
}

/// Known differences, with the grade they get today. Every other case must
/// grade "exact" or "numbered". See docs/SPECIFICATION_QUESTIONS.md
/// ("Oracle replay fixes — 2026-11-04").
const EXPECTED: &[(&str, &str)] = &[
    // Q-MS-02: two separate edges make nothing; Operandi says so instead
    // of staying silent.
    (
        "MP18",
        "refused: Make: these edges aren't on one connected surface; nothing was made",
    ),
    // Operandi keeps the end opening closed instead of two coincident
    // zero-area triangles (deliberate; EBC-B3-06).
    (
        "EBC-O-E3-06-one-chamfer",
        "geometry: 19/10 points/polygons, expected 19/12",
    ),
    // PE-O-02 not built: a sideways shift should reuse the points it lands on.
    (
        "PE-O-E1-nonplanar-shiftX",
        "geometry: 8/6 points/polygons, expected 7/6",
    ),
    (
        "PE-O-E5-moved-item",
        "geometry: 8/6 points/polygons, expected 6/6",
    ),
    // Q-EX-03 (decided 2026-11-05): border edges get a proper ridge, not
    // the reference's open sliver and notch; EE07 has the same border
    // edges.
    ("EE04", "geometry: 9/4 points/polygons, expected 7/2"),
    ("EE07", "geometry: 25/20 points/polygons, expected 25/12"),
    // Known, intentional difference (Q-SS-04 decided, Jason 2026-11-18):
    // Scale follows the selected action centre, the selection's centre
    // with none; the reference ignores the action centre and fits the
    // origin. Not a failure.
    ("SS06", "geometry: positions or polygon shapes differ"),
    ("TH10", "geometry: positions or polygon shapes differ"),
    // Known, intentional difference (Q-SS-03, Operandi's rule 2026-11-20):
    // Sharp counts only the vertex's own smoothing group, so split faces
    // move exactly Offset; the reference's result is unexplained.
    ("SS11b", "geometry: positions or polygon shapes differ"),
    // Polygon Angle Weighting off on a triangulated cube: not looked at
    // yet (not on the 2026-11-18 behaviour-check list).
    ("TH08a", "geometry: positions or polygon shapes differ"),
    // Q-MS-03 (decided 2026-11-05): three points cut between each
    // consecutive pair, not the reference's single cut.
    ("SP04", "geometry: 6/3 points/polygons, expected 6/2"),
    ("SP10", "geometry: 6/3 points/polygons, expected 6/2"),
    ("SP13", "geometry: 6/3 points/polygons, expected 6/2"),
    // Q-QD-01 (decided 2026-11-05): never join points of one row; the
    // reference's QD09 triangle and QD18 quad do (zero area).
    ("QD09", "geometry: positions or polygon shapes differ"),
    ("QD18", "geometry: positions or polygon shapes differ"),
    // Superseded by CMD-B3-01 (the 25 % record drove an out-of-range value);
    // CMD-O-M3-01-percent25 is the case that applies.
    (
        "CMD-O-edge-subdivide-percent25",
        "geometry: positions or polygon shapes differ",
    ),
];

const CASES: &[&str] = &[
    // Selection after the operation
    "PE-O-E1-nonplanar",
    "PE-O-E3-bowtie",
    "PE-O-E4-two-orientations",
    "PE-O-E9-two-adjacent",
    "PE-O-M-auto-quad-diagonal",
    "PE-O-M-auto-quad-down",
    "CMD-O-M3-01-count1",
    "CMD-O-M3-01-count2",
    "CMD-O-M3-01-count3",
    "CMD-O-M3-01-count4",
    "CMD-O-M3-01-percent10",
    "CMD-O-M3-01-percent25",
    "CMD-O-M3-01-percent50",
    "CMD-O-M3-01-percent75",
    "CMD-O-edge-subdivide-count2",
    "CMD-O-M3-02-asymmetric3",
    "CMD-O-M3-02-border",
    "CMD-O-M3-02-valence3",
    "CMD-O-M3-02-valence4",
    "CMD-O-M3-02-valence5",
    "CMD-O-vert-collapse",
    "CMD-O-vert-join-average",
    "CMD-O-vert-join-last",
    "CMD-O-vert-split",
    "EBC-O-E3-03-sides3",
    "EBC-O-E3-03-sides6",
    "EBC-O-E3-06-one-bevel",
    "EBC-O-E3-06-row-bevel",
    "CMD-O-merge-nonplanar-block",
    "SEL-O-L2-02-2",
    "SEL-O-L2-04-1",
    "SEL-O-L2-04-2",
    "SEL-O-L2-04-3",
    "SEL-O-L2-04-4",
    "SYM-O-S2-01-1",
    "SYM-O-S2-01-2",
    "SYM-O-S2-01-3",
    "SYM-O-S2-01-4",
    "SYM-O-S2-01-5",
    "SYM-O-S2-01-6",
    // Make Polygon, Type Auto (make-split-2026-11-05)
    "MP01",
    "MP02",
    "MP03",
    "MP04",
    "MP05",
    "MP06",
    "MP14",
    "MP15",
    "MP16",
    "MP17",
    "MP18",
    "MP19",
    "MP21",
    // Geometry
    "EBC-O-chamfer-corner3-seg2",
    "EBC-O-chamfer-one-seg3-round",
    "EBC-O-chamfer-one-depth50",
    "EBC-O-E3-06-one-chamfer",
    "PB-O-B6-zero",
    "PE-O-E8-zero",
    "PE-O-E1-nonplanar-shiftX",
    "PE-O-E5-moved-item",
    "CMD-O-edge-join-two",
    "CMD-O-edge-subdivide-percent25",
    // Refused
    "PB-O-B4-rect-inset0.6",
    "PB-O-B4-square-inset0.5",
    "PB-O-B4-square-inset0.6",
    "PB-O-B5-L-inset0.6",
    "CMD-O-merge-ring-with-hole",
    "CMD-O-vert-align-all",
    "PRIM-O-cube-sizeneg",
    "PE-O-E6-empty-selection",
    // Mixed winding
    "CMD-O-align-mixed-winding",
    "CMD-O-M3-03-grid-mixed",
    "CMD-O-M3-03-pair-mixed",
    "CMD-O-M3-04-flipped1",
    "CMD-O-M3-04-flipped3",
    "PE-O-E2-fan-3-quads-extrude-one",
    "PE-O-E2-fan-3-quads-extrude-two",
    // Edge / Vertex Extrude (extrude-2026-11-05)
    "EE01",
    "EE02",
    "EE03",
    "EE04",
    "EE05",
    "EE06",
    "EE07",
    "EE08",
    "EE09",
    "VE01",
    "VE02",
    "VE03",
    "VE04",
    "VE05",
    "VE06",
    "VE07",
    "VE08",
    "VE09",
    // Smooth Shift / Thicken (smooth-shift-thicken-2026-11-05)
    "SS01",
    "SS02",
    "SS03",
    "SS04",
    "SS05",
    "SS06",
    "SS07",
    "SS08",
    "SS09",
    "SS10",
    "SS11a",
    "SS11b",
    "SS11c",
    "SS11d",
    "TH01",
    "TH02",
    "TH03",
    "TH04",
    "TH05",
    "TH06",
    "TH07a",
    "TH07b",
    "TH08a",
    "TH08b",
    "TH09",
    "TH10",
    "TH11",
    "TH12",
    "TH13",
    // Split (make-split-2026-11-05)
    "SP01",
    "SP02",
    "SP03",
    "SP04",
    "SP05",
    "SP06",
    "SP07",
    "SP08",
    "SP09",
    "SP10",
    "SP12",
    "SP13",
    // Quadruple (quadruple-2026-11-05)
    "QD01",
    "QD02",
    "QD03",
    "QD04",
    "QD05",
    "QD06",
    "QD07",
    "QD08",
    "QD09",
    "QD10",
    "QD11",
    "QD12",
    "QD13",
    "QD14",
    "QD15",
    "QD16",
    "QD17",
    "QD18",
    // Flip (poly-flip-2026-11-05)
    "FL01",
    "FL02",
    "FL03",
    "FL04",
    "FL05",
    "FL06",
    "FL07",
];

fn replay(id: &str, o: &Value) -> String {
    let mut b = Bridge::new();
    load_input(&mut b, o);
    if let Err(e) = run(&mut b, id, o) {
        // A case the reference refuses (QD16, FL07) passes on a refusal
        // that leaves the scene and selection as they were.
        let x = &o["expected"];
        if !(x["failed"].as_bool() == Some(true) || x.get("message").is_some()) {
            return format!("refused: {e}");
        }
    }
    let tolerance = o["tolerance"].as_f64().unwrap_or(1e-5).max(1e-6);
    grade(&actual(&b), &expected(o), tolerance)
}

#[test]
fn oracle_replay() {
    let oracles = all_oracles();
    let mut report = Vec::new();
    let mut failures = Vec::new();
    for &id in CASES {
        let o = oracles.get(id).unwrap_or_else(|| panic!("no oracle {id}"));
        let got = replay(id, o);
        report.push(format!("{id:40} {got}"));
        let pinned = EXPECTED.iter().find(|(i, _)| *i == id).map(|(_, g)| *g);
        let pass = got == "exact" || got == "numbered";
        match pinned {
            Some(p) if p == got => {}
            Some(p) => failures.push(format!("{id}: pinned `{p}`, now `{got}`")),
            None if pass => {}
            None => failures.push(format!("{id}: {got}")),
        }
    }
    println!("{}", report.join("\n"));
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
