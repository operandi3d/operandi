use super::*;
use crate::shader_tree::{
    BlendMode, NodeKind, Swizzle, TextureEffect, TextureLayer, TextureSource,
};

fn map(stem: &str) -> Option<(String, MapKind, Option<u32>)> {
    match parse_map_name(stem)? {
        MapName::Map {
            base,
            kind,
            resolution,
            ..
        } => Some((base, kind, resolution)),
        MapName::Preview { .. } => None,
    }
}

#[test]
fn map_names_ignore_case_separators_and_resolution_tags() {
    assert_eq!(
        map("Bricks01_1K-JPG_Color"),
        Some(("Bricks01".into(), MapKind::BaseColor, Some(1)))
    );
    assert_eq!(
        map("Bricks01_1K-JPG_NormalGL"),
        Some(("Bricks01".into(), MapKind::NormalGl, Some(1)))
    );
    assert_eq!(
        map("wall_nor_dx_2k"),
        Some(("wall".into(), MapKind::NormalDx, Some(2)))
    );
    assert_eq!(
        map("wall_diff_2k"),
        Some(("wall".into(), MapKind::BaseColor, Some(2)))
    );
    assert_eq!(
        map("wall_arm_2k"),
        Some(("wall".into(), MapKind::Packed, Some(2)))
    );
    assert_eq!(
        map("Rusty Metal 02 base_color 4k-png"),
        Some(("Rusty Metal 02".into(), MapKind::BaseColor, Some(4)))
    );
    assert_eq!(
        map("rock.AO"),
        Some(("rock".into(), MapKind::AmbientOcclusion, None))
    );
    // A lone word is never a map; nor is a name with no known suffix.
    assert_eq!(map("color"), None);
    assert_eq!(map("holiday_photo"), None);
    assert!(matches!(
        parse_map_name("Bricks01_Preview"),
        Some(MapName::Preview { .. })
    ));
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|n| PathBuf::from(format!("/lib/{n}")))
        .collect()
}

#[test]
fn cd_t1_bricks_fold_into_one_set_with_the_gl_normal() {
    let files = paths(&[
        "Bricks01_1K-JPG_Color.jpg",
        "Bricks01_1K-JPG_NormalGL.jpg",
        "Bricks01_1K-JPG_NormalDX.jpg",
        "Bricks01_1K-JPG_Roughness.jpg",
        "Bricks01_1K-JPG_Displacement.jpg",
        "sky.jpg",
    ]);
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let sets = texture_sets(&refs);
    assert_eq!(sets.len(), 1);
    let set = &sets[0];
    assert_eq!(set.name, "Bricks01");
    assert_eq!(set.maps.len(), 5);
    let kinds: Vec<MapKind> = set
        .pick(ResolutionChoice::Highest)
        .iter()
        .map(|m| m.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            MapKind::BaseColor,
            MapKind::Roughness,
            MapKind::NormalGl,
            MapKind::Height
        ]
    );
    assert!(set.representative().ends_with("Bricks01_1K-JPG_Color.jpg"));
}

#[test]
fn several_resolutions_are_one_entry_and_the_choice_picks_files() {
    let files = paths(&[
        "rock_color_1k.jpg",
        "rock_color_4k.jpg",
        "rock_rough_1k.jpg",
        "rock_rough_4k.jpg",
    ]);
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let sets = texture_sets(&refs);
    assert_eq!(sets.len(), 1);
    let pick = |c| {
        sets[0]
            .pick(c)
            .iter()
            .map(|m| m.resolution)
            .collect::<Vec<_>>()
    };
    assert_eq!(pick(ResolutionChoice::Highest), [Some(4), Some(4)]);
    assert_eq!(pick(ResolutionChoice::Lowest), [Some(1), Some(1)]);
    assert_eq!(pick(ResolutionChoice::Set(2)), [Some(1), Some(1)]);
}

