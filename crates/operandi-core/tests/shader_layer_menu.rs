//! The shader layer right-click menu, stage 1 (shader layer menu v0.1
//! §2, §5, §7). Acceptance criteria:
//!
//! 1. Duplicate puts an independent copy directly above the original, in
//!    the same parent, named "<name> (2)", then "(3)"; a texture copy gets
//!    its own locator with the same settings and transform; one undo step.
//! 2. Duplicating a group copies everything inside it.
//! 3. Copy / Paste: pasted directly above the selected layer, or below the
//!    Base Shader with nothing selected; copy names and own locators; an
//!    empty clipboard is an error.
//! 4. Paste Values copies every setting onto layers of the same type and
//!    keeps their name and locator; another type is refused with "Paste
//!    Values needs a layer of the same type"; one undo step.
//! 5. Editor Color sets and clears the row colour; one undo step; it saves
//!    as native version 31 and an older version claiming it is refused.
//! 6. Select Polygons selects the polygons the Material Group around the
//!    layer matches, on every visible mesh, in Polygon mode; a layer with
//!    no group around it is refused.
use glam::DVec3;
use operandi_core::shader_layer_menu::unnumbered;
use operandi_core::shader_tree::*;
use operandi_core::*;

fn edit(e: &mut Editor, edit: ShaderTreeEdit) {
    e.dispatch(Command::ShaderTree(edit)).unwrap();
}
fn add(e: &mut Editor, layer: NewLayer, selected: Option<NodeId>) -> NodeId {
    let id = e.scene.shader_tree.next_id;
    edit(e, ShaderTreeEdit::Add { layer, selected });
    id
}
fn node(e: &Editor, id: NodeId) -> &Node {
    e.scene.shader_tree.get(id).unwrap()
}
fn by_name<'a>(e: &'a Editor, name: &str) -> &'a Node {
    e.scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| n.name == name)
        .unwrap_or_else(|| panic!("no layer {name}"))
}
/// Children of `parent`, top first, by name.
fn order(e: &Editor, parent: Option<NodeId>) -> Vec<String> {
    e.scene
        .shader_tree
        .children(parent)
        .map(|n| n.name.clone())
        .collect()
}
fn locator(e: &Editor, id: NodeId) -> u64 {
    match &node(e, id).kind {
        NodeKind::Texture(t) => t.locator.unwrap(),
        _ => panic!("not a texture"),
    }
}
fn locator_item(e: &Editor, id: u64) -> &scene::Item {
    e.scene.items.iter().find(|i| i.id == id).unwrap()
}
fn set_constant(e: &mut Editor, id: NodeId, c: [f64; 3]) {
    let mut n = node(e, id).clone();
    if let NodeKind::Texture(TextureLayer {
        source: TextureSource::Constant { color, .. },
        ..
    }) = &mut n.kind
    {
        *color = c;
    }
    edit(
        e,
        ShaderTreeEdit::Replace {
            node: n,
            continuing: false,
        },
    );
}
fn constant_color(e: &Editor, id: NodeId) -> [f64; 3] {
    match &node(e, id).kind {
        NodeKind::Texture(TextureLayer {
            source: TextureSource::Constant { color, .. },
            ..
        }) => *color,
        _ => panic!("not a constant"),
    }
}

#[test]
fn unnumbered_strips_the_index() {
    assert_eq!(unnumbered("Layer (2)"), "Layer");
    assert_eq!(unnumbered("Layer (12)"), "Layer");
    assert_eq!(unnumbered("Layer"), "Layer");
    assert_eq!(unnumbered("Layer (1)"), "Layer (1)");
    assert_eq!(unnumbered("(2)"), "(2)");
    assert_eq!(unnumbered("Noise 3D"), "Noise 3D");
}

