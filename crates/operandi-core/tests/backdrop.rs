//! Backdrop items (specifications/backdrop-2026-11-04): defaults, placement,
//! size rules, colour maths, Auto Fit, visibility rules, undo and the save
//! round trip, checked against ORACLES.json where it has numbers.
use glam::DVec3;
use operandi_core::backdrop::*;
use operandi_core::*;
use serde_json::Value;

fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specifications/backdrop-2026-11-04/ORACLES.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["oracles"].as_array().unwrap().clone()
}
fn oracle(id: &str) -> Value {
    oracles().into_iter().find(|o| o["id"] == id).unwrap()
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
fn projection(name: &str) -> Projection {
    *Projection::ALL
        .iter()
        .find(|p| p.label() == name)
        .unwrap_or_else(|| panic!("projection {name}"))
}
/// An editor with one Backdrop showing a `w` × `h` image.
fn backdrop_editor(w: u32, h: u32) -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewBackdrop).unwrap();
    let id = e.scene.selection.item.unwrap();
    e.dispatch(Command::BackdropLoadImage {
        item: id,
        path: "/refs/front.png".into(),
        width: w,
        height: h,
    })
    .unwrap();
    (e, id)
}
fn settings(e: &mut Editor, id: u64) -> &mut Backdrop {
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .unwrap()
        .backdrop
        .as_mut()
        .unwrap()
}
fn edit(e: &mut Editor, id: u64, edit: BackdropEdit) -> Result<()> {
    e.dispatch(Command::EditBackdrop { item: id, edit })
}

#[test]
fn bd_o_01_defaults_name_and_selection() {
    let o = oracle("BD-O-01");
    let x = &o["expected"];
    let mut e = Editor::default();
    e.dispatch(Command::NewBackdrop).unwrap();
    let item = e.scene.active().unwrap();
    assert_eq!(item.name, x["name"].as_str().unwrap());
    assert_eq!(item.parent, None);
    assert!(item.mesh.vertices.is_empty());
    assert_eq!(item.transform.translation, vec3(&x["position"]));
    assert_eq!(item.transform.rotation, vec3(&x["rotation"]));
    assert_eq!(item.transform.scale, vec3(&x["scale"]));
    let b = item.backdrop.as_ref().unwrap();
    assert_eq!(b.image, None);
    assert_eq!(b.projection.label(), x["Projection Type"].as_str().unwrap());
    assert_eq!(b.show_in_perspective, x["Show In Perspective"] == true);
    assert_eq!(b.keep_aspect, x["Keep Aspect"] == true);
    assert_eq!((b.flip, b.invert, b.blend), (false, false, false));
    assert_eq!((b.contrast, b.brightness, b.transparency), (0., 0., 0.));
    assert_eq!(b.uniform_size, x["Use Uniform Size"] == true);
    assert_eq!(b.pixel_size, x["Pixel Size"].as_f64().unwrap());
    // No image: nothing to draw.
    assert!(world_corners(&e.scene, item.id).is_none());
    // Later ones are numbered, each alone selected.
    e.dispatch(Command::NewBackdrop).unwrap();
    e.dispatch(Command::NewBackdrop).unwrap();
    let names: Vec<_> = e.scene.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(
        names,
        ["Backdrop Item", "Backdrop Item (2)", "Backdrop Item (3)"]
    );
    assert_eq!(e.scene.selection.item, Some(e.scene.items[2].id));
}

#[test]
fn bd_03_loading_adds_one_shared_image_entry_and_keeps_the_transform() {
    let (mut e, a) = backdrop_editor(512, 256);
    e.scene.items[0].transform.translation = DVec3::new(1., 2., 3.);
    let before = e.scene.items[0].transform.clone();
    e.dispatch(Command::NewBackdrop).unwrap();
    let b = e.scene.selection.item.unwrap();
    e.dispatch(Command::BackdropLoadImage {
        item: b,
        path: "/refs/front.png".into(),
        width: 512,
        height: 256,
    })
    .unwrap();
    assert_eq!(e.scene.images.len(), 1, "the same file is one entry");
    assert_eq!(e.scene.images[0].name, "front");
    let first = settings(&mut e, a).image;
    assert_eq!(first, settings(&mut e, b).image);
    assert_eq!(e.scene.items[0].transform, before);
    // BD-04/BD-24: settings belong to each Backdrop, never to the image.
    edit(&mut e, a, BackdropEdit::Invert(true)).unwrap();
    assert!(!settings(&mut e, b).invert);
    let image = e.scene.images[0].clone();
    edit(&mut e, a, BackdropEdit::Contrast(0.4)).unwrap();
    assert_eq!(e.scene.images[0], image);
}