#[test]
fn texture_set_loads_as_a_material_with_one_layer_per_map() {
    let files = paths(&["wall_diff_2k.jpg", "wall_nor_dx_2k.png", "wall_arm_2k.jpg"]);
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let set = &texture_sets(&refs)[0];
    let (preset, notes) = set
        .material_preset(ResolutionChoice::Highest, &|_| Some((8, 8)))
        .unwrap();
    assert!(notes.is_empty());
    assert_eq!(preset.header.kind, PresetKind::Material);
    let tree = &preset.scene.shader_tree;
    let group = tree.children(None).next().unwrap();
    assert_eq!(group.name, "wall");
    let layers: Vec<(TextureEffect, Swizzle, bool, BlendMode)> = tree
        .children(Some(group.id))
        .filter_map(|n| match &n.kind {
            NodeKind::Texture(TextureLayer {
                effect,
                source: TextureSource::ImageMap { settings, .. },
                ..
            }) => Some((
                *effect,
                settings.swizzle,
                settings.invert_green,
                n.layer.blend,
            )),
            _ => None,
        })
        .collect();
    use TextureEffect as E;
    assert_eq!(
        layers,
        [
            // The packed AO multiplies, above the colour it darkens.
            (E::DiffuseColor, Swizzle::Red, false, BlendMode::Multiply),
            (E::DiffuseColor, Swizzle::Rgba, false, BlendMode::Normal),
            (E::Roughness, Swizzle::Green, false, BlendMode::Normal),
            (E::Metallic, Swizzle::Blue, false, BlendMode::Normal),
            // CD-T2: the DirectX normal is green-flipped.
            (E::Normal, Swizzle::Rgba, true, BlendMode::Normal),
        ]
    );
    // The material is last (bottom); pictures are linked by path.
    assert!(matches!(
        tree.children(Some(group.id)).last().unwrap().kind,
        NodeKind::Material(_)
    ));
    assert_eq!(preset.scene.images.len(), 3);
    assert!(
        preset
            .scene
            .images
            .iter()
            .any(|i| i.path.ends_with("wall_arm_2k.jpg"))
    );
}

#[test]
fn an_unreadable_map_is_left_out_with_a_note() {
    let files = paths(&["a_color.jpg", "a_rough.jpg"]);
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let set = &texture_sets(&refs)[0];
    let (preset, notes) = set
        .material_preset(ResolutionChoice::Highest, &|p| {
            (!p.ends_with("a_rough.jpg")).then_some((4, 4))
        })
        .unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(preset.scene.images.len(), 1);
    assert!(
        set.material_preset(ResolutionChoice::Highest, &|_| None)
            .is_err()
    );
}

#[test]
fn listing_hides_junk_folds_sets_and_marks_hdrs_as_environments() {
    let dir = std::env::temp_dir().join(format!("operandi-pb-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Sub")).unwrap();
    std::fs::create_dir_all(dir.join(".hidden")).unwrap();
    for name in [
        ".DS_Store",
        "Thumbs.db",
        "studio_4k.exr",
        "photo.png",
        "notes.txt",
        "a_color.jpg",
        "a_rough.jpg",
        "chair.oppreset",
        "scene.opscene",
        "model.obj",
    ] {
        std::fs::write(dir.join(name), b"x").unwrap();
    }
    let kind = |_: &Path| Some(PresetKind::Item);
    let listing = list_folder(&dir, ListOptions::default(), &kind, &|| false).unwrap();
    assert_eq!(listing.folders, vec![dir.join("Sub")]);
    let mut got: Vec<(String, EntryKind)> = listing
        .entries
        .iter()
        .map(|e| (e.name.clone(), e.kind))
        .collect();
    got.sort();
    assert_eq!(
        got,
        [
            ("a".into(), EntryKind::TextureSet),
            ("chair".into(), EntryKind::Preset(PresetKind::Item)),
            ("model".into(), EntryKind::Model),
            ("photo".into(), EntryKind::Image),
            ("scene".into(), EntryKind::Scene),
            ("studio_4k".into(), EntryKind::Environment),
        ]
    );
    // "Show only usable files" off: the text file shows too.
    let all = list_folder(&dir, ListOptions { only_usable: false }, &kind, &|| false).unwrap();
    assert!(all.entries.iter().any(|e| e.kind == EntryKind::Other));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn sorting_is_natural_and_newest_or_largest_first() {
    let e = |name: &str, size: u64, secs: u64, kind| Entry {
        name: name.into(),
        path: PathBuf::from(name),
        kind,
        modified: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs)),
        size,
        texture_set: None,
        modo_kind: None,
    };
    let mut entries = vec![
        e("brick 10", 1, 3, EntryKind::Image),
        e("Brick 2", 5, 1, EntryKind::Scene),
        e("apple", 3, 2, EntryKind::Image),
    ];
    let names = |v: &[Entry]| v.iter().map(|e| e.name.clone()).collect::<Vec<_>>();
    sort(&mut entries, SortBy::Name);
    assert_eq!(names(&entries), ["apple", "Brick 2", "brick 10"]);
    sort(&mut entries, SortBy::Modified);
    assert_eq!(names(&entries), ["brick 10", "apple", "Brick 2"]);
    sort(&mut entries, SortBy::Size);
    assert_eq!(names(&entries), ["Brick 2", "apple", "brick 10"]);
    sort(&mut entries, SortBy::Type);
    assert_eq!(names(&entries), ["apple", "brick 10", "Brick 2"]);
}

