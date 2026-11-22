//! Test bridge requests (specification BRIDGE-REQUESTS, BRQ-xx): commands
//! and readbacks the research side's recorded checks need. Each command
//! is one undo step, takes the tool's Reset values for options not given
//! (BRQ-01), and a refusal leaves the mesh as it was (BRQ-03).
use operandi_mcp::{Bridge, protocol};
use serde_json::{Map, Value, json};

fn call(b: &mut Bridge, name: &str, arguments: Value) -> Result<Value, String> {
    let r = protocol::handle(
        b,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": name, "arguments": arguments}}),
    )
    .expect("a request gets a response");
    let result = &r["result"];
    if result["isError"] == json!(true) {
        Err(result["content"][0]["text"].as_str().unwrap().to_string())
    } else {
        Ok(result["structuredContent"].clone())
    }
}

fn command(b: &mut Bridge, name: &str, args: Value) -> Result<Value, String> {
    call(b, "command_run", json!({"command": name, "args": args}))
}

fn tool(b: &mut Bridge, name: &str, params: Value) -> Result<Value, String> {
    call(b, "tool_run", json!({"tool": name, "params": params}))
}

fn mesh(b: &mut Bridge) -> Value {
    call(b, "mesh_get", json!({"edges": false})).unwrap()
}

fn faces(b: &mut Bridge) -> usize {
    mesh(b)["faces"].as_array().unwrap().len()
}

fn undo_len(b: &mut Bridge) -> usize {
    call(b, "scene_info", json!({})).unwrap()["undo"]
        .as_array()
        .unwrap()
        .len()
}

/// A unit cube in Polygon mode with nothing selected.
fn cube(b: &mut Bridge, position: [f64; 3]) {
    call(
        b,
        "primitive_add",
        json!({"shape": "cube", "params": {"position": position}}),
    )
    .unwrap();
    call(b, "select", json!({"mode": "polygon"})).unwrap();
}

/// World bounding box (min, max) of the active mesh.
fn bounds(b: &mut Bridge) -> ([f64; 3], [f64; 3]) {
    let m = call(b, "mesh_get", json!({"world": true, "edges": false})).unwrap();
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for v in m["vertices"].as_array().unwrap() {
        for i in 0..3 {
            let x = v[i].as_f64().unwrap();
            lo[i] = lo[i].min(x);
            hi[i] = hi[i].max(x);
        }
    }
    (lo, hi)
}

/// A refusal changes nothing: same mesh, same history.
fn refused(result: Result<Value, String>, words: &str) {
    let e = result.expect_err("refused");
    assert!(e.contains(words), "{e}");
}

/// BRQ-10: Subdivide by method and iterations, one undo step each.
#[test]
fn subdivide_takes_method_and_iterations() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    let undo = undo_len(&mut b);
    command(&mut b, "poly.subdivide", json!({})).unwrap();
    assert_eq!(faces(&mut b), 24, "Catmull-Clark by default");
    assert_eq!(undo_len(&mut b), undo + 1);
    call(&mut b, "undo", json!({})).unwrap();
    command(
        &mut b,
        "poly.subdivide",
        json!({"type": "flat", "iterations": 2}),
    )
    .unwrap();
    assert_eq!(faces(&mut b), 96);
    assert_eq!(undo_len(&mut b), undo + 1);
    // Faceted keeps the cube's shape; Catmull-Clark shrinks it.
    let (lo, hi) = bounds(&mut b);
    assert!((hi[0] - lo[0] - 1.).abs() < 1e-9);
    call(&mut b, "undo", json!({})).unwrap();
    command(
        &mut b,
        "poly.subdivide",
        json!({"type": "ccsds", "bound": "crease", "maxangle": 1.0, "fractal": 0.0}),
    )
    .unwrap();
    assert_eq!(faces(&mut b), 24);
    let before = mesh(&mut b);
    refused(
        command(&mut b, "poly.subdivide", json!({"type": "cubic"})),
        "Unknown subdivision type",
    );
    refused(
        command(&mut b, "poly.subdivide", json!({"fractal": 2.0})),
        "Fractal",
    );
    assert_eq!(mesh(&mut b), before);
}

/// BRQ-10: UnSubdivide undoes a Catmull-Clark step.
#[test]
fn unsubdivide_merges_quads_back() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    command(&mut b, "poly.subdivide", json!({"type": "flat"})).unwrap();
    assert_eq!(faces(&mut b), 24);
    let undo = undo_len(&mut b);
    command(&mut b, "poly.unsubdiv", json!({"iteration": 1})).unwrap();
    assert_eq!(faces(&mut b), 6);
    assert_eq!(undo_len(&mut b), undo + 1);
    refused(
        command(&mut b, "poly.unsubdiv", json!({"iteration": 99})),
        "Iteration",
    );
}

/// BRQ-10: Center moves the selection (all when none) to 0 on the axes.
#[test]
fn center_moves_to_the_origin_on_the_given_axes() {
    let mut b = Bridge::new();
    cube(&mut b, [2., 3., 0.]);
    command(&mut b, "vert.center", json!({"axis": "x"})).unwrap();
    let (lo, hi) = bounds(&mut b);
    assert!(((lo[0] + hi[0]) / 2.).abs() < 1e-9);
    assert!(((lo[1] + hi[1]) / 2. - 3.).abs() < 1e-9, "Y stays");
    command(&mut b, "vert.center", json!({"axis": "all"})).unwrap();
    let (lo, hi) = bounds(&mut b);
    assert!(((lo[1] + hi[1]) / 2.).abs() < 1e-9);
    refused(command(&mut b, "vert.center", json!({"axis": "w"})), "axis");
    refused(command(&mut b, "vert.center", json!({})), "axis");
}

/// BRQ-10: Dice (Polygon Axis Slice) by count and by size.
#[test]
fn dice_cuts_by_number_and_by_size() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    let undo = undo_len(&mut b);
    tool(
        &mut b,
        "poly.julienne",
        json!({"slice": {"number": [1, 0, 0]}}),
    )
    .unwrap();
    // One cut across X splits the four sides around it.
    assert_eq!(faces(&mut b), 10);
    assert_eq!(undo_len(&mut b), undo + 1);
    call(&mut b, "undo", json!({})).unwrap();
    tool(
        &mut b,
        "poly.julienne",
        json!({"slice": {"mode": "size", "size": [0.25, 0., 0.]}}),
    )
    .unwrap();
    // Cuts at -0.25, 0 and 0.25: three cuts, four rows on each side.
    assert_eq!(faces(&mut b), 2 + 4 * 4);
    refused(
        tool(&mut b, "poly.julienne", json!({"slice": {"points": []}})),
        "no slice option",
    );
}

