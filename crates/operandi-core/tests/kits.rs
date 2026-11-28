//! The kit contract, version 1 (docs/KITS.md): the manifest, the Kits
//! folder, installing, turning kits on, and importing through a kit's
//! helper program.
use operandi_core::interchange;
use operandi_core::kit_folder::{self, KitState};
use operandi_core::kit_import::{self, KitImporter};
use operandi_core::kit_manifest::{KIT_API_VERSION, KitManifest, this_platform};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The importer table is one per process: tests that set it take turns.
static TABLE: Mutex<()> = Mutex::new(());

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-kits-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn manifest(id: &str, ext: &str, helper: &str) -> String {
    format!(
        r#"
[kit]
id = "{id}"
name = "Test {id}"
version = "0.1.0"
api_version = 1
licence = "GPL-3.0-or-later"
authors = ["Tester"]

[[importer]]
label = "Test files"
extensions = ["{ext}"]
output = "ply"
timeout_seconds = 2

[importer.helper]
any = "{helper}"
"#
    )
}

#[test]
fn a_good_manifest_parses() {
    let m = KitManifest::parse(&manifest("toy_import", "toy", "bin/helper")).unwrap();
    assert_eq!(m.kit.id, "toy_import");
    assert_eq!(m.kit.api_version, KIT_API_VERSION);
    assert_eq!(m.importers.len(), 1);
    let imp = &m.importers[0];
    assert_eq!(imp.extensions, ["toy"]);
    assert_eq!(imp.helper_for(&this_platform()), Some("bin/helper"));
    assert_eq!(imp.timeout_seconds, 2);
}

#[test]
fn a_kit_for_a_newer_api_says_so() {
    let text = manifest("toy_import", "toy", "bin/helper")
        .replace("api_version = 1", "api_version = 99\nshiny_new_field = 1");
    let e = KitManifest::parse(&text).unwrap_err();
    assert!(e.contains("needs kit API 99"), "{e}");
}

#[test]
fn bad_manifests_are_refused_with_a_reason() {
    let good = manifest("toy_import", "toy", "bin/helper");
    let cases = [
        (good.replace("authors", "auhtors"), "unknown field"),
        (good.replace("toy_import", "Toy-Import"), "lower-case"),
        (good.replace("toy_import", "poly"), "own command namespaces"),
        (
            good.replace("toy_import", "operandi"),
            "own command namespaces",
        ),
        (
            good.replace("[\"toy\"]", "[\"obj\"]"),
            "read by Operandi itself",
        ),
        (good.replace("[\"toy\"]", "[\".toy\"]"), "without the dot"),
        (
            good.replace("[\"toy\"]", "[\"toy\", \"toy\"]"),
            "two importers",
        ),
        (
            good.replace("output = \"ply\"", "output = \"obj\""),
            "output must be",
        ),
        (
            good.replace("bin/helper", "../outside"),
            "inside the kit folder",
        ),
        (
            good.replace("bin/helper", "/usr/bin/true"),
            "inside the kit folder",
        ),
        (
            good.replace("any =", "beos-m68k ="),
            "Unknown helper platform",
        ),
        (
            good.replace("timeout_seconds = 2", "timeout_seconds = 0"),
            "timeout",
        ),
        (
            good.replace("licence = \"GPL-3.0-or-later\"", "licence = \" \""),
            "licence",
        ),
    ];
    for (text, want) in cases {
        let e = KitManifest::parse(&text).unwrap_err();
        assert!(e.contains(want), "wanted \"{want}\", got: {e}");
    }
}

#[test]
fn discover_lists_kits_and_the_ones_that_cannot_load() {
    let kits = temp_dir("discover");
    assert!(kit_folder::discover(&kits.join("missing")).is_empty());
    for (folder, id) in [("alpha", "alpha"), ("beta", "not_beta")] {
        std::fs::create_dir_all(kits.join(folder)).unwrap();
        std::fs::write(
            kits.join(folder).join("kit.toml"),
            manifest(id, "abc1", "h"),
        )
        .unwrap();
    }
    std::fs::create_dir_all(kits.join("broken")).unwrap();
    std::fs::create_dir_all(kits.join(".installing-x")).unwrap();
    let found = kit_folder::discover(&kits);
    let ids: Vec<String> = found.iter().map(|k| k.id()).collect();
    assert_eq!(ids, ["alpha", "beta", "broken"]);
    assert!(found[0].manifest.is_ok());
    assert!(
        found[1]
            .manifest
            .as_ref()
            .unwrap_err()
            .contains("folder named not_beta")
    );
    assert!(
        found[2]
            .manifest
            .as_ref()
            .unwrap_err()
            .contains("Can't read")
    );
}