#[test]
fn duplicate_lands_above_with_the_next_number_and_its_own_locator() {
    let mut e = Editor::default();
    let group = add(&mut e, NewLayer::Group, None);
    let layer = add(&mut e, NewLayer::Constant, Some(group));
    let renamed = Node {
        name: "Layer".into(),
        ..node(&e, layer).clone()
    };
    edit(
        &mut e,
        ShaderTreeEdit::Replace {
            node: renamed,
            continuing: false,
        },
    );
    let loc = locator(&e, layer);
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == loc)
        .unwrap()
        .transform
        .translation = DVec3::new(1., 2., 3.);
    let steps = e.undo.len();
    edit(&mut e, ShaderTreeEdit::Duplicate(vec![layer]));
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(order(&e, Some(group)), ["Layer (2)", "Layer"]);
    let copy = by_name(&e, "Layer (2)").id;
    // Its own locator, same settings and placement.
    let new_loc = locator(&e, copy);
    assert_ne!(new_loc, loc);
    assert_eq!(
        locator_item(&e, new_loc).texture_locator(),
        locator_item(&e, loc).texture_locator()
    );
    assert_eq!(
        locator_item(&e, new_loc).transform.translation,
        DVec3::new(1., 2., 3.)
    );
    // Independent: an edit on either never reaches the other.
    set_constant(&mut e, layer, [1., 0., 0.]);
    assert_eq!(constant_color(&e, copy), [0.; 3]);
    set_constant(&mut e, copy, [0., 1., 0.]);
    assert_eq!(constant_color(&e, layer), [1., 0., 0.]);
    // Again: the next free number.
    edit(&mut e, ShaderTreeEdit::Duplicate(vec![layer]));
    assert_eq!(order(&e, Some(group)), ["Layer (2)", "Layer (3)", "Layer"]);
    // Duplicating a copy numbers from the base name.
    edit(&mut e, ShaderTreeEdit::Duplicate(vec![copy]));
    assert!(
        e.scene
            .shader_tree
            .nodes
            .iter()
            .any(|n| n.name == "Layer (4)")
    );
    // Undo takes it all back, locators too.
    let items = e.scene.items.len();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items.len(), items - 1);
}

#[test]
fn duplicate_copies_a_group_with_its_contents() {
    let mut e = Editor::default();
    let group = add(&mut e, NewLayer::Group, None);
    let inner = add(&mut e, NewLayer::Material, Some(group));
    let tex = add(&mut e, NewLayer::Constant, Some(group));
    edit(&mut e, ShaderTreeEdit::Duplicate(vec![group, inner]));
    let tree = &e.scene.shader_tree;
    let copy = tree.top_level_since(tex + 1);
    assert_eq!(copy.len(), 1, "the inner layer goes with its group");
    let names: Vec<_> = tree
        .children(Some(copy[0]))
        .map(|n| n.type_label())
        .collect();
    assert_eq!(names, ["Constant", "Material"]);
    // The copy sits directly above the original group.
    let top: Vec<NodeId> = tree.children(None).map(|n| n.id).collect();
    let at = top.iter().position(|&i| i == group).unwrap();
    assert_eq!(top[at - 1], copy[0]);
    tree.validate().unwrap();
}

#[test]
fn copy_and_paste_above_the_selection_or_below_the_base_shader() {
    let mut e = Editor::default();
    let group = add(&mut e, NewLayer::Group, None);
    let a = add(&mut e, NewLayer::Constant, Some(group));
    let b = add(&mut e, NewLayer::Material, Some(group));
    let clip = e.scene.copy_shader_layers(&[a]);
    assert_eq!(clip.roots().count(), 1);
    assert_eq!(clip.locators.len(), 1);
    edit(
        &mut e,
        ShaderTreeEdit::Paste {
            clip: Box::new(clip.clone()),
            selected: Some(b),
        },
    );
    assert_eq!(
        order(&e, Some(group)),
        ["Constant (2)", "Material", "Constant"]
    );
    // Pasted directly above Material (the selection).
    let kids: Vec<&Node> = e.scene.shader_tree.children(Some(group)).collect();
    let at = kids.iter().position(|n| n.id == b).unwrap();
    assert_eq!(kids[at - 1].type_label(), "Constant");
    assert_ne!(kids[at - 1].id, a);
    assert_ne!(locator(&e, kids[at - 1].id), locator(&e, a));
    // Nothing selected: below the Base Shader at the top level.
    edit(
        &mut e,
        ShaderTreeEdit::Paste {
            clip: Box::new(clip),
            selected: None,
        },
    );
    let top = order(&e, None);
    let shader = top.iter().position(|n| n == "Base Shader").unwrap();
    assert!(top[shader + 1].starts_with("Constant ("), "{top:?}");
    // Empty clipboard.
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Paste {
            clip: Box::default(),
            selected: None,
        }))
        .unwrap_err();
    assert!(err.to_string().contains("Nothing to paste"));
}

