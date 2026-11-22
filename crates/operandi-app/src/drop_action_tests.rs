//! Drop Action viewport gesture (spec drop-action-2026-11-06, DA-4..9):
//! in Item mode with no tool and an action chosen, press on an item, drag
//! and release on another to apply it; one undo step.
use super::tests::{draw, key, pointer};
use super::*;
use operandi_core::DropAction;

struct Rig {
    a: Operandi,
    ctx: Context,
    size: Vec2,
    left: u64,
    right: u64,
}
impl Rig {
    /// Two 2 m boxes side by side, seen from the top, in Item mode.
    fn new(action: DropAction) -> Self {
        let ctx = Context::default();
        theme::install(&ctx);
        let size = vec2(1440., 900.);
        let mut a = Operandi::empty();
        for x in [-3., 3.] {
            a.run(Command::NewMesh);
            let id = a.editor.scene.items.last().unwrap().id;
            a.editor.scene.active_mut().unwrap().mesh =
                operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
                    .unwrap()
                    .into();
            a.run(Command::ItemTransform {
                item: id,
                transform: operandi_core::scene::Transform {
                    translation: DVec3::new(x, 0., 0.),
                    ..Default::default()
                },
            });
        }
        a.camera.view = View::Top;
        a.run(Command::Mode(Mode::Item));
        a.run(Command::SelectItem(None));
        a.drop_action.action = action;
        a.editor.undo.clear();
        let left = a.editor.scene.items[0].id;
        let right = a.editor.scene.items[1].id;
        let mut rig = Self {
            a,
            ctx,
            size,
            left,
            right,
        };
        rig.frame(vec![]);
        rig.a.frame(false);
        rig.frame(vec![]);
        rig
    }
    fn frame(&mut self, events: Vec<Event>) {
        draw(&mut self.a, &self.ctx, self.size, events);
    }
    /// A viewport point over item `id`.
    fn over(&self, id: u64) -> Pos2 {
        let r = self.a.viewport_rect;
        for y in (0..r.height() as i32).step_by(12) {
            for x in (0..r.width() as i32).step_by(12) {
                let p = r.left_top() + vec2(x as f32, y as f32);
                if self.a.pick_component(p, r) == Some((id, Component::Item)) {
                    return p;
                }
            }
        }
        panic!("item {id} not on screen");
    }
    fn empty(&self) -> Pos2 {
        let p = self.a.viewport_rect.left_top() + vec2(60., 60.);
        assert_eq!(self.a.pick_component(p, self.a.viewport_rect), None);
        p
    }
    fn press(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![pointer(at, true, Modifiers::NONE)]);
    }
    fn move_to(&mut self, from: Pos2, to: Pos2) {
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(
                from + (to - from) * (step as f32 / 4.),
            )]);
        }
    }
    fn release(&mut self, at: Pos2) {
        self.frame(vec![pointer(at, false, Modifiers::NONE)]);
    }
    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.press(from);
        self.move_to(from, to);
        self.release(to);
    }
    fn item(&self, id: u64) -> &operandi_core::Item {
        self.a.editor.scene.item(id).unwrap()
    }
}

/// DA-4/6/8: Parent by dragging left onto right; the source doesn't move
/// during the drag; one undo step "Parent".
#[test]
fn drag_onto_another_item_parents_it() {
    let mut rig = Rig::new(DropAction::Parent);
    let (from, to) = (rig.over(rig.left), rig.over(rig.right));
    let before = rig.a.editor.scene.clone();
    rig.press(from);
    rig.move_to(from, to);
    // Nothing changes until release, and the target is highlighted.
    assert_eq!(rig.item(rig.left).parent, None);
    assert_eq!(rig.item(rig.left).transform, before.items[0].transform);
    assert_eq!(
        rig.a.drop_action_target(to, rig.a.viewport_rect),
        Some(rig.right)
    );
    rig.release(to);
    assert_eq!(
        rig.item(rig.left).parent,
        Some(rig.right),
        "{}",
        rig.a.status
    );
    assert_eq!(
        rig.a
            .editor
            .history_eligibility(operandi_core::HistoryAction::Undo)
            .label
            .map(|l| l.to_string())
            .as_deref(),
        Some("Parent")
    );
    rig.a.run(Command::Undo);
    assert_eq!(rig.item(rig.left).parent, None);
    assert_eq!(rig.a.editor.scene.items, before.items);
}