#[test]
fn keep_both_adds_a_number() {
    let dir = std::env::temp_dir().join(format!("operandi-pb-free-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(
        free_path(&dir, "Rust", "oppreset"),
        dir.join("Rust.oppreset")
    );
    std::fs::write(dir.join("Rust.oppreset"), b"x").unwrap();
    assert_eq!(
        free_path(&dir, "Rust", "oppreset"),
        dir.join("Rust 2.oppreset")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_picture_named_like_the_set_is_its_preview_and_separate_maps_beat_packed() {
    let files = paths(&[
        "Bricks113A.png",
        "Bricks113A_1K-JPG_Color.jpg",
        "Bricks113A_1K-JPG_Roughness.jpg",
        "fab_rough_1k.jpg",
        "fab_arm_1k.jpg",
        "OLD-BRICKS_rough.Jpg",
        "Old-Bricks_COLOR.JPG",
    ]);
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let sets = texture_sets(&refs);
    let bricks = sets.iter().find(|s| s.name == "Bricks113A").unwrap();
    assert!(bricks.preview.as_ref().unwrap().ends_with("Bricks113A.png"));
    assert!(bricks.representative().ends_with("Bricks113A.png"));
    // Mixed case: one set, named as its colour map writes it.
    assert!(
        sets.iter()
            .any(|s| s.name == "Old-Bricks" && s.maps.len() == 2)
    );
    let fab = sets.iter().find(|s| s.name == "fab").unwrap();
    let (preset, _) = fab
        .material_preset(ResolutionChoice::Highest, &|_| Some((2, 2)))
        .unwrap();
    let effects: Vec<TextureEffect> = preset
        .scene
        .shader_tree
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Texture(t) => Some(t.effect),
            _ => None,
        })
        .collect();
    // Roughness from its own map; metal from the packed blue; no AO (no colour).
    assert_eq!(effects, [TextureEffect::Metallic, TextureEffect::Roughness]);
}

#[test]
fn mx_07_a_materialx_file_beats_its_texture_sets_names() {
    let dir = std::env::temp_dir().join(format!("operandi-mx07-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in [
        "Rock_1K-JPG_Color.jpg",
        "Rock_1K-JPG_Roughness.jpg",
        "Moss_Color.png",
        "Moss_Roughness.png",
    ] {
        std::fs::write(dir.join(f), b"x").unwrap();
    }
    std::fs::write(
        dir.join("Rock_1K-JPG.mtlx"),
        r#"<materialx version="1.39" fileprefix="./">
  <image name="c" type="color3"><input name="file" type="filename" value="Rock_1K-JPG_Color.jpg"/></image>
  <open_pbr_surface name="s" type="surfaceshader"><input name="base_color" type="color3" nodename="c"/></open_pbr_surface>
  <surfacematerial name="Rock" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s"/></surfacematerial>
</materialx>"#,
    )
    .unwrap();
    // A .mtlx reading no set's pictures is listed on its own.
    std::fs::write(dir.join("Plain.mtlx"), "<materialx version=\"1.39\"/>").unwrap();
    let listing = list_folder(&dir, ListOptions::default(), &|_| None, &|| false).unwrap();
    let mut got: Vec<(String, EntryKind)> = listing
        .entries
        .iter()
        .map(|e| (e.name.clone(), e.kind))
        .collect();
    got.sort();
    assert_eq!(
        got,
        [
            ("Moss".to_string(), EntryKind::TextureSet),
            ("Plain".to_string(), EntryKind::MaterialX),
            ("Rock".to_string(), EntryKind::MaterialX),
        ]
    );
    let rock = listing.entries.iter().find(|e| e.name == "Rock").unwrap();
    assert!(rock.path.ends_with("Rock_1K-JPG.mtlx") && rock.texture_set.is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A FORM file of `form` with one DESC chunk (MC-03's published layout).
fn modo_file(form: &[u8; 4], desc: &[u8]) -> Vec<u8> {
    let mut body = form.to_vec();
    body.extend_from_slice(b"DESC");
    body.extend_from_slice(&(desc.len() as u32).to_be_bytes());
    body.extend_from_slice(desc);
    if desc.len() % 2 == 1 {
        body.push(0);
    }
    let mut out = b"FORM".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend(body);
    out
}

#[test]
fn mc_modo_content_folder_lists_by_file_type() {
    let dir = std::env::temp_dir().join(format!("operandi-modo-folder-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Fish.lxl"), modo_file(b"LXPM", b"mesh\0")).unwrap();
    std::fs::write(
        dir.join("Paint.lxp"),
        modo_file(b"LXPR", b"advancedMaterial\0Red\0"),
    )
    .unwrap();
    std::fs::write(dir.join("Studio.lxe"), modo_file(b"LXPE", b"")).unwrap();
    std::fs::write(dir.join("Site.lrl"), b"<link/>").unwrap();
    std::fs::write(dir.join("Room.lxo"), modo_file(b"LXOB", b"")).unwrap();
    std::fs::write(dir.join("Chair.obj"), b"v 0 0 0").unwrap();
    // MC-13: never shown.
    for f in ["run.py", "macro.lxm", "a.pl", "b.lua", "index.cfg"] {
        std::fs::write(dir.join(f), b"x").unwrap();
    }
    let before: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (e.path(), e.metadata().unwrap().modified().unwrap())
        })
        .collect();
    let kind = |_: &Path| None;
    let names = |l: &Listing| {
        let mut v: Vec<(String, EntryKind, String)> = l
            .entries
            .iter()
            .map(|e| (e.name.clone(), e.kind, e.kind_label()))
            .collect();
        v.sort();
        v
    };
    // "Show only usable files" on: the greyed Modo files stay out.
    let usable = list_folder(&dir, ListOptions::default(), &kind, &|| false).unwrap();
    assert_eq!(
        names(&usable),
        [
            ("Chair".into(), EntryKind::Model, "Model".into()),
            (
                "Fish".into(),
                EntryKind::ModoMeshPreset,
                "Modo mesh preset".into()
            ),
            ("Room".into(), EntryKind::Scene, "Modo Scene".into()),
        ]
    );
    // Off: they're listed with the kind their header names (MC-12).
    let all = list_folder(&dir, ListOptions { only_usable: false }, &kind, &|| false).unwrap();
    let all_names = names(&all);
    assert!(all_names.contains(&(
        "Paint".into(),
        EntryKind::ModoPreset,
        "Modo material preset".into()
    )));
    assert!(all_names.contains(&(
        "Studio".into(),
        EntryKind::ModoPreset,
        "Modo environment preset".into()
    )));
    assert!(all_names.contains(&("Site".into(), EntryKind::ModoPreset, "Modo web link".into())));
    // MC-T4: scripts and config never appear.
    assert_eq!(all.entries.len(), 6);
    let paint = all.entries.iter().find(|e| e.name == "Paint").unwrap();
    assert!(!paint.kind.usable());
    assert_eq!(
        paint.unsupported_note().as_deref(),
        // LXP-04: a material preset with no material item in it.
        Some(super::super::lxp::NO_MATERIAL)
    );
    // MC-20 buckets.
    let in_bucket = |b: Bucket| {
        let mut v: Vec<&str> = all
            .entries
            .iter()
            .filter(|e| e.in_bucket(b))
            .map(|e| e.name.as_str())
            .collect();
        v.sort();
        v
    };
    assert_eq!(in_bucket(Bucket::Meshes), ["Chair", "Fish", "Room"]);
    assert_eq!(in_bucket(Bucket::Scenes), ["Room"]);
    assert_eq!(in_bucket(Bucket::Materials), ["Paint"]);
    assert_eq!(in_bucket(Bucket::Environments), ["Studio"]);
    assert_eq!(in_bucket(Bucket::All).len(), 6);
    // MC-T1 / MC-04: listing wrote nothing in the folder.
    for (path, time) in before {
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), time);
    }
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 11);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mc_kind_filter_names_and_numbers_are_fixed() {
    for (i, b) in Bucket::ALL.into_iter().enumerate() {
        assert_eq!(Bucket::from_script(&i.to_string()), Some(b));
        assert_eq!(Bucket::from_script(b.script_name()), Some(b));
    }
    assert_eq!(Bucket::from_script("MESHES"), Some(Bucket::Meshes));
    assert_eq!(Bucket::from_script("6"), None);
    assert_eq!(
        Bucket::Meshes.empty_hint(),
        Some("Add a folder of models to see them here.")
    );
}

/// MC-01, MC-11: in or below a "Matcaps" folder (any case) every picture
/// is a matcap, even ones named like texture-set maps; a picture's sidecar
/// note isn't listed, and is read as its note.
#[test]
fn pictures_in_a_matcaps_folder_are_matcaps() {
    let dir = std::env::temp_dir().join(format!("operandi-pb-matcaps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let clay = dir.join("matcaps").join("Clay");
    std::fs::create_dir_all(&clay).unwrap();
    for name in [
        "Jade.png",
        "Jade.txt",
        "a_color.jpg",
        "a_rough.jpg",
        "Studio.exr",
        "readme.md",
    ] {
        std::fs::write(clay.join(name), b"x").unwrap();
    }
    std::fs::write(clay.join("Jade.txt"), "  CC0, by someone\n").unwrap();
    let kind = |_: &Path| None;
    let listing = list_folder(&clay, ListOptions::default(), &kind, &|| false).unwrap();
    let mut got: Vec<(String, EntryKind)> = listing
        .entries
        .iter()
        .map(|e| (e.name.clone(), e.kind))
        .collect();
    got.sort();
    assert_eq!(
        got,
        [
            ("Jade".into(), EntryKind::Matcap),
            ("Studio".into(), EntryKind::Matcap),
            ("a_color".into(), EntryKind::Matcap),
            ("a_rough".into(), EntryKind::Matcap),
        ]
    );
    assert_eq!(EntryKind::Matcap.label(), "Matcap");
    assert!(is_matcap(&clay.join("Jade.png")));
    assert!(!is_matcap(&dir.join("Jade.png")));
    assert!(!is_matcap(&clay.join("readme.md")));
    assert_eq!(
        matcap_note(&clay.join("Jade.png")).as_deref(),
        Some("CC0, by someone")
    );
    assert_eq!(matcap_note(&clay.join("Studio.exr")), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// PG-01, MC-09: a fresh Presets root gets the five empty folders; a root
/// that already holds something is left alone.
#[test]
fn a_fresh_presets_root_gets_the_starter_folders() {
    let dir = std::env::temp_dir().join(format!("operandi-pb-starter-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(make_starter_folders(&dir).unwrap());
    for name in STARTER_FOLDERS {
        assert!(dir.join(name).is_dir(), "{name}");
    }
    std::fs::remove_dir_all(dir.join("Colours")).unwrap();
    assert!(!make_starter_folders(&dir).unwrap());
    assert!(!dir.join("Colours").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// MC-05: matcaps are found below every "Matcaps" folder in a root,
/// grouped by the folders under it.
#[test]
fn matcaps_are_found_and_grouped() {
    let dir = std::env::temp_dir().join(format!("operandi-pb-find-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let m = dir.join("Pack").join("Matcaps");
    std::fs::create_dir_all(m.join("Metal")).unwrap();
    std::fs::create_dir_all(dir.join("Other")).unwrap();
    std::fs::write(m.join("Plain.png"), b"x").unwrap();
    std::fs::write(m.join("Metal").join("Gold 10.png"), b"x").unwrap();
    std::fs::write(m.join("Metal").join("Gold 2.png"), b"x").unwrap();
    std::fs::write(m.join("Metal").join("notes.txt"), b"x").unwrap();
    std::fs::write(dir.join("Other").join("photo.png"), b"x").unwrap();
    let found = find_matcaps(&dir, &|| false);
    let got: Vec<(&str, &str)> = found
        .iter()
        .map(|f| (f.group.as_str(), f.name.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("Matcaps", "Plain"),
            ("Metal", "Gold 2"),
            ("Metal", "Gold 10")
        ]
    );
    // A root that is itself a Matcaps folder.
    assert_eq!(find_matcaps(&m, &|| false).len(), 3);
    std::fs::remove_dir_all(&dir).unwrap();
}