/// BRQ-10: Curve Slice cuts along its curve, projected on the axis.
#[test]
fn curve_slice_cuts_along_the_curve() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    let undo = undo_len(&mut b);
    tool(
        &mut b,
        "Curve Slice",
        json!({"slice": {"points": [[-1, 0, 0], [1, 0, 0]], "steps": 1}}),
    )
    .unwrap();
    // Projected along Y, the line sweeps the plane z = 0: top, bottom and
    // both X sides are each cut in two.
    assert_eq!(faces(&mut b), 10);
    assert_eq!(undo_len(&mut b), undo + 1);
    let before = mesh(&mut b);
    refused(
        tool(&mut b, "Curve Slice", json!({"slice": {}})),
        "two points",
    );
    refused(
        tool(
            &mut b,
            "Curve Slice",
            json!({"slice": {"points": [[0, 0, -1], [0, 0, 1]], "operation": "dice"}}),
        ),
        "operation",
    );
    assert_eq!(mesh(&mut b), before);
    assert_eq!(undo_len(&mut b), undo + 1);
}

/// BRQ-10: Primitive Slice places its shape at a centre on a face.
#[test]
fn primitive_slice_cuts_its_shape() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    tool(
        &mut b,
        "Primitive Slice",
        json!({"slice": {"shape": "rectangle", "center": [0, 0.5, 0], "half_size": [0.2, 0.2]}}),
    )
    .unwrap();
    let after = faces(&mut b);
    assert!(after > 6, "{after}");
    call(&mut b, "undo", json!({})).unwrap();
    // Turned a quarter turn about the face normal: the same cut.
    tool(
        &mut b,
        "Primitive Slice",
        json!({"slice": {"center": [0, 0.5, 0], "half_size": [0.2, 0.2], "bank": std::f64::consts::FRAC_PI_2}}),
    )
    .unwrap();
    assert_eq!(faces(&mut b), after);
    call(&mut b, "undo", json!({})).unwrap();
    refused(
        tool(
            &mut b,
            "Primitive Slice",
            json!({"slice": {"shape": "circle"}}),
        ),
        "center",
    );
    assert_eq!(faces(&mut b), 6);
}

/// BRQ-02: command_help lists the new commands and their options.
#[test]
fn help_lists_the_new_commands() {
    for (name, arg) in [
        ("poly.subdivide", "iterations"),
        ("poly.unsubdiv", "iteration"),
        ("vert.center", "axis"),
    ] {
        let help = call(&mut Bridge::new(), "command_help", json!({"command": name})).unwrap();
        let text = help.to_string();
        assert!(text.contains(arg), "{name}: {text}");
    }
    let tools = Bridge::tool_definitions();
    let run = tools.iter().find(|t| t["name"] == "tool_run").unwrap();
    let ids = run["inputSchema"]["properties"]["tool"]["enum"].to_string();
    for id in ["Curve Slice", "Primitive Slice", "poly.julienne"] {
        assert!(ids.contains(id), "{id}");
    }
}

/// Y of every point of the active mesh, in item space.
fn ys(b: &mut Bridge) -> Vec<f64> {
    mesh(b)["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v[1].as_f64().unwrap())
        .collect()
}

/// BRQ-20: a Linear falloff weighs Move from 1 at Start to 0 at End.
#[test]
fn linear_falloff_weighs_a_move() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    call(&mut b, "select", json!({"mode": "vertex"})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "TransformMove", "params": {"delta": [0, 0.5, 0]},
               "falloffs": [{"type": "linear", "start": [0, -0.5, 0], "end": [0, 0.5, 0]}]}),
    )
    .unwrap();
    // Bottom points (weight 1) rise 0.5 to 0; top points (weight 0) stay.
    let y = ys(&mut b);
    assert_eq!(y.iter().filter(|&&v| v.abs() < 1e-9).count(), 4, "{y:?}");
    assert_eq!(
        y.iter().filter(|&&v| (v - 0.5).abs() < 1e-9).count(),
        4,
        "{y:?}"
    );
    // Symmetric and custom shapes are taken; an unknown key is refused.
    call(&mut b, "undo", json!({})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "TransformMove", "params": {"delta": [0, 0.5, 0]},
               "falloffs": [{"type": "linear", "start": [0, -0.5, 0], "end": [0, 0.5, 0],
                             "shape": "custom", "in": 0.5, "out": 0.5, "symmetric": "start"}]}),
    )
    .unwrap();
    call(&mut b, "undo", json!({})).unwrap();
    let before = mesh(&mut b);
    refused(
        call(
            &mut b,
            "tool_run",
            json!({"tool": "TransformMove", "params": {"delta": [0, 1, 0]},
                   "falloffs": [{"type": "linear", "radius": 2}]}),
        ),
        "radius",
    );
    assert_eq!(mesh(&mut b), before);
}

/// BRQ-20: a Radial falloff moves only the points inside it, and stacks
/// with Invert.
#[test]
fn radial_falloff_reaches_its_size() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    call(&mut b, "select", json!({"mode": "vertex"})).unwrap();
    let radial = json!({"type": "radial", "center": [0.5, 0.5, 0.5], "size": [0.5, 0.5, 0.5]});
    let before = ys(&mut b);
    let moved = |b: &mut Bridge| {
        ys(b)
            .iter()
            .zip(&before)
            .filter(|(a, z)| (*a - *z).abs() > 1e-9)
            .count()
    };
    call(
        &mut b,
        "tool_run",
        json!({"tool": "TransformMove", "params": {"delta": [0, 0.25, 0]}, "falloffs": [radial]}),
    )
    .unwrap();
    assert_eq!(moved(&mut b), 1, "only the corner at the centre");
    call(&mut b, "undo", json!({})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "TransformMove", "params": {"delta": [0, 0.25, 0]},
               "falloffs": [radial, {"type": "invert"}]}),
    )
    .unwrap();
    assert_eq!(moved(&mut b), 7, "every point but the corner");
}

