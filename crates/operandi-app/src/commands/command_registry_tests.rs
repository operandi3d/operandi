//! The command registry (operandi-core `command_registry.rs`) held against
//! the typed parser in both directions, and the UIC-164 check: every menu
//! entry, toolbox button and property row runs a command, or is a known
//! gap listed in `command_registry_gaps.rs`.
use super::tests::{box_app, draw};
use super::*;
use crate::command_registry_gaps::{CHROME_EXEMPT, KNOWN_GAPS};
use operandi_core::command_registry::{self as registry, ArgType, TOOL_IDS};
use std::collections::{BTreeMap, BTreeSet};

const SIZE: Vec2 = vec2(1440., 900.);

/// What the parser says when it doesn't know a line at all.
fn unknown(status: &str) -> bool {
    status.starts_with("Unknown command")
        || status.starts_with("Unknown help topic")
        || status == "Unsupported command or arguments; use help"
}

/// The lines that should run `entry`: every name of a command without
/// required arguments, else its example under each name.
fn lines(entry: &registry::Entry) -> Vec<String> {
    if entry.args.iter().all(|a| a.optional) {
        return entry.names().map(str::to_owned).collect();
    }
    let rest = &entry.example[entry.name.len()..];
    assert!(
        entry.example[..entry.name.len()].eq_ignore_ascii_case(entry.name),
        "{}: example should start with the name",
        entry.name
    );
    // A Modo-named command's phrases (UIC-165) take no arguments.
    entry
        .names()
        .map(|n| {
            if n != entry.name && entry.spelled.is_some() {
                n.to_owned()
            } else {
                format!("{n}{rest}")
            }
        })
        .collect()
}