#[test]
fn paste_values_needs_the_same_type_and_keeps_name_and_locator() {
    let mut e = Editor::default();
    let a = add(&mut e, NewLayer::Constant, None);
    let b = add(&mut e, NewLayer::Constant, None);
    let m = add(&mut e, NewLayer::Material, None);
    set_constant(&mut e, a, [0.2, 0.4, 0.6]);
    let mut changed = node(&e, a).clone();
    changed.layer.opacity = 0.5;
    edit(
        &mut e,
        ShaderTreeEdit::Replace {
            node: changed,
            continuing: false,
        },
    );
    let from = node(&e, a).clone();
    let (name, loc) = (node(&e, b).name.clone(), locator(&e, b));
    let steps = e.undo.len();
    edit(
        &mut e,
        ShaderTreeEdit::PasteValues {
            from: Box::new(from.clone()),
            targets: vec![b],
        },
    );
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(constant_color(&e, b), [0.2, 0.4, 0.6]);
    assert_eq!(node(&e, b).layer.opacity, 0.5);
    assert_eq!(node(&e, b).name, name);
    assert_eq!(locator(&e, b), loc);
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::PasteValues {
            from: Box::new(from),
            targets: vec![m],
        }))
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Paste Values needs a layer of the same type")
    );
    assert_eq!(e.undo.len(), steps + 1);
}

#[test]
fn editor_color_is_one_step_and_saves_as_version_31_instances_as_32() {
    let mut e = Editor::default();
    let a = add(&mut e, NewLayer::Material, None);
    let steps = e.undo.len();
    edit(
        &mut e,
        ShaderTreeEdit::SetEditorColor {
            ids: vec![a],
            color: Some(EditorColor::Cyan),
        },
    );
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(node(&e, a).layer.editor_color, Some(EditorColor::Cyan));
    // A pre-OpenPBR material, so the older version is what is tested.
    e.scene.shader_tree.legacy_openpbr();
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(text.contains("\"version\": 31"));
    let back = persistence::decode(text.as_bytes()).unwrap();
    assert_eq!(
        back.shader_tree.get(a).unwrap().layer.editor_color,
        Some(EditorColor::Cyan)
    );
    let older = text.replacen("\"version\": 31", "\"version\": 30", 1);
    assert!(persistence::decode(older.as_bytes()).is_err());
    edit(
        &mut e,
        ShaderTreeEdit::SetEditorColor {
            ids: vec![a],
            color: None,
        },
    );
    assert_eq!(node(&e, a).layer.editor_color, None);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(node(&e, a).layer.editor_color, Some(EditorColor::Cyan));
}

fn tagged_cube(e: &mut Editor, tags: [&str; 6]) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.active_mut().unwrap();
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    for (f, t) in tags.iter().enumerate() {
        mesh.faces[f].material = (*t).into();
    }
    item.mesh = mesh.into();
    item.id
}

#[test]
fn select_polygons_selects_what_the_group_mask_matches() {
    let mut e = Editor::default();
    let a = tagged_cube(&mut e, ["Red", "Red", "", "", "", ""]);
    let b = tagged_cube(&mut e, ["", "", "", "Red", "", ""]);
    let group = add(&mut e, NewLayer::Group, None);
    let mut g = node(&e, group).clone();
    if let NodeKind::Group(mask) = &mut g.kind {
        mask.tag = TagMask::Name("Red".into());
    }
    edit(
        &mut e,
        ShaderTreeEdit::Replace {
            node: g,
            continuing: false,
        },
    );
    let inner = add(&mut e, NewLayer::Constant, Some(group));
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    // A layer inside the group uses the group.
    edit(&mut e, ShaderTreeEdit::SelectPolygons(inner));
    let s = &e.scene.selection;
    assert_eq!(s.mode, Mode::Polygon);
    assert_eq!(s.item, Some(a));
    assert_eq!(s.faces, [0, 1]);
    assert_eq!(s.also, [b]);
    assert_eq!(s.others.get(&b).unwrap().faces, [3]);
    // Undo puts the selection back.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Item);
    // No group around it.
    let top = add(&mut e, NewLayer::Constant, None);
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::SelectPolygons(top)))
        .unwrap_err();
    assert!(err.to_string().contains("Material Group"));
}

