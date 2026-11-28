//! The command table's own checks (names, examples, sorted lists).
/// Contributor rule (the maintainer, 2026-11-19): one added line per new
/// tool phrase, in sorted order, so branches don't clash.
#[test]
fn tool_phrases_stay_sorted() {
    let ids: Vec<String> = super::TOOL_PHRASES
        .iter()
        .map(|(id, _)| id.to_lowercase())
        .collect();
    if let Some(i) = ids.windows(2).position(|w| w[0] > w[1]) {
        panic!(
            "command_registry/tool_ids.rs: TOOL_PHRASES must stay in sorted order \
             (case-insensitive); `{}` belongs before `{}`",
            super::TOOL_PHRASES[i + 1].0,
            super::TOOL_PHRASES[i].0
        );
    }
}

use super::*;
use std::collections::BTreeSet;

#[test]
fn names_are_unique_and_examples_start_with_their_entry() {
    let mut seen = BTreeSet::new();
    for e in COMMANDS {
        for n in e.names() {
            assert!(seen.insert(n.to_ascii_lowercase()), "{n} is listed twice");
        }
        let (found, _) = find_line(e.example)
            .unwrap_or_else(|| panic!("{}: example {:?} names nothing", e.name, e.example));
        assert_eq!(found.name, e.name, "example {:?}", e.example);
        for a in e.args {
            assert!(!a.name.is_empty());
            if let (Some(d), ArgType::Choice(c)) = (a.default, a.ty) {
                assert!(c.contains(&d), "{}: default {d} isn't a choice", e.name);
            }
        }
    }
}

/// L5: every old command name in the alias table reaches an entry, and
/// says it's deprecated.
#[test]
fn every_old_command_name_resolves_and_is_deprecated() {
    for (old, new) in command_names::COMMAND_RENAMES {
        let e = find(old).unwrap_or_else(|| panic!("{old} → {new} has no entry"));
        assert!(e.names().any(|n| n.eq_ignore_ascii_case(new)), "{old}");
        assert!(deprecation(old).unwrap().contains(new));
    }
    for (old, new, _) in command_names::COMMAND_EXPANSIONS {
        let e = find(old).unwrap_or_else(|| panic!("{old} → {new} has no entry"));
        assert!(e.names().any(|n| n.eq_ignore_ascii_case(new)), "{old}");
    }
    for (old, new) in command_names::TOOL_RENAMES {
        assert!(
            TOOL_IDS.iter().any(|t| t.eq_ignore_ascii_case(new)),
            "tool {old} → {new} isn't a tool.set id"
        );
        assert!(deprecation(old).is_some(), "{old}");
    }
    for (star, _) in command_names::STAR_TOOLS {
        assert!(TOOL_IDS.contains(&star), "{star}");
    }
    assert_eq!(deprecation("poly.flip"), None);
}

/// Every name in the admitted Modo name lists
/// (specifications/command-names): the subset's command names, the
/// button invocations' commands and tool ids, and the `view3d.*` names
/// listed without their prefix.
fn map_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for text in [
        include_str!("../../../../specifications/command-names/MODO-NAMES-SUBSET.txt"),
        include_str!("../../../../specifications/command-names/MODO-BUTTON-INVOCATIONS.txt"),
    ] {
        for line in text.lines() {
            let view3d = line.starts_with("view3d.");
            for word in line
                .split(|c: char| !(c.is_ascii_alphanumeric() || "._@*".contains(c)))
                .filter(|w| !w.is_empty())
            {
                names.insert(word.to_ascii_lowercase());
                if view3d && !word.contains('.') {
                    names.insert(format!("view3d.{}", word.to_ascii_lowercase()));
                    // "view3d.TransparencyOverrideSolidAlphaBack /
                    // SolidAlphaFront" lists the second one short.
                    names.insert(format!(
                        "view3d.transparencyoverride{}",
                        word.to_ascii_lowercase()
                    ));
                }
            }
        }
    }
    names
}