/// BRQ-20: the falloff tools take the spatial falloffs (Bevel, Rotate,
/// Scale, Move in Edge mode); Extrude refuses one.
#[test]
fn falloff_tools_take_spatial_falloffs() {
    let linear = json!([{"type": "linear", "start": [0, -0.5, 0], "end": [0, 0.5, 0]}]);
    for (tool, mode, params) in [
        ("TransformRotate", "vertex", json!({"delta": [0, 0.5, 0]})),
        ("TransformScale", "vertex", json!({"delta": [2, 2, 2]})),
        ("poly.bevel", "polygon", json!({"shift": 0.2})),
        ("TransformMove", "edge", json!({"delta": [0.2, 0, 0]})),
    ] {
        let mut b = Bridge::new();
        cube(&mut b, [0.; 3]);
        call(&mut b, "select", json!({"mode": mode})).unwrap();
        if mode == "polygon" {
            call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
        }
        call(
            &mut b,
            "tool_run",
            json!({"tool": tool, "params": params, "falloffs": linear}),
        )
        .unwrap_or_else(|e| panic!("{tool}: {e}"));
    }
    // Extrude has no falloff yet: the user's words, and nothing changes.
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    let before = mesh(&mut b);
    refused(
        call(
            &mut b,
            "tool_run",
            json!({"tool": "poly.extrude", "params": {"extent": [0, 0.2, 0]}, "falloffs": linear}),
        ),
        "doesn't work with a falloff yet",
    );
    assert_eq!(mesh(&mut b), before);
    for kind in [
        json!({"type": "cylinder", "axis": "x", "size": [1, 0.5, 0.5]}),
        json!({"type": "noise", "scale": 2}),
        json!({"type": "curvature", "scale": 1}),
    ] {
        let mut b = Bridge::new();
        cube(&mut b, [0.; 3]);
        call(&mut b, "select", json!({"mode": "vertex"})).unwrap();
        call(
            &mut b,
            "tool_run",
            json!({"tool": "TransformMove", "params": {"delta": [0, 1, 0]}, "falloffs": [kind]}),
        )
        .unwrap_or_else(|e| panic!("{kind}: {e}"));
    }
}

/// BRQ-30: Backdrop items show in scene_info and item_get with every
/// panel property, and backdrop_set edits them in one undo step.
#[test]
fn backdrops_read_back_and_set() {
    let mut b = Bridge::new();
    command(&mut b, "item.create", json!({"type": "backdrop"})).unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    let item = info["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "backdrop")
        .expect("a backdrop item")["id"]
        .clone();
    let got = call(&mut b, "item_get", json!({"item": item})).unwrap();
    assert_eq!(got["kind"], "backdrop");
    let bd = &got["backdrop"];
    for key in [
        "image",
        "projection",
        "show_in_perspective",
        "keep_aspect",
        "flip",
        "invert",
        "blend",
        "contrast",
        "brightness",
        "transparency",
        "uniform_size",
        "pixel_size",
    ] {
        assert!(bd.get(key).is_some(), "{key}: {bd}");
    }
    assert_eq!(bd["image"], Value::Null);
    let undo = undo_len(&mut b);
    let picture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/relief-stripes.png"
    );
    call(
        &mut b,
        "backdrop_set",
        json!({"item": item, "image": picture, "projection": "top", "flip": true,
               "contrast": 0.5, "transparency": 0.25, "uniform_size": false, "pixel_size": 0.02}),
    )
    .unwrap();
    assert_eq!(undo_len(&mut b), undo + 1);
    let bd = call(&mut b, "item_get", json!({"item": item})).unwrap()["backdrop"].clone();
    assert_eq!(bd["projection"], "Top");
    assert_eq!(bd["flip"], true);
    assert_eq!(bd["contrast"], 0.5);
    assert_eq!(bd["transparency"], 0.25);
    assert_eq!(bd["pixel_size"], 0.02);
    assert!(bd["image"]["width"].as_u64().unwrap() > 0, "{bd}");
    assert_eq!(bd["image"]["path"], picture);
    // Refusals change nothing.
    refused(
        call(
            &mut b,
            "backdrop_set",
            json!({"item": item, "projection": "sideways"}),
        ),
        "projection",
    );
    refused(
        call(&mut b, "backdrop_set", json!({"item": item, "gamma": 2})),
        "gamma",
    );
    let first = info["items"][0]["id"].clone();
    refused(
        call(
            &mut b,
            "backdrop_set",
            json!({"item": first, "flip": false}),
        ),
        "not a backdrop",
    );
    assert_eq!(undo_len(&mut b), undo + 1);
    assert_eq!(info["items"][0]["kind"], "mesh");
}

/// BRQ-40: mesh_get subdivided returns the displayed surface.
#[test]
fn mesh_get_reads_the_subdivided_surface() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    // Face polygons are shown as they are.
    let flat = call(&mut b, "mesh_get", json!({"subdivided": true})).unwrap();
    assert_eq!(flat["faces"].as_array().unwrap().len(), 6);
    call(
        &mut b,
        "subdivision_set",
        json!({"toggle": "catmull_clark", "catmull_clark_level": 2}),
    )
    .unwrap();
    let cage = call(&mut b, "mesh_get", json!({})).unwrap();
    assert_eq!(
        cage["faces"].as_array().unwrap().len(),
        6,
        "the cage by default"
    );
    let shown = call(
        &mut b,
        "mesh_get",
        json!({"subdivided": true, "edges": false}),
    )
    .unwrap();
    assert_eq!(shown["faces"].as_array().unwrap().len(), 6 * 16);
    assert_eq!(shown["vertices"].as_array().unwrap().len(), 98);
    assert_eq!(shown["source_faces"].as_array().unwrap().len(), 96);
    let one = call(&mut b, "mesh_get", json!({"subdivided": 1})).unwrap();
    assert_eq!(one["faces"].as_array().unwrap().len(), 24);
    // Catmull-Clark pulls the corners in.
    let v = &shown["vertices"].as_array().unwrap();
    assert!(
        v.iter().all(|p| p[0].as_f64().unwrap().abs() < 0.5 - 1e-6
            || p[1].as_f64().unwrap().abs() < 0.5 - 1e-6)
    );
    refused(call(&mut b, "mesh_get", json!({"subdivided": 9})), "level");
    refused(call(&mut b, "mesh_get", json!({"levels": 1})), "levels");
}