/// Registry → parser: every name (and example) is understood, and has
/// help. The run-time registry: core's table and the app's own commands.
#[test]
fn every_registry_command_is_understood_by_the_parser() {
    let mut failures = Vec::new();
    let commands = command_handlers::registry();
    for entry in commands.iter().map(|c| &c.entry) {
        for line in lines(entry) {
            let mut a = Operandi::empty();
            a.execute(&line);
            if unknown(&a.status) {
                failures.push(format!("{line:?}: {}", a.status));
            }
        }
        let mut a = Operandi::empty();
        a.execute(&format!("help {}", entry.name));
        if unknown(&a.status) || a.status.is_empty() {
            failures.push(format!("help {}: {}", entry.name, a.status));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every `tool.set` id in the registry starts a tool the parser knows.
#[test]
fn every_registry_tool_id_is_a_tool() {
    let mut expected: BTreeSet<String> = command::IDS
        .iter()
        .map(|(id, _)| id.to_ascii_lowercase())
        .collect();
    // Handled before the IDS table: Pen, the Edge click tools, Drag Weld,
    // the sculpt brushes and the mode-picked *.Slide.
    for id in [
        "prim.pen",
        "solid.sketch",
        "edge.addLoop",
        "edge.addPoint",
        "DragWeld",
        "*.Slide",
    ] {
        expected.insert(id.to_ascii_lowercase());
    }
    for brush in operandi_core::SculptBrush::every() {
        expected.insert(brush.command().to_ascii_lowercase());
    }
    // Falloffs added from the Add Falloff menu (VMF-10).
    for id in [
        "falloff.vertexMap",
        "falloff.invert",
        "falloff.selection",
        "falloff.softSelection",
        "falloff.element",
        "falloff.coplanar",
        "falloff.screen",
        "falloff.lasso",
        "falloff.airbrush",
        "falloff.noise",
        "falloff.image",
        "falloff.path",
        "falloff.curvature",
    ] {
        expected.insert(id.to_ascii_lowercase());
    }
    let listed: BTreeSet<String> = TOOL_IDS.iter().map(|t| t.to_ascii_lowercase()).collect();
    assert_eq!(listed, expected);
    for id in TOOL_IDS.iter().filter(|id| !id.starts_with("*.Slide")) {
        let mut a = box_app();
        // Every id may be grouped with braces, as in scripts.
        let id = format!("{{{id}}}");
        a.execute(&format!("tool.set {id} ?"));
        assert!(
            a.status == "0" || a.status == "1",
            "tool.set {id} ?: {}",
            a.status
        );
    }
}

/// The string literals in `source` between `start` and `end`; with
/// `arms`, only those before a `=>` on their line (match arm patterns).
fn literals(source: &str, start: &str, end: &str, arms: bool) -> Vec<String> {
    let from = source
        .find(start)
        .unwrap_or_else(|| panic!("marker {start:?} moved"));
    let to = from
        + source[from..]
            .find(end)
            .unwrap_or_else(|| panic!("marker {end:?} moved"));
    let mut out = Vec::new();
    for line in source[from..to].lines() {
        let line = if arms {
            // A pattern: before `=>`, or a line of a multi-line `"a" | "b"`.
            match line.find("=>") {
                Some(i) => &line[..i],
                None if line.trim_start().starts_with('"') && line.trim_end().ends_with('|') => {
                    line
                }
                None => continue,
            }
        } else {
            line
        };
        let mut parts = line.split('"');
        parts.next();
        while let (Some(literal), Some(_)) = (parts.next(), parts.next()) {
            out.push(literal.to_owned());
        }
    }
    out
}

/// Parser → registry: every name the parser's tables and match arms
/// accept is in the registry. A new parser arm or table row without a
/// registry entry fails here.
#[test]
fn every_parser_name_is_in_the_registry() {
    let mut names: Vec<String> = Vec::new();
    // The prefixes `execute` checks itself. (Its old typed `match` became
    // registry handlers in command_handlers.rs, which attach by registry
    // name, so they can't name a missing command.)
    let main = include_str!("../main.rs");
    names.extend(
        literals(
            main,
            "if self.canonical_command(text)",
            "if self.headless_command(text)",
            false,
        )
        .into_iter()
        .filter(|l| l.ends_with(' ') || l.starts_with("operandi.")),
    );
    // The script verbs `canonical_command` takes.
    names.extend(literals(
        include_str!("command.rs"),
        "if !matches!(",
        ") && !EdgeCommand::ALL",
        false,
    ));
    names.extend(literals(
        include_str!("../polygons/subdivide_ui.rs"),
        "let args: Vec<&str> = words.collect();",
        "_ => return false,\n        }\n        true",
        true,
    ));
    // Sculpt's lock verbs (parsed in core's command_line).
    names.extend(literals(
        include_str!("../../../operandi-core/src/commands/command_line.rs"),
        "let corners = match first.as_str() {",
        "_ => return None,",
        true,
    ));
    // The pie phrases' prefixes ("view ", "toggle "); their words are
    // checked in `pie_phrase_choices_match_the_registry`.
    names.extend(
        literals(
            include_str!("../menus/pie_menu.rs"),
            "fn execute_pie_command",
            "fn pie_enabled",
            false,
        )
        .into_iter()
        .filter(|l| l.ends_with(' ')),
    );
    // Tables.
    names.extend(command::IDS.iter().map(|(_, n)| n.to_string()));
    for tool in Tool::ALL {
        names.extend(tool_info::info(tool).names.iter().map(|n| n.to_string()));
    }
    names.extend(
        operandi_core::tool_preset::factory()
            .iter()
            .filter(|r| !r.layers.is_empty() && r.tool().is_some())
            .flat_map(|r| r.names.iter().cloned()),
    );
    names.extend(
        vertex_panel::VertexRow::ALL
            .iter()
            .filter(|r| r.owns_action())
            .map(|r| r.action().to_owned()),
    );
    for c in EdgeCommand::ALL {
        names.push(c.verb().to_owned());
        names.push(edge_panel::edge_action(c).to_owned());
    }
    for brush in operandi_core::SculptBrush::ALL {
        names.push(brush.command().to_owned());
        names.push(format!("Sculpt {}", brush.label()));
    }
    for tip in [
        operandi_core::sculpt::TipKind::Smooth,
        operandi_core::sculpt::TipKind::Hard,
    ] {
        names.push(tip.command().to_owned());
    }
    names.extend(vertex_panel::VERBS.iter().map(|v| v.to_string()));
    names.extend(backdrop_ui::COMMANDS.iter().map(|(c, _)| c.to_string()));
    names.extend(item_list_ui::COMMANDS.iter().map(|(c, _)| c.to_string()));
    names.extend(
        shading_panel::IMAGE_COMMANDS
            .iter()
            .map(|(c, _)| c.to_string()),
    );
    names.extend(
        shading_panel::NODE_COMMANDS
            .iter()
            .map(|(c, _)| c.to_string()),
    );
    names.extend(preset_ui::COMMANDS.iter().map(|(c, _)| c.to_string()));
    names.extend(materialx_ui::COMMANDS.iter().map(|(c, _)| c.to_string()));
    names.extend(
        selection_set_dialog::COMMANDS
            .iter()
            .map(|(c, _)| c.to_string()),
    );
    names.extend(pie_menu::OLD_ACTIONS.iter().map(|(c, _, _)| c.to_string()));
    names.extend(input::NAV_LOCK_HIDE_ACTIONS.iter().map(|c| c.to_string()));
    // The Input Map: every action is a command, except the Image Viewer's
    // keys, which only that window handles.
    names.extend(
        input::ACTIONS
            .iter()
            .map(|(a, _)| a.to_string())
            .filter(|a| !a.starts_with("Image Viewer:")),
    );
    let missing: BTreeSet<String> = names
        .iter()
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty() && registry::find_line(n).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "accepted by the parser but missing from command_registry.rs: {missing:?}"
    );
}

/// The sub-parsers `execute` and `canonical_command` hand lines to. Each
/// one's names are read in `every_parser_name_is_in_the_registry`; a new
/// sub-parser fails here until that test reads its table too.
#[test]
fn every_sub_parser_is_read_by_the_registry_test() {
    let mut calls: Vec<String> = Vec::new();
    for (source, start, end) in [
        (
            include_str!("command.rs"),
            "fn canonical_command",
            "let first = trimmed",
        ),
        (
            include_str!("../main.rs"),
            "fn execute(&mut self, text: &str)",
            "if self.headless_command(text)",
        ),
    ] {
        let from = source.find(start).unwrap();
        let to = from + source[from..].find(end).unwrap();
        for line in source[from..to].lines() {
            if let Some(rest) = line.trim().strip_prefix("if self.")
                && let Some((name, _)) = rest.split_once('(')
                && !name.contains('.')
            {
                calls.push(name.to_owned());
            }
        }
    }
    assert_eq!(
        calls,
        [
            "subdivide_canonical",
            "vertex_canonical",
            "pref_value_command",
            "view3d_canonical",
            "item_list_canonical",
            "backdrop_canonical",
            "image_canonical",
            "file_canonical",
            "preset_canonical",
            "selection_set_canonical",
            "run_registered",
            "sculpt_execute",
            "canonical_command",
            "execute_bridge_parameter",
            "execute_pie_command",
        ]
    );
}

/// The pie prefixes' word lists match the registry's choices.
#[test]
fn pie_phrase_choices_match_the_registry() {
    let pie = include_str!("../menus/pie_menu.rs");
    for (entry, start, end) in [
        ("View", "fn view_named", "_ => return None"),
        ("Shading", "fn style_named", "_ => return None"),
        ("Action Center", "fn center_named", "fn mode_named"),
        ("Toggle", "fn toggle_state", "_ => return None"),
    ] {
        let ArgType::Choice(choices) = registry::find(entry).unwrap().args[0].ty else {
            panic!("{entry} takes a choice");
        };
        let mut words: BTreeSet<String> = literals(pie, start, end, true).into_iter().collect();
        if entry == "View" {
            words.insert("camera".into());
        }
        let listed: BTreeSet<String> = choices.iter().map(|c| c.to_string()).collect();
        assert_eq!(listed, words, "{entry}");
    }
}

/// L5: old names still run, as their new command.
#[test]
fn deprecated_names_still_run() {
    for (old, _) in operandi_core::command_names::COMMAND_RENAMES {
        let entry = registry::find(old).unwrap();
        let mut a = Operandi::empty();
        let line = lines(entry)[0].replacen(entry.name, old, 1);
        a.execute(&line);
        assert!(!unknown(&a.status), "{line}: {}", a.status);
    }
}

/// Stage 2: commands that run without a window are typed like any other
/// (core command_line + command_exec), as one staged step.
#[test]
fn headless_commands_run_from_the_command_line() {
    let mut a = box_app();
    let undo = a.editor.history.undo.len();
    a.execute("item.rename {Road Sign}");
    assert_eq!(a.status, "Ready");
    assert_eq!(a.editor.scene.active().unwrap().name, "Road Sign");
    assert_eq!(a.editor.history.undo.len(), undo + 1);
    a.execute("mesh.new name:Second");
    assert_eq!(a.editor.scene.active().unwrap().name, "Second");
    a.run(Command::Undo);
    let before = a.editor.clone();
    a.execute("mesh.new name:{  }");
    assert_eq!(a.status, "Name must contain 1–1024 bytes");
    assert!(a.editor == before);
    a.run(Command::Redo);
    assert_eq!(a.editor.scene.active().unwrap().name, "Second");

    let mut mcp = operandi_mcp::Bridge::new();
    let named = |name: &str| serde_json::json!({"command": "mesh.new", "args": {"name": name}});
    mcp.call("command_run", &named("Bridge Mesh")).unwrap();
    mcp.editor.dispatch(Command::Undo).unwrap();
    let before = mcp.editor.clone();
    assert!(mcp.call("command_run", &named("  ")).is_err());
    assert!(mcp.editor == before);
    mcp.editor.dispatch(Command::Redo).unwrap();
    assert_eq!(mcp.editor.scene.active().unwrap().name, "Bridge Mesh");
    // item.channel, typed: set and ? read back.
    a.execute("item.create light.point");
    a.execute("item.channel intensity 4");
    a.execute("item.channel intensity ?");
    assert_eq!(a.status, "4");
    a.execute("item.channel brightness 1");
    assert!(a.status.contains("intensity"), "{}", a.status);
    // item.name renames a shader-tree layer with layer:.
    let layer = a.editor.scene.shader_tree.nodes[3].id;
    a.execute(&format!("item.name {{Clay Base}} layer:{layer}"));
    assert_eq!(
        a.editor.scene.shader_tree.get(layer).unwrap().name,
        "Clay Base",
        "{}",
        a.status
    );
    // The Rotation Order row's command.
    a.execute("item.channel rotOrder ZXY");
    assert!(
        a.editor.scene.active().unwrap().transform.zxy,
        "{}",
        a.status
    );
    a.execute("symmetry.set axis:x");
    assert_eq!(a.editor.symmetry.axis, Some(0));
    let id = a.editor.scene.items[0].id;
    a.execute(&format!("item.transform item:{id} translation:1,2,3"));
    let t = &a.editor.scene.items[0].transform.translation;
    assert_eq!([t.x, t.y, t.z], [1., 2., 3.]);
    // Errors leave the scene as it was.
    let before = a.editor.scene.clone();
    a.execute("item.transform translation:1,2");
    assert!(a.status.contains("translation"), "{}", a.status);
    assert_eq!(a.editor.scene, before);
    a.execute("item.rename");
    assert!(a.status.contains("name"), "{}", a.status);
    // An open dialog is the person's: refused, as over MCP.
    a.show_preferences = true;
    a.execute("item.rename Other");
    assert_ne!(a.editor.scene.active().unwrap().name, "Other");
    assert!(a.status.contains("Preferences window"), "{}", a.status);
}

/// Stage 2 fixes: phrases ignore case everywhere, and sculpt brush ids
/// take braces like every other tool id.
#[test]
fn stage_two_parser_fixes() {
    let mut a = Operandi::empty();
    a.execute("OPEN /nonexistent/scene.opscene");
    assert!(!unknown(&a.status), "{}", a.status);
    a.execute("open render window");
    assert!(a.render.open, "{}", a.status);
    a.execute("Spike Strength 1");
    assert!(!unknown(&a.status), "{}", a.status);
    let mut a = box_app();
    a.execute("tool.set {sculpt.push} on");
    a.execute("tool.set {sculpt.push} ?");
    assert_eq!(a.status, "1");
}

/// UIC-165: Modo names run the same operation as their phrase.
#[test]
fn modo_names_run_their_phrases() {
    let mut a = box_app();
    a.execute("select.all");
    let all = a.editor.scene.selection.clone();
    a.execute("Invert");
    a.execute("Select All");
    assert_eq!(a.editor.scene.selection, all);
    a.execute("select.convert edge");
    assert_eq!(a.editor.scene.selection.mode, Mode::Edge);
    a.execute("select.convert type:polygon");
    assert_eq!(a.editor.scene.selection.mode, Mode::Polygon);
    a.execute("select.convert nonsense");
    assert!(a.status.contains("vertex, edge or polygon"), "{}", a.status);
    let undo = a.editor.history.undo.len();
    a.execute("item.name Renamed");
    assert_eq!(a.editor.history.undo.len(), undo + 1);
    a.execute("app.undo");
    assert_eq!(a.editor.history.undo.len(), undo);
    a.execute("app.undo steps:2");
    assert!(
        a.status.contains("arguments aren't supported"),
        "{}",
        a.status
    );
    a.execute("cmds.showHelp command:poly.remove");
    assert!(a.status.contains("Shift+Backspace"), "{}", a.status);
    // Old names still run, under their new command.
    a.execute("item.add mesh");
    assert!(!unknown(&a.status), "{}", a.status);
}

/// view3d.*: Viewport Properties settings by command, read back with ?.
#[test]
fn view3d_commands_set_and_read_viewport_settings() {
    let mut a = box_app();
    a.execute("view3d.showLights false");
    assert!(!a.display.settings.drawing.lights, "{}", a.status);
    a.execute("view3d.showLights ?");
    assert_eq!(a.status, "false");
    a.execute("view3d.wireframeAlpha 0.25 bgmesh:1");
    assert_eq!(a.display.settings.inactive.opacity, 25);
    a.execute("view3d.wireframeAlpha ? bgmesh:1");
    assert_eq!(a.status, "0.25");
    a.execute("view3d.shadingStyle wireframe");
    assert_eq!(
        a.display.settings.active.style,
        view_display::Style::Wireframe,
        "{}",
        a.status
    );
    a.execute("view3d.projection top");
    assert_eq!(a.camera.view, View::Top);
    a.execute("view3d.showLights maybe");
    assert!(a.status.contains("true or false"), "{}", a.status);
}

/// pref.value: preferences by name, read back with ?, and the
/// Preferences rows run it.
#[test]
fn pref_value_sets_reads_and_backs_the_preferences_rows() {
    let mut a = Operandi::empty();
    a.execute("pref.value input.trackball true");
    assert!(a.input_preferences.options.trackball, "{}", a.status);
    a.execute("pref.value input.trackball ?");
    assert_eq!(a.status, "true");
    a.execute("pref.value input.wheelAcceleration 0.5");
    assert_eq!(a.input_preferences.options.wheel_acceleration, 50);
    a.execute("pref.value input.wheelAcceleration ?");
    assert_eq!(a.status, "0.5");
    a.execute("pref.value input.zoomDragAxis {Top to Bottom}");
    a.execute("pref.value input.zoomDragAxis ?");
    assert_eq!(a.status, "Top to Bottom");
    a.execute("pref.value input.tooltipDelay 99");
    assert!(a.status.contains("Tooltip Delay is"), "{}", a.status);
    a.execute("pref.value no.such.pref 1");
    assert!(a.status.contains("input.trackball"), "{}", a.status);
    // Slider Min/Max Labels (UI page) is reachable too, and takes effect.
    a.execute("pref.value ui.slider_min_max_labels true");
    assert!(a.ui_preferences.slider_min_max_labels, "{}", a.status);
    assert!(crate::form::range_labels::on());
    a.execute("pref.value ui.slider_min_max_labels ?");
    assert_eq!(a.status, "true");
    a.execute("pref.value ui.slider_min_max_labels false");
    assert!(!crate::form::range_labels::on());
    // A row edit is run again through pref.value.
    let before = a.pref_snapshot(preferences::Page::Remapping);
    a.input_preferences.options.invert_wheel_zoom = true;
    a.pref_rows_through_commands(before);
    assert!(a.input_preferences.options.invert_wheel_zoom);
    a.execute("pref.value input.invertWheelZoom ?");
    assert_eq!(a.status, "true");
}

/// pref.value by preference group and the colour family, and the
/// window's rows running them.
#[test]
fn pref_value_groups_and_colours() {
    let mut a = Operandi::empty();
    a.execute("pref.value application.undo_levels ?");
    let levels = a.status.clone();
    assert!(
        levels.parse::<u32>().is_ok() || levels.starts_with("No preference"),
        "{levels}"
    );
    a.execute("pref.value ui ?");
    a.execute("pref.value selection.nonsense 1");
    assert!(a.status.starts_with("No preference"), "{}", a.status);
    // Some field of every group reads back.
    for group in prefs_commands::GROUPS {
        let mut all = Vec::new();
        prefs_commands::leaves_of(&(group.get)(&a), &mut all);
        let (path, _) = all.first().expect("a group has fields");
        a.execute(&format!("pref.value {}.{path} ?", group.name));
        assert!(
            !a.status.starts_with("No preference"),
            "{}: {}",
            group.name,
            a.status
        );
    }
    // A wrong type is refused and changes nothing.
    let before = a.ui_preferences;
    let mut all = Vec::new();
    prefs_commands::leaves_of(&serde_json::to_value(a.ui_preferences).unwrap(), &mut all);
    let (path, _) = all.first().unwrap().clone();
    a.execute(&format!("pref.value ui.{path} [1,2,3,4,5]"));
    assert_eq!(a.ui_preferences, before, "{}", a.status);
    // Colours: set, read, back to default.
    a.execute("pref.value color.viewport.background #102030");
    a.execute("pref.value color.viewport.background ?");
    assert_eq!(a.status.to_ascii_lowercase(), "#102030");
    a.execute("pref.value color.viewport.background default");
    assert_eq!(
        theme::override_of(theme::COLORS[0].id),
        None,
        "{}",
        a.status
    );
    a.execute("pref.value color.viewport.background blue");
    assert!(a.status.contains("#rrggbb"), "{}", a.status);
    // The window's edit on a group page runs pref.value.
    let before = a.pref_snapshot(preferences::Page::Selection);
    let old = serde_json::to_value(a.selection_preferences).unwrap();
    let mut all = Vec::new();
    prefs_commands::leaves_of(&old, &mut all);
    let (path, leaf) = all
        .iter()
        .find(|(_, v)| v.is_boolean())
        .cloned()
        .expect("a switch");
    let flipped = !leaf.as_bool().unwrap();
    let edited =
        prefs_commands::with_leaf_of(old, &path, serde_json::Value::Bool(flipped)).unwrap();
    a.selection_preferences = serde_json::from_value(edited).unwrap();
    a.pref_rows_through_commands(before);
    a.execute(&format!("pref.value selection.{path} ?"));
    assert_eq!(a.status, flipped.to_string());
}

/// pref.value takes map units for every percent and angle preference
/// (percent 0–1, radians; 50% and 30deg too), whatever the field stores.
/// Every Percent or Angle row on the pages must be in `UNITS`.
#[test]
fn pref_value_takes_map_units_for_every_percent_and_angle() {
    use prefs_commands::{Stored, UNITS};
    // The pages' Percent / Angle rows (Remapping has its own entries).
    for source in [
        include_str!("../settings/selection_preferences.rs"),
        include_str!("../settings/units_preferences.rs"),
        include_str!("../settings/viewport_preferences.rs"),
        include_str!("../settings/handle_preferences.rs"),
        include_str!("../settings/defaults_preferences.rs"),
        include_str!("../settings/undo_autosave.rs"),
        include_str!("../settings/mesh_items_preferences.rs"),
        include_str!("../settings/ui_preferences.rs"),
        include_str!("../settings/render_preferences.rs"),
        include_str!("../settings/file_io_preferences.rs"),
        include_str!("../settings/preferences.rs"),
    ] {
        for call in source.split("rows::number_d(").skip(1) {
            let head: String = call.chars().take(400).collect();
            if !(head.contains("Unit::Percent") || head.contains("Unit::Angle")) {
                continue;
            }
            let field = head
                .split("&mut ")
                .nth(1)
                .and_then(|r| r.split(',').next())
                .and_then(|r| r.rsplit('.').next())
                .unwrap()
                .trim();
            assert!(
                UNITS.iter().any(|(k, _)| k.ends_with(&format!(".{field}"))),
                "{field}: a Percent/Angle row missing from prefs_commands::UNITS"
            );
        }
    }
    let mut a = Operandi::empty();
    for (key, unit) in UNITS {
        let (value, typed) = match unit {
            Stored::Percent100 => (0.25, vec!["0.25", "25%"]),
            Stored::Degrees => (0.5, vec!["0.5", "28.64788975654116deg"]),
        };
        for text in typed {
            a.execute(&format!("pref.value {key} {text}"));
            assert_eq!(a.status, "Ready", "{key} {text}");
            a.execute(&format!("pref.value {key} ?"));
            let back: f64 = a
                .status
                .parse()
                .unwrap_or_else(|_| panic!("{key}: {}", a.status));
            assert!(
                (back - value).abs() < 1e-6,
                "{key} {text}: read back {back}"
            );
        }
    }
}

/// pref.value refuses values outside each ranged row's range (the same
/// constants the Preferences rows use), and every ranged row is listed.
#[test]
fn pref_value_checks_every_ranged_row() {
    let listed = prefs_commands::ranges();
    // Every ranged group row on the pages is in ranges() (rows of other
    // state, the scene's draw size and the Preview's samples, aren't).
    for source in [
        include_str!("../settings/selection_preferences.rs"),
        include_str!("../settings/units_preferences.rs"),
        include_str!("../settings/viewport_preferences.rs"),
        include_str!("../settings/handle_preferences.rs"),
        include_str!("../settings/undo_autosave.rs"),
        include_str!("../settings/ui_preferences.rs"),
        include_str!("../settings/render_preferences.rs"),
        include_str!("../settings/file_io_preferences.rs"),
    ] {
        for call in source.split("rows::number_d(").skip(1) {
            let head: String = call.chars().take(300).collect();
            let Some(target) = head.split(',').nth(2) else {
                continue;
            };
            let Some(field) = target.trim().strip_prefix("&mut ") else {
                continue; // scene_size, samples: not group fields
            };
            let field = field.rsplit('.').next().unwrap().trim();
            assert!(
                listed.iter().any(|(_, f, _, _)| *f == field),
                "{field}: a ranged row missing from prefs_commands::ranges()"
            );
        }
    }
    let mut a = Operandi::empty();
    for (group, field, min, max) in listed {
        let g = prefs_commands::GROUPS
            .iter()
            .find(|g| g.name == group)
            .unwrap();
        let mut all = Vec::new();
        prefs_commands::leaves_of(&(g.get)(&a), &mut all);
        let (path, leaf) = all
            .iter()
            .find(|(p, _)| p.rsplit('.').next() == Some(field))
            .unwrap_or_else(|| panic!("{group}.{field} isn't a field of its group"));
        let key = format!("{group}.{path}");
        let integer = leaf.is_u64() || leaf.is_i64();
        let step = if integer {
            1.
        } else {
            (max - min).abs().max(1.) * 0.01
        };
        let text = |v: f64| prefs_commands::map_text_of(&key, &serde_json::json!(v));
        a.execute(&format!("pref.value {key} {}", text(max)));
        assert_eq!(a.status, "Ready", "{key} at its max");
        for outside in [max + step, min - step] {
            let before = (g.get)(&a);
            a.execute(&format!("pref.value {key} {}", text(outside)));
            assert!(a.status.contains(" to "), "{key} {outside}: {}", a.status);
            assert_eq!((g.get)(&a), before, "{key}: refused values change nothing");
        }
    }
}

/// Environments: renames go through the commands; Rotation Order is gone.
#[test]
fn environment_renames_and_rotation_order_run_commands() {
    let mut a = box_app();
    a.execute("item.create environment");
    let env = a.editor.scene.active().unwrap();
    let (id, material) = (env.id, env.environment.as_ref().unwrap().material.id);
    // The Environment Material's name, by item.name layer:.
    a.execute(&format!("item.name {{Studio Sky}} layer:{material}"));
    let env = a.editor.scene.item(id).unwrap();
    assert_eq!(
        env.environment.as_ref().unwrap().material.name,
        "Studio Sky",
        "{}",
        a.status
    );
    // Add a layer first, so the layer path is always proven.
    a.run(Command::EditEnvironmentLayers {
        item: id,
        edit: operandi_core::environment::EnvironmentLayerEdit::Add {
            layer: operandi_core::shader_tree::NewLayer::Gradient,
            at: operandi_core::environment::LayerPlace::Top,
        },
    });
    let env = a.editor.scene.item(id).unwrap();
    let layer = env
        .environment
        .as_ref()
        .unwrap()
        .layers
        .first()
        .expect("a layer")
        .id;
    // The panel's layer rename runs the same command.
    a.rename_environment_layer(id, layer, "Haze".into());
    let env = a.editor.scene.item(id).unwrap();
    let node = env
        .environment
        .as_ref()
        .unwrap()
        .layers
        .iter()
        .find(|n| n.id == layer)
        .unwrap();
    assert_eq!(node.name, "Haze", "{}", a.status);
    // And by typed command.
    a.execute(&format!("item.name Mist layer:{layer}"));
    let env = a.editor.scene.item(id).unwrap();
    let node = env
        .environment
        .as_ref()
        .unwrap()
        .layers
        .iter()
        .find(|n| n.id == layer)
        .unwrap();
    assert_eq!(node.name, "Mist", "{}", a.status);
    // ENVIRONMENT-PLACEMENT §2: an environment has no Rotation Order.
    a.execute(&format!("item.channel rotOrder ZXY item:{id}"));
    let env = a.editor.scene.item(id).unwrap();
    assert!(!env.transform.zxy, "{}", a.status);
    assert!(a.status.contains("no channel"), "{}", a.status);
}

/// tool.attr reaches every data-driven Tool Properties field: for every
/// tool, each field reads back with ? and takes its own value again.
#[test]
fn tool_attr_reaches_every_tool_properties_field() {
    let mut reached = 0;
    for tool in Tool::ALL {
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![5];
        a.activate(tool);
        let Some(i) = a.command_active() else {
            continue;
        };
        let owner = command::IDS[i].0;
        for (key, label) in a.tool_attr_fields() {
            let id = if owner.contains(' ') {
                format!("{{{owner}}}")
            } else {
                owner.to_owned()
            };
            a.execute(&format!("tool.attr {id} {key} ?"));
            let value = a.status.clone();
            assert!(
                !value.starts_with("Unknown owner") && !value.contains("has no setting"),
                "{owner} {key} ({label}): {value}"
            );
            // Setting it to what it is changes nothing and isn't refused
            // as unknown (some tools refuse values while idle).
            let braced = if value.contains(' ') || value.is_empty() {
                format!("{{{value}}}")
            } else {
                value.clone()
            };
            a.execute(&format!("tool.attr {id} {key} {braced}"));
            assert!(
                !a.status.starts_with("Unknown owner") && !a.status.contains("has no setting"),
                "{owner} {key} = {value}: {}",
                a.status
            );
            reached += 1;
        }
    }
    assert!(reached > 100, "only {reached} fields reached");
}

/// Every light edit a Light Properties row makes gives the same scene
/// through its command line as straight to the editor.
#[test]
fn light_rows_through_commands_match_direct_edits() {
    use operandi_core::camera_light::*;
    let mut base = box_app();
    let mesh = base.editor.scene.items[0].id;
    let mut edits: Vec<(&str, LightEdit)> = vec![
        ("light.spot", LightEdit::Kind(LightKind::Area)),
        ("light.spot", LightEdit::Render(LightRender::Off)),
        ("light.spot", LightEdit::Dissolve(0.3)),
        ("light.spot", LightEdit::Intensity(7.5)),
        ("light.spot", LightEdit::Shadow(ShadowType::None)),
        ("light.spot", LightEdit::VisibleToCamera(false)),
        ("light.spot", LightEdit::Color([0.2, 0.4, 0.6])),
        ("light.spot", LightEdit::AffectDiffuse(0.5)),
        ("light.spot", LightEdit::AffectSpecular(0.25)),
        ("light.spot", LightEdit::Radius(0.2)),
        ("light.spot", LightEdit::Falloff(LightFalloff::None)),
        ("light.spot", LightEdit::ConeAngle(0.7)),
        ("light.spot", LightEdit::SoftEdge(0.1)),
        ("light.spot", LightEdit::VisibleToReflection(false)),
        ("light.spot", LightEdit::VisibleToRefraction(false)),
        ("light.spot", LightEdit::Temperature(3200.)),
        ("light.spot", LightEdit::LpeLabel("key light".into())),
        ("light.spot", LightEdit::LpeLabel(String::new())),
        ("light.spot", LightEdit::Importance(0.4)),
        ("light.spot", LightEdit::BlendMode(LightBlend::Screen)),
        ("light.spot", LightEdit::MaterialEnabled(false)),
        ("light.spot", LightEdit::Opacity(0.6)),
        ("light.directional", LightEdit::SpreadAngle(0.05)),
        ("light.area", LightEdit::Shape(AreaShape::Elliptical)),
        ("light.area", LightEdit::Width(2.)),
        ("light.area", LightEdit::Height(3.)),
        ("light.cylinder", LightEdit::Length(4.)),
        ("light.portal", LightEdit::EnvironmentOnly(true)),
        ("light.mesh", LightEdit::Prototype(Some(mesh))),
        ("light.mesh", LightEdit::UsePrototypeTransform(true)),
    ];
    for (kind, edit) in edits.drain(..) {
        base.execute(&format!("item.create {kind}"));
        let item = base.editor.scene.active().unwrap().id;
        let line = operandi_core::channels::light_line(item, &edit)
            .unwrap_or_else(|| panic!("{edit:?} has no command"));
        let mut direct = base.editor.clone();
        direct
            .dispatch(Command::EditLight {
                item,
                edit: edit.clone(),
            })
            .unwrap();
        let mut typed = box_app();
        typed.editor = base.editor.clone();
        typed.run_command_line(&line);
        assert_eq!(typed.editor.scene, direct.scene, "{line}: {}", typed.status);
    }
}

/// Shader tree: for every layer type, a one-field edit of the layer's own
/// settings (and of its common block and name) becomes an item.channel /
/// item.name line that builds exactly the panel's node.
#[test]
fn shader_layer_edits_round_trip_through_commands() {
    use operandi_core::shader_tree::{NewLayer, Node};
    let mut a = box_app();
    a.upper_frame.show_page(1);
    let mut layers = vec![
        NewLayer::Folder,
        NewLayer::Group,
        NewLayer::Shader,
        NewLayer::Material,
        NewLayer::Constant,
        NewLayer::ImageMap,
        NewLayer::Gradient,
        NewLayer::VertexMap,
        NewLayer::Variation,
        NewLayer::Process,
        NewLayer::Occlusion,
    ];
    layers.extend(
        operandi_core::procedural::ProceduralKind::ALL
            .iter()
            .map(|&k| NewLayer::Procedural(k)),
    );
    for layer in layers {
        a.shading.selected.clear();
        a.shading_add(layer);
    }
    let nodes: Vec<Node> = a.editor.scene.shader_tree.nodes.clone();
    let (mut tried, mut matched) = (0, 0);
    let check = |a: &mut Operandi, edited: &Node| -> bool {
        let Some(line) = a.layer_edit_line(edited) else {
            return false;
        };
        matches!(
            operandi_core::command_line::command_for_line(&mut a.editor, &line),
            Some(Ok(Some(Command::ShaderTree(
                operandi_core::shader_tree::ShaderTreeEdit::Replace { node, .. }
            )))) if node == *edited
        )
    };
    for node in &nodes {
        // Name and the common block.
        let mut n = node.clone();
        n.name = "Renamed Layer".into();
        assert!(check(&mut a, &n), "{}: name", node.name);
        for edit in [
            |n: &mut Node| n.layer.enable = !n.layer.enable,
            |n: &mut Node| n.layer.invert = !n.layer.invert,
            |n: &mut Node| n.layer.opacity = 0.25,
        ] {
            let mut n = node.clone();
            edit(&mut n);
            assert!(check(&mut a, &n), "{}: common block", node.name);
        }
        // Every number and switch of the layer's own settings.
        let target = operandi_core::channels::Target::Layer(node.id);
        for path in operandi_core::channels::layer_fields(&a.editor.scene, target) {
            let (variant, payload) = operandi_core::channels::kind_payload(&node.kind).unwrap();
            let mut payload = payload.clone();
            if !payload.is_object() {
                continue; // one value (an effect): its choices aren't numbers
            }
            let mut at = &mut payload;
            for part in path.split('.') {
                at = at
                    .get_mut(part)
                    .unwrap_or_else(|| panic!("{} {path}: no {part}", node.name));
            }
            *at = match at.clone() {
                serde_json::Value::Bool(b) => serde_json::Value::Bool(!b),
                serde_json::Value::Number(n) if n.is_u64() => {
                    serde_json::json!(n.as_u64().unwrap() + 1)
                }
                serde_json::Value::Number(n) => serde_json::json!(n.as_f64().unwrap() + 0.125),
                _ => continue,
            };
            let Ok(kind) = serde_json::from_value(serde_json::json!({ variant.clone(): payload }))
            else {
                continue;
            };
            let mut n = node.clone();
            n.kind = kind;
            tried += 1;
            if check(&mut a, &n) {
                matched += 1;
            } else {
                panic!("{} {path}: no matching command", node.name);
            }
        }
    }
    assert!(tried > 100 && matched == tried, "{matched}/{tried}");
}

/// Backs the Shading rows' UIC-164 coverage with real row evidence: for
/// one layer of each type, every row is clicked and dragged through the
/// panel; whenever that changes the layer, the panel must have run an
/// item.channel / item.name line whose setting is that row's. Rows the
/// gestures can't change (dropdowns, text) are counted, not driven.
#[test]
fn shading_rows_driven_through_the_panel_run_their_commands() {
    shading_rows_driven(0);
}
// Split three ways so the parts run side by side (test speed,
// 2026-11-16: one run took ~16 s).
#[test]
fn shading_rows_driven_through_the_panel_part_2() {
    shading_rows_driven(1);
}
#[test]
fn shading_rows_driven_through_the_panel_part_3() {
    shading_rows_driven(2);
}

/// Every third layer kind, starting at `part`: each row clicked and
/// dragged must run its own command.
fn shading_rows_driven(part: usize) {
    use super::tests::pointer;
    use operandi_core::shader_tree::{NewLayer, Node};
    let size = vec2(1440., 4000.);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.upper_frame.show_page(1);
    let mut kinds = vec![
        NewLayer::Group,
        NewLayer::Shader,
        NewLayer::Material,
        NewLayer::Constant,
        NewLayer::Gradient,
        NewLayer::VertexMap,
        NewLayer::Variation,
        NewLayer::Process,
        NewLayer::Occlusion,
    ];
    kinds.extend(
        operandi_core::procedural::ProceduralKind::ALL
            .iter()
            .map(|&k| NewLayer::Procedural(k)),
    );
    let before_ids: BTreeSet<u64> = a
        .editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .map(|n| n.id)
        .collect();
    for kind in kinds {
        a.shading.selected.clear();
        a.shading_add(kind);
    }
    let mut layers: Vec<u64> = a
        .editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .map(|n| n.id)
        .filter(|id| !before_ids.contains(id))
        .collect();
    // A Render Output too (Final Color Output is there from the start).
    layers.extend(
        a.editor
            .scene
            .shader_tree
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.kind,
                    operandi_core::shader_tree::NodeKind::RenderOutput(_)
                )
            })
            .map(|n| n.id)
            .take(1),
    );
    let frames = |a: &mut Operandi, events: Vec<Event>| {
        draw(a, &ctx, size, events);
        draw(a, &ctx, size, vec![]);
    };
    let matches_row = |label: &str, line: &str| {
        let mut words = line.split_whitespace();
        let command = words.next().unwrap_or("");
        let setting = words.next().unwrap_or("");
        if command == "item.name" {
            return label == "Name";
        }
        let last = setting.rsplit('.').next().unwrap_or(setting);
        loose(label) == loose(setting)
            || loose(label) == loose(last)
            || (loose(label) == "blendmode" && setting == "blend")
            || shader_label_field(label).is_some_and(|f| loose(f) == loose(last))
    };
    let (mut driven, mut untouched) = (0, 0);
    let mut unclaimed: Vec<String> = Vec::new();
    let mut wrong = Vec::new();
    for id in layers.into_iter().skip(part).step_by(3) {
        a.shading.selected = vec![id];
        frames(&mut a, vec![]);
        crate::ui_inventory::take_rects();
        frames(&mut a, vec![]);
        let rects = crate::ui_inventory::take_rects();
        for (label, rect) in rects {
            if label.is_empty() || label == "Name" {
                continue;
            }
            let state = |a: &Operandi| {
                (
                    a.editor.scene.shader_tree.get(id).cloned(),
                    a.editor.scene.shader_tree.outputs.get(&id).cloned(),
                )
            };
            let at = rect.center();
            let mut changed = false;
            for gesture in 0..2 {
                let before = state(&a);
                crate::ui_inventory::take_command();
                let events = if gesture == 0 {
                    vec![
                        Event::PointerMoved(at),
                        pointer(at, true, Modifiers::NONE),
                        pointer(at, false, Modifiers::NONE),
                    ]
                } else {
                    let mut e = vec![Event::PointerMoved(at), pointer(at, true, Modifiers::NONE)];
                    for k in 1..=6 {
                        e.push(Event::PointerMoved(at + vec2(8. * k as f32, 0.)));
                    }
                    e.push(pointer(at + vec2(48., 0.), false, Modifiers::NONE));
                    e
                };
                frames(&mut a, events);
                // Close anything the gesture opened.
                frames(
                    &mut a,
                    vec![Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                );
                if state(&a) != before {
                    changed = true;
                    match crate::ui_inventory::take_command() {
                        Some(line) if matches_row(&label, &line) => {}
                        Some(line) => wrong.push(format!("{label}: ran {line}")),
                        // A row UIC-164 counts as covered must run its command.
                        None if shading_claims(&label) => {
                            wrong.push(format!("{label}: changed the layer without a command"))
                        }
                        None => unclaimed.push(label.clone()),
                    }
                    break;
                }
            }
            let _: &Node;
            if changed {
                driven += 1;
            } else {
                untouched += 1;
            }
        }
    }
    println!(
        "Shading rows driven through the panel: {driven}; not changed by a click or drag: {untouched}; changed without a command (gaps): {unclaimed:?}"
    );
    assert!(wrong.is_empty(), "{wrong:#?}");
    // 87 in all, about 29 a part (2026-11-16).
    assert!(driven > 15, "only {driven} rows driven");
}

