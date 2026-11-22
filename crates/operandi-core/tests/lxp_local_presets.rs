//! Opt-in check against Modo material presets on this machine (never
//! committed): `OPERANDI_LXP_DIR=/path cargo test -p operandi-core --test
//! lxp_local_presets -- --ignored --nocapture`. Prints what the reader and
//! converter make of each `.lxp`.
use operandi_core::lxo::preset_items;

fn presets() -> Vec<std::path::PathBuf> {
    let Ok(dir) = std::env::var("OPERANDI_LXP_DIR") else {
        return Vec::new();
    };
    let mut files = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("lxp")) {
                files.push(p);
            }
        }
    }
    files.sort();
    files
}

#[test]
#[ignore]
fn reader_survey() {
    let mut kinds = std::collections::BTreeMap::<String, usize>::new();
    let mut skipped = std::collections::BTreeMap::<String, usize>::new();
    let (mut channels, mut with_channels, mut envelopes, mut keys) =
        (0usize, 0usize, 0usize, 0usize);
    let (mut ok, mut failed) = (0, 0);
    for path in presets() {
        let bytes = std::fs::read(&path).unwrap();
        match preset_items::read(&bytes) {
            Ok(p) => {
                ok += 1;
                for i in p.items.values() {
                    *kinds.entry(i.kind.clone()).or_default() += 1;
                    channels += i.channels.len();
                    with_channels += usize::from(!i.channels.is_empty());
                }
                envelopes += p.envelopes.len();
                keys += p.envelopes.values().map(|e| e.keys.len()).sum::<usize>();
                for (k, n) in p.skipped {
                    *skipped.entry(k).or_default() += n;
                }
            }
            Err(e) => {
                failed += 1;
                println!("FAIL {}: {e}", path.display());
            }
        }
    }
    println!(
        "read {ok}, refused {failed}; {channels} channels, {with_channels} items with channels, {envelopes} envelopes, {keys} keys"
    );
    for (k, n) in &skipped {
        println!("  skipped {n:6} {k}");
    }
    for (k, n) in kinds {
        println!("  {n:5} {k}");
    }
}