/// A cube cut once across X, in Edge mode with its x = 0 loop selected.
fn cube_with_loop(b: &mut Bridge) {
    call(
        b,
        "primitive_add",
        json!({"shape": "cube", "params": {"segments": [2, 1, 1]}}),
    )
    .unwrap();
    let m = call(b, "mesh_get", json!({})).unwrap();
    let v = m["vertices"].as_array().unwrap();
    let on_loop = |i: u64| v[i as usize][0].as_f64().unwrap().abs() < 1e-9;
    let edges: Vec<Value> = m["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| on_loop(e[0].as_u64().unwrap()) && on_loop(e[1].as_u64().unwrap()))
        .cloned()
        .collect();
    assert_eq!(edges.len(), 4);
    call(b, "select", json!({"mode": "edge", "edges": edges})).unwrap();
}

/// The x of the points that started on the x = 0 loop.
fn loop_xs(b: &mut Bridge, before: &Value) -> Vec<f64> {
    let after = mesh(b);
    before["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["vertices"].as_array().unwrap())
        .filter(|(p, _)| p[0].as_f64().unwrap().abs() < 1e-9)
        .map(|(_, q)| q[0].as_f64().unwrap())
        .collect()
}

/// BRQ-50: Edge Slide takes Percent or Distance, and refuses what isn't
/// built.
#[test]
fn edge_slide_takes_percent_and_distance() {
    let mut b = Bridge::new();
    cube_with_loop(&mut b);
    let before = mesh(&mut b);
    let undo = undo_len(&mut b);
    tool(&mut b, "edge.slide", json!({"slide": {"percent": 0.5}})).unwrap();
    let xs = loop_xs(&mut b, &before);
    assert_eq!(xs.len(), 4);
    assert!(xs.iter().all(|x| (x.abs() - 0.25).abs() < 1e-9), "{xs:?}");
    assert_eq!(undo_len(&mut b), undo + 1);
    call(&mut b, "undo", json!({})).unwrap();
    // Distance interpolation: 0.1 m along each 0.5 m rail.
    tool(&mut b, "edge.slide", json!({"slide": {"distance": 0.1}})).unwrap();
    let xs = loop_xs(&mut b, &before);
    assert!(xs.iter().all(|x| (x.abs() - 0.1).abs() < 1e-9), "{xs:?}");
    call(&mut b, "undo", json!({})).unwrap();
    // Stop Negative Slide: a negative percent does nothing.
    tool(
        &mut b,
        "edge.slide",
        json!({"slide": {"interpolation": "percent", "percent": -0.5, "stop": true}}),
    )
    .unwrap();
    let xs = loop_xs(&mut b, &before);
    assert!(xs.iter().all(|x| x.abs() < 1e-9), "{xs:?}");
    assert_eq!(mesh(&mut b), before);
    for (bad, words) in [
        (json!({"merge": false}), "Merge"),
        (json!({"mode": "linear"}), "Radial"),
        (json!({"reverse": true}), "reverse"),
        (json!({"duplicate": true}), "duplicate"),
        (json!({"interpolation": "angle"}), "interpolation"),
        (json!({"spin": 1}), "spin"),
    ] {
        let e = tool(&mut b, "edge.slide", json!({"slide": bad.clone()})).expect_err("refused");
        assert!(e.contains(words), "{bad}: {e}");
    }
    assert_eq!(mesh(&mut b), before);
    refused(
        tool(&mut b, "TransformMove", json!({"slide": {"percent": 0.5}})),
        "no slide options",
    );
}

/// BRQ-60: Mesh Cleanup with each switch its own option.
#[test]
fn mesh_cleanup_takes_each_switch() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    command(&mut b, "vert.new", json!({"position": [3, 0, 0]})).unwrap();
    let count = |b: &mut Bridge| mesh(b)["vertices"].as_array().unwrap().len();
    assert_eq!(count(&mut b), 9);
    // Floating vertices off: the stray point stays.
    command(&mut b, "mesh.cleanup", json!({"floatingVertex": false})).unwrap();
    assert_eq!(count(&mut b), 9);
    let undo = undo_len(&mut b);
    command(&mut b, "mesh.cleanup", json!({})).unwrap();
    assert_eq!(count(&mut b), 8);
    assert_eq!(undo_len(&mut b), undo + 1);
    let none = json!({
        "floatingVertex": false, "onePointPolygon": false, "twoPointPolygon": false,
        "dupPointPolygon": false, "colinear": false, "faceNormal": false,
        "mergeVertex": false, "mergeDisco": false, "unifyPolygon": false,
        "forceUnify": false, "removeDiscoWeight": false, "fixGaps": false,
    });
    refused(
        command(&mut b, "mesh.cleanup", none),
        "No Mesh Cleanup operations selected",
    );
    refused(
        command(&mut b, "mesh.cleanup", json!({"tidy": true})),
        "tidy",
    );
    assert_eq!(undo_len(&mut b), undo + 1);
}

/// BRQ-70: mesh_get uvs returns each UV map, one UV per polygon corner.
#[test]
fn mesh_get_reads_uvs_per_corner() {
    let mut b = Bridge::new();
    call(
        &mut b,
        "primitive_add",
        json!({"shape": "cube", "params": {"make_uvs": true}}),
    )
    .unwrap();
    let m = call(&mut b, "mesh_get", json!({"uvs": true, "edges": false})).unwrap();
    let maps = m["uv_maps"].as_object().expect("uv_maps");
    assert!(!maps.is_empty(), "{m}");
    for (name, faces) in maps {
        let faces = faces.as_array().unwrap();
        assert_eq!(faces.len(), 6, "{name}");
        for (f, corners) in faces.iter().enumerate() {
            let corners = corners.as_array().unwrap();
            assert_eq!(
                corners.len(),
                m["faces"][f]["vertices"].as_array().unwrap().len()
            );
            for uv in corners {
                let (u, v) = (uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap());
                assert!((-1e-9..=1. + 1e-9).contains(&u) && (-1e-9..=1. + 1e-9).contains(&v));
            }
        }
    }
    // Off by default.
    assert!(mesh(&mut b).get("uv_maps").is_none());
    // A map follows a Bevel: the new polygons get UVs too.
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    tool(&mut b, "poly.bevel", json!({"inset": 0.1, "shift": 0.1})).unwrap();
    let m = call(&mut b, "mesh_get", json!({"uvs": true, "edges": false})).unwrap();
    let faces = m["faces"].as_array().unwrap().len();
    for faces_uv in m["uv_maps"].as_object().unwrap().values() {
        assert_eq!(faces_uv.as_array().unwrap().len(), faces);
    }
}

/// The two cube faces whose points all share y = ±0.5.
fn top_and_bottom(b: &mut Bridge) -> Vec<usize> {
    let m = mesh(b);
    let v = m["vertices"].as_array().unwrap().clone();
    m["faces"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            let ys: Vec<f64> = f["vertices"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| v[i.as_u64().unwrap() as usize][1].as_f64().unwrap())
                .collect();
            ys.iter()
                .all(|y| (y.abs() - 0.5).abs() < 1e-9 && (y - ys[0]).abs() < 1e-9)
        })
        .map(|(i, _)| i)
        .collect()
}

