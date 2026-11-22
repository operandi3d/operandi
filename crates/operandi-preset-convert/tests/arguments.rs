mod support;

use support::{Scratch, command};

#[test]
fn missing_values_and_unknown_options_report_argument_errors() {
    let scratch = Scratch::new();
    for flag in [
        "--out",
        "--graph",
        "--material",
        "--set",
        "--licence",
        "--license",
        "--url",
        "--credit",
    ] {
        let output = command().arg(scratch.path()).arg(flag).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{flag}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        let name = if flag == "--license" {
            "--licence"
        } else {
            flag
        };
        assert_eq!(stderr.trim(), format!("{name} needs a value"));
    }
    let output = command().arg("--unknown").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("unknown option --unknown\nusage:"));
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}

#[test]
fn missing_folder_missing_output_and_help_print_usage() {
    let scratch = Scratch::new();
    let output_path = scratch.path().join("unused.oppreset");
    let outputs = [
        command().output().unwrap(),
        command().arg("--out").arg(&output_path).output().unwrap(),
        command().arg(scratch.path()).output().unwrap(),
        command().arg("--help").output().unwrap(),
        command().arg("-h").output().unwrap(),
    ];
    for output in outputs {
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .starts_with("usage:")
        );
    }
    assert!(!output_path.exists());
}

#[test]
fn pack_steps_reject_missing_arguments_and_unknown_steps_or_options() {
    let scratch = Scratch::new();
    for missing in ["--name", "--version", "--checked"] {
        let mut cmd = command();
        cmd.arg("pack-manifest").arg(scratch.path());
        for (flag, value) in [
            ("--name", "Test"),
            ("--version", "1"),
            ("--checked", "2026-11-18"),
        ] {
            if flag != missing {
                cmd.args([flag, value]);
            }
        }
        let output = cmd.output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .starts_with("usage:")
        );
        let output = command()
            .arg("pack-manifest")
            .arg(scratch.path())
            .arg(missing)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            String::from_utf8(output.stderr).unwrap().trim(),
            format!("{missing} needs a value")
        );
    }
    for (args, message) in [
        (vec!["pack-manifest"], "usage:"),
        (
            vec![
                "pack-manifest",
                "--name",
                "Test",
                "--version",
                "1",
                "--checked",
                "2026-11-18",
            ],
            "usage:",
        ),
        (
            vec!["pack-manifest", "--unknown"],
            "unknown option --unknown",
        ),
        (vec!["pack-sizes"], "usage:"),
        (vec!["pack-unknown"], "unknown step pack-unknown"),
    ] {
        let output = command().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .starts_with(message)
        );
    }
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}