#[test]
fn kit_state_round_trips_and_defaults_to_nothing_on() {
    let dir = temp_dir("state");
    let path = dir.join("kits.toml");
    assert_eq!(KitState::read(&path), KitState::default());
    std::fs::write(&path, "not toml [[[").unwrap();
    assert!(KitState::read(&path).enabled.is_empty());
    let mut state = KitState::default();
    state.enabled.insert("alpha".into());
    state.write(&path).unwrap();
    assert_eq!(KitState::read(&path), state);
}

/// A kit folder at `dir/<id>` holding `kit.toml` and a helper script.
fn make_kit(root: &Path, id: &str, ext: &str, script: &str) -> PathBuf {
    let dir = root.join(id);
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::write(dir.join("kit.toml"), manifest(id, ext, "bin/helper")).unwrap();
    std::fs::write(dir.join("bin/helper"), script).unwrap();
    dir
}

#[test]
fn install_copies_a_kit_folder_and_leaves_it_off() {
    let root = temp_dir("install");
    let source = make_kit(&root.join("src"), "gamma", "gam", "#!/bin/sh\n");
    let kits = root.join("Kits");
    assert_eq!(kit_folder::install(&source, &kits, false).unwrap(), "gamma");
    assert!(kits.join("gamma/kit.toml").is_file());
    assert!(kits.join("gamma/bin/helper").is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(kits.join("gamma/bin/helper"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "helper made runnable");
    }
    let e = kit_folder::install(&source, &kits, false).unwrap_err();
    assert!(e.contains("already installed"), "{e}");
    kit_folder::install(&source, &kits, true).unwrap();
    // Installing never turns a kit on, so nothing loads.
    let report = kit_import::load(&kit_folder::discover(&kits), &KitState::default());
    assert!(report.importers.is_empty() && report.problems.is_empty());
    // A folder without a manifest is refused and installs nothing.
    std::fs::create_dir_all(root.join("empty")).unwrap();
    assert!(kit_folder::install(&root.join("empty"), &kits, false).is_err());
    assert_eq!(kit_folder::discover(&kits).len(), 1);
}

#[test]
fn load_takes_only_kits_that_are_on() {
    let kits = temp_dir("load");
    make_kit(&kits, "delta", "dlt", "#!/bin/sh\n");
    make_kit(&kits, "epsilon", "dlt", "#!/bin/sh\n");
    std::fs::create_dir_all(kits.join("zeta")).unwrap();
    std::fs::write(
        kits.join("zeta/kit.toml"),
        manifest("zeta", "zet", "bin/nothing"),
    )
    .unwrap();
    let found = kit_folder::discover(&kits);
    let mut state = KitState::default();
    assert!(kit_import::load(&found, &state).importers.is_empty());
    state
        .enabled
        .extend(["delta".into(), "epsilon".into(), "zeta".into()]);
    let report = kit_import::load(&found, &state);
    assert_eq!(report.importers.len(), 1);
    assert_eq!(report.importers[0].kit_id, "delta");
    assert_eq!(report.problems.len(), 2, "{:?}", report.problems);
    assert!(report.problems[0].contains("already imported by another kit"));
    assert!(report.problems[1].contains("bin/nothing is missing"));
}

#[cfg(unix)]
mod helper_route {
    use super::*;

    const TRIANGLE_PLY: &str = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n";

    fn importer(tag: &str, ext: &str, script: &str) -> (PathBuf, KitImporter) {
        let root = temp_dir(tag);
        let id = format!("route_{ext}");
        let source = make_kit(&root.join("src"), &id, ext, script);
        let kits = root.join("Kits");
        kit_folder::install(&source, &kits, false).unwrap();
        let mut state = KitState::default();
        state.enabled.insert(id);
        let mut report = kit_import::load(&kit_folder::discover(&kits), &state);
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        let input = root.join(format!("Part 7.{ext}"));
        std::fs::write(&input, "toy file").unwrap();
        (input, report.importers.remove(0))
    }

    #[test]
    fn a_helper_converts_the_file_and_its_notes_reach_the_user() {
        let script = format!(
            "#!/bin/sh\n[ \"$1\" = import ] || exit 9\n[ \"$OPERANDI_KIT_PROTOCOL\" = 1 ] || exit 8\n[ -f \"$2\" ] || exit 7\necho 'note: Tolerance 0.1 mm'\necho 'chatter that is not a note'\ncat > \"$3\" <<'END'\n{TRIANGLE_PLY}END\n"
        );
        let (input, imp) = importer("ok", "tri", &script);
        let imported = kit_import::run(&imp, &input).unwrap();
        assert_eq!(imported.scene.items.len(), 1);
        assert_eq!(imported.scene.items[0].mesh.faces.len(), 1);
        assert!(
            imported.scene.items[0].name.contains("Part 7"),
            "{}",
            imported.scene.items[0].name
        );
        assert_eq!(
            imported.notes.first().map(String::as_str),
            Some("Tolerance 0.1 mm")
        );
        assert!(!imported.notes.iter().any(|n| n.contains("chatter")));
    }

    #[test]
    fn import_goes_through_the_kit_table() {
        let _turn = TABLE.lock().unwrap_or_else(|e| e.into_inner());
        let script = format!("#!/bin/sh\ncat > \"$3\" <<'END'\n{TRIANGLE_PLY}END\n");
        let (input, imp) = importer("table", "tbl", &script);
        assert!(!interchange::is_importable(&input));
        assert!(interchange::import(&input).is_err());
        kit_import::set_kit_importers(vec![imp]);
        assert!(interchange::is_importable(&input));
        assert!(
            interchange::import_extensions()
                .iter()
                .any(|(e, l)| e == "tbl" && l == "Test files")
        );
        let imported = interchange::import(&input).unwrap();
        assert_eq!(imported.scene.items.len(), 1);
        kit_import::set_kit_importers(Vec::new());
        assert!(!interchange::is_importable(&input));
    }

    #[test]
    fn a_failing_helper_reports_its_last_error_lines() {
        let (input, imp) = importer(
            "fail",
            "bad",
            "#!/bin/sh\necho 'reading...' >&2\necho 'Bad STEP header' >&2\nexit 3\n",
        );
        let e = kit_import::run(&imp, &input).unwrap_err();
        assert!(
            e.contains("exit code 3") && e.contains("Bad STEP header"),
            "{e}"
        );
    }

    #[test]
    fn a_helper_that_writes_nothing_fails() {
        let (input, imp) = importer("empty", "nil", "#!/bin/sh\nexit 0\n");
        let e = kit_import::run(&imp, &input).unwrap_err();
        assert!(e.contains("wrote no file"), "{e}");
    }

    #[test]
    fn a_helper_that_hangs_is_stopped() {
        let (input, mut imp) = importer("hang", "slo", "#!/bin/sh\nexec sleep 30\n");
        imp.timeout = std::time::Duration::from_millis(300);
        let started = std::time::Instant::now();
        let e = kit_import::run(&imp, &input).unwrap_err();
        assert!(e.contains("was stopped"), "{e}");
        assert!(started.elapsed().as_secs() < 10);
    }
}

/// The example kit end to end: build and pack it, install the zip, turn
/// it on, import its cube. Needs cargo and zip, so it's opt-in:
/// `cargo test --locked --workspace -- kits::the_example_kit --ignored`.
#[cfg(unix)]
#[test]
#[ignore = "builds the example kit's helper; run by hand (see the comment)"]
fn the_example_kit_imports_its_cube() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = temp_dir("example");
    let packed = std::process::Command::new(repo.join("scripts/package-kit.sh"))
        .arg(repo.join("examples/kits/off_import"))
        .arg(&out)
        .status()
        .unwrap();
    assert!(packed.success());
    let zip = std::fs::read_dir(&out)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "zip"))
        .expect("a kit zip");
    let kits = out.join("Kits");
    assert_eq!(
        kit_folder::install(&zip, &kits, false).unwrap(),
        "off_import"
    );
    let mut state = KitState::default();
    state.enabled.insert("off_import".into());
    let report = kit_import::load(&kit_folder::discover(&kits), &state);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    let imported = kit_import::run(
        &report.importers[0],
        &repo.join("examples/kits/off_import/cube.off"),
    )
    .unwrap();
    assert_eq!(imported.scene.items.len(), 1);
    assert_eq!(imported.scene.items[0].mesh.faces.len(), 6);
    assert_eq!(imported.notes[0], "Read 8 points and 6 polygons");
}