/// BRQ-80: Bridge as a command, with the panel's built options.
#[test]
fn bridge_runs_with_its_options() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    let ends = top_and_bottom(&mut b);
    assert_eq!(ends.len(), 2);
    call(&mut b, "select", json!({"mode": "polygon", "faces": ends})).unwrap();
    let undo = undo_len(&mut b);
    tool(&mut b, "edge.bridge", json!({})).unwrap();
    // Remove on: the two ends go, a tunnel of four quads joins them.
    assert_eq!(faces(&mut b), 4 + 4);
    assert_eq!(undo_len(&mut b), undo + 1);
    call(&mut b, "undo", json!({})).unwrap();
    tool(
        &mut b,
        "edge.bridge",
        json!({"bridge": {"segments": 2, "mode": "linear", "remove": false}}),
    )
    .unwrap();
    // Remove off keeps both ends; Segments add rows to the tunnel.
    assert_eq!(top_and_bottom(&mut b).len(), 2);
    assert!(faces(&mut b) > 6 + 4, "{}", faces(&mut b));
    call(&mut b, "undo", json!({})).unwrap();
    let before = mesh(&mut b);
    for (bad, words) in [
        (json!({"bridge": {"mode": "spline"}}), "mode"),
        (json!({"bridge": {"orient": true}}), "isn't built"),
        (json!({"bridge": {"segs": 2}}), "segs"),
        (json!({"segments": 2}), "bridge"),
    ] {
        let e = tool(&mut b, "edge.bridge", bad.clone()).expect_err("refused");
        assert!(e.contains(words), "{bad}: {e}");
    }
    assert_eq!(mesh(&mut b), before);
    // One polygon: nothing to bridge, said out loud.
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    refused(tool(&mut b, "edge.bridge", json!({})), "two");
    assert_eq!(mesh(&mut b), before);
}

/// Two unit cubes, the second 3 m along X; the first is active. Their ids.
fn two_cubes(b: &mut Bridge) -> (u64, u64) {
    call(b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let first = call(b, "scene_info", json!({})).unwrap()["selection"]["item"]
        .as_u64()
        .unwrap();
    command(b, "mesh.new", json!({})).unwrap();
    call(
        b,
        "primitive_add",
        json!({"shape": "cube", "params": {"position": [3, 0, 0]}}),
    )
    .unwrap();
    let second = call(b, "scene_info", json!({})).unwrap()["selection"]["item"]
        .as_u64()
        .unwrap();
    command(b, "select.item", json!({"item": first})).unwrap();
    (first, second)
}

/// BRQ-90..94: build a stack, set an op's options and item inputs, read
/// the stack and its result.
#[test]
fn mesh_ops_read_back_their_stack_and_result() {
    let mut b = Bridge::new();
    let (first, second) = two_cubes(&mut b);
    command(&mut b, "meshop.create", json!({"type": "pmodel.meshmerge"})).unwrap();
    let ops = call(&mut b, "item_get", json!({"item": first})).unwrap()["mesh_ops"].clone();
    assert_eq!(ops.as_array().unwrap().len(), 1, "{ops}");
    let op = ops[0]["op"].as_str().unwrap().to_owned();
    assert_eq!(ops[0]["type"], "pmodel.meshmerge");
    assert_eq!(ops[0]["enabled"], true);
    assert_eq!(ops[0]["values"]["sources"], json!([]));
    // The Merge Meshes source is an item input.
    let undo = undo_len(&mut b);
    call(
        &mut b,
        "meshop_set",
        json!({"op": op, "values": {"sources": [second], "hierarchy": true}}),
    )
    .unwrap();
    assert_eq!(undo_len(&mut b), undo + 1);
    let ops = call(&mut b, "item_get", json!({"item": first})).unwrap()["mesh_ops"].clone();
    assert_eq!(ops[0]["values"]["sources"], json!([second]));
    assert_eq!(ops[0]["values"]["hierarchy"], true);
    assert_eq!(ops[0]["status"], "ok");
    // mesh_get returns the Base Mesh unless asked for the result.
    let base = call(&mut b, "mesh_get", json!({"item": first, "edges": false})).unwrap();
    assert_eq!(base["vertices"].as_array().unwrap().len(), 8);
    let result = call(
        &mut b,
        "mesh_get",
        json!({"item": first, "evaluated": true, "edges": false}),
    )
    .unwrap();
    assert_eq!(result["vertices"].as_array().unwrap().len(), 16);
    assert_eq!(result["faces"].as_array().unwrap().len(), 12);
    assert_eq!(result["faces"][0]["type"], "face");
    assert_eq!(result["mesh_ops_status"], json!(["ok"]));
    // Off passes the input through.
    command(
        &mut b,
        "deformer.enable",
        json!({"deformer": op, "enable": false}),
    )
    .unwrap();
    let off = call(
        &mut b,
        "mesh_get",
        json!({"item": first, "evaluated": true}),
    )
    .unwrap();
    assert_eq!(off["vertices"].as_array().unwrap().len(), 8);
    assert_eq!(off["mesh_ops_status"], json!(["off"]));
    // Refusals change nothing.
    let undo = undo_len(&mut b);
    refused(
        call(
            &mut b,
            "meshop_set",
            json!({"op": op, "values": {"spin": 1}}),
        ),
        "spin",
    );
    refused(
        call(&mut b, "meshop_set", json!({"op": "first", "values": {}})),
        "op:<item>:<op>",
    );
    assert_eq!(undo_len(&mut b), undo);
}

/// BRQ-92: meshop.create takes every op type, and command_help lists them.
#[test]
fn every_mesh_op_type_is_listed_and_can_be_added() {
    let help = call(
        &mut Bridge::new(),
        "command_help",
        json!({"command": "meshop.create"}),
    )
    .unwrap();
    let text = help.to_string();
    let types: Vec<String> = help
        .as_array()
        .map(|a| a[0].clone())
        .unwrap_or(help.clone())["types"]
        .as_array()
        .unwrap_or_else(|| panic!("{text}"))
        .iter()
        .map(|t| t["type"].as_str().unwrap().to_owned())
        .collect();
    for wanted in [
        "pmodel.meshmerge",
        "boolean.meshop.item",
        "solidDrill.meshop.item",
        "axisDrill.meshop.item",
        "poly.bevel.item",
        "curve.circle.item",
    ] {
        assert!(types.iter().any(|t| t == wanted), "{wanted}: {types:?}");
    }
    for t in &types {
        let mut b = Bridge::new();
        call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
        command(&mut b, "meshop.create", json!({"type": t})).unwrap_or_else(|e| panic!("{t}: {e}"));
        // Pack UVs takes many seconds in a debug build; adding it is enough.
        if t == "uvpack.meshop.item" {
            continue;
        }
        let got = call(
            &mut b,
            "mesh_get",
            json!({"evaluated": true, "edges": false}),
        );
        assert!(got.is_ok(), "{t}: {got:?}");
    }
}

/// Undoing every step of a call leaves the document unmodified, so the
/// next scene_load isn't refused (replay quirk after Curve Slice).
#[test]
fn undoing_everything_clears_modified() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    let dir = std::env::temp_dir().join(format!("operandi-brq-dirty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cube.opscene");
    call(&mut b, "scene_save", json!({"path": path})).unwrap();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    let modified =
        |b: &mut Bridge| call(b, "scene_info", json!({})).unwrap()["document"]["modified"].clone();
    assert_eq!(modified(&mut b), false);
    call(&mut b, "select", json!({"mode": "polygon"})).unwrap();
    tool(
        &mut b,
        "Curve Slice",
        json!({"slice": {"points": [[-1, 0, 0], [1, 0, 0]], "axis": "z"}}),
    )
    .unwrap();
    assert_eq!(modified(&mut b), true);
    let steps = undo_len(&mut b);
    call(&mut b, "undo", json!({"count": steps})).unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["document"]["modified"], false, "{info}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// BRQ-100: one tool_run call runs under the given Action Center, and the
/// setting from before comes back.
#[test]
fn action_center_applies_for_one_call() {
    let scale = |b: &mut Bridge, center: Value| {
        let mut args = json!({"tool": "TransformScale", "params": {"delta": [2, 2, 2]}});
        if !center.is_null() {
            args["action_center"] = center;
        }
        call(b, "tool_run", args)
    };
    let mid_x = |b: &mut Bridge| {
        let (lo, hi) = bounds(b);
        (lo[0] + hi[0]) / 2.
    };
    let mut b = Bridge::new();
    cube(&mut b, [2., 0., 0.]);
    call(&mut b, "select", json!({"mode": "vertex"})).unwrap();
    scale(&mut b, json!({"mode": "origin"})).unwrap();
    assert!((mid_x(&mut b) - 4.).abs() < 1e-9, "about the origin");
    call(&mut b, "undo", json!({})).unwrap();
    scale(&mut b, json!({"mode": "auto"})).unwrap();
    assert!(
        (mid_x(&mut b) - 2.).abs() < 1e-9,
        "about the points' centre"
    );
    call(&mut b, "undo", json!({})).unwrap();
    scale(&mut b, json!({"mode": "screen", "center": [1.5, 0, 0]})).unwrap();
    assert!((mid_x(&mut b) - 2.5).abs() < 1e-9, "about the given point");
    call(&mut b, "undo", json!({})).unwrap();
    // The earlier setting is back: same as a call without action_center.
    let before = b.editor.action_context;
    scale(&mut b, json!({"mode": "selection", "axis": "xy"})).unwrap();
    assert_eq!(b.editor.action_context, before);
    call(&mut b, "undo", json!({})).unwrap();
    let m = mesh(&mut b);
    for (bad, words) in [
        // Pivot and Parent work since the pivot service (ACS table).
        (json!({"mode": "pivot_world_axis"}), "isn't built"),
        (json!({"mode": "sideways"}), "Unknown action center"),
        (json!({"mode": "element"}), "center"),
        (
            json!({"mode": "origin", "center": [0, 0, 0]}),
            "takes no center",
        ),
        (json!({"mode": "origin", "axis": "diagonal"}), "axis"),
    ] {
        let e = scale(&mut b, bad.clone()).expect_err("refused");
        assert!(e.contains(words), "{bad}: {e}");
    }
    assert_eq!(mesh(&mut b), m);
    // Move, Rotate, Transform and Smooth Shift take it too.
    for (tool, params) in [
        ("TransformMove", json!({"delta": [0, 0.1, 0]})),
        ("TransformRotate", json!({"delta": [0, 0.5, 0]})),
        ("Transform", json!({"scale_delta": [1.5, 1.5, 1.5]})),
    ] {
        call(
            &mut b,
            "tool_run",
            json!({"tool": tool, "params": params, "action_center": {"mode": "origin"}}),
        )
        .unwrap_or_else(|e| panic!("{tool}: {e}"));
    }
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "poly.smshift", "params": {"smooth_shift": {"offset": 0.1, "scale": 0.5}},
               "action_center": {"mode": "selection"}}),
    )
    .unwrap();
}