// ---------------------------------------------------------------- stage 2
//
// 7. Change Type replaces a texture layer in place (same id, parent and
//    position), keeps Enable, Opacity, Blend Mode, Invert, Effect, its
//    masks and locator, resets the type's own settings to Add Layer's,
//    and names it "<name> (2)"; one undo step. Only texture layers, and
//    only to another type.

fn rename(e: &mut Editor, id: NodeId, name: &str) {
    let renamed = Node {
        name: name.into(),
        ..node(e, id).clone()
    };
    edit(
        e,
        ShaderTreeEdit::Replace {
            node: renamed,
            continuing: false,
        },
    );
}

#[test]
fn change_type_replaces_in_place_and_keeps_the_layer_settings() {
    use operandi_core::procedural::ProceduralKind;
    let mut e = Editor::default();
    let group = add(&mut e, NewLayer::Group, None);
    let below = add(&mut e, NewLayer::Material, Some(group));
    let layer = add(&mut e, NewLayer::Constant, Some(group));
    let mask = add(&mut e, NewLayer::Constant, None);
    edit(
        &mut e,
        ShaderTreeEdit::Move {
            ids: vec![mask],
            drop: Drop {
                parent: Some(layer),
                index: 0,
            },
        },
    );
    rename(&mut e, layer, "Layer");
    set_constant(&mut e, layer, [1., 0., 0.]);
    let mut n = node(&e, layer).clone();
    n.layer.opacity = 0.25;
    n.layer.blend = BlendMode::Multiply;
    n.layer.invert = true;
    if let NodeKind::Texture(t) = &mut n.kind {
        t.effect = TextureEffect::SpecularAmount;
    }
    edit(
        &mut e,
        ShaderTreeEdit::Replace {
            node: n,
            continuing: false,
        },
    );
    let loc = locator(&e, layer);
    let before = order(&e, Some(group));
    let steps = e.undo.len();
    let noise = NewLayer::Procedural(ProceduralKind::Noise);
    edit(
        &mut e,
        ShaderTreeEdit::ChangeType {
            node: layer,
            to: noise,
        },
    );
    assert_eq!(e.undo.len(), steps + 1);
    let changed = node(&e, layer);
    assert_eq!(changed.name, "Layer (2)");
    assert_eq!(changed.parent, Some(group));
    assert_eq!(changed.layer.opacity, 0.25);
    assert_eq!(changed.layer.blend, BlendMode::Multiply);
    assert!(changed.layer.invert);
    let NodeKind::Texture(t) = &changed.kind else {
        panic!()
    };
    assert_eq!(t.effect, TextureEffect::SpecularAmount);
    assert_eq!(t.locator, Some(loc));
    assert_eq!(
        t.source,
        TextureSource::Procedural(Box::new(operandi_core::procedural::Procedural::new(
            ProceduralKind::Noise
        )))
    );
    // Same place, mask still inside.
    let after = order(&e, Some(group));
    assert_eq!(after.len(), before.len());
    assert_eq!(after[0], "Layer (2)");
    assert_eq!(node(&e, below).parent, Some(group));
    assert_eq!(node(&e, mask).parent, Some(layer));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(constant_color(&e, layer), [1., 0., 0.]);
    assert_eq!(node(&e, layer).name, "Layer");
}

#[test]
fn change_type_names_and_refusals() {
    let mut e = Editor::default();
    let c = add(&mut e, NewLayer::Constant, None);
    // Still named after its type: takes the new type's name.
    edit(
        &mut e,
        ShaderTreeEdit::ChangeType {
            node: c,
            to: NewLayer::Gradient,
        },
    );
    assert_eq!(node(&e, c).name, "Gradient");
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::ChangeType {
            node: c,
            to: NewLayer::Gradient,
        }))
        .unwrap_err();
    assert!(err.to_string().contains("already"));
    let m = add(&mut e, NewLayer::Material, None);
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::ChangeType {
            node: m,
            to: NewLayer::Constant,
        }))
        .unwrap_err();
    assert!(err.to_string().contains("Only texture layers"));
    assert_eq!(
        NewLayer::texture_types().len(),
        7 + operandi_core::procedural::ProceduralKind::ALL.len()
    );
}