/// The map's texture and mask commands build exactly the panel's edits.
#[test]
fn texture_and_mask_map_commands_match_the_panel() {
    use operandi_core::shader_tree::{NewLayer, NodeKind, TagMask, TagType};
    let mut a = box_app();
    a.upper_frame.show_page(1);
    let check = |a: &mut Operandi, edited: &operandi_core::shader_tree::Node, want: &str| {
        let line = a
            .layer_edit_line(edited)
            .unwrap_or_else(|| panic!("no line for {want}"));
        assert!(line.starts_with(want), "{line}");
        let built = operandi_core::command_line::command_for_line(&mut a.editor, &line);
        assert!(
            matches!(
                built,
                Some(Ok(Some(Command::ShaderTree(
                    operandi_core::shader_tree::ShaderTreeEdit::Replace { ref node, .. }
                )))) if node == edited
            ),
            "{line}"
        );
    };
    a.shading.selected.clear();
    a.shading_add(NewLayer::Group);
    let group = a
        .editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .rev()
        .find(|n| matches!(n.kind, NodeKind::Group(_)))
        .unwrap()
        .clone();
    let mut n = group.clone();
    if let NodeKind::Group(g) = &mut n.kind {
        g.tag_type = TagType::Part;
        g.tag = TagMask::None;
    }
    check(&mut a, &n, "mask.setPTagType");
    let mut n = group.clone();
    if let NodeKind::Group(g) = &mut n.kind {
        g.tag = TagMask::Name("Front Panel".into());
    }
    check(&mut a, &n, "mask.setPTag");
    a.shading.selected.clear();
    a.shading_add(NewLayer::Constant);
    let texture = a
        .editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .rev()
        .find(|n| matches!(n.kind, NodeKind::Texture(_)))
        .unwrap()
        .clone();
    let locator = a.editor.scene.items[0].id;
    let mut n = texture.clone();
    if let NodeKind::Texture(t) = &mut n.kind {
        t.locator = Some(locator);
    }
    check(&mut a, &n, "texture.setLocator");
    // Typed: refusals change nothing.
    let before = a.editor.scene.clone();
    a.execute(&format!(
        "texture.setIMap image:nothing layer:{}",
        texture.id
    ));
    assert!(a.status.contains("No scene image"), "{}", a.status);
    a.execute(&format!(
        "mask.setPTagType name:Nonsense layer:{}",
        group.id
    ));
    assert!(a.status.contains("Unknown tag type"), "{}", a.status);
    a.execute(&format!("texture.setVMap name:none layer:{}", group.id));
    assert!(a.status.contains("doesn't apply"), "{}", a.status);
    assert_eq!(a.editor.scene, before);
}