/// BRQ-60 checks start from broken meshes: scene_load opens 1- and
/// 2-point, repeated-point and zero-area polygons as-is, in order, and
/// Mesh Cleanup's matching options remove or fix exactly those.
#[test]
fn scene_load_keeps_broken_polygons_for_mesh_cleanup() {
    let dir = std::env::temp_dir().join(format!("operandi-brq-broken-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("broken.opscene");
    let face = |v: &[u32]| json!({"vertices": v, "uv": null, "material": "", "smooth": false});
    let polygons = [
        vec![0, 1, 2, 3], // a good quad
        vec![4],          // one point
        vec![4, 5],       // two points
        vec![0, 1, 1, 2], // a repeated point
        vec![0, 1, 6],    // zero area: 6 lies between 0 and 1
    ];
    let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
        "id": 1, "name": "Broken", "visible": true,
        "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
        "mesh": {
            "vertices": [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0], [3, 0, 0], [4, 0, 0], [0.5, 0, 0]],
            "faces": polygons.iter().map(|p| face(p)).collect::<Vec<_>>()
        }
    }], "next_id": 2,
        "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [], "faces": []},
        "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
    std::fs::write(&path, scene.to_string()).unwrap();
    let mut b = Bridge::new();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    let corners = |b: &mut Bridge| -> Vec<Vec<u64>> {
        mesh(b)["faces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                f["vertices"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect()
            })
            .collect()
    };
    let given: Vec<Vec<u64>> = polygons
        .iter()
        .map(|p| p.iter().map(|&v| v as u64).collect())
        .collect();
    assert_eq!(corners(&mut b), given, "as given, in order");
    assert_eq!(mesh(&mut b)["vertices"].as_array().unwrap().len(), 7);
    let only = |key: &str| {
        let mut flags = Map::new();
        for k in [
            "floatingVertex",
            "onePointPolygon",
            "twoPointPolygon",
            "dupPointPolygon",
            "colinear",
            "faceNormal",
            "mergeVertex",
            "mergeDisco",
            "unifyPolygon",
            "forceUnify",
            "removeDiscoWeight",
            "fixGaps",
        ] {
            flags.insert(k.into(), json!(k == key));
        }
        Value::Object(flags)
    };
    // Left alone, an unrelated option keeps them.
    command(&mut b, "mesh.cleanup", only("mergeDisco")).unwrap();
    assert_eq!(corners(&mut b), given);
    command(&mut b, "mesh.cleanup", only("onePointPolygon")).unwrap();
    assert!(!corners(&mut b).contains(&vec![4]));
    assert!(corners(&mut b).contains(&vec![4, 5]));
    command(&mut b, "mesh.cleanup", only("twoPointPolygon")).unwrap();
    assert!(!corners(&mut b).iter().any(|f| f.len() < 3));
    command(&mut b, "mesh.cleanup", only("dupPointPolygon")).unwrap();
    assert!(
        corners(&mut b).contains(&vec![0, 1, 2]),
        "{:?}",
        corners(&mut b)
    );
    // File > Open's loader still refuses the file.
    assert!(operandi_core::persistence::load(&path).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

/// MC15 (mesh-cleanup-2026-11-05): two polygons on the same points, the
/// second tagged "Other"; Unify on, Force off keeps both, opened from a
/// file or tagged through poly.setMaterial.
#[test]
fn mesh_cleanup_unify_keeps_duplicates_with_another_material() {
    let dir = std::env::temp_dir().join(format!("operandi-mc15-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let only_unify = json!({
        "floatingVertex": false, "onePointPolygon": false, "twoPointPolygon": false,
        "dupPointPolygon": false, "colinear": false, "faceNormal": false,
        "mergeVertex": false, "mergeDisco": false, "unifyPolygon": true,
        "forceUnify": false, "removeDiscoWeight": false, "fixGaps": false,
    });
    for (first, second) in [("Default", "Other"), ("", "Other")] {
        let path = dir.join(format!("mc15-{first}.opscene"));
        let face =
            |m: &str| json!({"vertices": [0, 1, 2, 3], "uv": null, "material": m, "smooth": false});
        let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
            "id": 1, "name": "MC15", "visible": true,
            "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
            "mesh": {
                "vertices": [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]],
                "faces": [face(first), face(second)]
            }
        }], "next_id": 2,
            "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [], "faces": []},
            "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
        std::fs::write(&path, scene.to_string()).unwrap();
        let mut b = Bridge::new();
        call(&mut b, "scene_load", json!({"path": path})).unwrap();
        assert_eq!(faces(&mut b), 2, "{first}: opened as given");
        command(&mut b, "mesh.cleanup", only_unify.clone()).unwrap();
        assert_eq!(faces(&mut b), 2, "{first}: {}", mesh(&mut b));
    }
    // Tagged through the command instead.
    let path = dir.join("mc15-command.opscene");
    let face = json!({"vertices": [0, 1, 2, 3], "uv": null, "material": "", "smooth": false});
    let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
        "id": 1, "name": "MC15", "visible": true,
        "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
        "mesh": {"vertices": [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]], "faces": [face.clone(), face]}
    }], "next_id": 2,
        "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [], "faces": []},
        "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
    std::fs::write(&path, scene.to_string()).unwrap();
    let mut b = Bridge::new();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    call(&mut b, "select", json!({"mode": "polygon", "faces": [1]})).unwrap();
    command(&mut b, "poly.setMaterial", json!({"name": "Other"})).unwrap();
    let m = mesh(&mut b);
    assert_eq!(m["faces"][1]["material"], "Other", "{m}");
    call(&mut b, "select", json!({"mode": "polygon", "faces": []})).unwrap();
    command(&mut b, "mesh.cleanup", only_unify).unwrap();
    assert_eq!(faces(&mut b), 2, "{}", mesh(&mut b));
}

