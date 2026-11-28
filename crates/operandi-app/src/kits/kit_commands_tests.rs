//! `operandi.kit.*` from the command line: install, turn on, import, turn
//! off (docs/KITS.md).
use super::*;
use std::path::{Path, PathBuf};

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-kitcmd-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_kit(dir: &Path) {
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    std::fs::write(
        dir.join("kit.toml"),
        "[kit]\nid = \"cmd_test\"\nname = \"Command Test\"\nversion = \"0.2.0\"\napi_version = 1\nlicence = \"GPL-3.0-or-later\"\n\n[[importer]]\nlabel = \"Command test files\"\nextensions = [\"cmdt\"]\noutput = \"ply\"\n\n[importer.helper]\nany = \"bin/helper\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("bin/helper"),
        "#!/bin/sh\ncat > \"$3\" <<'END'\nply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\nEND\n",
    )
    .unwrap();
}

#[test]
fn a_kit_installs_off_turns_on_imports_and_turns_off() {
    let root = temp("flow");
    let data = root.join("data");
    std::fs::create_dir_all(&data).unwrap();
    write_kit(&root.join("cmd_test"));
    let mut a = Operandi::empty_at(data.clone());
    let input = root.join("part.cmdt");
    std::fs::write(&input, "x").unwrap();

    a.execute("operandi.kit.list");
    assert!(a.status.contains("No kits installed"), "{}", a.status);

    a.execute(&format!(
        "operandi.kit.install path:\"{}\"",
        root.join("cmd_test").display()
    ));
    assert!(
        a.status.contains("Installed kit cmd_test, off"),
        "{}",
        a.status
    );
    assert!(data.join("Kits/cmd_test/kit.toml").is_file());
    assert!(
        !operandi_core::interchange::is_importable(&input),
        "off until turned on"
    );

    a.execute("operandi.kit.list");
    assert!(
        a.status.contains("cmd_test 0.2.0 (off, imports .cmdt)"),
        "{}",
        a.status
    );

    a.execute("operandi.kit.enable id:nope");
    assert!(a.status.contains("No kit nope"), "{}", a.status);
    a.execute("operandi.kit.enable id:cmd_test");
    assert!(
        a.status.contains("File > Import reads .cmdt"),
        "{}",
        a.status
    );
    assert!(operandi_core::interchange::is_importable(&input));
    #[cfg(unix)]
    {
        let imported = operandi_core::interchange::import(&input).unwrap();
        assert_eq!(imported.scene.items.len(), 1);
    }

    // The choice is kept: a new start reads it back.
    let mut b = Operandi::empty_at(data.clone());
    b.load_kits();
    assert!(operandi_core::interchange::is_importable(&input));

    b.execute("operandi.kit.disable id:cmd_test");
    assert!(b.status.contains("Kit cmd_test is off"), "{}", b.status);
    assert!(!operandi_core::interchange::is_importable(&input));
    b.execute("operandi.kit.disable id:cmd_test");
    assert!(b.status.contains("already off"), "{}", b.status);
}
