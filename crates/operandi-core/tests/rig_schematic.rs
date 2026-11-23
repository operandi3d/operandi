//! BEHAVIOR-RIG-PHASE4 §7 in core: the Schematic's nodes and wires
//! (RG-70), wiring from it (RG-71) and deleting wires (RG-72).
use glam::DVec3;
use operandi_core::*;

fn line(e: &mut Editor, text: &str) -> Result<Option<String>> {
    command_line::run_line(e, text).expect("headless")
}

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = line(e, text) {
        panic!("{text}: {err}");
    }
}

fn item(e: &mut Editor) -> u64 {
    e.scene.new_mesh().unwrap();
    e.scene.items.last().unwrap().id
}

#[test]
fn rg70_pieces_and_used_channels_are_nodes_and_wires() {
    let mut e = Editor::default();
    let a = item(&mut e);
    let b = item(&mut e);
    let c = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a}"),
    );
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{b}:scl.Z driver:{a}:pos.X"),
    );
    let s = rig_schematic::build(&e.scene);
    assert_eq!(s.nodes.iter().filter(|n| n.piece).count(), 2);
    let node = |id: u64| {
        s.nodes
            .iter()
            .find(|n| n.key == format!("item:{id}"))
            .unwrap()
    };
    // a: only the ports in use (its world for the constraint, pos.X for the link).
    assert_eq!(node(a).outputs, ["world", "pos.X"]);
    assert_eq!(node(b).inputs, ["scl.Z"]);
    assert_eq!(node(c).inputs, ["world"]);
    assert_eq!(node(a).column, 0);
    assert_eq!(s.wires.len(), 4);
    // As a query too.
    let q = command_exec::answer(&e, "operandi.rig.schematic", &Default::default())
        .unwrap()
        .unwrap();
    assert!(q.contains("\"wires\""));
}

#[test]
fn rg71_a_modifier_added_alone_is_wired_by_hand_and_rg72_unwired() {
    let mut e = Editor::default();
    let a = item(&mut e);
    let b = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.modifierAdd kind:math item:{b}"),
    );
    let m = e.scene.animation.rig.pieces[0].id;
    let s = rig_schematic::build(&e.scene);
    assert!(s.wires.is_empty(), "created, not wired");
    ok(
        &mut e,
        &format!("operandi.rig.modifierSet piece:{m} inputs:A={a}:pos.Y output:{b}:pos.X"),
    );
    e.scene.item_mut(a).unwrap().transform.translation.y = 3.;
    let shown = e.evaluated(evaluate::Purpose::Final);
    assert_eq!(shown.item(b).unwrap().transform.translation.x, 3.);
    assert_eq!(rig_schematic::build(&e.scene).wires.len(), 2);
    // Deleting the input wire unwires it; the output wire too.
    ok(&mut e, &format!("operandi.rig.unwire piece:{m} port:A"));
    ok(&mut e, &format!("operandi.rig.unwire piece:{m} port:out"));
    assert!(rig_schematic::build(&e.scene).wires.is_empty());
    // A constraint loses a target; with its last, the piece.
    let c = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a},{b}"),
    );
    let p = e.scene.animation.rig.pieces.last().unwrap().id;
    ok(
        &mut e,
        &format!("operandi.rig.unwire piece:{p} port:{{target 1}}"),
    );
    assert_eq!(e.scene.animation.rig.piece(p).unwrap().targets, [b]);
    ok(
        &mut e,
        &format!("operandi.rig.unwire piece:{p} port:{{target 1}}"),
    );
    assert!(e.scene.animation.rig.piece(p).is_none());
    // A link wire removes the link.
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{b}:scl.Z driver:{a}:pos.X"),
    );
    let l = e.scene.animation.rig.pieces.last().unwrap().id;
    ok(
        &mut e,
        &format!("operandi.rig.unwire piece:{l} port:driver"),
    );
    assert!(e.scene.animation.rig.piece(l).is_none());
    let _ = DVec3::ZERO;
}

#[test]
fn rg02_broken_pieces_show_as_broken_nodes() {
    let mut e = Editor::default();
    let a = item(&mut e);
    let b = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{a} targets:{b}"),
    );
    e.scene
        .animation
        .rig
        .pieces
        .push(rig::Piece::new(900, "position", b, vec![a]));
    let s = rig_schematic::build(&e.scene);
    assert_eq!(s.nodes.iter().filter(|n| n.broken).count(), 2);
}
