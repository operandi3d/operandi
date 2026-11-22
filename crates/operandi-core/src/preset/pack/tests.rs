use super::*;
use crate::preset::{Preset, PresetCredit, basics};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-pack-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn item(path: &str, licence: &str, url: &str, credit: &str) -> ManifestItem {
    ManifestItem {
        path: path.into(),
        kind: "material".into(),
        licence: licence.into(),
        source_url: url.into(),
        credit: credit.into(),
        checked: "2026-11-11".into(),
        category: category_of(path),
    }
}

/// A material preset at `pack/<rel>` with this licence header.
fn write_preset(pack: &Path, rel: &str, source: Option<PresetCredit>) {
    let path = pack.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let (_, mut preset): (_, Preset) = basics::presets().remove(0);
    preset.header.source = source;
    crate::persistence::save_preset(&path, &preset).unwrap();
}

fn cc0(url: &str) -> Option<PresetCredit> {
    Some(PresetCredit {
        licence: CC0.into(),
        url: url.into(),
        credit: "ambientCG".into(),
    })
}

#[test]
fn manifest_round_trips_through_json_and_disk() {
    let m = Manifest::new(
        "Operandi Library",
        "2026.11.0",
        vec![item(
            "Starter/Bricks113A/Bricks113A.oppreset",
            CC0,
            "https://ambientcg.com/a/Bricks113A",
            "ambientCG",
        )],
    );
    let json = m.to_json();
    // The documented field names, exactly.
    for key in [
        "\"format\": \"operandi-library\"",
        "\"format_version\": 1",
        "\"name\": \"Operandi Library\"",
        "\"version\": \"2026.11.0\"",
        "\"path\": \"Starter/Bricks113A/Bricks113A.oppreset\"",
        "\"kind\": \"material\"",
        "\"licence\": \"CC0-1.0\"",
        "\"source_url\": \"https://ambientcg.com/a/Bricks113A\"",
        "\"credit\": \"ambientCG\"",
        "\"checked\": \"2026-11-11\"",
    ] {
        assert!(json.contains(key), "{key} missing from {json}");
    }
    assert_eq!(Manifest::parse(&json).unwrap(), m);
    let dir = scratch("round-trip");
    m.write(&dir).unwrap();
    assert_eq!(Manifest::read(&dir).unwrap(), m);
    assert_eq!(packs_in(&dir), vec![dir.clone()]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn other_formats_and_versions_are_refused() {
    let mut m = Manifest::new("X", "1", Vec::new());
    m.format = "something-else".into();
    assert!(Manifest::parse(&m.to_json()).is_err());
    let mut m = Manifest::new("X", "1", Vec::new());
    m.format_version = 2;
    assert!(Manifest::parse(&m.to_json()).is_err());
    assert!(Manifest::parse("not json").is_err());
}

#[test]
fn starter_takes_only_cc0_with_a_source() {
    let url = "https://example.org/a";
    assert!(check_item(&item("Starter/A/A.oppreset", CC0, url, "")).is_ok());
    for bad in [
        item("Starter/A/A.oppreset", "CC-BY-4.0", url, "Someone"),
        item("Starter/A/A.oppreset", "CC-BY-NC-4.0", url, "Someone"),
        item("Starter/A/A.oppreset", "", url, ""),
        item("Starter/A/A.oppreset", CC0, "", ""),
        item("Loose/A.oppreset", CC0, url, ""),
    ] {
        assert!(check_item(&bad).is_err(), "{bad:?} should be refused");
    }
}

#[test]
fn extras_take_cc_by_with_a_credit_but_nothing_non_commercial() {
    let url = "https://example.org/a";
    assert!(check_item(&item("Extras/A/A.oppreset", "CC-BY-4.0", url, "Someone")).is_ok());
    assert!(check_item(&item("Extras/A/A.oppreset", "CC-BY-3.0", url, "Someone")).is_ok());
    assert!(check_item(&item("Extras/A/A.oppreset", CC0, url, "")).is_ok());
    for bad in [
        item("Extras/A/A.oppreset", "CC-BY-4.0", url, ""),
        item("Extras/A/A.oppreset", "CC-BY-NC-4.0", url, "Someone"),
        item("Extras/A/A.oppreset", "CC-BY-ND-4.0", url, "Someone"),
        item("Extras/A/A.oppreset", "CC-BY-4.0", "", "Someone"),
    ] {
        assert!(check_item(&bad).is_err(), "{bad:?} should be refused");
    }
}

#[test]
fn build_manifest_reads_each_presets_licence_header() {
    let dir = scratch("build-ok");
    write_preset(
        &dir,
        "Starter/Bricks/Bricks.oppreset",
        cc0("https://ambientcg.com/a/Bricks"),
    );
    write_preset(
        &dir,
        "Extras/Moss/Moss.oppreset",
        Some(PresetCredit {
            licence: "CC-BY-4.0".into(),
            url: "https://example.org/moss".into(),
            credit: "A. Person".into(),
        }),
    );
    let m = build_manifest(&dir, "Operandi Library", "2026.11.0", "2026-11-11").unwrap();
    assert_eq!(m.items.len(), 2);
    assert_eq!(m.items[0].path, "Starter/Bricks/Bricks.oppreset");
    assert_eq!(m.items[0].licence, CC0);
    assert_eq!(m.items[0].source_url, "https://ambientcg.com/a/Bricks");
    assert_eq!(m.items[0].kind, "material");
    assert_eq!(m.items[1].licence, "CC-BY-4.0");
    assert!(credits_text(&m).contains("Extras/ items are CC-BY"));
    assert!(check(&m).is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn build_manifest_fails_on_a_non_cc0_starter_item_or_a_missing_licence() {
    let dir = scratch("build-bad");
    write_preset(
        &dir,
        "Starter/Good/Good.oppreset",
        cc0("https://a.org/good"),
    );
    write_preset(
        &dir,
        "Starter/ByItem/ByItem.oppreset",
        Some(PresetCredit {
            licence: "CC-BY-4.0".into(),
            url: "https://a.org/by".into(),
            credit: "Someone".into(),
        }),
    );
    write_preset(&dir, "Starter/NoUrl/NoUrl.oppreset", cc0(""));
    write_preset(&dir, "Starter/Bare/Bare.oppreset", None);
    let problems = build_manifest(&dir, "Operandi Library", "1", "2026-11-11").unwrap_err();
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(
        problems
            .iter()
            .any(|p| p.contains("ByItem") && p.contains("CC0"))
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("NoUrl") && p.contains("source URL"))
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("Bare") && p.contains("no licence"))
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_items_maps_share_its_credit() {
    let pack = Path::new("/packs/Operandi Library");
    let m = Manifest::new(
        "Operandi Library",
        "1",
        vec![item(
            "Starter/Bricks/Bricks.oppreset",
            CC0,
            "https://a.org/b",
            "ambientCG",
        )],
    );
    let exact = pack.join("Starter/Bricks/Bricks.oppreset");
    let map = pack.join("Starter/Bricks/Bricks_Color.jpg");
    assert_eq!(m.item_for(pack, &exact).unwrap().credit, "ambientCG");
    assert_eq!(m.item_for(pack, &map).unwrap().credit, "ambientCG");
    assert!(
        m.item_for(pack, &pack.join("Starter/Other/x.jpg"))
            .is_none()
    );
    assert!(m.item_for(Path::new("/elsewhere"), &exact).is_none());
    let info = m.items[0].info();
    assert!(info.contains("Licence: CC0-1.0") && info.contains("Source: https://a.org/b"));
    assert_eq!(m.licence_summary(), "1 CC0-1.0");
}

#[test]
fn versions_compare_number_by_number() {
    assert_eq!(compare_versions("2026.11.0", "2026.12.0"), Ordering::Less);
    assert_eq!(compare_versions("2026.12.0", "2026.9.3"), Ordering::Greater);
    assert_eq!(compare_versions("2026.11.0", "2026.11.0"), Ordering::Equal);
    assert_eq!(compare_versions("2026.11.1", "2026.11"), Ordering::Greater);
}

#[test]
fn packs_in_a_library_folder_are_its_subfolders_with_a_manifest() {
    let lib = scratch("library");
    for (folder, version) in [
        ("Operandi Library", "2026.11.0"),
        ("Operandi Library 2", "2026.12.0"),
    ] {
        let pack = lib.join(folder);
        std::fs::create_dir_all(&pack).unwrap();
        Manifest::new("Operandi Library", version, Vec::new())
            .write(&pack)
            .unwrap();
    }
    std::fs::create_dir_all(lib.join("Not a pack")).unwrap();
    assert_eq!(
        packs_in(&lib),
        vec![lib.join("Operandi Library"), lib.join("Operandi Library 2")]
    );
    assert!(packs_in(&lib.join("missing")).is_empty());
    let _ = std::fs::remove_dir_all(lib);
}

#[test]
fn basics_are_six_valid_gpl_material_presets() {
    let all = basics::presets();
    let names: Vec<&str> = all.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        ["Plastic", "Metal", "Glass", "Clay", "Rubber", "Emissive"]
    );
    for (name, p) in &all {
        assert_eq!(p.header.kind, crate::preset::PresetKind::Material);
        assert_eq!(p.header.source.as_ref().unwrap().licence, basics::LICENCE);
        p.scene.validate().unwrap();
        assert_eq!(p.scene.shader_tree.nodes[0].name, *name);
    }
    let dir = scratch("basics");
    let paths = basics::write_to(&dir).unwrap();
    assert_eq!(paths.len(), 6);
    let stamp = std::fs::metadata(&paths[0]).unwrap().modified().unwrap();
    // Rewriting the same bytes leaves the files alone (thumbnail cache).
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(dir.join("Stale.oppreset"), b"old").unwrap();
    basics::write_to(&dir).unwrap();
    assert_eq!(
        std::fs::metadata(&paths[0]).unwrap().modified().unwrap(),
        stamp
    );
    assert!(!dir.join("Stale.oppreset").exists());
    for p in &paths {
        let loaded = crate::persistence::load_preset(p).unwrap();
        assert_eq!(loaded.header.kind, crate::preset::PresetKind::Material);
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// 2026.11.1: an item's category is its folder below Starter/, written to
/// the manifest; an item without one (2026.11.0's layout) has none, and a
/// 2026.11.0 manifest with no category field still reads.
#[test]
fn categories_come_from_the_starter_subfolder() {
    assert_eq!(
        category_of("Starter/Metal/Metal049A/Metal049A.oppreset"),
        "Metal"
    );
    assert_eq!(category_of("Extras/Wood/Oak/Oak.oppreset"), "Wood");
    assert_eq!(category_of("Starter/Metal049A/Metal049A.oppreset"), "");
    assert_eq!(category_of("Other/Metal/A/A.oppreset"), "");

    let pack = scratch("categories");
    write_preset(
        &pack,
        "Starter/Metal/Metal049A/Metal049A.oppreset",
        cc0("https://ambientcg.com/a/Metal049A"),
    );
    write_preset(
        &pack,
        "Starter/Ground/Ground111/Ground111.oppreset",
        cc0("https://ambientcg.com/a/Ground111"),
    );
    write_preset(
        &pack,
        "Starter/Plain/Plain.oppreset",
        cc0("https://ambientcg.com/a/Plain"),
    );
    let m = build_manifest(&pack, "Operandi Library", "2026.11.1", "2026-11-11").unwrap();
    let cat = |p: &str| {
        m.items
            .iter()
            .find(|i| i.path.ends_with(p))
            .unwrap()
            .category
            .clone()
    };
    assert_eq!(cat("Metal049A.oppreset"), "Metal");
    assert_eq!(cat("Ground111.oppreset"), "Ground");
    assert_eq!(cat("Plain.oppreset"), "");
    let json = m.to_json();
    assert!(json.contains("\"category\": \"Metal\""), "{json}");
    assert_eq!(
        json.matches("\"category\"").count(),
        2,
        "no empty category: {json}"
    );
    assert_eq!(Manifest::parse(&json).unwrap(), m);
    assert!(credits_text(&m).contains("| Metal |"));
    // The licence gate is the same inside a category folder.
    write_preset(&pack, "Starter/Metal/Bad/Bad.oppreset", None);
    assert!(build_manifest(&pack, "Operandi Library", "2026.11.1", "2026-11-11").is_err());

    let old = r#"{"format":"operandi-library","format_version":1,"name":"Operandi Library",
        "version":"2026.11.0","items":[{"path":"Starter/A/A.oppreset","kind":"material",
        "licence":"CC0-1.0","source_url":"https://ambientcg.com/a/A","credit":"ambientCG",
        "checked":"2026-11-11"}]}"#;
    assert_eq!(Manifest::parse(old).unwrap().items[0].category, "");
    let _ = std::fs::remove_dir_all(&pack);
}

/// A picture at `pack/<rel>` (its bytes don't matter to the manifest).
fn write_picture(pack: &Path, rel: &str) {
    let path = pack.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"picture").unwrap();
}

#[test]
fn matcaps_and_environments_come_from_their_sources_lists() {
    let dir = scratch("pictures-ok");
    write_preset(
        &dir,
        "Starter/Brick/Brick.oppreset",
        cc0("https://a.org/brick"),
    );
    write_picture(&dir, "Matcaps/Clay/Clay Warm.png");
    write_picture(&dir, "Environments/Studio/Studio Small 08.hdr");
    write_picture(&dir, "Environments/Night/Night Sky 003.exr");
    // A hidden file is never an item.
    write_picture(&dir, "Environments/Night/._Night Sky 003.exr");
    std::fs::write(
        dir.join("Matcaps/SOURCES.tsv"),
        "# path\tlicence\turl\tcredit\nClay/Clay Warm.png\tCC0-1.0\thttps://a.org/clay\tOperandi\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Environments/SOURCES.tsv"),
        "Studio/Studio Small 08.hdr\tCC0-1.0\thttps://polyhaven.com/a/studio_small_08\tSergej Majboroda\t2026-11-12\n\
         Night/Night Sky 003.exr\tCC0-1.0\thttps://ambientcg.com/a/NightSkyHDRI003\tambientCG\n",
    )
    .unwrap();
    let m = build_manifest(&dir, "Operandi Library", "2026.11.4", "2026-11-11").unwrap();
    assert_eq!(m.items.len(), 4, "{:?}", m.items);
    let matcap = m.items.iter().find(|i| i.kind == "matcap").unwrap();
    assert_eq!(matcap.path, "Matcaps/Clay/Clay Warm.png");
    assert_eq!(matcap.category, "Clay");
    let studio = m
        .items
        .iter()
        .find(|i| i.path == "Environments/Studio/Studio Small 08.hdr")
        .unwrap();
    assert_eq!(studio.kind, "environmentImage");
    assert_eq!(studio.category, "Studio");
    assert_eq!(studio.credit, "Sergej Majboroda");
    assert_eq!(studio.checked, "2026-11-12", "its own check date");
    assert_eq!(matcap.checked, "2026-11-11", "the pack's");
    assert!(credits_text(&m).contains("Environments/Studio/Studio Small 08.hdr"));
    assert!(check(&m).is_empty());
    // The browser's tooltip finds a picture's own line.
    let pic = dir.join("Environments/Night/Night Sky 003.exr");
    assert_eq!(
        m.item_for(&dir, &pic).unwrap().source_url,
        "https://ambientcg.com/a/NightSkyHDRI003"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_picture_without_a_cc0_source_line_refuses_the_pack() {
    let dir = scratch("pictures-bad");
    write_picture(&dir, "Matcaps/Clay/Listed.png");
    write_picture(&dir, "Matcaps/Clay/Unlisted.png");
    write_picture(&dir, "Environments/Sky/By.hdr");
    std::fs::write(
        dir.join("Matcaps/SOURCES.tsv"),
        "Clay/Listed.png\tCC0-1.0\thttps://a.org/l\tA\nClay/Gone.png\tCC0-1.0\thttps://a.org/g\tA\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Environments/SOURCES.tsv"),
        "Sky/By.hdr\tCC-BY-4.0\thttps://a.org/by\tSomeone\n",
    )
    .unwrap();
    let problems = build_manifest(&dir, "Operandi Library", "1", "2026-11-12").unwrap_err();
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(
        problems
            .iter()
            .any(|p| p.contains("Unlisted.png") && p.contains("no line"))
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("Gone.png") && p.contains("not in the pack"))
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("By.hdr") && p.contains("CC0"))
    );
    // No sources list at all is refused too.
    std::fs::remove_file(dir.join("Matcaps/SOURCES.tsv")).unwrap();
    let problems = build_manifest(&dir, "Operandi Library", "1", "2026-11-12").unwrap_err();
    assert!(
        problems.iter().any(|p| p.contains("Matcaps/SOURCES.tsv")),
        "{problems:?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// 2026.11.5: Meshes/<Category>/<Name>.oppreset items come from their own
/// headers like Starter's, CC0 only.
#[test]
fn meshes_are_cc0_item_presets_with_a_category() {
    let pack = scratch("meshes");
    let mesh = &crate::preset::starter_meshes::ALL[0];
    let path = pack.join(MESHES).join(mesh.relative_path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    crate::persistence::save_preset(&path, &mesh.preset().unwrap()).unwrap();
    let manifest = build_manifest(&pack, "Operandi Library", "2026.11.5", "2026-11-13").unwrap();
    let item = &manifest.items[0];
    assert_eq!(item.path, format!("Meshes/{}", mesh.relative_path()));
    assert_eq!(item.category, mesh.category);
    assert_eq!(item.kind, "item");
    assert!(credits_text(&manifest).contains("Meshes/ is CC0"));
    // A mesh under another licence refuses the pack.
    let mut other = mesh.preset().unwrap();
    other.header.source.as_mut().unwrap().licence = "CC-BY-4.0".into();
    crate::persistence::save_preset(&pack.join("Meshes/Primitives/Other.oppreset"), &other)
        .unwrap();
    let problems =
        build_manifest(&pack, "Operandi Library", "2026.11.5", "2026-11-13").unwrap_err();
    assert!(
        problems.iter().any(|p| p.contains("Other.oppreset")),
        "{problems:?}"
    );
    std::fs::remove_dir_all(pack).ok();
}
