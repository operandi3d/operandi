//! The icon studio scene and how icon scenes render through it
//! (`icon_studio.rs`).
use glam::DVec3;
use operandi_core::camera_light::Projection;
use operandi_core::icon_studio::{self, MATERIALS, Sample};
use operandi_core::shader_tree::{NodeKind, TagMask};

fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("icon-studio-{}-{name}", std::process::id()))
}

#[test]
fn the_studio_has_an_isometric_orthographic_camera_a_key_light_and_a_sky() {
    let studio = icon_studio::studio_scene().unwrap();
    let camera_id = studio.render_camera().expect("a render camera");
    let camera = studio.item(camera_id).unwrap().camera.clone().unwrap();
    assert_eq!(camera.projection, Projection::Orthographic);
    let (half_w, half_h) = camera.orthographic_half_size(1.);
    assert!((half_w - icon_studio::HALF_SIZE).abs() < 1e-9, "{half_w}");
    assert!((half_h - icon_studio::HALF_SIZE).abs() < 1e-9, "{half_h}");
    // Looking down the (1, 1, 1) diagonal at the origin, +Y up on screen.
    let frame = operandi_core::camera_light::item_frame(&studio, camera_id).unwrap();
    let diagonal = -DVec3::ONE.normalize();
    assert!(
        frame.forward.dot(diagonal) > 0.999_999,
        "{:?}",
        frame.forward
    );
    assert!(frame.up.y > 0.8, "{:?}", frame.up);
    // +X goes right, +Z goes left, as in the generator's projection.
    assert!(frame.right.dot(DVec3::X) > 0.5 && frame.right.dot(DVec3::Z) < -0.5);

    assert_eq!(studio.items.iter().filter(|i| i.light.is_some()).count(), 1);
    assert_eq!(studio.environments().count(), 1);
    assert!(
        studio.items.iter().all(|i| i.mesh.faces.is_empty()),
        "the studio itself has no geometry"
    );
    assert_eq!(studio.render.frame, [32, 32]);
    assert!(!studio.render.denoise);
}

#[test]
fn every_shared_material_is_picked_by_its_polygon_tag() {
    let studio = icon_studio::studio_scene().unwrap();
    for look in &MATERIALS[1..] {
        let group = studio
            .shader_tree
            .nodes
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Group(g) if g.tag == TagMask::Name(look.tag.into())))
            .unwrap_or_else(|| panic!("no group for {}", look.tag));
        let material = studio
            .shader_tree
            .nodes
            .iter()
            .find(|n| n.parent == Some(group.id) && matches!(n.kind, NodeKind::Material(_)))
            .unwrap_or_else(|| panic!("no material in {}", look.tag));
        let NodeKind::Material(m) = &material.kind else {
            unreachable!()
        };
        assert_eq!(m.diffuse_color, icon_studio::linear_from_hex(look.hex));
    }
    // Selected is the generator's yellow-orange #FFD470, as linear.
    let [r, g, b] = icon_studio::linear_from_hex(0xFFD470);
    assert!((r - 1.).abs() < 1e-9 && (g - 0.658).abs() < 0.01 && (b - 0.162).abs() < 0.01);
}

#[test]
fn samples_fit_inside_the_icon_box_and_use_studio_tags() {
    for sample in Sample::ALL {
        let scene = icon_studio::sample_scene(sample).unwrap();
        assert!(icon_studio::unknown_tags(&scene).is_empty());
        let item = scene
            .items
            .iter()
            .find(|i| !i.mesh.faces.is_empty())
            .unwrap();
        assert_eq!(item.name, sample.slug());
        // Inside the picture, with room for the outline: no point lands
        // further from the centre than the cube's own corners do.
        let camera = scene.render_camera().unwrap();
        let frame = operandi_core::camera_light::item_frame(&scene, camera).unwrap();
        let reach = item
            .mesh
            .vertices
            .iter()
            .map(|v| v.dot(frame.right).abs().max(v.dot(frame.up).abs()))
            .fold(0., f64::max);
        // Two 32 px pixels short of the picture's edge, for the outline.
        let limit = icon_studio::picture_reach();
        assert!(reach <= limit, "{} reaches {reach}", sample.slug());
        assert_eq!(Sample::named(sample.slug()), Some(sample));
    }
    let selected = icon_studio::sample_scene(Sample::SelectedFace).unwrap();
    let mesh = &selected.items.last().unwrap().mesh;
    let tagged: Vec<_> = mesh
        .faces
        .iter()
        .filter(|f| f.material == "Selected")
        .collect();
    assert_eq!(tagged.len(), 1, "only the top face is selected");
    assert!(
        tagged[0]
            .vertices
            .iter()
            .all(|&v| mesh.vertices[v as usize].y > 0.)
    );
}