/// Names that pass UIC-165 without a line in
/// specifications/command-names (the registry thread's names moved
/// into MODO-NAMES-SUBSET.txt on 2026-11-11).
/// transform.reset and transform.freeze are named by the admitted spec
/// freeze-transforms-2026-11-14 (section 8); layout.restore by
/// layouts-2026-11-16 (LAY-30); the history commands by
/// ui-architecture-2026-11-16 (CMD-40, HIS-11, HIS-22, HIS-50).
const RECORDED: &[&str] = &[
    "transform.reset",
    "transform.freeze",
    "layout.restore",
    "cmds.batch",
    "cmds.undoTo",
    "history.clear",
    "history.max",
    "history.resultsMax",
];

/// UIC-165: every command is a Modo name (or a phrase whose Modo
/// invocation is known), or is marked Operandi-only on purpose. A new
/// command fails here until it is matched or marked.
#[test]
fn every_command_has_a_modo_name_or_is_marked_operandi_only() {
    let map = map_names();
    let known = |name: &str| {
        map.contains(&name.to_ascii_lowercase())
            || RECORDED.iter().any(|r| r.eq_ignore_ascii_case(name))
    };
    let mut unmatched = Vec::new();
    let mut operandi = Vec::new();
    for e in COMMANDS {
        if e.flags.operandi {
            operandi.push(e.name);
            assert!(
                !(e.kind == Kind::Script && known(e.name)),
                "{} is a Modo name; don't mark it Operandi-only",
                e.name
            );
            continue;
        }
        let covered = match (e.kind, e.modo) {
            (Kind::Script, _) => known(e.name),
            (Kind::Action, Some(modo)) => {
                let mut words = modo.split_whitespace();
                let head = words.next().unwrap_or("");
                let id = words
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c| c == '{' || c == '}' || c == '"');
                known(head) && (head != "tool.set" || known(id) || TOOL_IDS.contains(&id))
            }
            (Kind::Action, None) => false,
        };
        if !covered {
            unmatched.push(e.name);
        }
    }
    println!(
        "Operandi-only ({}): {}",
        operandi.len(),
        operandi.join(", ")
    );
    assert!(
        unmatched.is_empty(),
        "UIC-165: no Modo name and not marked Operandi-only: {unmatched:?}"
    );
}

/// Every phrase's Modo invocation names a registered command.
#[test]
fn modo_invocations_name_registered_commands() {
    for e in COMMANDS {
        if let Some(modo) = e.modo {
            assert!(find_line(modo).is_some(), "{}: {modo}", e.name);
        }
    }
    assert_eq!(tool_phrase("util.ruler"), Some("Ruler"));
    for (_, phrase) in TOOL_PHRASES {
        assert!(find(phrase).is_some(), "{phrase}");
    }
}

/// PD-Q12's three commands, and only they, join a live transform tool.
#[test]
fn three_commands_join_a_live_transform_tool() {
    let names: BTreeSet<&str> = COMMANDS
        .iter()
        .filter(|e| e.flags.allowed_during_transform_tool)
        .map(|e| e.name)
        .collect();
    assert_eq!(
        names,
        BTreeSet::from(["select.item", "transform.channel", "item.transform"])
    );
}

/// The headless flag and the JSON arguments come together, and every
/// JSON argument name is unique within its command.
#[test]
fn headless_entries_carry_their_arguments() {
    for e in COMMANDS {
        assert_eq!(e.flags.headless, e.json.is_some(), "{}", e.name);
        if let Some(json) = e.json {
            let names: BTreeSet<&str> = json.args.iter().map(|a| a.name).collect();
            assert_eq!(names.len(), json.args.len(), "{}", e.name);
            assert!(!json.about.is_empty(), "{}", e.name);
        }
    }
    assert_eq!(crate::command_exec::specs().count(), 354);
}

#[test]
fn lines_pick_the_longest_name() {
    assert_eq!(find_line("spike reset").unwrap().0.name, "Spike Reset");
    assert_eq!(find_line("Spike").unwrap().0.name, "Spike");
    let (e, rest) = find_line("smooth shift offset 0.1").unwrap();
    assert_eq!((e.name, rest), ("Smooth Shift", "offset 0.1"));
    assert_eq!(find_line("selset.use A").unwrap().0.name, "select.useSet");
    assert!(find_line("no.such.command").is_none());
}

#[test]
fn list_filters_by_name_or_area() {
    assert_eq!(list("").len(), COMMANDS.len());
    assert!(list("sculpt").iter().all(|l| l.contains("sculpt")));
    assert!(list("poly.flip")[0].contains("poly.flip"));
}