/// DA-6: releasing on empty space, on the source, or after Esc does
/// nothing; a click still selects.
#[test]
fn empty_space_source_and_escape_do_nothing() {
    let mut rig = Rig::new(DropAction::MatchPosition);
    let (from, to, empty) = (rig.over(rig.left), rig.over(rig.right), rig.empty());
    let items = rig.a.editor.scene.items.clone();
    rig.drag(from, empty);
    assert_eq!(rig.a.editor.scene.items, items);
    rig.drag(from, from + vec2(6., 0.));
    assert_eq!(rig.a.editor.scene.items, items);
    rig.press(from);
    rig.move_to(from, to);
    rig.frame(vec![key(Key::Escape)]);
    assert!(rig.a.drop_drag.is_none());
    rig.release(to);
    assert_eq!(rig.a.editor.scene.items, items);
    // A click selects the item, as without a Drop Action.
    rig.press(to);
    rig.release(to);
    assert_eq!(rig.a.editor.scene.selection.item, Some(rig.right));
    assert_eq!(rig.a.editor.scene.items, items);
}

/// DA-5: with (none) a drag in Item mode changes nothing, as before.
#[test]
fn none_keeps_todays_drag() {
    let mut rig = Rig::new(DropAction::None);
    let (from, to) = (rig.over(rig.left), rig.over(rig.right));
    let items = rig.a.editor.scene.items.clone();
    rig.drag(from, to);
    assert!(rig.a.drop_drag.is_none());
    assert_eq!(rig.a.editor.scene.items, items);
}

/// DA-14 through the gesture: Match moves the left box onto the right one.
#[test]
fn match_through_the_gesture() {
    let mut rig = Rig::new(DropAction::Match);
    let (from, to) = (rig.over(rig.left), rig.over(rig.right));
    rig.drag(from, to);
    assert_eq!(
        rig.item(rig.left).transform.translation,
        DVec3::new(3., 0., 0.),
        "{}",
        rig.a.status
    );
}

/// DA-21: Place puts the source's pivot on the box top under the pointer.
#[test]
fn place_lands_on_the_surface_under_the_pointer() {
    let mut rig = Rig::new(DropAction::Place);
    let (from, to) = (rig.over(rig.left), rig.over(rig.right));
    rig.drag(from, to);
    let t = rig.item(rig.left).transform.translation;
    // The top of a 2 m box centred at y = 0.
    assert!((t.y - 1.).abs() < 1e-6, "{t:?} {}", rig.a.status);
    assert!((t.x - 3.).abs() <= 1., "{t:?}");
}

/// DA-2/3: the menu entries and the saved choice; not an undo step.
#[test]
fn choosing_an_action_is_not_an_undo_step() {
    let mut rig = Rig::new(DropAction::None);
    let undo = rig.a.editor.undo.len();
    rig.a.set_drop_action(DropAction::Insert);
    assert_eq!(rig.a.drop_action.action, DropAction::Insert);
    assert_eq!(rig.a.editor.undo.len(), undo);
    // DA-Q4: the Insert family takes only the pressed item.
    rig.a.editor.scene.selection.item = Some(rig.left);
    rig.a.editor.scene.selection.also = vec![rig.right];
    assert_eq!(
        rig.a.drop_sources(DropAction::Insert, rig.left),
        vec![rig.left]
    );
    assert_eq!(
        rig.a.drop_sources(DropAction::Parent, rig.right),
        vec![rig.left, rig.right]
    );
}
