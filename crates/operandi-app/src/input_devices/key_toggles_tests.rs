//! MN-02: key toggles. Hold a tool's key and drag to use it; let go to
//! drop it. A tap still turns the tool on.
use super::tests::{box_app, draw, pointer};
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

struct Rig {
    a: Operandi,
    ctx: Context,
}

fn key_event(key: Key, pressed: bool, repeat: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat,
        modifiers: Modifiers::NONE,
    }
}

impl Rig {
    fn new() -> Self {
        let ctx = Context::default();
        theme::install(&ctx);
        let mut a = box_app();
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![0];
        a.editor.history.undo.clear();
        let mut rig = Self { a, ctx };
        rig.frame(vec![]);
        rig.frame(vec![]);
        rig
    }
    fn frame(&mut self, events: Vec<Event>) {
        draw(&mut self.a, &self.ctx, SIZE, events);
    }
    fn press(&mut self, key: Key) {
        let at = self.corner();
        self.frame(vec![Event::PointerMoved(at), key_event(key, true, false)]);
    }
    fn release(&mut self, key: Key) {
        self.frame(vec![key_event(key, false, false)]);
        self.frame(vec![]);
    }
    /// Away from the mesh and every handle: a haul.
    fn corner(&self) -> Pos2 {
        self.a.viewport_rect.left_top() + vec2(80., 80.)
    }
    fn drag_down(&mut self) {
        self.drag_down_held();
        self.drag_release();
    }
    fn drag_down_held(&mut self) {
        let from = self.corner();
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![pointer(from, true, Modifiers::NONE)]);
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(
                from + vec2(0., 15. * step as f32),
            )]);
        }
    }
    fn drag_release(&mut self) {
        let to = self.corner() + vec2(0., 60.);
        self.frame(vec![pointer(to, false, Modifiers::NONE)]);
        self.frame(vec![]);
    }
    fn tool(&self) -> Option<Tool> {
        self.a.editor.session.live.as_ref().map(|l| l.tool)
    }
    fn mesh(&self) -> Vec<DVec3> {
        self.a.editor.scene.active().unwrap().mesh.vertices.clone()
    }
}

#[test]
fn hold_drag_release_uses_the_tool_once_and_drops_it() {
    let mut r = Rig::new();
    let before = r.mesh();
    r.press(Key::W);
    assert_eq!(r.tool(), Some(Tool::Move), "{}", r.a.status);
    r.drag_down();
    assert_eq!(r.tool(), Some(Tool::Move), "still held");
    let moved = r.mesh();
    assert_ne!(moved, before, "{}", r.a.status);
    r.release(Key::W);
    assert_eq!(r.tool(), None, "released: back to selecting");
    assert_eq!(r.mesh(), moved, "the edit stays");
    // One undo step, named after the tool.
    assert_eq!(r.a.editor.history.undo.len(), 1);
    let undo = r.a.line_state("app.undo").unwrap().title.unwrap();
    assert!(undo.contains("Move"), "{undo}");
    r.a.run(Command::Undo);
    assert_eq!(r.mesh(), before);
}

#[test]
fn a_tap_keeps_the_tool_on() {
    let mut r = Rig::new();
    r.press(Key::W);
    r.release(Key::W);
    assert_eq!(r.tool(), Some(Tool::Move));
    // A drag after the key came up is ordinary use.
    r.drag_down();
    assert_eq!(r.tool(), Some(Tool::Move));
}

#[test]
fn releasing_mid_drag_drops_once_the_drag_ends() {
    let mut r = Rig::new();
    r.press(Key::W);
    r.drag_down_held();
    r.release(Key::W);
    assert_eq!(r.tool(), Some(Tool::Move), "the drag is still going");
    r.drag_release();
    assert_eq!(r.tool(), None);
    assert_eq!(r.a.editor.history.undo.len(), 1);
}

#[test]
fn a_tool_already_on_is_not_dropped() {
    let mut r = Rig::new();
    r.a.activate(Tool::Move);
    r.frame(vec![]);
    r.press(Key::W);
    let on = r.tool();
    r.drag_down();
    r.release(Key::W);
    assert_eq!(
        r.tool(),
        on,
        "holding the active tool's key changes nothing"
    );
}

#[test]
fn key_repeat_does_not_toggle_the_tool() {
    let mut r = Rig::new();
    r.press(Key::W);
    for _ in 0..5 {
        r.frame(vec![key_event(Key::W, true, true)]);
    }
    assert_eq!(r.tool(), Some(Tool::Move));
    r.release(Key::W);
    assert_eq!(r.tool(), Some(Tool::Move));
}

#[test]
fn holding_another_tools_key_swaps_back_after() {
    let mut r = Rig::new();
    r.a.activate(Tool::Rotate);
    r.frame(vec![]);
    r.press(Key::W);
    assert_eq!(r.tool(), Some(Tool::Move), "{}", r.a.status);
    r.drag_down();
    r.release(Key::W);
    assert_eq!(r.tool(), Some(Tool::Rotate), "back to the first tool");
}

#[test]
fn the_preference_turns_key_toggles_off() {
    assert!(preferences::InputPreferences::default().key_toggles);
    let mut r = Rig::new();
    r.a.input_preferences.key_toggles = false;
    r.press(Key::W);
    r.drag_down();
    r.release(Key::W);
    assert_eq!(r.tool(), Some(Tool::Move), "off: the key only turns it on");
}