/// A full undo after a multi-item MeshOps case leaves the document clean.
#[test]
fn undoing_a_mesh_ops_case_clears_modified() {
    let mut b = Bridge::new();
    cube(&mut b, [0.; 3]);
    let dir = std::env::temp_dir().join(format!("operandi-brq-dirty2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cube.opscene");
    call(&mut b, "scene_save", json!({"path": path})).unwrap();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    let saved = b.editor.scene.clone();
    let first = b.editor.scene.items[0].id;
    command(&mut b, "mesh.new", json!({})).unwrap();
    call(
        &mut b,
        "primitive_add",
        json!({"shape": "cube", "params": {"position": [3, 0, 0]}}),
    )
    .unwrap();
    let second = b.editor.scene.selection.item.unwrap();
    command(&mut b, "select.item", json!({"item": first})).unwrap();
    command(&mut b, "meshop.create", json!({"type": "pmodel.meshmerge"})).unwrap();
    let op = call(&mut b, "item_get", json!({"item": first})).unwrap()["mesh_ops"][0]["op"].clone();
    call(
        &mut b,
        "meshop_set",
        json!({"op": op, "values": {"sources": [second]}}),
    )
    .unwrap();
    let steps = undo_len(&mut b);
    call(&mut b, "undo", json!({"count": steps})).unwrap();
    if b.editor.scene != saved {
        let ours = format!("{:#?}", b.editor.scene);
        let theirs = format!("{:#?}", saved);
        let diff: Vec<_> = ours
            .lines()
            .zip(theirs.lines())
            .filter(|(x, y)| x != y)
            .take(10)
            .collect();
        panic!("scene differs after full undo: {diff:#?}");
    }
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["document"]["modified"], false, "{info}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The active mesh's UVs in `map`: one [u, v] per corner, per face.
fn uvs(b: &mut Bridge, map: &str) -> Vec<Value> {
    let m = call(b, "mesh_get", json!({"edges": false, "uvs": true})).unwrap();
    m["uv_maps"][map]
        .as_array()
        .unwrap_or_else(|| panic!("no UV map {map}: {m}"))
        .clone()
}

fn hidden_faces(b: &Bridge) -> Vec<u32> {
    b.editor
        .scene
        .active()
        .unwrap()
        .elements
        .hidden
        .faces
        .iter()
        .copied()
        .collect()
}

/// A unit cube unwrapped with no seams: six one-face islands in Texture.
fn unwrapped_cube(b: &mut Bridge) {
    cube(b, [0., 0., 0.]);
    command(b, "uv.unwrap", json!({})).unwrap();
}

/// BRQ-110: command_run uv.relax takes every Tool Properties field UV
/// Relax shows, over its Reset values, as one undo step on the UV map.
#[test]
fn uv_relax_runs_with_the_panel_options() {
    let mut b = Bridge::new();
    unwrapped_cube(&mut b);
    let before = uvs(&mut b, "Texture");
    let steps = undo_len(&mut b);
    // Reset values: Iterations 0 relaxes nothing and records nothing.
    command(&mut b, "uv.relax", json!({})).unwrap();
    assert_eq!(uvs(&mut b, "Texture"), before);
    assert_eq!(undo_len(&mut b), steps);
    // Spring pulls each square's corners in: the UVs move, 3D doesn't.
    let points = mesh(&mut b)["vertices"].clone();
    command(&mut b, "uv.relax", json!({"mode": "spring", "iter": 5})).unwrap();
    assert_ne!(uvs(&mut b, "Texture"), before);
    assert_eq!(mesh(&mut b)["vertices"], points);
    assert_eq!(undo_len(&mut b), steps + 1, "one undo step");
    call(&mut b, "undo", json!({})).unwrap();
    assert_eq!(uvs(&mut b, "Texture"), before);
    call(&mut b, "redo", json!({})).unwrap();
    assert_ne!(uvs(&mut b, "Texture"), before);
    call(&mut b, "undo", json!({})).unwrap();
    // Every mode and border choice runs, by name or by the reference's
    // integer; Locked holds the borders (here every point).
    for args in [
        json!({"mode": "conformal", "iter": 10, "seal": true, "boundary": "smooth"}),
        json!({"mode": 2, "iter": 10, "boundary": "maintain"}),
        json!({"mode": "angleBased", "iter": 10, "boundary": "straighten", "uniform": true}),
        json!({"mode": "adaptive", "iter": 3, "area": 0.5, "noEdge": true, "name": "Texture"}),
    ] {
        command(&mut b, "uv.relax", args.clone()).unwrap_or_else(|e| panic!("{args}: {e}"));
        let n = undo_len(&mut b);
        if n > steps {
            call(&mut b, "undo", json!({})).unwrap();
        }
    }
    command(
        &mut b,
        "uv.relax",
        json!({"mode": "conformal", "iter": 20, "boundary": "locked"}),
    )
    .unwrap();
    let locked = uvs(&mut b, "Texture");
    for (f, (x, y)) in locked.iter().zip(&before).enumerate() {
        for (p, q) in x.as_array().unwrap().iter().zip(y.as_array().unwrap()) {
            for k in 0..2 {
                let (p, q) = (p[k].as_f64().unwrap(), q[k].as_f64().unwrap());
                assert!((p - q).abs() < 1e-9, "face {f}: locked borders stay");
            }
        }
    }
}

/// BRQ-110: a polygon selection doesn't limit Relax; with Ignore Edge
/// Selection off, islands holding a selected edge stay still (UVS-1 §3).
#[test]
fn uv_relax_selection_rules() {
    let mut b = Bridge::new();
    unwrapped_cube(&mut b);
    let before = uvs(&mut b, "Texture");
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    command(&mut b, "uv.relax", json!({"mode": "spring", "iter": 5})).unwrap();
    let after = uvs(&mut b, "Texture");
    let moved = (0..6).filter(|&f| after[f] != before[f]).count();
    assert_eq!(moved, 6, "every island relaxes, not only the selected one");
    call(&mut b, "undo", json!({})).unwrap();
    // One edge of face 0: it and its neighbour across the edge hold still.
    let face0: Vec<u64> = mesh(&mut b)["faces"][0]["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    call(
        &mut b,
        "select",
        json!({"mode": "edge", "edges": [[face0[0], face0[1]]]}),
    )
    .unwrap();
    command(&mut b, "uv.relax", json!({"mode": "spring", "iter": 5})).unwrap();
    let all_relax = uvs(&mut b, "Texture");
    assert_eq!(
        (0..6).filter(|&f| all_relax[f] != before[f]).count(),
        6,
        "Ignore Edge Selection on (default): the edge is ignored"
    );
    call(&mut b, "undo", json!({})).unwrap();
    command(
        &mut b,
        "uv.relax",
        json!({"mode": "spring", "iter": 5, "noEdge": false}),
    )
    .unwrap();
    let held = uvs(&mut b, "Texture");
    let still = (0..6).filter(|&f| held[f] == before[f]).count();
    assert_eq!(still, 2, "the two islands with the edge stay still");
    assert_eq!(held[0], before[0]);
}

/// BRQ-110: refusals give the user's words and leave the mesh alone.
#[test]
fn uv_relax_refusals_leave_the_mesh() {
    let mut b = Bridge::new();
    cube(&mut b, [0., 0., 0.]);
    // A fresh cube has its primitive UVs; drop them to test "no map".
    let m = mesh(&mut b);
    let steps = undo_len(&mut b);
    for (args, words) in [
        (
            json!({"mode": "sideways", "iter": 5}),
            "mode must be one of",
        ),
        (
            json!({"boundary": "wobbly", "iter": 5}),
            "boundary must be one of",
        ),
        (
            json!({"area": 2, "iter": 5}),
            "Area Weight must be from 0 % to 100 %",
        ),
        (
            json!({"iter": 5, "name": "Nope"}),
            "no UV map to relax named Nope",
        ),
        (json!({"live": true}), "live"),
    ] {
        let e = command(&mut b, "uv.relax", args.clone()).expect_err("refused");
        assert!(e.contains(words), "{args}: {e}");
        assert_eq!(mesh(&mut b), m);
        assert_eq!(undo_len(&mut b), steps);
    }
}

/// BRQ-110: hide.sel / unhide on polygons, one undo step each; UV Unwrap
/// and UV Relax leave hidden polygons' UVs alone (UVW-04, UVS-1 §4).
#[test]
fn hidden_polygons_drop_out_of_unwrap_and_relax() {
    let mut b = Bridge::new();
    unwrapped_cube(&mut b);
    let before = uvs(&mut b, "Texture");
    call(&mut b, "select", json!({"mode": "polygon", "faces": [2]})).unwrap();
    let steps = undo_len(&mut b);
    command(&mut b, "hide.sel", json!({})).unwrap();
    assert_eq!(hidden_faces(&b), vec![2]);
    assert_eq!(undo_len(&mut b), steps + 1);
    // Unwrap at another angle moves every visible face, not the hidden one.
    command(&mut b, "uv.unwrap", json!({"angle": 0.5})).unwrap();
    let after = uvs(&mut b, "Texture");
    assert_eq!(after[2], before[2], "hidden face keeps its UVs");
    assert!((0..6).filter(|&f| f != 2).all(|f| after[f] != before[f]));
    command(&mut b, "uv.relax", json!({"mode": "spring", "iter": 5})).unwrap();
    assert_eq!(uvs(&mut b, "Texture")[2], before[2]);
    // Unhide shows everything again, one step; nothing hidden refuses.
    let steps = undo_len(&mut b);
    command(&mut b, "unhide", json!({})).unwrap();
    assert!(hidden_faces(&b).is_empty());
    assert_eq!(undo_len(&mut b), steps + 1);
    let m = mesh(&mut b);
    let e = command(&mut b, "unhide", json!({})).expect_err("nothing hidden");
    assert!(e.contains("Nothing is hidden"), "{e}");
    assert_eq!(mesh(&mut b), m);
    call(&mut b, "undo", json!({})).unwrap();
    assert_eq!(hidden_faces(&b), vec![2]);
    // The other hide forms run too.
    command(&mut b, "hide.invert", json!({})).unwrap();
    assert_eq!(hidden_faces(&b).len(), 5);
    command(&mut b, "unhide", json!({})).unwrap();
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    command(&mut b, "hide.unsel", json!({})).unwrap();
    assert_eq!(hidden_faces(&b).len(), 5);
}

/// BRQ-110: command_help lists uv.relax's options and the hide commands.
#[test]
fn command_help_lists_relax_and_hide() {
    let mut b = Bridge::new();
    let help = call(&mut b, "command_help", json!({"command": "uv.relax"})).unwrap();
    let names: Vec<&str> = help["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "mode", "iter", "seal", "boundary", "area", "uniform", "noEdge", "name"
        ]
    );
    for name in ["hide.sel", "hide.unsel", "hide.invert", "unhide"] {
        call(&mut b, "command_help", json!({"command": name}))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}