/// Every preset through the converter: counts of what converts, what's
/// dropped and why (aggregated), and a few named presets in full.
#[test]
#[ignore]
fn converter_survey() {
    use operandi_core::preset::lxp;
    let root = std::env::var("OPERANDI_LXP_DIR").map(std::path::PathBuf::from);
    let roots: Vec<std::path::PathBuf> = root.iter().cloned().collect();
    let size = |p: &std::path::Path| p.is_file().then_some((64u32, 64u32));
    let (mut ok, mut refused, mut partial, mut clean) = (0, 0, 0, 0);
    let mut reasons = std::collections::BTreeMap::<String, usize>::new();
    let mut refusals = std::collections::BTreeMap::<String, usize>::new();
    let mut grey = Vec::new();
    let show = std::env::var("OPERANDI_LXP_SHOW").unwrap_or_default();
    for path in presets() {
        match lxp::material_preset(&path, &roots, &size) {
            Ok(c) => {
                ok += 1;
                partial += usize::from(c.partial);
                clean += usize::from(!c.changed_look);
                for n in &c.notes {
                    // Group by the note's shape, not the layer's name.
                    let shape = n.split(": ").last().unwrap_or(n);
                    let shape: String = shape
                        .split_whitespace()
                        .filter(|w| {
                            !w.contains('/')
                                && !w.ends_with(".png")
                                && !w.ends_with(".jpg")
                                && !w.ends_with(".tif")
                                && !w.ends_with(".exr")
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    *reasons.entry(shape).or_default() += 1;
                }
                let name = path.file_stem().unwrap().to_string_lossy();
                if c.notes.iter().any(|n| n.contains("had only a value curve")) {
                    grey.push(name.to_string());
                }
                if !show.is_empty() && show.split(',').any(|s| name.eq_ignore_ascii_case(s)) {
                    println!(
                        "== {name}: {} layers, {} dropped, partial {}",
                        c.layers, c.dropped, c.partial
                    );
                    for n in &c.preset.scene.shader_tree.nodes {
                        println!(
                            "   node {:?} {:?}",
                            n.name,
                            match &n.kind {
                                operandi_core::shader_tree::NodeKind::Texture(t) =>
                                    format!("{:?} {}", t.effect, t.source.label()),
                                operandi_core::shader_tree::NodeKind::Material(m) => format!(
                                    "metal {:.2} diff {:?}x{:.2} rough {:.2} spec {:.3} ior {:.2} useIor {} tran {:.2} lum {:.1}",
                                    m.metallic,
                                    m.diffuse_color,
                                    m.diffuse_amount,
                                    m.roughness,
                                    m.specular_amount,
                                    m.refractive_index,
                                    m.more.use_refractive_index,
                                    m.transparent_amount,
                                    m.luminous_intensity
                                ),
                                _ => "group".to_string(),
                            }
                        );
                    }
                    for n in &c.notes {
                        println!("   note {n}");
                    }
                }
            }
            Err(e) => {
                refused += 1;
                if std::env::var_os("OPERANDI_LXP_REFUSED").is_some() {
                    let kinds: std::collections::BTreeSet<String> = std::fs::read(&path)
                        .ok()
                        .and_then(|b| preset_items::read(&b).ok())
                        .map(|p| p.items.values().map(|i| i.kind.clone()).collect())
                        .unwrap_or_default();
                    println!("REFUSED {}: {e} kinds {kinds:?}", path.display());
                }
                *refusals.entry(e).or_default() += 1;
            }
        }
    }
    println!("converted {ok} (no look change {clean}, partial {partial}), refused {refused}");
    println!("grey colour gradients in {} presets: {grey:?}", grey.len());
    for (r, n) in refusals {
        println!("  refused {n:5} {r}");
    }
    let mut by: Vec<_> = reasons.into_iter().collect();
    by.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (r, n) in by.into_iter().take(70) {
        println!("  {n:5} {r}");
    }
}

/// BUILD-BRIEF: ten presets load without a crash, each makes a material,
/// and the note names any dropped layer.
#[test]
#[ignore]
fn ten_presets_convert() {
    use operandi_core::preset::lxp;
    use operandi_core::shader_tree::NodeKind;
    let files = presets();
    if files.is_empty() {
        return;
    }
    let step = (files.len() / 10).max(1);
    let mut checked = 0;
    for path in files.iter().step_by(step).take(10) {
        let size = |p: &std::path::Path| p.is_file().then_some((64u32, 64u32));
        match lxp::material_preset(path, &[], &size) {
            Ok(c) => {
                let tree = &c.preset.scene.shader_tree;
                assert!(
                    tree.nodes
                        .iter()
                        .any(|n| matches!(n.kind, NodeKind::Material(_)))
                );
                // Repeated lines are folded, so each drop's line is there
                // at least once.
                let left_out = c.notes.iter().filter(|n| n.starts_with("left out")).count();
                assert!(
                    (c.dropped == 0) == (left_out == 0) && left_out <= c.dropped,
                    "{}: {:?}",
                    path.display(),
                    c.notes
                );
                println!(
                    "OK   {}: {} layers, {} dropped",
                    path.display(),
                    c.layers,
                    c.dropped
                );
            }
            Err(e) => {
                assert_eq!(e, lxp::NO_MATERIAL, "{}", path.display());
                println!("NONE {}: {e}", path.display());
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 10);
}

/// Names the reader sees on gradient layers and material models, counted.
#[test]
#[ignore]
fn channel_name_survey() {
    let mut grads = std::collections::BTreeMap::<String, usize>::new();
    let mut params = std::collections::BTreeMap::<String, usize>::new();
    let mut brdf = std::collections::BTreeMap::<String, usize>::new();
    let mut gchan = std::collections::BTreeMap::<String, usize>::new();
    for path in presets() {
        let Ok(p) = preset_items::read(&std::fs::read(&path).unwrap()) else {
            continue;
        };
        for i in p.items.values() {
            if i.kind == "gradient" {
                for g in &i.gradients {
                    let have = p.envelopes.get(&g.envelope).map_or(0, |e| e.keys.len());
                    *grads
                        .entry(format!(
                            "{:?} flag {:?} {:?} effect {:?} keys>0:{}",
                            g.channel,
                            g.interpolation,
                            g.types,
                            i.text("effect"),
                            have > 0
                        ))
                        .or_default() += 1;
                }
                if i.gradients.is_empty() {
                    *grads.entry("(no GRAD)".into()).or_default() += 1;
                }
                *params
                    .entry(format!("{:?}", i.channels.get("param")))
                    .or_default() += 1;
                for k in i.channels.keys() {
                    *gchan.entry(k.clone()).or_default() += 1;
                }
            }
            if i.kind == "advancedMaterial" {
                *brdf
                    .entry(format!("{:?}", i.channels.get("brdfType")))
                    .or_default() += 1;
            }
        }
    }
    println!(
        "GRAD names: {grads:#?}\nparam: {params:#?}\nbrdf: {brdf:#?}\ngradient channels: {gchan:?}"
    );
}
