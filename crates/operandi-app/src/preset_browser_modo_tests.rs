//! Modo content folder in the preset browser (BEHAVIOR-MODO-CONTENT-FOLDER
//! MC-T1..T5, stages 1 and 2).
use super::*;

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.load_preset_browser();
    a
}

fn listed(a: &mut Operandi) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut t = 0.;
    while a.preset_browser.listing_busy() || a.preset_browser.listing.is_none() {
        t += 2.;
        a.preset_browser.tick(t);
        assert!(
            std::time::Instant::now() < deadline,
            "listing never arrived"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// A FORM file framed to the published layout.
fn form(form: &[u8; 4], chunks: &[(&[u8; 4], &[u8])], tail: &[u8]) -> Vec<u8> {
    let mut body = form.to_vec();
    for (id, data) in chunks {
        body.extend_from_slice(*id);
        body.extend_from_slice(&(data.len() as u32).to_be_bytes());
        body.extend_from_slice(data);
        if data.len() % 2 == 1 {
            body.push(0);
        }
    }
    body.extend_from_slice(tail);
    let mut out = b"FORM".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend(body);
    out
}

/// A Modo mesh preset holding one triangle mesh "Fish" with material
/// "Scales": Operandi's own LXO export with the mesh-preset FORM type.
fn fish_lxl() -> Vec<u8> {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.name = "Fish".into();
    item.mesh = operandi_core::Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
        faces: vec![operandi_core::Face {
            vertices: vec![0, 1, 2],
            uv: None,
            material: "Scales".into(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    let bytes = operandi_core::lxo::export::write(&scene, &mut Vec::new()).unwrap();
    form(b"LXPM", &[(b"DESC", b"mesh\0Fish\0")], &bytes[12..])
}

/// A Modo-like content folder added as a location.
fn content_folder(a: &mut Operandi) -> PathBuf {
    let dir = a.environment.paths().data_root().join("Modo Content");
    std::fs::create_dir_all(dir.join("Assets")).unwrap();
    std::fs::write(dir.join("Assets/Fish.lxl"), fish_lxl()).unwrap();
    std::fs::write(
        dir.join("Assets/Paint.lxp"),
        form(b"LXPR", &[(b"DESC", b"advancedMaterial\0Red\0")], &[]),
    )
    .unwrap();
    std::fs::write(
        dir.join("Assets/Chair.obj"),
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    )
    .unwrap();
    std::fs::write(dir.join("Assets/setup.py"), b"print()").unwrap();
    std::fs::write(dir.join("Assets/do.lxm"), b"x").unwrap();
    a.execute(&format!("pbview.basePathAdd path:\"{}\"", dir.display()));
    dir
}

fn names(a: &Operandi) -> Vec<String> {
    let mut v: Vec<String> = a
        .preset_browser
        .shown()
        .into_iter()
        .map(|e| e.name)
        .collect();
    v.sort();
    v
}

#[test]
fn mc_t1_t4_a_location_lists_without_changing_the_folder() {
    let mut a = app();
    let dir = content_folder(&mut a);
    let assets = dir.join("Assets");
    let stamp = |p: &Path| {
        std::fs::read_dir(p)
            .unwrap()
            .map(|e| e.unwrap().metadata().unwrap().modified().unwrap())
            .collect::<Vec<_>>()
    };
    let before = stamp(&assets);
    assert!(
        a.preset_browser.roots.iter().any(|r| r.path == dir),
        "added, not copied"
    );
    a.preset_browser.show_folder(assets.clone());
    listed(&mut a);
    assert_eq!(names(&a), ["Chair", "Fish"]);
    // Filter off: the Modo material preset shows greyed; scripts never.
    a.execute("operandi.pbview.onlyUsable state:false");
    assert!(!a.preset_browser.only_usable);
    listed(&mut a);
    assert_eq!(names(&a), ["Chair", "Fish", "Paint"]);
    a.execute("operandi.pbview.onlyUsable ?");
    assert_eq!(a.status, "false");
    a.execute("operandi.pbview.onlyUsable");
    assert!(a.preset_browser.only_usable, "bare toggles");
    assert_eq!(stamp(&assets), before, "nothing in the folder changed");
    assert_eq!(std::fs::read_dir(&assets).unwrap().count(), 5);
}

#[test]
fn mc_20_kind_filter_command_and_meshes_empty_state() {
    let mut a = app();
    let dir = content_folder(&mut a);
    a.preset_browser.show_folder(dir.join("Assets"));
    a.execute("operandi.pbview.onlyUsable state:false");
    listed(&mut a);
    a.execute("operandi.pbview.kindFilter kind:meshes");
    assert_eq!(a.preset_browser.kind_filter, Bucket::Meshes);
    assert_eq!(names(&a), ["Chair", "Fish"]);
    a.execute("operandi.pbview.kindFilter kind:2");
    assert_eq!(names(&a), ["Paint"]);
    a.execute("operandi.pbview.kindFilter ?");
    assert_eq!(a.status, "materials");
    a.execute("operandi.pbview.kindFilter kind:sounds");
    assert!(a.status.contains("kind is all (0)"), "{}", a.status);
    assert_eq!(a.preset_browser.kind_filter, Bucket::Materials);
    // MC-21: an empty Meshes bucket says what to do.
    a.execute("operandi.pbview.kindFilter kind:meshes");
    let empty = a.environment.paths().data_root().join("Empty");
    std::fs::create_dir_all(&empty).unwrap();
    a.preset_browser.show_folder(empty);
    listed(&mut a);
    let ctx = Context::default();
    let out = ctx.run(Default::default(), |ctx| {
        CentralPanel::default().show(ctx, |ui| a.preset_browser_body(ui, false));
    });
    let text = super::tests::drawn_text(&out);
    assert!(
        text.contains("Add a folder of models to see them here."),
        "{text}"
    );
    // The filter survives a restart.
    a.save_preset_browser();
    let mut b = Operandi::with_environment(a.environment.clone());
    b.load_preset_browser();
    assert_eq!(b.preset_browser.kind_filter, Bucket::Meshes);
}

#[test]
fn mc_11_a_mesh_preset_adds_a_mesh_item_in_one_undo_step() {
    let mut a = app();
    let dir = content_folder(&mut a);
    let items = a.editor.scene.items.len();
    let entry = a.entry_for_path(&dir.join("Assets/Fish.lxl"));
    assert_eq!(entry.kind, EntryKind::ModoMeshPreset);
    assert!(a.browser_load(&entry, Target::Default), "{}", a.status);
    let fish = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.name == "Fish")
        .expect("added");
    assert_eq!(fish.mesh.faces.len(), 1);
    assert_eq!(fish.transform.translation, DVec3::ZERO, "at the origin");
    assert_eq!(a.editor.scene.selection.item, Some(fish.id), "selected");
    assert_eq!(a.editor.scene.items.len(), items + 1);
    // MC-T3: its material arrives as a named material group.
    assert!(
        a.editor
            .scene
            .shader_tree
            .material_for_tag("Scales")
            .is_some()
    );
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.items.len(), items, "one undo removes it");
    // The same through preset.do.
    a.execute(&format!("preset.do preset:\"{}\"", entry.path.display()));
    assert!(
        a.editor.scene.items.iter().any(|i| i.name == "Fish"),
        "{}",
        a.status
    );
}

#[test]
fn mc_t2_a_greyed_modo_preset_only_says_why() {
    let mut a = app();
    let dir = content_folder(&mut a);
    let entry = a.entry_for_path(&dir.join("Assets/Paint.lxp"));
    assert_eq!(entry.kind, EntryKind::ModoPreset);
    assert_eq!(entry.kind_label(), "Modo material preset");
    let before = a.editor.scene.clone();
    assert!(!a.browser_load(&entry, Target::Default));
    // LXP-04: a material preset with no material item says so.
    assert_eq!(a.status, operandi_core::preset::lxp::NO_MATERIAL);
    assert!(a.editor.scene == before, "nothing changed");
}

/// An S0 string (zero-terminated, padded to even length).
fn s0(text: &str) -> Vec<u8> {
    let mut v = text.as_bytes().to_vec();
    v.push(0);
    if v.len() % 2 == 1 {
        v.push(0);
    }
    v
}
fn sub(id: &[u8; 4], data: Vec<u8>) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend_from_slice(&(data.len() as u16).to_be_bytes());
    v.extend(data);
    v
}
fn chnl(name: &str, v: f32) -> Vec<u8> {
    let mut d = s0(name);
    d.extend_from_slice(&2u16.to_be_bytes());
    d.extend_from_slice(&v.to_be_bytes());
    sub(b"CHNL", d)
}
fn chns(name: &str, v: &str) -> Vec<u8> {
    let mut d = s0(name);
    d.extend(s0(v));
    sub(b"CHNS", d)
}
fn item(kind: &str, name: &str, id: u32, subs: Vec<Vec<u8>>) -> Vec<u8> {
    let mut d = s0(kind);
    d.extend(s0(name));
    d.extend_from_slice(&id.to_be_bytes());
    for s in subs {
        d.extend(s);
    }
    d
}

/// A material preset: a gold-like hand-made metal under a mask, a Modo
/// constant on roughness and a plug-in noise on diffuse colour.
fn gold_lxp() -> Vec<u8> {
    let material = item(
        "advancedMaterial",
        "Gold",
        2,
        vec![
            chnl("diffAmt", 0.),
            chnl("specAmt", 1.),
            chnl("specCol.R", 1.),
            chnl("specCol.G", 0.77),
            chnl("specCol.B", 0.34),
            chnl("rough", 0.1),
            chns("brdfType", "gtr"),
        ],
    );
    let constant = item(
        "constant",
        "Rough",
        3,
        vec![chns("effect", "rough"), chnl("value", 0.25)],
    );
    let plugin = item(
        "val.Noise_Scruffed.RJJ",
        "Scruff",
        4,
        vec![chns("effect", "diffColor")],
    );
    form(
        b"LXPR",
        &[
            (b"DESC", b"advancedMaterial\0Gold\0"),
            (b"ITEM", &item("mask", "Polished Gold", 1, vec![])),
            (b"ITEM", &material),
            (b"ITEM", &constant),
            (b"ITEM", &plugin),
        ],
        &[],
    )
}

#[test]
fn lxp_01_03_a_modo_material_loads_in_one_undo_step() {
    let mut a = app();
    let dir = content_folder(&mut a);
    std::fs::write(dir.join("Assets/Gold.lxp"), gold_lxp()).unwrap();
    let entry = a.entry_for_path(&dir.join("Assets/Gold.lxp"));
    assert_eq!(entry.kind, EntryKind::ModoMaterial);
    assert!(entry.kind.usable());
    assert!(entry.in_bucket(operandi_core::preset::library::Bucket::Materials));
    assert!(crate::preset_browser::drops_material(&entry));
    // LXP-02/05: the plug-in noise was the only Diffuse Color layer.
    let info = a.lxp_info(&entry).unwrap();
    assert!(info.partial && info.changed_look, "{info:?}");
    assert!(
        info.notes
            .iter()
            .any(|n| n.contains("val.Noise_Scruffed.RJJ"))
    );
    assert!(a.entry_tip(&entry).contains("Partial"));
    // Into a material group, as any library material (LXP-03).
    let group = a
        .editor
        .scene
        .shader_tree
        .add(operandi_core::shader_tree::NewLayer::Group, None)
        .unwrap();
    let before = a.editor.scene.clone();
    let undo = a.editor.undo.len();
    assert!(a.browser_load_into_group(&entry, group), "{}", a.status);
    assert_eq!(a.editor.undo.len(), undo + 1, "one undo step");
    let tree = &a.editor.scene.shader_tree;
    // The constant and the material arrive; the plug-in layer doesn't.
    let kids: Vec<_> = tree.children(Some(group)).collect();
    assert_eq!(kids.len(), 2, "{kids:?}");
    let metal = kids.iter().find_map(|n| match &n.kind {
        operandi_core::shader_tree::NodeKind::Material(m) => Some(m.metallic),
        _ => None,
    });
    assert_eq!(metal, Some(1.));
    a.run(Command::Undo);
    assert!(a.editor.scene == before, "undo puts it back");
    // Nothing is written into the content folder (LXP-03).
    let names: Vec<_> = std::fs::read_dir(dir.join("Assets"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(names.len(), 6, "{names:?}");
}

#[test]
fn mc_10_a_dropped_modo_scene_joins_this_scene() {
    let mut a = app();
    let dir = a.environment.paths().data_root().join("Scenes");
    std::fs::create_dir_all(&dir).unwrap();
    // A Modo scene: the mesh preset's body under the scene FORM type.
    let lxl = fish_lxl();
    let lxo = form(b"LXOB", &[], &lxl[12 + 4 + 4 + 10..]);
    std::fs::write(dir.join("Pond.lxo"), &lxo).unwrap();
    let entry = a.entry_for_path(&dir.join("Pond.lxo"));
    assert!(entry.is_modo_scene());
    let items = a.editor.scene.items.len();
    // Dropped outside any view: no plane point, so it lands as saved.
    assert!(
        a.browser_load(&entry, Target::Drop(pos2(-9999., -9999.))),
        "{}",
        a.status
    );
    assert!(a.editor.scene.items.iter().any(|i| i.name == "Fish"));
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.items.len(), items);
}

#[test]
fn mc_t5_a_folder_put_in_the_library_appears_without_a_restart() {
    let mut a = app();
    let library = a.environment.paths().library().to_path_buf();
    let modo = library.join("Modo Content");
    std::fs::create_dir_all(&modo).unwrap();
    a.preset_browser.tick(1000.);
    assert!(a.preset_browser.roots.iter().any(|r| r.path == modo));
    // Removed by the user, it doesn't come back.
    a.preset_browser.remove_root(&modo);
    a.preset_browser.tick(2000.);
    assert!(!a.preset_browser.roots.iter().any(|r| r.path == modo));
}

#[test]
fn mc_02_copy_into_library_runs_dircache_copy() {
    let mut a = app();
    let dir = content_folder(&mut a);
    let library = a.environment.paths().library().to_path_buf();
    a.execute(&format!("dircache.copy source:\"{}\"", dir.display()));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let message = loop {
        if let Some(m) = a.preset_browser.poll_copy() {
            break m;
        }
        assert!(std::time::Instant::now() < deadline, "copy never finished");
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    assert!(message.contains("5 files"), "{message}");
    let copy = library.join("Modo Content");
    assert!(copy.join("Assets/Fish.lxl").is_file());
    assert!(
        a.preset_browser.roots.iter().any(|r| r.path == copy),
        "shown as a root"
    );
    // Again: refused unless overwrite:true.
    a.execute(&format!("dircache.copy source:\"{}\"", dir.display()));
    assert!(a.status.contains("overwrite:true"), "{}", a.status);
    assert!(a.preset_browser.copy_job.is_none());
    // The drop menu asks first instead.
    a.browser_copy_to_library(&dir, false);
    assert!(matches!(a.preset_browser.ask, Some(Ask::CopyOverwrite(_))));
}

#[test]
fn mc_02_a_folder_dropped_on_the_browser_asks_what_to_do() {
    let mut a = app();
    let dir = a.environment.paths().data_root().join("Dropped");
    std::fs::create_dir_all(&dir).unwrap();
    // Not over the browser: nothing asked.
    a.file_dropped(dir.clone(), Some(pos2(5., 5.)));
    assert!(a.preset_browser.folder_drop.is_none());
    a.preset_browser.body_rect = Some((Rect::from_min_size(pos2(0., 0.), vec2(100., 100.)), false));
    a.file_dropped(dir.clone(), Some(pos2(5., 5.)));
    let drop = a.preset_browser.folder_drop.clone().expect("menu");
    assert_eq!(drop.folder, dir);
    assert!(
        !a.preset_browser.roots.iter().any(|r| r.path == dir),
        "nothing added yet"
    );
}