#[test]
fn the_edge_bevel_sample_is_a_closed_cube_with_one_new_face_facing_out() {
    let scene = icon_studio::sample_scene(Sample::EdgeBevel).unwrap();
    let mesh = &scene.items.last().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 7);
    let cut: Vec<_> = mesh.faces.iter().filter(|f| f.material == "New").collect();
    assert_eq!(cut.len(), 1);
    // Every polygon faces away from the centre (Newell normal · centroid).
    for face in &mesh.faces {
        let p: Vec<DVec3> = face
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let normal = (0..p.len()).fold(DVec3::ZERO, |n, i| n + p[i].cross(p[(i + 1) % p.len()]));
        let centre = p.iter().copied().sum::<DVec3>() / p.len() as f64;
        assert!(normal.dot(centre) > 0., "{face:?} faces inward");
    }
    // The cut faces up and toward +Z.
    let p: Vec<DVec3> = cut[0]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect();
    let centre = p.iter().copied().sum::<DVec3>() / 4.;
    assert!(centre.y > 0.2 && centre.z > 0.2);
}

#[test]
fn the_studio_wins_over_the_icon_scenes_own_camera_lights_and_materials() {
    let studio = icon_studio::studio_scene().unwrap();
    let mut icon = icon_studio::sample_scene(Sample::Cube).unwrap();
    // The icon's copy of the studio drifted: another light, a moved camera.
    icon.add_light(Default::default(), Default::default())
        .unwrap();
    let cam = icon.render_camera().unwrap();
    icon.items
        .iter_mut()
        .find(|i| i.id == cam)
        .unwrap()
        .transform
        .translation = DVec3::new(0., 0., 5.);
    let render = icon_studio::render_scene(&studio, &icon).unwrap();
    assert_eq!(render.items.iter().filter(|i| i.light.is_some()).count(), 1);
    assert_eq!(render.render_camera(), studio.render_camera());
    let camera = render.item(render.render_camera().unwrap()).unwrap();
    assert_eq!(camera.transform, studio.item(cam).unwrap().transform);
    assert_eq!(render.shader_tree, studio.shader_tree);
    // The cube came across with a fresh id, its polygons and tags intact.
    let cube = render
        .items
        .iter()
        .find(|i| !i.mesh.faces.is_empty())
        .unwrap();
    assert!(studio.items.iter().all(|i| i.id != cube.id));
    assert!(cube.id < render.next_id);
    assert_eq!(cube.mesh.faces.len(), 6);
}

#[test]
fn parents_follow_their_items_and_an_empty_icon_is_an_error() {
    let studio = icon_studio::studio_scene().unwrap();
    let mut icon = icon_studio::sample_scene(Sample::Cube).unwrap();
    icon.new_mesh_named(Some("Child".into())).unwrap();
    let parent = icon
        .items
        .iter()
        .find(|i| i.name == "prim-cube")
        .unwrap()
        .id;
    let mut child_mesh =
        operandi_core::primitive::cube_geometry(DVec3::ZERO, DVec3::splat(0.2)).unwrap();
    child_mesh
        .faces
        .iter_mut()
        .for_each(|f| f.material = "New".into());
    let child = icon.items.last_mut().unwrap();
    child.parent = Some(parent);
    child.mesh = child_mesh.into();
    let render = icon_studio::render_scene(&studio, &icon).unwrap();
    let cube = render.items.iter().find(|i| i.name == "prim-cube").unwrap();
    let child = render.items.iter().find(|i| i.name == "Child").unwrap();
    assert_eq!(child.parent, Some(cube.id));

    let empty = icon_studio::studio_scene().unwrap();
    assert!(icon_studio::render_scene(&studio, &empty).is_err());
}