// ---------------------------------------------------------------- stage 3
//
// 8. Create Instance puts a linked clone directly above, named "(2)";
//    one undo step.
// 9. Source edits reach every instance setting that isn't overridden.
// 10. An instance edit overrides only that setting, down to one colour
//     component; the source isn't changed; later source edits to it don't
//     reach the instance. Revert to Source drops the override.
// 11. Deleting the source deletes its instances; deleting an instance
//     leaves the source.
// 12. Duplicating an instance makes a plain layer with the source's values.
// 13. Save/reopen keeps the link and overrides.

fn set_opacity(e: &mut Editor, id: NodeId, o: f64) {
    let mut n = node(e, id).clone();
    n.layer.opacity = o;
    edit(
        e,
        ShaderTreeEdit::Replace {
            node: n,
            continuing: false,
        },
    );
}

fn instance_rig() -> (Editor, NodeId, NodeId) {
    let mut e = Editor::default();
    let source = add(&mut e, NewLayer::Constant, None);
    rename(&mut e, source, "Layer");
    let steps = e.undo.len();
    edit(&mut e, ShaderTreeEdit::CreateInstance(vec![source]));
    assert_eq!(e.undo.len(), steps + 1);
    let inst = by_name(&e, "Layer (2)").id;
    (e, source, inst)
}

#[test]
fn create_instance_lands_above_and_follows_its_source() {
    let (mut e, source, inst) = instance_rig();
    let top: Vec<NodeId> = e.scene.shader_tree.children(None).map(|n| n.id).collect();
    let at = top.iter().position(|&i| i == source).unwrap();
    assert_eq!(top[at - 1], inst);
    assert_eq!(e.scene.shader_tree.instance_source(inst), Some(source));
    assert_eq!(e.scene.shader_tree.instances_of(source), [inst]);
    // Shares the source's locator.
    assert_eq!(locator(&e, inst), locator(&e, source));
    set_constant(&mut e, source, [0.5, 0.25, 0.75]);
    set_opacity(&mut e, source, 0.4);
    let mut n = node(&e, source).clone();
    if let NodeKind::Texture(t) = &mut n.kind {
        t.effect = TextureEffect::SpecularAmount;
    }
    edit(
        &mut e,
        ShaderTreeEdit::Replace {
            node: n,
            continuing: false,
        },
    );
    assert_eq!(constant_color(&e, inst), [0.5, 0.25, 0.75]);
    assert_eq!(node(&e, inst).layer.opacity, 0.4);
    assert!(
        matches!(&node(&e, inst).kind, NodeKind::Texture(t) if t.effect == TextureEffect::SpecularAmount)
    );
    // The eye is the row's own.
    edit(
        &mut e,
        ShaderTreeEdit::SetEnable {
            ids: vec![source],
            enable: false,
        },
    );
    assert!(node(&e, inst).layer.enable);
}

#[test]
fn overrides_are_per_setting_and_per_component() {
    let (mut e, source, inst) = instance_rig();
    // Override G on the instance.
    let mut n = node(&e, inst).clone();
    if let NodeKind::Texture(TextureLayer {
        source: TextureSource::Constant { color, .. },
        ..
    }) = &mut n.kind
    {
        color[1] = 0.9;
    }
    edit(
        &mut e,
        ShaderTreeEdit::Replace {
            node: n,
            continuing: false,
        },
    );
    assert_eq!(
        constant_color(&e, source),
        [0.; 3],
        "the source isn't changed"
    );
    // R and B still follow; G stays overridden.
    set_constant(&mut e, source, [0.1, 0.2, 0.3]);
    assert_eq!(constant_color(&e, inst), [0.1, 0.9, 0.3]);
    let labels = operandi_core::shader_instances::overridden_labels(node(&e, inst));
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].0, "Color");
    // Opacity on the instance: its own; the source's later opacity doesn't reach it.
    set_opacity(&mut e, inst, 0.5);
    set_opacity(&mut e, source, 0.2);
    assert_eq!(node(&e, inst).layer.opacity, 0.5);
    assert_eq!(node(&e, source).layer.opacity, 0.2);
    // Revert Color: G follows again, Opacity stays overridden.
    edit(
        &mut e,
        ShaderTreeEdit::RevertToSource {
            node: inst,
            paths: Some(labels[0].1.clone()),
        },
    );
    assert_eq!(constant_color(&e, inst), [0.1, 0.2, 0.3]);
    assert_eq!(node(&e, inst).layer.opacity, 0.5);
    edit(
        &mut e,
        ShaderTreeEdit::RevertToSource {
            node: inst,
            paths: None,
        },
    );
    assert_eq!(node(&e, inst).layer.opacity, 0.2);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(node(&e, inst).layer.opacity, 0.5);
}