#[test]
fn command_list_and_help_read_the_registry() {
    let mut a = Operandi::empty();
    a.execute("command.list");
    assert!(
        a.status.starts_with(&format!(
            "{} commands (API 1)",
            command_handlers::registry().iter().count()
        )),
        "{}",
        a.status
    );
    a.execute("command.list sculpt");
    assert!(a.status.contains("sculpt.push"), "{}", a.status);
    a.execute("command.list nothing-like-this");
    assert!(a.status.starts_with("No command matches"));
    a.execute("help item.add");
    assert!(a.status.contains("item.add"));
    a.execute("help top view");
    assert!(a.status.contains("Top view"), "{}", a.status);
    // Bespoke help topics keep their own text.
    a.execute("help poly.remove");
    assert!(a.status.contains("Shift+Backspace"));
    a.execute("help no.such.thing");
    assert!(a.status.starts_with("Unknown help topic"));
}

// ---------------------------------------------------------------- UIC-164

/// One control a person can click, and the command it runs (if any).
#[derive(Debug)]
struct Control {
    area: &'static str,
    key: String,
    command: Option<String>,
}

/// The gap-report bucket for a context the test shows.
fn bucket(context: &str) -> &'static str {
    match context {
        "lights" => "lights",
        "environments" => "environments",
        "cameras" => "cameras",
        "shader tree" => "shader tree",
        "textures" => "textures",
        "viewport options" => "viewport options",
        "prefs" => "prefs",
        c if c.starts_with("tools/") => "tool properties",
        _ => "other",
    }
}

