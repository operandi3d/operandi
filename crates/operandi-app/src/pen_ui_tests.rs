//! Numeric Pen commands, picking and visibility of Pen geometry.
use super::tests::{draw, key, pen_command};
use super::*;

#[test]
fn pen_s1_command_text_correction_cancel_repeat_history_and_save() {
    let mut a = Operandi::empty();
    a.execute("new mesh");
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut a, &ctx, size, vec![]);
    // A refused entry keeps its text for correction.
    let bad = pen_command("vertices", false)
        .replace("\"flip\":false", "\"flip\":false,\"wall\":\"sideways\"");
    a.command = bad.clone();
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    draw(&mut a, &ctx, size, vec![key(Key::Enter)]);
    assert!(a.status.contains("wall"), "{}", a.status);
    assert_eq!(a.command, bad);
    assert_eq!(a.editor.undo.len(), 1);
    a.command = pen_command("vertices", false);
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    draw(&mut a, &ctx, size, vec![key(Key::Escape)]);
    assert!(a.editor.scene.active().unwrap().mesh.vertices.is_empty());
    assert_eq!(a.editor.undo.len(), 1);
    for _ in 0..2 {
        ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
        draw(&mut a, &ctx, size, vec![key(Key::Enter)]);
        assert_eq!(a.status, "Ready");
    }
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 8);
    a.execute("undo");
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 4);
    a.execute("undo");
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 0);
    a.execute("redo");
    a.execute("redo");
    let before = a.editor.scene.clone();
    let path = std::env::temp_dir().join(format!("pen-app-{}.opscene", std::process::id()));
    a.path = Some(path.clone());
    a.save_current();
    a.open(path.clone());
    assert_eq!(a.editor.scene, before);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn pen_s1_visible_pickable_surface_vertices_and_line_spans() {
    let r = Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.));
    for kind in ["polygon", "vertices", "polyline"] {
        for flag in [false, true] {
            if kind == "vertices" && flag {
                continue;
            }
            let mut a = Operandi::empty();
            a.execute("new mesh");
            a.execute(&pen_command(kind, flag));
            a.camera.view = if kind == "polygon" && flag {
                View::Back
            } else {
                View::Front
            };
            a.camera.distance = 12.;
            a.camera.target = DVec3::new(1., 0.5, 0.);
            let s = picking::Surface::new(&a.editor.scene);
            let camera = a.camera.clone();
            let p = |v| camera.project(v, r).unwrap().0;
            if kind == "vertices" {
                a.execute("vertices");
                for i in 0..4 {
                    let v = a.editor.scene.active().unwrap().mesh.vertices[i];
                    assert!(s.visible(&a.camera, v, r));
                    assert_eq!(
                        a.pick_component(p(v), r),
                        Some((1, Component::Vertex(i as u32)))
                    );
                }
            } else if kind == "polygon" {
                assert_eq!(
                    a.pick_component(p(DVec3::new(1., 0.5, 0.)), r),
                    Some((1, Component::Polygon(0)))
                );
            } else {
                assert!(s.corners(1, 0).is_empty());
                assert!(a.pick_component(p(DVec3::new(1., 0.5, 0.)), r).is_none());
                let m = &a.editor.scene.active().unwrap().mesh;
                for (u, v) in m.element_spans(0) {
                    let world = (m.vertices[u as usize] + m.vertices[v as usize]) * 0.5;
                    assert_eq!(
                        a.pick_component(p(world), r),
                        Some((1, Component::Polygon(0)))
                    );
                }
                let closing = (m.vertices[3] + m.vertices[0]) * 0.5;
                assert_eq!(a.pick_component(p(closing), r).is_some(), flag);
                a.execute("edges");
                assert_eq!(
                    a.pick_component(p(DVec3::new(1., -0.5, 0.)), r),
                    Some((1, Component::Edge(0, 1)))
                );
            }
        }
    }
}
#[test]
fn pen_s1_refusal_does_not_confirm_live_fields() {
    let mut a = Operandi::empty();
    a.execute("new mesh");
    a.execute("cube");
    a.fields.insert("size0".into(), "123 m".into());
    let before = a.editor.clone();
    a.execute(&pen_command("polygon", false));
    assert!(a.editor == before);
    assert!(a.status.contains("active"));
}
#[test]
fn pen_s1_perspective_span_pick_depth_matches_actual_projected_point() {
    let mut a = Operandi::empty();
    a.execute("new mesh");
    a.execute(&pen_command("polyline", false));
    a.camera.view = View::Perspective;
    a.camera.distance = 20.;
    let r = Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.));
    let surface = picking::Surface::new(&a.editor.scene);
    let m = &a.editor.scene.active().unwrap().mesh;
    for (u, v) in m.element_spans(0) {
        for t in [0.15, 0.35, 0.7, 0.9] {
            let world = m.vertices[u as usize].lerp(m.vertices[v as usize], t);
            let p = a.camera.project(world, r).unwrap().0;
            let (o, d) = a.camera.ray(p, r);
            let hit = surface.pick(&a.camera, p, r).unwrap();
            assert_eq!((hit.0, hit.1), (1, 0));
            assert!(
                (hit.2 - (world - o).dot(d)).abs() < 1e-5,
                "span {u}:{v}, t={t}, hit={}, expected={}",
                hit.2,
                (world - o).dot(d)
            );
        }
    }
}
#[test]
fn pen_rev01_parented_line_and_components_follow_world_transform() {
    let mut a = Operandi::empty();
    a.execute("new mesh");
    a.execute(&pen_command("polyline", false));
    a.editor.scene.new_mesh().unwrap();
    let parent = a.editor.scene.selection.item.unwrap();
    a.editor.scene.items[0].parent = Some(parent);
    a.editor.scene.items[1].transform.translation = DVec3::new(3., 2., 0.);
    a.editor.scene.selection.item = Some(1);
    a.camera.view = View::Front;
    a.camera.distance = 12.;
    let r = Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.));
    let p = |v| {
        a.camera
            .project(a.editor.scene.world_point(1, v), r)
            .unwrap()
            .0
    };
    let m = &a.editor.scene.items[0].mesh;
    let midpoint = p((m.vertices[0] + m.vertices[1]) * 0.5);
    let vertex = p(m.vertices[0]);
    assert_eq!(
        a.pick_component(midpoint, r),
        Some((1, Component::Polygon(0)))
    );
    a.editor.scene.selection.mode = Mode::Vertex;
    assert_eq!(a.pick_component(vertex, r), Some((1, Component::Vertex(0))));
    a.editor.scene.selection.mode = Mode::Edge;
    assert_eq!(
        a.pick_component(midpoint, r),
        Some((1, Component::Edge(0, 1)))
    );
}
#[test]
fn pen_rev02_line_visibility_uses_span_point_not_cursor_ray() {
    for view in [View::Front, View::Perspective] {
        let mut a = Operandi::empty();
        a.execute("new mesh");
        a.execute("pen {\"positions_m\":[[-2,0,0],[2,0,0],[3,1,0],[-2,1,0]],\"type\":\"polyline\",\"flip\":false,\"close\":false,\"select_new\":false}");
        a.editor.scene.new_mesh().unwrap();
        let mesh = &mut a.editor.scene.items[1].mesh;
        mesh.vertices = vec![
            DVec3::new(-0.3, -0.04, 1.),
            DVec3::new(0.3, -0.04, 1.),
            DVec3::new(0.3, 0.04, 1.),
            DVec3::new(-0.3, 0.04, 1.),
        ];
        mesh.faces.push(Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        a.camera.view = view;
        a.camera.yaw = 0.;
        a.camera.pitch = 0.;
        a.camera.distance = 10.;
        let r = Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.));
        let surface = picking::Surface::new(&a.editor.scene);
        let line = a.camera.project(DVec3::ZERO, r).unwrap().0;
        assert!(
            surface.pick(&a.camera, line + vec2(0., 8.), r).is_none(),
            "hidden span picked in {view:?}"
        );
        let visible = a.camera.project(DVec3::new(1., 0., 0.), r).unwrap().0;
        assert_eq!(surface.pick(&a.camera, visible, r).map(|h| h.0), Some(1));
        // The span is visible, but the established cursor-face depth priority
        // gives the foreground face under the cursor the hit.
        a.editor.scene.items[1].transform.translation.y = -0.11;
        let surface = picking::Surface::new(&a.editor.scene);
        assert!(surface.visible(&a.camera, DVec3::ZERO, r));
        assert_eq!(
            surface.pick(&a.camera, line + vec2(0., 8.), r).map(|h| h.0),
            Some(2),
            "foreground cursor face in {view:?}"
        );
        // A parent transform must place an occluder at its drawn position.
        a.editor.scene.new_mesh().unwrap();
        let parent = a.editor.scene.selection.item.unwrap();
        a.editor.scene.items[1].parent = Some(parent);
        a.editor.scene.items[2].transform.translation.y = 0.11;
        let surface = picking::Surface::new(&a.editor.scene);
        assert!(!surface.visible(&a.camera, DVec3::ZERO, r));
    }
}
#[test]
fn pen_rev04_readme_examples_parse_and_dispatch() {
    let examples = [
        r#"pen {"positions_m":[[-2,-1,0],[4,0,0],[3,2,0],[-1,1,0]],"type":"polygon","flip":false,"close":false,"select_new":true}"#,
        r#"pen {"positions_m":[[-2,-1,0],[4,0,0],[3,2,0],[-1,1,0]],"type":"vertices","flip":false,"close":false,"select_new":true}"#,
        r#"pen {"positions_m":[[-2,-1,0],[4,0,0],[3,2,0],[-1,1,0]],"type":"polyline","flip":false,"close":true,"select_new":true}"#,
    ];
    for example in examples {
        PenInput::parse(example.strip_prefix("pen ").unwrap()).unwrap();
        let mut a = Operandi::empty();
        a.execute("new mesh");
        a.execute(example);
        assert_eq!(a.editor.scene.active().unwrap().mesh.vertices.len(), 4);
    }
}
#[test]
fn pen_rev05_edge_visibility_uses_perspective_correct_span_point() {
    for view in [View::Front, View::Perspective] {
        let mut a = Operandi::empty();
        // The occluder is another mesh: it only hides the active mesh while
        // picking sees every mesh.
        a.selection_preferences.pick_other_meshes = true;
        a.execute("new mesh");
        a.execute("pen {\"positions_m\":[[-2,0,0],[2,0,0],[3,2,0],[-2,2,0]],\"type\":\"polyline\",\"flip\":false,\"close\":false,\"select_new\":false}");
        a.editor.scene.new_mesh().unwrap();
        let parent = a.editor.scene.selection.item.unwrap();
        a.editor.scene.items[0].parent = Some(parent);
        a.editor.scene.items[1].transform.rotation.y = 0.7;
        a.editor.scene.items[1].transform.translation.x = 0.25;
        a.editor.scene.new_mesh().unwrap();
        let occ = &mut a.editor.scene.items[2].mesh;
        occ.vertices = vec![
            DVec3::new(-0.05, -0.05, 0.),
            DVec3::new(0.05, -0.05, 0.),
            DVec3::new(0.05, 0.05, 0.),
            DVec3::new(-0.05, 0.05, 0.),
        ];
        occ.faces.push(Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        a.editor.scene.selection.item = Some(1);
        a.editor.scene.selection.mode = Mode::Edge;
        a.camera.view = view;
        a.camera.yaw = 0.;
        a.camera.pitch = 0.;
        a.camera.distance = 6.;
        let r = Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.));
        let v = &a.editor.scene.items[0].mesh.vertices;
        let wa = a.editor.scene.world_point(1, v[0]);
        let wb = a.editor.scene.world_point(1, v[1]);
        let qa = a.camera.project(wa, r).unwrap().0;
        let qb = a.camera.project(wb, r).unwrap().0;
        let cursor = qa.lerp(qb, 0.5);
        let (o, d) = a.camera.ray(cursor, r);
        let plane_z = 2.;
        let hit = o + d * ((plane_z - o.z) / d.z);
        a.editor.scene.items[2].transform.translation = hit;
        assert_ne!(
            a.pick_component(cursor, r),
            Some((1, Component::Edge(0, 1))),
            "hidden edge in {view:?}"
        );
        // Move the occluder to the incorrect world-midpoint ray. In perspective
        // the true screen midpoint remains exposed; in orthographic use a distant control.
        let wrong = a.camera.project(wa.lerp(wb, 0.5), r).unwrap().0;
        let displaced = if view == View::Perspective {
            wrong
        } else {
            cursor + vec2(80., 0.)
        };
        let (o, d) = a.camera.ray(displaced, r);
        a.editor.scene.items[2].transform.translation = o + d * ((plane_z - o.z) / d.z);
        assert_eq!(
            a.pick_component(cursor, r),
            Some((1, Component::Edge(0, 1))),
            "visible edge in {view:?}"
        );
    }
}