#[test]
fn deleting_the_source_deletes_its_instances_but_not_the_other_way() {
    let (mut e, source, inst) = instance_rig();
    assert_eq!(e.scene.shader_tree.instances_deleted_with(&[source]), 1);
    assert_eq!(e.scene.shader_tree.instances_deleted_with(&[inst]), 0);
    edit(&mut e, ShaderTreeEdit::Remove(vec![inst]));
    assert!(e.scene.shader_tree.get(source).is_some());
    e.dispatch(Command::Undo).unwrap();
    let loc = locator(&e, source);
    edit(&mut e, ShaderTreeEdit::Remove(vec![source]));
    assert!(e.scene.shader_tree.get(inst).is_none());
    assert!(
        e.scene.items.iter().all(|i| i.id != loc),
        "the locator goes too"
    );
}

#[test]
fn duplicating_an_instance_copies_the_source_values_as_a_plain_layer() {
    let (mut e, source, inst) = instance_rig();
    set_constant(&mut e, source, [0.3, 0.3, 0.3]);
    set_opacity(&mut e, inst, 0.5);
    let first = e.scene.shader_tree.next_id;
    edit(&mut e, ShaderTreeEdit::Duplicate(vec![inst]));
    let copy = e.scene.shader_tree.top_level_since(first)[0];
    assert!(!e.scene.shader_tree.is_instance(copy));
    assert_eq!(node(&e, copy).layer.opacity, 1.);
    assert_eq!(constant_color(&e, copy), [0.3, 0.3, 0.3]);
    assert_ne!(locator(&e, copy), locator(&e, source));
}

#[test]
fn instances_save_and_reopen_with_their_overrides() {
    let (mut e, source, inst) = instance_rig();
    set_opacity(&mut e, inst, 0.5);
    // A pre-OpenPBR material, so the older version is what is tested.
    e.scene.shader_tree.legacy_openpbr();
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(text.contains("\"version\": 32"));
    assert!(text.contains("/layer/opacity"));
    // A file whose instance values are stale gets the source's on load.
    let mut stale = e.scene.clone();
    set_constant(&mut e, source, [0.7, 0.7, 0.7]);
    let src = e.scene.shader_tree.get(source).unwrap().clone();
    let i = stale
        .shader_tree
        .nodes
        .iter()
        .position(|n| n.id == source)
        .unwrap();
    stale.shader_tree.nodes[i] = src;
    let back = persistence::decode(&persistence::encode(&stale).unwrap()).unwrap();
    let n = back.shader_tree.get(inst).unwrap();
    assert_eq!(n.layer.opacity, 0.5);
    assert!(matches!(&n.kind, NodeKind::Texture(TextureLayer {
        source: TextureSource::Constant { color, .. }, ..
    }) if *color == [0.7; 3]));
}

#[test]
fn change_type_on_an_instance_is_refused_and_follows_the_source() {
    let (mut e, source, inst) = instance_rig();
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::ChangeType {
            node: inst,
            to: NewLayer::Gradient,
        }))
        .unwrap_err();
    assert!(err.to_string().contains("instance"));
    set_opacity(&mut e, inst, 0.5);
    edit(
        &mut e,
        ShaderTreeEdit::ChangeType {
            node: source,
            to: NewLayer::Gradient,
        },
    );
    let n = node(&e, inst);
    assert!(
        matches!(&n.kind, NodeKind::Texture(t) if matches!(t.source, TextureSource::Gradient(_)))
    );
    assert_eq!(n.layer.opacity, 0.5, "Layer block overrides stay");
}