/// The command a menu entry runs: every menu is a form (menus as forms),
/// so each entry runs its line. `None`: the entry calls the app directly
/// with no command (a gap).
fn menu_command(action: &app_menu::Action) -> Option<String> {
    match action {
        app_menu::Action::Line(i) => crate::app_menu::action_line(*i),
        _ => None,
    }
}

/// A menu entry's gap key: its action's variant, without the payload
/// (recent files and scene names change).
fn menu_key(action: &app_menu::Action) -> String {
    let debug = format!("{action:?}");
    debug.split('(').next().unwrap_or(&debug).to_owned()
}

/// The command a recorded property row or toolbox button runs. Toolbox
/// keys are Input Map action names; property rows have no commands yet,
/// except the few listed here.
fn control_command(context: &str, surface: &str, label: &str) -> Option<String> {
    if surface == "toolbox" {
        // Buttons whose label isn't a command name run the operation the
        // Modo button invocation names (MODO-BUTTON-INVOCATIONS.txt).
        let invocation = match label {
            "Add Loop" => "tool.set edge.addLoop on",
            "Add Point" => "tool.set edge.addPoint on",
            "Assign Selection Set..." => "select.editSet",
            "Use Selection Set..." => "select.useSet",
            "Delete Selection Set..." => "select.deleteSet",
            "Freeze..." => "poly.freeze",
            "Edge Bridge" => "tool.set edge.bridge on",
            "Vertex New Vertex" => "vert.new",
            "Vertex Make Polygon" => "poly.make auto",
            "Vertex Reorder Index" => "vert.reorder",
            "Vertex Sync Index Order" => "vert.syncOrder",
            "Hard Tip" => "brush.hard",
            "Merge" => "layer.mergeMeshes",
            _ => label,
        };
        return registry::find_line(invocation)
            .map(|(e, _)| e.name.to_owned())
            // The app's own commands (painting's on the Sculpt tab).
            .or_else(|| {
                command_handlers::registry()
                    .find_line(invocation)
                    .map(|(e, _)| e.entry.name.to_owned())
            });
    }
    let page = match surface {
        "Preferences: Mesh Items" => Some(preferences::Page::MeshItems),
        _ => surface.strip_prefix("preferences/").and_then(|page| {
            preferences::Page::ALL
                .into_iter()
                .find(|p| format!("{p:?}") == page)
        }),
    };
    if let Some(page) = page {
        if let Some(key) = prefs_commands::pref_for_label(page, label) {
            return Some(format!("pref.value {key} ?"));
        }
        // Rows of a page wired by its preference group (pref.value
        // <group>.<field>), except buttons and values kept elsewhere.
        return prefs_commands::page_by_group(page)
            .filter(|_| !prefs_commands::NOT_IN_GROUP.contains(&label))
            .map(|g| format!("pref.value {g}.* ?"));
    }
    if surface == "viewport-properties" {
        return viewport_properties::view3d_for_label(label).map(str::to_owned);
    }
    // Environments (environments-2026-11-09): every row runs item.channel,
    // its Name rows item.name, its Sun Light row environment.light.
    if let Some(tab) = surface.strip_prefix("properties/") {
        use crate::environment_ui as env;
        if [
            env::ENVIRONMENT_TAB,
            env::ENVIRONMENT_MATERIAL_TAB,
            env::ENVIRONMENT_LAYER_TAB,
        ]
        .contains(&tab)
        {
            match label {
                "Name" => return Some("item.name".into()),
                "Sun Light" => return Some("environment.light".into()),
                "Texture Locator" => return Some("item.select".into()),
                "Load Image…" => return Some("operandi.environment.loadImage".into()),
                "Clear Image" => return Some("texture.delete".into()),
                "Rotation" => return Some("transform.channel rot.Y ?".into()),
                _ => {}
            }
            if let Some(channel) = env::row_channel(tab, label) {
                return Some(format!("item.channel {channel} ?"));
            }
        }
    }
    // Tool Properties fields run tool.attr: a row counts only when it is
    // a field of the tool whose panel drew it (not a button, and not a
    // label some other tool's field happens to share).
    if surface == "tool-properties"
        && let Some(tool) = context.strip_prefix("tools/")
        && TOOL_FIELD_LABELS.with(|l| {
            l.borrow()
                .get(tool)
                .is_some_and(|labels| labels.contains(label))
        })
    {
        return Some("tool.attr".to_owned());
    }
    // The Weight Map picker (Falloff menu, Vertex Map Tools) runs
    // select.vertexMap (VM-02).
    if label == "Weight Map" {
        return Some("select.vertexMap".to_owned());
    }
    // The Select tab's Polygon Tag Type drop-down runs
    // operandi.select.polygonTagType (missing-tools-g1 MT1-02).
    if label == "Polygon Tag Type" {
        return Some("operandi.select.polygonTagType".to_owned());
    }
    // A Shading tab row runs item.channel / item.name through
    // replace_layer when it edits one of the layer's own settings, its
    // common block or its name (shader_layer_edits_round_trip_through_commands).
    let shading = surface.strip_prefix("properties/").is_some_and(|t| {
        [
            "Shader",
            "Surface",
            "Group",
            "Folder",
            "Texture",
            "Render Output",
            // A Material layer's tabs (material_layout).
            "Material Ref",
            "Reflection",
            "Transparency",
            "Rays",
        ]
        .contains(&t)
    });
    if shading && shading_claims(label) {
        return Some("item.channel".to_owned());
    }
    // An Image Map's Browse… (#56) runs its own command.
    if surface == "properties/Texture" && label == "Browse…" {
        return Some("operandi.texture.loadImage".into());
    }
    // An item tab's Name row runs item.name (rename_active).
    let shading_tab = |t: &str| {
        [
            "Shader",
            "Surface",
            "Group",
            "Folder",
            "Texture",
            "Render Output",
            // A Material layer's tabs (material_layout).
            "Material Ref",
            "Reflection",
            "Transparency",
            "Rays",
        ]
        .contains(&t)
    };
    if label == "Name"
        && surface
            .strip_prefix("properties/")
            .is_some_and(|t| !shading_tab(t))
    {
        return Some("item.name".to_owned());
    }
    // Light Properties rows run their command (camera_light_ui run_light:
    // item.channel, item.setType, light.mesh, light.temperature).
    let light_tab = surface
        .strip_prefix("properties/")
        .is_some_and(|t| t.ends_with(" Light") || t == "Portal");
    if light_tab
        && [
            "Type",
            "Render",
            "Falloff Type",
            "Shadow Type",
            "Shape",
            "Affect Diffuse",
            "Affect Specular",
            "Blend Mode",
            "Enable",
            "LPE Label",
            "Visible to Camera",
            "Visible to Reflection Rays",
            "Visible to Refraction Rays",
            "Prototype",
            "Use Prototype Transform",
            "Environment Only",
        ]
        .contains(&label)
    {
        return Some("item.channel".to_owned());
    }
    // Display / Assembly rows and Set Target run item.channel through
    // run_common_on (item_common_rows_round_trip_through_item_channel).
    let common_tab = surface
        .strip_prefix("properties/")
        .is_some_and(|t| t == "Display" || t == "Assembly");
    if (common_tab && COMMON_ROW_LABELS.contains(&label))
        || (light_tab && label == "Set Target")
        || (label.starts_with("Drop Action: ") && common_tab)
    {
        return Some("item.channel".to_owned());
    }
    // Item Transform > Reset / Freeze run transform.reset / transform.freeze
    // (spec freeze-transforms-2026-11-14, item_transform_ui.rs).
    if surface.starts_with("properties/") && (label == "Reset" || label == "Freeze") {
        return Some(format!("transform.{}", label.to_ascii_lowercase()));
    }
    // Item Transform > Rotation Order runs item.channel rotOrder.
    if label == "Rotation Order" && surface.starts_with("properties/") {
        return Some("item.channel rotOrder ?".to_owned());
    }
    // Rows a typed command already sets: the Backdrop's switches
    // (backdrop.attr). Other rows have no command yet.
    let _ = context;
    let attr = match (surface, label) {
        ("properties/Backdrop", "Blend") => "blend",
        ("properties/Backdrop", "Flip") => "flip",
        ("properties/Backdrop", "Invert") => "invert",
        ("properties/Backdrop", "Keep Aspect") => "keepAspect",
        ("properties/Backdrop", "Show In Perspective") => "showInPerspective",
        ("properties/Backdrop", "Use Uniform Size") => "uniformSize",
        _ => return None,
    };
    Some(format!("backdrop.attr {attr} true"))
}