#[test]
fn unknown_tags_are_reported() {
    let mut icon = icon_studio::sample_scene(Sample::Cube).unwrap();
    let item = icon.items.last_mut().unwrap();
    let mut mesh = item.mesh.to_owned_value();
    mesh.faces[0].material = "Chrome".into();
    item.mesh = mesh.into();
    assert_eq!(icon_studio::unknown_tags(&icon), vec!["Chrome".to_string()]);
}

#[test]
fn studio_and_samples_survive_save_and_reopen() {
    for (name, scene) in [
        ("studio", icon_studio::studio_scene().unwrap()),
        ("cube", icon_studio::sample_scene(Sample::Cube).unwrap()),
    ] {
        let path = temp(&format!("{name}.opscene"));
        operandi_core::persistence::save(&path, &scene).unwrap();
        let back = operandi_core::persistence::load(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(back.render_camera(), scene.render_camera());
        assert_eq!(back.render.frame, scene.render.frame);
        assert_eq!(back.shader_tree.nodes.len(), scene.shader_tree.nodes.len());
        let studio = icon_studio::studio_scene().unwrap();
        if name == "cube" {
            let render = icon_studio::render_scene(&studio, &back).unwrap();
            assert!(render.items.iter().any(|i| i.mesh.faces.len() == 6));
        }
    }
}

#[test]
fn glow_glows_and_the_light_sample_uses_it() {
    let studio = icon_studio::studio_scene().unwrap();
    for look in &MATERIALS {
        let material = studio
            .shader_tree
            .nodes
            .iter()
            .filter(|n| n.name == format!("{} Material", look.tag))
            .find_map(|n| match &n.kind {
                NodeKind::Material(m) => Some(m),
                _ => None,
            });
        let Some(m) = material else { continue };
        assert_eq!(
            m.luminous_intensity > 0.,
            look.tag == "Glow",
            "{}",
            look.tag
        );
    }
    let light = icon_studio::sample_scene(Sample::PointLight).unwrap();
    let mesh = &light.items.last().unwrap().mesh;
    assert!(mesh.faces.iter().any(|f| f.material == "Glow"));
    assert!(mesh.faces.iter().any(|f| f.material == "Ink"));
}

#[test]
fn revolved_and_tube_shapes_are_closed_and_face_outward() {
    use operandi_core::icon_studio::{revolve, through, tube};
    let check = |mesh: &operandi_core::Mesh, centre_of: &dyn Fn(DVec3) -> DVec3, name: &str| {
        mesh.validate().unwrap();
        for face in &mesh.faces {
            let p: Vec<DVec3> = face
                .vertices
                .iter()
                .map(|&v| mesh.vertices[v as usize])
                .collect();
            let normal =
                (0..p.len()).fold(DVec3::ZERO, |n, i| n + p[i].cross(p[(i + 1) % p.len()]));
            let c = p.iter().copied().sum::<DVec3>() / p.len() as f64;
            assert!(
                normal.dot(c - centre_of(c)) > 0.,
                "{name}: {face:?} faces inward"
            );
        }
    };
    let cone = revolve(&[(0., -0.5), (0.5, -0.5), (0., 0.5)], 16, false, "Clay").unwrap();
    check(&cone, &|_| DVec3::ZERO, "cone");
    // A torus faces away from its tube's centre line.
    let profile: Vec<(f64, f64)> = (0..12)
        .map(|k| {
            let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * f64::from(k) / 12.;
            (0.4 + 0.1 * a.cos(), 0.1 * a.sin())
        })
        .collect();
    let torus = revolve(&profile, 16, true, "Clay").unwrap();
    check(
        &torus,
        &|c| DVec3::new(c.x, 0., c.z).normalize() * 0.4,
        "torus",
    );
    // A straight tube faces away from its axis.
    let straight = tube(&[DVec3::ZERO, DVec3::X], 0.1, 8, "New").unwrap();
    check(
        &straight,
        &|c| DVec3::new(c.x.clamp(0.1, 0.9), 0., 0.),
        "tube",
    );
    // A smooth curve passes through its points.
    let pts = [DVec3::ZERO, DVec3::new(1., 1., 0.), DVec3::new(2., 0., 0.)];
    let curve = through(&pts, 8);
    for p in pts {
        assert!(curve.iter().any(|q| q.distance(p) < 1e-12));
    }
}