// ---------------------------------------------------------------- stage 4
//
// 14. Dragging a Material Group onto the Library makes it a Library item
//     (with its contents) that doesn't render; only groups go in.
// 15. Dropping a Library item into the tree adds a reference that shades
//     as the Library group's contents; editing the Library group updates
//     it.
// 16. De-Reference turns a reference into a real, unlinked copy.
// 17. Lock Preset refuses edits to the item until Unlock Preset.
// 18. Deleting a Library item deletes its references, or De-References
//    them first; a Library item can't be moved out by a plain move.
// 19. Save/reopen keeps the Library, references and locks.

use operandi_core::shader_eval::{Ch, Context};

fn red_ctx() -> Context<'static> {
    Context {
        item: 1,
        ancestors: &[],
        material: "Red",
        part: None,
        sets: &[],
        locators: &[],
    }
}
fn diffuse(e: &Editor) -> [f64; 3] {
    e.scene
        .shading_tree()
        .evaluate(&red_ctx())
        .get(Ch::DiffuseColor)
        .constant()
        .unwrap()
}
fn set_diffuse(e: &mut Editor, id: NodeId, c: [f64; 3]) {
    let mut n = node(e, id).clone();
    if let NodeKind::Material(m) = &mut n.kind {
        m.diffuse_color = c;
    }
    edit(
        e,
        ShaderTreeEdit::Replace {
            node: n,
            continuing: false,
        },
    );
}

/// A Material Group masked to "Red" holding a red Material, moved into
/// the Library.
fn library_rig() -> (Editor, NodeId, NodeId) {
    let mut e = Editor::default();
    let group = add(&mut e, NewLayer::Group, None);
    let mut g = node(&e, group).clone();
    g.name = "Paint".into();
    if let NodeKind::Group(mask) = &mut g.kind {
        mask.tag = TagMask::Name("Red".into());
    }
    edit(
        &mut e,
        ShaderTreeEdit::Replace {
            node: g,
            continuing: false,
        },
    );
    let material = add(&mut e, NewLayer::Material, Some(group));
    set_diffuse(&mut e, material, [1., 0., 0.]);
    assert_eq!(diffuse(&e), [1., 0., 0.]);
    edit(&mut e, ShaderTreeEdit::ToLibrary(vec![group]));
    (e, group, material)
}

#[test]
fn library_items_do_not_render_and_only_groups_go_in() {
    let (mut e, group, material) = library_rig();
    assert!(node(&e, group).extras.library);
    assert!(e.scene.shader_tree.in_library(material));
    assert_ne!(diffuse(&e), [1., 0., 0.], "a Library item doesn't render");
    assert!(e.scene.shading_tree().get(material).is_none());
    let m = add(&mut e, NewLayer::Material, None);
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::ToLibrary(vec![m])))
        .unwrap_err();
    assert!(err.to_string().contains("Only Material Groups"));
    // Set Material doesn't reach into the Library.
    assert_ne!(e.scene.shader_tree.material_for_tag("Red"), Some(material));
}

fn reference(e: &mut Editor, library: NodeId) -> NodeId {
    let id = e.scene.shader_tree.next_id;
    // Directly below the Base Shader, where the group was.
    let index = e
        .scene
        .shader_tree
        .children(None)
        .position(|n| n.name == "Base Shader")
        .unwrap()
        + 1;
    edit(
        e,
        ShaderTreeEdit::AddReference {
            library,
            drop: Drop {
                parent: None,
                index,
            },
        },
    );
    id
}

#[test]
fn a_reference_shades_as_its_library_group_and_follows_edits() {
    let (mut e, group, material) = library_rig();
    let r = reference(&mut e, group);
    assert_eq!(node(&e, r).name, "Paint (2)");
    assert_eq!(node(&e, r).type_label(), "Reference");
    assert_eq!(e.scene.shader_tree.references_to(group), [r]);
    assert_eq!(diffuse(&e), [1., 0., 0.]);
    set_diffuse(&mut e, material, [0., 0., 1.]);
    assert_eq!(diffuse(&e), [0., 0., 1.]);
    // The reference's eye turns it off.
    edit(
        &mut e,
        ShaderTreeEdit::SetEnable {
            ids: vec![r],
            enable: false,
        },
    );
    assert_ne!(diffuse(&e), [0., 0., 1.]);
    // A reference holds nothing and can't go into the Library.
    assert!(!node(&e, r).is_container());
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::AddReference {
            library: group,
            drop: Drop {
                parent: Some(group),
                index: 0,
            },
        }))
        .unwrap_err();
    assert!(err.to_string().contains("inside the Library"));
}