/// Shading rows whose label isn't their field's name: (label, field).
/// Each field must exist (checked where it's used).
const SHADER_LABEL_FIELDS: &[(&str, &str)] = &[
    ("Output Filename", "filename"),
    ("Light Path Expression", "lpe"),
    ("Direct Illum Multiplier", "direct_multiplier"),
    ("Indirect Illum Multiplier", "indirect_multiplier"),
    ("Indirect Illum Type", "indirect_type"),
    ("Fog Start Distance", "fog_start"),
    ("Fog End Distance", "fog_end"),
    ("Use Environment Color", "fog_use_environment"),
    ("Visible to Indirect Rays", "visible_to_indirect"),
    ("Visible to Occlusion Rays", "visible_to_occlusion"),
    ("Visible to Reflection Rays", "visible_to_reflection"),
    ("Visible to Refraction Rays", "visible_to_refraction"),
    ("Mode", "link_mode"),
    ("Specular Fresnel", "fresnel"),
    // The OpenPBR material channels (openpbr.rs, OPM-02).
    ("Abbe Number", "dispersion_abbe_number"),
    ("Coat Anisotropy", "coat_roughness_anisotropy"),
    ("Scatter Anisotropy", "transmission_scatter_anisotropy"),
    ("Sheen Color", "fuzz_color"),
    ("Sheen Roughness", "fuzz_roughness"),
    ("Albedo from Diffuse", "subsurface_uses_diffuse"),
    ("Thin Film", "thin_film_weight"),
    ("Volume Absorption", "volume_absorb"),
    ("Volume Scattering", "volume_scatter"),
    ("Apply to Item Instances", "apply_to_instances"),
    // Confirmed by driving the rows (shading_rows_driven_through_the_panel...).
    ("Solid 3D Cells", "solid"),
    ("Use Profile Gradient", "profile_gradient"),
    ("Flow Noise", "flow"),
    ("UV Horizontal and Vertical", "uv"),
    ("Unpremultiply Colors", "unpremultiply"),
    ("Remap Pixel Values", "remap"),
    ("Clamp Colors", "clamp"),
    // A procedural's two zones, named by its pattern (procedural.rs
    // zone_names): the first zone's Color/Value/Alpha, then the second's.
    ("Checker 1 Color", "zone1_color"),
    ("Checker 1 Value", "zone1_value"),
    ("Checker 1 Alpha", "zone1_alpha"),
    ("Line Color", "zone1_color"),
    ("Line Value", "zone1_value"),
    ("Line Alpha", "zone1_alpha"),
    ("Dot Color", "zone1_color"),
    ("Dot Value", "zone1_value"),
    ("Dot Alpha", "zone1_alpha"),
    ("Zone 1 Color", "zone1_color"),
    ("Zone 1 Value", "zone1_value"),
    ("Zone 1 Alpha", "zone1_alpha"),
    ("Cell Color", "zone1_color"),
    ("Cell Value", "zone1_value"),
    ("Cell Alpha", "zone1_alpha"),
    ("Ring Color", "zone1_color"),
    ("Ring Value", "zone1_value"),
    ("Ring Alpha", "zone1_alpha"),
    ("Crest Color", "zone1_color"),
    ("Crest Value", "zone1_value"),
    ("Crest Alpha", "zone1_alpha"),
    ("Yarn Color", "zone1_color"),
    ("Yarn Value", "zone1_value"),
    ("Yarn Alpha", "zone1_alpha"),
    ("Checker 2 Color", "zone2_color"),
    ("Checker 2 Value", "zone2_value"),
    ("Checker 2 Alpha", "zone2_alpha"),
    ("Filler Color", "zone2_color"),
    ("Filler Value", "zone2_value"),
    ("Filler Alpha", "zone2_alpha"),
    ("Zone 2 Color", "zone2_color"),
    ("Zone 2 Value", "zone2_value"),
    ("Zone 2 Alpha", "zone2_alpha"),
    ("Trough Color", "zone2_color"),
    ("Trough Value", "zone2_value"),
    ("Trough Alpha", "zone2_alpha"),
    ("Gap Color", "zone2_color"),
    ("Gap Value", "zone2_value"),
    ("Gap Alpha", "zone2_alpha"),
];