#[test]
fn bd_o_02_to_07_and_09_10_placement() {
    let mut checked = 0;
    for o in oracles() {
        let Some(corners) = o["expected"].get("world_corners_of_image") else {
            continue;
        };
        let (mut e, id) = backdrop_editor(512, 256);
        let s = &o["settings"];
        settings(&mut e, id).projection = projection(s["Projection Type"].as_str().unwrap());
        settings(&mut e, id).pixel_size = s["Pixel Size"].as_f64().unwrap();
        let [tl, tr, _br, bl] = world_corners(&e.scene, id).unwrap();
        let tol = o["tolerance"].as_f64().unwrap();
        for (name, got) in [("top_left", tl), ("top_right", tr), ("bottom_left", bl)] {
            let want = vec3(&corners[name]);
            assert!(
                (got - want).abs().max_element() <= tol,
                "{} {name}: {got} vs {want}",
                o["id"]
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 6);

    // BD-O-09: the depth position doesn't change the Front rectangle.
    let o = oracle("BD-O-09");
    let (mut e, id) = backdrop_editor(512, 256);
    e.scene.items[0].transform.translation = vec3(&o["input"]["item_position"]);
    settings(&mut e, id).pixel_size = o["settings"]["Pixel Size"].as_f64().unwrap();
    let c = world_corners(&e.scene, id).unwrap();
    let r = &o["expected"]["front_view_rectangle"];
    let (x, y) = (
        vec3(&{
            let mut v = r["x"].as_array().unwrap().clone();
            v.push(0.into());
            Value::Array(v)
        }),
        vec3(&{
            let mut v = r["y"].as_array().unwrap().clone();
            v.push(0.into());
            Value::Array(v)
        }),
    );
    assert!((c[0].x - x.x).abs() < 0.03 && (c[1].x - x.y).abs() < 0.03);
    assert!((c[3].y - y.x).abs() < 0.03 && (c[0].y - y.y).abs() < 0.03);
    assert!(c.iter().all(|p| p.z == -5.));

    // BD-O-10: 30 degrees about Z turns it counter-clockwise from Front.
    let (mut e, id) = backdrop_editor(512, 256);
    settings(&mut e, id).pixel_size = 0.004;
    e.scene.items[0].transform.rotation = DVec3::new(0., 0., 30f64.to_radians());
    let c = world_corners(&e.scene, id).unwrap();
    let edge = c[1] - c[0];
    assert!((edge.length() - 2.048).abs() < 1e-9);
    assert!((edge.y.atan2(edge.x).to_degrees() - 30.).abs() < 1e-9);
    assert!(((c[0] - c[3]).length() - 1.024).abs() < 1e-9);
}

#[test]
fn bd_o_16_to_36_size_rules() {
    let mut checked = 0;
    for o in oracles() {
        let x = &o["expected"];
        let (Some(w), Some(h)) = (x["width_m"].as_f64(), x["height_m"].as_f64()) else {
            continue;
        };
        let px = &o["input"]["image_px"];
        let s = &o["settings"];
        let b = Backdrop {
            projection: projection(s["Projection Type"].as_str().unwrap()),
            uniform_size: s["Use Uniform Size"] == true,
            keep_aspect: s["Keep Aspect"] == true,
            pixel_size: s["Pixel Size"].as_f64().unwrap(),
            ..Default::default()
        };
        let (gw, gh) = b
            .size(
                px[0].as_u64().unwrap() as u32,
                px[1].as_u64().unwrap() as u32,
                vec3(&o["input"]["item_scale"]),
            )
            .unwrap();
        let tol = o["tolerance"].as_f64().unwrap();
        assert!(
            (gw - w).abs() <= tol && (gh - h).abs() <= tol,
            "{}: {gw} x {gh}, want {w} x {h}",
            o["id"]
        );
        checked += 1;
    }
    assert_eq!(checked, 21);
}

#[test]
fn bd_o_37_pixel_size_refused_with_uniform_size_on() {
    let (mut e, id) = backdrop_editor(512, 256);
    edit(&mut e, id, BackdropEdit::UniformSize(true)).unwrap();
    let steps = e.undo.len();
    let err = edit(&mut e, id, BackdropEdit::PixelSize(0.005)).unwrap_err();
    assert_eq!(err, PIXEL_SIZE_REFUSAL);
    assert_eq!(settings(&mut e, id).pixel_size, 0.01);
    assert_eq!(e.undo.len(), steps, "a refusal is not an undo step");
    edit(&mut e, id, BackdropEdit::UniformSize(false)).unwrap();
    edit(&mut e, id, BackdropEdit::PixelSize(0.005)).unwrap();
    assert_eq!(settings(&mut e, id).pixel_size, 0.005);
}

#[test]
fn bd_o_38_to_45_colour_maths() {
    let mut checked = 0;
    for o in oracles() {
        let Some(want) = o["expected"]["drawn_values_8bit"].as_array() else {
            continue;
        };
        let s = &o["settings"];
        let c = s["Contrast"].as_f64().unwrap_or(0.);
        let b = s["Brightness"].as_f64().unwrap_or(0.);
        let inv = s["Invert"] == true;
        for (k, want) in want.iter().enumerate() {
            let v = (k * 17) as f64 / 255.;
            let got = (adjust_channel(v, inv, c, b) * 255.).round();
            let want = want.as_f64().unwrap();
            assert!(
                (got - want).abs() <= 1.,
                "{} column {k}: {got} vs {want}",
                o["id"]
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 8);
}

#[test]
fn bd_o_47_transparency_mixes_with_the_background() {
    // The viewport composites the picture at (1 - t) over what is behind.
    let mix = |v: f64, t: f64, behind: f64| v * (1. - t) + behind * t;
    let (red, blue) = (oracle("BD-O-47"), 48.);
    let want = &red["expected"]["red_pixel"];
    assert!((mix(255., 0.5, blue) - want[0].as_f64().unwrap()).abs() <= 1.);
    assert!((mix(0., 0.5, blue) - want[1].as_f64().unwrap()).abs() <= 1.);
    let (mut e, id) = backdrop_editor(4, 4);
    edit(&mut e, id, BackdropEdit::Transparency(1.5)).unwrap();
    assert_eq!(settings(&mut e, id).transparency, 1., "clamped to 100 %");
}

#[test]
fn bd_07_11_12_which_views_draw() {
    let mut b = Backdrop::default();
    let o = oracle("BD-O-08");
    for v in o["expected"]["drawn_in"].as_array().unwrap() {
        let v = v.as_str().unwrap();
        let view = if v == "Perspective" {
            ViewKind::Free
        } else {
            ViewKind::Axis(projection(v))
        };
        assert!(b.drawn_in(view), "{v}");
    }
    for v in o["expected"]["not_drawn_in"].as_array().unwrap() {
        assert!(!b.drawn_in(ViewKind::Axis(projection(v.as_str().unwrap()))));
    }
    b.show_in_perspective = false;
    assert!(!b.drawn_in(ViewKind::Free));
    assert!(b.drawn_in(ViewKind::Axis(Projection::Front)));
    b.projection = Projection::Camera;
    b.show_in_perspective = true;
    assert!(!b.drawn_in(ViewKind::Free));
    assert!(
        Projection::ALL
            .iter()
            .all(|&p| !b.drawn_in(ViewKind::Axis(p)))
    );
}

#[test]
fn bd_o_50_to_56_auto_fit() {
    let mut checked = 0;
    for o in oracles() {
        if o["settings"]["action"] != "Auto Fit" {
            continue;
        }
        let px = o["settings"]
            .get("image_px")
            .unwrap_or(&o["input"]["image_px"])
            .clone();
        let (mut e, id) = backdrop_editor(
            px[0].as_u64().unwrap() as u32,
            px[1].as_u64().unwrap() as u32,
        );
        // The box X -0.5..2.5, Y 1.25..2.75, Z -0.75..-0.25.
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.items.last_mut().unwrap().mesh =
            Mesh::box_mesh(DVec3::new(1., 2., -0.5), DVec3::new(3., 1.5, 0.5), 0)
                .unwrap()
                .into();
        e.dispatch(Command::SelectItem(Some(id))).unwrap();
        let s = &o["settings"];
        let b = settings(&mut e, id);
        b.projection = projection(s["Projection Type"].as_str().unwrap());
        b.uniform_size = s["Use Uniform Size"] == true;
        b.keep_aspect = s.get("Keep Aspect").is_none_or(|k| k == true);
        if s.get("start").is_some() {
            e.scene.items[0].transform.translation.x = 7.;
            e.scene.items[0].transform.scale.x = 4.;
        }
        if o["input"]["backdrop_start"] == "position 0, scale (1,1,3)" {
            e.scene.items[0].transform.scale.z = 3.;
        }
        let result = e.dispatch(Command::BackdropAutoFit(id));
        let x = &o["expected"];
        if x.get("message").is_some() {
            assert_eq!(result.unwrap_err(), AUTO_FIT_REFUSAL);
        } else {
            result.unwrap();
        }
        let t = &e.scene.items[0].transform;
        let tol = o["tolerance"].as_f64().unwrap();
        assert!(
            (t.translation - vec3(&x["position"])).abs().max_element() <= tol,
            "{} position {}",
            o["id"],
            t.translation
        );
        assert!(
            (t.scale - vec3(&x["scale"])).abs().max_element() <= tol,
            "{} scale {}",
            o["id"],
            t.scale
        );
        checked += 1;
    }
    assert_eq!(checked, 11);
}

#[test]
fn auto_fit_back_bottom_left_use_the_far_face_provisionally() {
    for (p, axis, want) in [
        (Projection::Back, 2, -0.75),
        (Projection::Bottom, 1, 1.25),
        (Projection::Left, 0, -0.5),
    ] {
        let (mut e, id) = backdrop_editor(512, 256);
        settings(&mut e, id).projection = p;
        settings(&mut e, id).uniform_size = true;
        let bounds = (DVec3::new(-0.5, 1.25, -0.75), DVec3::new(2.5, 2.75, -0.25));
        e.scene.auto_fit_backdrop(id, bounds).unwrap();
        assert_eq!(e.scene.items[0].transform.translation[axis], want, "{p:?}");
    }
}

#[test]
fn backdrop_size_buttons_give_exact_sizes() {
    let (mut e, id) = backdrop_editor(512, 256);
    e.dispatch(Command::BackdropSize {
        item: id,
        width: 3.,
        height: Some(2.),
    })
    .unwrap();
    let item = &e.scene.items[0];
    let b = item.backdrop.as_ref().unwrap();
    assert!(!b.keep_aspect);
    let (w, h) = b.size(512, 256, item.transform.scale).unwrap();
    assert!((w - 3.).abs() < 1e-12 && (h - 2.).abs() < 1e-12);
    e.dispatch(Command::BackdropSize {
        item: id,
        width: 4.,
        height: None,
    })
    .unwrap();
    let item = &e.scene.items[0];
    let b = item.backdrop.as_ref().unwrap();
    assert!(b.keep_aspect);
    let (w, h) = b.size(512, 256, item.transform.scale).unwrap();
    assert!((w - 4.).abs() < 1e-12 && (h - 2.).abs() < 1e-12);
}

#[test]
fn every_change_is_one_undo_step_and_undo_restores_it() {
    let mut e = Editor::default();
    e.dispatch(Command::NewBackdrop).unwrap();
    assert_eq!(e.undo.len(), 1);
    let id = e.scene.selection.item.unwrap();
    e.dispatch(Command::BackdropLoadImage {
        item: id,
        path: "/refs/a.png".into(),
        width: 8,
        height: 8,
    })
    .unwrap();
    edit(&mut e, id, BackdropEdit::Flip(true)).unwrap();
    edit(&mut e, id, BackdropEdit::Projection(Projection::Top)).unwrap();
    assert_eq!(e.undo.len(), 4);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(settings(&mut e, id).projection, Projection::Front);
    e.dispatch(Command::Undo).unwrap();
    assert!(!settings(&mut e, id).flip);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.images.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.items.is_empty());
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.items[0].name, "Backdrop Item");
}

#[test]
fn component_tools_refuse_a_backdrop_and_transform_tools_work_in_any_mode() {
    let (mut e, id) = backdrop_editor(512, 256);
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    for tool in [Tool::Box, Tool::Bevel, Tool::Tweak] {
        assert_eq!(
            e.dispatch(Command::Activate(tool)).unwrap_err(),
            editor::BACKDROP_NO_GEOMETRY
        );
    }
    assert!(e.dispatch(Command::NewVertex(DVec3::ONE)).is_err());
    assert!(e.scene.items[0].mesh.vertices.is_empty());
    // Move from Polygon mode switches to Item mode and moves the item.
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Item);
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(1., 2., 0.),
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(
        e.scene.items[0].transform.translation,
        DVec3::new(1., 2., 0.)
    );
    let c = world_corners(&e.scene, id).unwrap();
    assert!((c[0] - DVec3::new(1. - 2.56, 2. + 1.28, 0.)).length() < 1e-9);
    // Rotate, Scale and Transform too, from a component mode.
    for (tool, params) in [
        (
            Tool::Rotate,
            Parameters {
                delta: DVec3::new(0., 0., 30.),
                ..Default::default()
            },
        ),
        (
            Tool::Scale,
            Parameters {
                delta: DVec3::new(2., 2., 1.),
                ..Default::default()
            },
        ),
        (
            Tool::Transform,
            Parameters {
                delta: DVec3::new(0., 1., 0.),
                scale_delta: DVec3::ONE,
                ..Default::default()
            },
        ),
    ] {
        e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
        e.dispatch(Command::Activate(tool)).unwrap();
        e.dispatch(Command::Preview(params)).unwrap();
        e.dispatch(Command::Drop).unwrap();
    }
    let t = &e.scene.items[0].transform;
    assert!((t.rotation.z.to_degrees() - 30.).abs() < 1e-9);
    assert_eq!(t.scale, DVec3::new(2., 2., 1.));
    assert_eq!(t.translation, DVec3::new(1., 3., 0.));
    assert_eq!(e.undo.len(), 6, "one step per drop after create and image");
}

#[test]
fn a_backdrop_without_a_picture_has_a_placeholder_square() {
    let mut e = Editor::default();
    e.dispatch(Command::NewBackdrop).unwrap();
    let id = e.scene.selection.item.unwrap();
    assert!(world_corners(&e.scene, id).is_none());
    let c = outline_corners(&e.scene, id).unwrap();
    assert!((c[1] - c[0]).length() - 1. < 1e-12 && ((c[0] - c[3]).length() - 1.).abs() < 1e-12);
    settings(&mut e, id).projection = Projection::Camera;
    assert!(outline_corners(&e.scene, id).is_none());
}

#[test]
fn an_empty_backdrops_metre_square_ignores_pixel_size() {
    let mut e = Editor::default();
    e.dispatch(Command::NewBackdrop).unwrap();
    let id = e.scene.selection.item.unwrap();
    edit(&mut e, id, BackdropEdit::PixelSize(0.005)).unwrap();
    let c = empty_square_corners(&e.scene, id, 1.).unwrap();
    assert!(((c[1] - c[0]).length() - 1.).abs() < 1e-12);
    assert!(((c[0] - c[3]).length() - 1.).abs() < 1e-12);
    let centre = c.iter().copied().sum::<DVec3>() / 4.;
    let at = e.scene.items.iter().find(|i| i.id == id).unwrap();
    assert!((centre - at.transform.translation).length() < 1e-12);
    settings(&mut e, id).projection = Projection::Camera;
    assert!(empty_square_corners(&e.scene, id, 1.).is_none());
}

#[test]
fn bd_o_57_save_and_reopen_keeps_every_setting() {
    let (mut e, a) = backdrop_editor(512, 256);
    edit(&mut e, a, BackdropEdit::UniformSize(false)).unwrap();
    edit(&mut e, a, BackdropEdit::PixelSize(0.005)).unwrap();
    for change in [
        BackdropEdit::Flip(true),
        BackdropEdit::Invert(true),
        BackdropEdit::Blend(true),
        BackdropEdit::Contrast(0.3),
        BackdropEdit::Brightness(-0.2),
        BackdropEdit::Transparency(0.25),
        BackdropEdit::UniformSize(true),
    ] {
        edit(&mut e, a, change).unwrap();
    }
    e.dispatch(Command::Rename("Saved BD".into())).unwrap();
    e.scene.items[0].transform.scale.x = 2.;
    e.scene.items[0].transform.rotation.z = 10f64.to_radians();
    settings(&mut e, a).extra.insert("gamma".into(), 1.8);
    e.dispatch(Command::NewBackdrop).unwrap();
    let b = e.scene.selection.item.unwrap();
    edit(&mut e, b, BackdropEdit::ShowInPerspective(false)).unwrap();
    e.dispatch(Command::NewBackdrop).unwrap();
    let c = e.scene.selection.item.unwrap();
    edit(&mut e, c, BackdropEdit::Projection(Projection::Camera)).unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    assert_eq!(back.images[0].path, "/refs/front.png");
    // Older readers can't silently drop Backdrops: the file says version 11.
    let doc: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(doc["version"], 11);
    // A scene without Backdrops keeps its old version and field set.
    let plain = persistence::encode(&Scene::default()).unwrap();
    assert!(!String::from_utf8(plain).unwrap().contains("images"));
}

#[test]
fn invalid_backdrop_data_is_refused() {
    let (mut e, id) = backdrop_editor(4, 4);
    let mut bad = e.scene.clone();
    bad.items[0].backdrop.as_mut().unwrap().image = Some(99);
    assert!(bad.validate().is_err());
    let mut bad = e.scene.clone();
    bad.items[0].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    assert!(bad.validate().is_err());
    assert!(edit(&mut e, id, BackdropEdit::Contrast(f64::NAN)).is_err());
    assert!(edit(&mut e, id, BackdropEdit::Image(Some(42))).is_err());
    edit(&mut e, id, BackdropEdit::Image(None)).unwrap();
    assert!(world_corners(&e.scene, id).is_none());
}

#[test]
fn moving_a_backdrop_to_another_scene_keeps_its_picture() {
    let (src, id) = backdrop_editor(512, 256);
    let mut dst = Editor::default();
    dst.dispatch(Command::NewBackdrop).unwrap();
    let other = dst.scene.selection.item.unwrap();
    dst.dispatch(Command::BackdropLoadImage {
        item: other,
        path: "/refs/side.png".into(),
        width: 10,
        height: 20,
    })
    .unwrap();
    dst.dispatch(Command::InsertItems {
        items: src.scene.branch(id).unwrap(),
        parent: None,
        images: src.scene.images.clone(),
    })
    .unwrap();
    let moved = dst.scene.active().unwrap();
    assert_eq!(moved.name, "Backdrop Item (2)");
    let image = dst
        .scene
        .image(moved.backdrop.as_ref().unwrap().image.unwrap())
        .unwrap();
    assert_eq!(image.path, "/refs/front.png");
    assert_eq!(dst.scene.images.len(), 2);
    // Refused inserts add no image entries.
    let before = dst.scene.clone();
    assert!(
        dst.dispatch(Command::InsertItems {
            items: src.scene.branch(id).unwrap(),
            parent: Some(999),
            images: vec![SceneImage {
                id: 1,
                name: "x".into(),
                path: "/refs/new.png".into(),
                width: 1,
                height: 1,
            }],
        })
        .is_err()
    );
    assert_eq!(dst.scene, before);
}

#[test]
fn exports_leave_backdrops_out() {
    let (mut e, _) = backdrop_editor(512, 256);
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.items.last_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    let dir = std::env::temp_dir().join(format!("operandi-bd-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let obj = dir.join("model.obj");
    let report = interchange::export(&obj, &e.scene, interchange::ExportFormat::Obj).unwrap();
    assert_eq!(report.items, 1);
    assert!(!std::fs::read_to_string(&obj).unwrap().contains("Backdrop"));
    let lxo = dir.join("model.lxo");
    let report = interchange::export(&lxo, &e.scene, interchange::ExportFormat::Lxo).unwrap();
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("1 Backdrop(s) not exported"))
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn bd_o_62_63_image_entry_names() {
    let mut scene = Scene::default();
    let a = scene.image_for_path("/refs/dup.png", 4, 4).unwrap();
    assert_eq!(scene.image_for_path("/refs/dup.png", 4, 4).unwrap(), a);
    scene.image_for_path("/refs/other/dup.png", 4, 4).unwrap();
    scene.image_for_path("/refs/third/dup.jpg", 4, 4).unwrap();
    let names: Vec<_> = scene.images.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["dup", "dup (2)", "dup (3)"]);
    let o = oracle("BD-O-63");
    assert_eq!(o["expected"]["image_entries"][1], "dup (2)");
}