#[test]
fn dereference_makes_an_unlinked_copy() {
    let (mut e, group, material) = library_rig();
    let r = reference(&mut e, group);
    let steps = e.undo.len();
    edit(&mut e, ShaderTreeEdit::DeReference(vec![r]));
    assert_eq!(e.undo.len(), steps + 1);
    assert!(e.scene.shader_tree.get(r).is_none());
    let copy = by_name(&e, "Paint (2)");
    assert!(matches!(copy.kind, NodeKind::Group(_)));
    assert!(!copy.extras.library && copy.extras.reference.is_none());
    assert_eq!(diffuse(&e), [1., 0., 0.]);
    set_diffuse(&mut e, material, [0., 1., 0.]);
    assert_eq!(diffuse(&e), [1., 0., 0.], "no longer linked");
}

#[test]
fn locked_library_items_refuse_edits_until_unlocked() {
    let (mut e, group, material) = library_rig();
    edit(
        &mut e,
        ShaderTreeEdit::SetLocked {
            node: group,
            locked: true,
        },
    );
    let mut n = node(&e, material).clone();
    if let NodeKind::Material(m) = &mut n.kind {
        m.diffuse_color = [0., 1., 0.];
    }
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
            node: n.clone(),
            continuing: false,
        }))
        .unwrap_err();
    assert!(err.to_string().contains("locked"), "{err}");
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::Constant,
            selected: Some(group),
        }))
        .unwrap_err();
    assert!(err.to_string().contains("locked"));
    // Editor colours are the row's, not settings.
    edit(
        &mut e,
        ShaderTreeEdit::SetEditorColor {
            ids: vec![material],
            color: Some(EditorColor::Red),
        },
    );
    edit(
        &mut e,
        ShaderTreeEdit::SetLocked {
            node: group,
            locked: false,
        },
    );
    edit(
        &mut e,
        ShaderTreeEdit::Replace {
            node: n,
            continuing: false,
        },
    );
    // Only a Library item locks.
    let m = add(&mut e, NewLayer::Material, None);
    assert!(
        e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetLocked {
            node: m,
            locked: true,
        }))
        .is_err()
    );
}

#[test]
fn deleting_a_library_item_takes_or_dereferences_its_references() {
    let (mut e, group, _) = library_rig();
    let r = reference(&mut e, group);
    assert_eq!(e.scene.shader_tree.references_deleted_with(&[group]), [r]);
    // A plain move can't take it out of the Library.
    let err = e
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Move {
            ids: vec![group],
            drop: Drop {
                parent: None,
                index: 0,
            },
        }))
        .unwrap_err();
    assert!(err.to_string().contains("reference"));
    edit(&mut e, ShaderTreeEdit::Remove(vec![group]));
    assert!(e.scene.shader_tree.get(r).is_none());
    e.dispatch(Command::Undo).unwrap();
    edit(&mut e, ShaderTreeEdit::DeReferenceAndRemove(vec![group]));
    assert!(e.scene.shader_tree.get(group).is_none());
    assert!(matches!(by_name(&e, "Paint (2)").kind, NodeKind::Group(_)));
    assert_eq!(diffuse(&e), [1., 0., 0.]);
}

#[test]
fn library_references_and_locks_save_and_reopen() {
    let (mut e, group, _) = library_rig();
    let r = reference(&mut e, group);
    edit(
        &mut e,
        ShaderTreeEdit::SetLocked {
            node: group,
            locked: true,
        },
    );
    // A pre-OpenPBR material, so the older version is what is tested.
    e.scene.shader_tree.legacy_openpbr();
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(text.contains("\"version\": 32"));
    let back = persistence::decode(text.as_bytes()).unwrap();
    assert!(back.shader_tree.get(group).unwrap().extras.locked);
    assert_eq!(back.shader_tree.reference_target(r), Some(group));
    let older = text.replacen("\"version\": 32", "\"version\": 31", 1);
    assert!(persistence::decode(older.as_bytes()).is_err());
}