/// Whether UIC-164 counts a Shading-tab row as running item.channel /
/// item.name: its label names one of a layer's own settings, its common
/// block or its name (Image and Locator run their own edits).
fn shading_claims(label: &str) -> bool {
    // Rows the map's own commands run (texture.setIMap / setLocator /
    // setVMap, mask.setPTag / setPTagType).
    [
        "Image",
        "Locator",
        "Vertex Map",
        "Polygon Tag",
        "Polygon Tag Type",
    ]
    .contains(&label)
        || (["enable", "invert", "blendmode", "opacity", "name", "effect"]
            .contains(&loose(label).as_str())
            || SHADER_FIELDS.with(|f| f.borrow().contains(&loose(label)))
            || shader_label_field(label)
                .is_some_and(|field| SHADER_FIELDS.with(|f| f.borrow().contains(&loose(field)))))
}

fn shader_label_field(label: &str) -> Option<&'static str> {
    SHADER_LABEL_FIELDS
        .iter()
        .find(|(l, _)| *l == label)
        .map(|(_, f)| *f)
}

/// A label or field name compared loosely (letters and digits, any case).
fn loose(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

thread_local! {
    /// Every shader-tree layer field name the inventory met, loosely, and
    /// each one's last part (a nested field's own name).
    static SHADER_FIELDS: std::cell::RefCell<BTreeSet<String>> =
        const { std::cell::RefCell::new(BTreeSet::new()) };
    /// Each tool's data-driven Tool Properties field labels, by tool.
    static TOOL_FIELD_LABELS: std::cell::RefCell<BTreeMap<String, BTreeSet<String>>> =
        const { std::cell::RefCell::new(BTreeMap::new()) };
}

/// Two frames, recording the controls drawn under `context`.
fn show(a: &mut Operandi, ctx: &Context, context: &str) {
    crate::ui_inventory::set_context(context);
    for _ in 0..2 {
        draw(a, ctx, SIZE, vec![]);
    }
}

/// Selects only the item `id` (Item mode).
fn select_only(a: &mut Operandi, id: u64) {
    a.execute("items");
    a.editor.scene.selection.item = Some(id);
    a.editor.scene.selection.also.clear();
}

/// Shows every Properties tab of what is selected.
fn every_property_tab(a: &mut Operandi, ctx: &Context, context: &str) {
    for tab in a.property_tabs() {
        a.properties_tab = tab.to_owned();
        show(a, ctx, context);
    }
}

/// Everything a person can click today, gathered by drawing the app.
fn inventory() -> Vec<Control> {
    let ctx = Context::default();
    theme::install(&ctx);
    crate::ui_inventory::start();

    // Toolbox: every category.
    {
        let mut a = box_app();
        use crate::toolbox::Category as C;
        for category in [
            C::Create,
            C::Select,
            C::Deform,
            C::Duplicate,
            C::Polygon,
            C::Edge,
            C::Vertex,
            C::Edit,
            C::Sculpt,
        ] {
            a.category = category;
            show(&mut a, &ctx, "toolbox");
        }
    }
    // Tool Properties of every tool.
    for tool in Tool::ALL {
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![5];
        a.activate(tool);
        let labels: BTreeSet<String> = a.tool_attr_fields().into_iter().map(|(_, l)| l).collect();
        let context = format!("tools/{tool:?}");
        TOOL_FIELD_LABELS.with(|l| l.borrow_mut().insert(format!("{tool:?}"), labels));
        show(&mut a, &ctx, &context);
    }
    // Properties of each item kind.
    {
        let mut a = box_app();
        let mesh = a.editor.scene.items[0].id;
        select_only(&mut a, mesh);
        every_property_tab(&mut a, &ctx, "items");
        for (line, context) in [
            ("backdrop.add", "items"),
            ("item.add camera", "cameras"),
            ("item.add light.area", "lights"),
            ("item.add light.cylinder", "lights"),
            ("item.add light.directional", "lights"),
            ("item.add light.dome", "lights"),
            ("item.add light.mesh", "lights"),
            ("item.add light.photometric", "lights"),
            ("item.add light.point", "lights"),
            ("item.add light.portal", "lights"),
            ("item.add light.spot", "lights"),
        ] {
            a.execute(line);
            every_property_tab(&mut a, &ctx, context);
        }
    }
    // The Shading panel: every layer type.
    {
        use operandi_core::shader_tree::{NewLayer, NodeKind, OutputEffect};
        let mut a = box_app();
        a.upper_frame.show_page(1);
        let mut layers = vec![
            NewLayer::Folder,
            NewLayer::Group,
            NewLayer::Shader,
            NewLayer::Material,
            NewLayer::Constant,
            NewLayer::ImageMap,
            NewLayer::Gradient,
            NewLayer::VertexMap,
            NewLayer::Variation,
            NewLayer::Process,
            NewLayer::Occlusion,
        ];
        layers.extend(
            operandi_core::procedural::ProceduralKind::ALL
                .iter()
                .map(|&k| NewLayer::Procedural(k)),
        );
        layers.extend(OutputEffect::ALL.iter().map(|&e| NewLayer::RenderOutput(e)));
        for layer in layers {
            a.shading.selected.clear();
            a.shading_add(layer);
        }
        let nodes: Vec<_> = a
            .editor
            .scene
            .shader_tree
            .nodes
            .iter()
            .map(|n| (n.id, matches!(n.kind, NodeKind::Texture(_))))
            .collect();
        for node in &a.editor.scene.shader_tree.nodes {
            let target = operandi_core::channels::Target::Layer(node.id);
            let mut fields = operandi_core::channels::layer_fields(&a.editor.scene, target);
            fields.extend(operandi_core::channels::output_fields(
                &a.editor.scene,
                target,
            ));
            for path in fields {
                let last = path.rsplit('.').next().unwrap_or(&path).to_owned();
                SHADER_FIELDS.with(|f| {
                    f.borrow_mut().insert(loose(&path));
                    f.borrow_mut().insert(loose(&last));
                });
            }
        }
        for (id, texture) in nodes {
            a.shading.selected = vec![id];
            every_property_tab(
                &mut a,
                &ctx,
                if texture { "textures" } else { "shader tree" },
            );
        }
    }
    // Environments: an environment, its material and a Constant and an
    // Image Map layer, shown from the Shader Tree.
    {
        use crate::environment_ui::EnvironmentRow;
        use operandi_core::shader_tree::NewLayer;
        let mut a = box_app();
        a.upper_frame.show_page(1);
        a.create_environment();
        let id = a.editor.scene.environments().next().unwrap().id;
        a.add_environment_layer(NewLayer::Constant);
        a.select_environment_row(EnvironmentRow::Item(id));
        a.add_environment_layer(NewLayer::ImageMap);
        let mut rows = vec![EnvironmentRow::Item(id), EnvironmentRow::Material(id)];
        rows.extend(
            a.environment(id)
                .unwrap()
                .layers
                .iter()
                .map(|n| EnvironmentRow::Layer(id, n.id)),
        );
        for row in rows {
            a.select_environment_row(row);
            every_property_tab(&mut a, &ctx, "environments");
        }
    }
    // Viewport Properties: every tab.
    {
        let mut a = box_app();
        a.execute("viewport properties");
        for tab in viewport_properties::Tab::ALL {
            a.corner.properties.tab = tab;
            show(&mut a, &ctx, "viewport options");
        }
    }
    // Preferences: every page.
    {
        let mut a = box_app();
        a.open_preferences();
        for page in preferences::Page::ALL {
            a.preferences_ui.page = page;
            show(&mut a, &ctx, "prefs");
        }
    }
    for (label, field) in SHADER_LABEL_FIELDS {
        assert!(
            SHADER_FIELDS.with(|f| f.borrow().contains(&loose(field))),
            "{label}: {field} isn't a shader layer field"
        );
    }
    let seen = crate::ui_inventory::take();

    let mut controls = Vec::new();
    for context in [
        "toolbox",
        "tools",
        "items",
        "cameras",
        "lights",
        "environments",
        "shader tree",
        "textures",
        "viewport options",
        "prefs",
    ] {
        assert!(
            seen.iter().any(|(c, _, _)| c == context
                || c.strip_prefix("tools/").is_some() && context == "tools"),
            "nothing recorded for {context}: the test no longer shows it"
        );
    }
    for (context, surface, label) in seen {
        controls.push(Control {
            area: bucket(&context),
            key: format!("{surface} | {label}"),
            command: control_command(&context, &surface, &label),
        });
    }
    // Menus.
    let a = box_app();
    for item in a.menu_model(&ctx).iter().flat_map(app_menu::Menu::leaves) {
        let area = match menu_command(&item.action).as_deref() {
            Some("operandi.inputMap") => "prefs",
            _ => "other",
        };
        controls.push(Control {
            area,
            key: format!("menu | {}", menu_key(&item.action)),
            command: menu_command(&item.action),
        });
    }
    controls
}

/// UIC-164: every menu entry, toolbox button and property row runs a
/// registered command. Today's click-only controls are listed in
/// `KNOWN_GAPS`; the list may only shrink. A new control without a
/// command fails here: give it a command (or, with Jason's OK, list it).
///
/// `OPERANDI_PRINT_GAPS=<file>` writes the current gap list to that file
/// (to refresh `command_registry_gaps.rs` after backfilling).
#[test]
fn uic164_every_control_runs_a_command() {
    let controls = inventory();
    let mut wrong = Vec::new();
    let mut gaps: BTreeMap<String, &'static str> = BTreeMap::new();
    for control in &controls {
        match &control.command {
            Some(command) => {
                // Core's table and the app's own commands (painting's).
                if command_handlers::registry().find_line(command).is_none() {
                    wrong.push(format!("{}: {command} isn't registered", control.key));
                }
            }
            None if CHROME_EXEMPT.contains(&control.key.as_str()) => {}
            // Painting builds: later stages' placeholder buttons, greyed
            // "Coming in a later stage" (painting_brush.rs), and the sculpt
            // brushes' Inks, greyed for good (PNT-243).
            None if cfg!(feature = "painting")
                && (control.key.contains("operandi.paint.notYet")
                    || control.key.contains("operandi.paint.sculptInks")) => {}
            None => {
                gaps.insert(control.key.clone(), control.area);
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    for key in CHROME_EXEMPT {
        assert!(
            controls.iter().any(|c| c.key == *key),
            "{key} is exempt but no longer drawn; remove it from CHROME_EXEMPT"
        );
    }

    if let Ok(path) = std::env::var("OPERANDI_PRINT_GAPS") {
        let mut text = String::new();
        for key in gaps.keys() {
            text.push_str(&format!("    {key:?},\n"));
        }
        std::fs::write(path, text).unwrap();
    }
    // `area | surface | label`, for reports.
    if let Ok(path) = std::env::var("OPERANDI_PRINT_GAP_AREAS") {
        let text: String = gaps
            .iter()
            .map(|(key, area)| format!("{area} | {key}\n"))
            .collect();
        std::fs::write(path, text).unwrap();
    }

    let known: BTreeSet<&str> = KNOWN_GAPS.iter().copied().collect();
    let new: Vec<&String> = gaps
        .keys()
        .filter(|k| !known.contains(k.as_str()))
        .collect();
    let fixed: Vec<&&str> = known.iter().filter(|k| !gaps.contains_key(**k)).collect();

    let mut by_area: BTreeMap<&str, usize> = BTreeMap::new();
    for area in gaps.values() {
        *by_area.entry(area).or_default() += 1;
    }
    println!(
        "UIC-164: {} controls, {} run commands, {} known gaps: {by_area:?}",
        controls.len(),
        controls.iter().filter(|c| c.command.is_some()).count(),
        gaps.len()
    );
    assert!(
        new.is_empty(),
        "UIC-164: these controls run no command (give each a registered command): {new:#?}"
    );
    assert!(
        fixed.is_empty(),
        "UIC-164: these gaps are gone; remove them from command_registry_gaps.rs: {fixed:#?}"
    );
}

/// Display and Assembly rows that run item.channel (item_common_channels.rs).
const COMMON_ROW_LABELS: &[&str] = &[
    "Visible",
    "Label",
    "Pivot",
    "Center",
    "Select",
    "Lock",
    "Redirect",
    "Pick Walk Up",
    "Pick Walk Down",
    "Pick Walk Left",
    "Pick Walk Right",
    "Tooltip",
    "User Tag",
];

/// Every Display / Assembly / targeting edit the rows make has an
/// item.channel line that builds that same edit, and running it through
/// the panel's path changes the scene the same way.
#[test]
fn item_common_rows_round_trip_through_item_channel() {
    use operandi_core::drop_action::DropAction;
    use operandi_core::item_common::{Choice, DisplayVisible, ItemCommonEdit as E};
    use operandi_core::item_extras::{ExtraEdit as X, Walk};
    let mut a = box_app();
    a.run(Command::NewMesh);
    let id = a.editor.scene.items.last().unwrap().id;
    let other = a.editor.scene.target_candidates(id)[0];
    let mut edits = vec![
        E::Label("A label".into()),
        E::Label(String::new()),
        E::ShowLabel(false),
        E::LabelOffsetX(4.5),
        E::LabelOffsetY(-2.0),
        E::Target(Some(other)),
        E::Target(None),
        E::Extra(X::Redirect(Some(other))),
        E::Extra(X::Redirect(None)),
        E::Extra(X::Tooltip("Tip here".into())),
        E::Extra(X::UserTag("tag".into())),
    ];
    edits.extend(DisplayVisible::ALL.map(E::Visible));
    for c in Choice::ALL {
        edits.extend([E::Select(c), E::Extra(X::Pivot(c)), E::Extra(X::Center(c))]);
    }
    for w in Walk::ALL {
        edits.push(E::Extra(X::PickWalk(w, Some(other))));
    }
    for slot in 0..3 {
        edits.push(E::Extra(X::DropAction(slot, None)));
        for d in DropAction::ALL
            .into_iter()
            .filter(|d| *d != DropAction::None)
        {
            edits.push(E::Extra(X::DropAction(slot, Some(d))));
        }
    }
    // Lock last: Yes refuses the display rows after it.
    edits.extend(Choice::ALL.map(E::Lock));
    for edit in edits {
        let line = operandi_core::item_common_channels::common_line(id, &edit)
            .unwrap_or_else(|| panic!("no line for {edit:?}"));
        let built = operandi_core::command_line::command_for_line(&mut a.editor, &line);
        assert!(
            matches!(&built, Some(Ok(Some(Command::EditItemCommon { items, edit: e })))
                if *items == vec![id] && *e == edit),
            "{line} doesn't build {edit:?}"
        );
        let mut direct = a.editor.clone();
        direct
            .dispatch(Command::EditItemCommon {
                items: vec![id],
                edit: edit.clone(),
            })
            .unwrap();
        a.editor.scene.selection.item = Some(id);
        a.editor.scene.selection.also.clear();
        a.run_common(edit.clone());
        assert_eq!(
            serde_json::to_value(&a.editor.scene.items).unwrap(),
            serde_json::to_value(&direct.scene.items).unwrap(),
            "{line}: {}",
            a.status
        );
        // And the value reads back.
        let name = line.split_whitespace().nth(1).unwrap();
        assert!(
            operandi_core::channels::get(
                &a.editor.scene,
                operandi_core::channels::Target::Item(id),
                name
            )
            .is_ok()
        );
    }
}

/// A line has one owner: `execute` asks the registry handlers first, so no
/// older parser after them may also take a line a handler takes (it would
/// have run before the move, and the move would change what runs).
#[test]
fn registry_handlers_take_lines_no_older_parser_takes() {
    let commands = command_handlers::registry();
    let mut handled = 0;
    let mut clashes = Vec::new();
    for c in commands.iter().filter(|c| c.host.is_some()) {
        handled += 1;
        for line in lines(&c.entry) {
            if commands.host_call(&line).is_none() {
                continue;
            }
            let lowered = line.trim().to_lowercase();
            if operandi_core::command_line::parse_sculpt(&line).is_some() {
                clashes.push(format!("sculpt: {line}"));
            }
            if Operandi::empty().canonical_command(&line) {
                clashes.push(format!("canonical_command: {line}"));
            }
            if Operandi::empty().execute_bridge_parameter(&line) {
                clashes.push(format!("bridge: {line}"));
            }
            if Operandi::empty().execute_pie_command(&lowered) {
                clashes.push(format!("pie: {line}"));
            }
        }
    }
    assert!(clashes.is_empty(), "{clashes:#?}");
    // The old typed `match` (about 170 phrases) moved here, with the
    // benchmarks; this only grows as parsers move over.
    assert!(handled >= 170, "{handled}");
}

/// The app's own commands live in its registry, not core's table, and
/// say they need the window.
#[test]
fn app_commands_need_the_window() {
    let commands = command_handlers::registry();
    for name in ["operandi.bench.orbit", "operandi.bench.drag"] {
        assert!(registry::find(name).is_none(), "{name} is in core's table");
        let c = commands.find(name).unwrap();
        assert!(c.entry.flags.ui_only, "{name}");
        assert_eq!(c.origin, operandi_core::command_runtime::Origin::Host);
    }
    let mut a = Operandi::empty();
    a.execute("operandi.bench.orbit many");
    assert_eq!(a.status, "Usage: operandi.bench.orbit [frames]");
    a.execute("help operandi.bench.drag");
    assert!(a.status.contains("needs the window"), "{}", a.status);
    // A phrase with a required choice only takes its choices; anything
    // else is still the older parsers' (here: unknown).
    a.execute("symmetry sideways");
    assert!(unknown(&a.status), "{}", a.status);
    a.execute("symmetry y");
    assert_eq!(a.editor.symmetry.axis, Some(1));
}
