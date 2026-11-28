//! Viewport side of the shared drag layer ("drag controls", hauls in code;
//! loop-slice-2026-11-15/BEHAVIOR-DRAG-LAYER.md, BIBLE UIC-76). The table
//! and the axis lock live in `operandi_core::haul`; this file starts a haul
//! from a press and, each frame, writes the new value through the tool's
//! own Tool Properties field, so a haul is checked, clamped and previewed
//! exactly like typing in that field.
//!
//! Where it sits (DL-01): presses on a tool's handles, its on-screen
//! controls (Loop Slice's bar) and the falloff layer's own right-drag are
//! taken before this; a button the tool's row doesn't list falls through to
//! the tool's own drag (DL-09). The whole press-to-release is one undo
//! tweak of the session (the press holds the pointer).
use super::*;
use crate::form::{Kind, Unit};
use crate::tool_form::{Bind, ToolFields};
use operandi_core::haul::{self, Button, Rate, Tracker, feel};

/// What a press that never moved does instead of hauling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum HaulClick {
    Nothing,
    /// Loop Slice: the click picks a ring (LS-03), as before hauls.
    LoopSliceRing(Modifiers),
    /// Smooth Shift: a middle click clones the shift, as before hauls.
    CloneSmoothShift,
}

/// The value a haul has locked onto.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Locked {
    key: String,
    label: String,
    start: f64,
    rate: Rate,
    /// The last value the tool took (the field may clamp).
    shown: f64,
}

/// Whose values a haul drives: the live editor tool, or a create tool
/// that runs its own numeric session (Cone, Ellipsoid, Torus, Capsule, Arc,
/// Tube; GitHub #13).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Owner {
    Tool(Tool),
    Session(haul::Session),
    /// The selected Mesh Stack primitive op, with no tool on (GitHub #26):
    /// its values change through the op, one undo step per drag.
    Op {
        op: operandi_core::mesh_stack_editor::OpRef,
        row: &'static haul::Row,
        gesture: u64,
    },
}
impl Owner {
    fn row(self) -> Option<&'static haul::Row> {
        match self {
            Owner::Tool(t) => haul::row(t),
            Owner::Session(s) => haul::session_row(s),
            Owner::Op { row, .. } => Some(row),
        }
    }
}

/// One haul in progress.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HaulDrag {
    pub(crate) owner: Owner,
    pub(crate) button: Button,
    pub(crate) tracker: Tracker,
    pub(crate) click: HaulClick,
    locked: Option<Locked>,
    /// Metres per screen pixel at the press, for distance values: the same
    /// view scale as the drag anywhere (`update_off_handle`).
    metres_per_pixel: f64,
    /// The axis has locked and the drag has begun changing a value: the
    /// release does no click.
    started: bool,
}

/// A field's number, unit and greying, as the haul needs them.
struct FieldInfo {
    value: f64,
    unit: Unit,
    label: String,
    greyed: Option<String>,
}

/// Mac stand-ins for the buttons a trackpad lacks (DL-Q04, provisional):
/// Cmd+left-drag hauls like a middle drag, Cmd+Shift+left-drag like a
/// right drag. Ctrl is left alone: on a Mac a Ctrl+click is the real Ctrl
/// and never a right-click (BIBLE "Keys", FLAGS F1), and Ctrl already
/// constrains drags. Cmd and Cmd+Shift mean nothing to a live tool off its
/// handles (the Cmd menu and Cmd+Shift Fill only run with no tool).
pub(crate) fn trackpad_button(mods: Modifiers) -> Option<Button> {
    if !mods.mac_cmd || mods.alt || mods.ctrl {
        return None;
    }
    Some(if mods.shift {
        Button::Right
    } else {
        Button::Middle
    })
}

pub(crate) fn button_of(button: PointerButton) -> Option<Button> {
    match button {
        PointerButton::Primary => Some(Button::Left),
        PointerButton::Middle => Some(Button::Middle),
        PointerButton::Secondary => Some(Button::Right),
        _ => None,
    }
}

/// Which panel serves the owner's fields.
#[derive(Clone, Copy)]
enum Panel {
    Live,
    Primitive,
    Surface,
    Cone,
    Ellipsoid,
    Shape,
}
fn panel(owner: Owner) -> Panel {
    let tool = match owner {
        Owner::Tool(tool) => tool,
        Owner::Session(haul::Session::Cone) => return Panel::Cone,
        Owner::Session(haul::Session::Ellipsoid) => return Panel::Ellipsoid,
        Owner::Session(_) => return Panel::Shape,
        // Served by `op_field` / `op_set` before the panels.
        Owner::Op { .. } => return Panel::Live,
    };
    if tool.is_primitive() {
        Panel::Primitive
    } else if tool.is_smooth_shift() {
        Panel::Surface
    } else {
        Panel::Live
    }
}

fn field_info<T: ToolFields>(app: &Operandi, key: &str) -> Option<FieldInfo> {
    let p = T::params(app)?;
    let form = T::form(app, &p);
    let field = form.fields().into_iter().find(|f| f.row.key == key)?;
    let (Bind::Number { get, .. }, Kind::Number { unit, .. }) = (&field.bind, &field.row.kind)
    else {
        return None;
    };
    Some(FieldInfo {
        value: get(&p),
        unit: *unit,
        label: field.row.label.clone(),
        greyed: field.disabled_tip.clone(),
    })
}

/// Write `value` through field `key`'s own setter and publish it. Returns
/// the value the tool then shows.
fn field_set<T: ToolFields>(app: &mut Operandi, key: &str, value: f64) -> Result<f64> {
    let p = T::params(app).ok_or("The tool isn't active")?;
    let form = T::form(app, &p);
    let field = form
        .fields()
        .into_iter()
        .find(|f| f.row.key == key)
        .ok_or_else(|| format!("{key} isn't on this tool's panel"))?;
    let Bind::Number { get, set } = &field.bind else {
        return Err(format!("{key} isn't a number"));
    };
    let mut q = p.clone();
    set(&mut q, value)?;
    T::publish(app, q)?;
    T::published(app);
    let p = T::params(app).ok_or("The tool isn't active")?;
    Ok(get(&p))
}

impl Operandi {
    fn haul_field(&self, owner: Owner, key: &str) -> Option<FieldInfo> {
        if let Owner::Op { op, .. } = owner {
            return self.op_haul_field(op, key);
        }
        match panel(owner) {
            Panel::Live => field_info::<crate::live_fields::LiveFields>(self, key),
            Panel::Primitive => field_info::<crate::primitive_panel::PrimitiveFields>(self, key),
            Panel::Surface => field_info::<crate::smooth_shift_panel::SurfaceFields>(self, key),
            Panel::Cone => field_info::<crate::cone_panel::ConeFields>(self, key),
            Panel::Ellipsoid => field_info::<crate::ellipsoid_panel::EllipsoidFields>(self, key),
            Panel::Shape => field_info::<crate::shape_panel::ShapeFields>(self, key),
        }
    }
    fn haul_set(&mut self, owner: Owner, key: &str, value: f64) -> Result<f64> {
        if let Owner::Op { op, gesture, .. } = owner {
            return self.op_haul_set(op, gesture, key, value);
        }
        match panel(owner) {
            Panel::Cone => field_set::<crate::cone_panel::ConeFields>(self, key, value),
            Panel::Ellipsoid => {
                field_set::<crate::ellipsoid_panel::EllipsoidFields>(self, key, value)
            }
            Panel::Shape => field_set::<crate::shape_panel::ShapeFields>(self, key, value),
            Panel::Live => field_set::<crate::live_fields::LiveFields>(self, key, value),
            Panel::Primitive => {
                field_set::<crate::primitive_panel::PrimitiveFields>(self, key, value)
            }
            Panel::Surface => {
                field_set::<crate::smooth_shift_panel::SurfaceFields>(self, key, value)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn haul_field_exists(&self, tool: Tool, key: &str) -> bool {
        self.haul_field(Owner::Tool(tool), key).is_some()
    }
    /// The armed create session's field `key`, as a haul reads it.
    #[cfg(test)]
    pub(crate) fn haul_session_value(&self, key: &str) -> Option<f64> {
        let session = self.haul_session(Button::Right)?;
        self.haul_field(Owner::Session(session), key)
            .map(|f| f.value)
    }

    /// The live tool, when the layer claims `button` for it.
    pub(crate) fn haul_tool(&self, button: Button) -> Option<Tool> {
        let tool = self.editor.session.live.as_ref()?.tool;
        haul::claims(tool, button).then_some(tool)
    }

    /// The create session whose row claims `button`, when no editor tool
    /// is live (GitHub #13).
    fn haul_session(&self, button: Button) -> Option<haul::Session> {
        use haul::Session;
        if self.editor.session.live.is_some() {
            return None;
        }
        let session = if self.editor.session.cone.is_some() {
            Session::Cone
        } else if self.editor.session.ellipsoid.is_some() {
            Session::Ellipsoid
        } else {
            match self.editor.session.shape.as_ref()?.kind() {
                ShapeKind::Torus => Session::Torus,
                ShapeKind::Capsule => Session::Capsule,
                ShapeKind::Arc => Session::Arc,
                ShapeKind::Tube => Session::Tube,
                ShapeKind::Text | ShapeKind::Teapot => return None,
            }
        };
        haul::session_row(session)
            .is_some_and(|r| r.claims(button))
            .then_some(session)
    }

    /// The selected primitive op, when its row claims `button` (GitHub
    /// #26). `selected_primitive_op` already needs no tool and no task.
    fn haul_op(&self, button: Button) -> Option<Owner> {
        let (op, _) = self.selected_primitive_op()?;
        let item = self.editor.scene.item(op.item)?;
        let stack = item.mesh_stack.as_ref()?;
        let kind = &stack.ops.iter().find(|o| o.id == op.op)?.kind;
        let row = haul::meshop_row(kind.type_id())?;
        row.claims(button).then(|| Owner::Op {
            op,
            row,
            gesture: crate::colour_field::next_gesture(),
        })
    }

    /// Whose values a drag with `button` drives, if the layer claims it.
    pub(crate) fn haul_owner(&self, button: Button) -> Option<Owner> {
        self.haul_tool(button)
            .map(Owner::Tool)
            .or_else(|| self.haul_session(button).map(Owner::Session))
            .or_else(|| self.haul_op(button))
    }

    /// Op value `key` as a haul reads it: a whole number, labelled from
    /// the op's own field.
    fn op_haul_field(
        &self,
        op: operandi_core::mesh_stack_editor::OpRef,
        key: &str,
    ) -> Option<FieldInfo> {
        let item = self.editor.scene.item(op.item)?;
        let kind = &item
            .mesh_stack
            .as_ref()?
            .ops
            .iter()
            .find(|o| o.id == op.op)?
            .kind;
        let value = kind.values().get(key)?.as_f64()?;
        let label = kind
            .def()
            .and_then(|d| {
                d.fields()
                    .iter()
                    .find(|f| f.key == key)
                    .map(|f| f.label.to_string())
            })
            .unwrap_or_else(|| key.to_owned());
        Some(FieldInfo {
            value,
            unit: Unit::Count,
            label,
            greyed: None,
        })
    }

    /// Sets op value `key` (at least 1) as one undo step per drag
    /// (SAFE-13, as the op's handles do); an op that refuses the value
    /// keeps the last one it took.
    fn op_haul_set(
        &mut self,
        op: operandi_core::mesh_stack_editor::OpRef,
        gesture: u64,
        key: &str,
        value: f64,
    ) -> Result<f64> {
        let n = value.round().max(1.) as u64;
        let mut values = serde_json::Map::new();
        values.insert(key.to_owned(), n.into());
        let previous = self.editor.history.undo_fold.replace(gesture);
        let ok = self.run_ok(Command::MeshOpSet { op, values });
        self.editor.history.undo_fold = previous;
        if !ok {
            return Err(self.status.clone());
        }
        self.op_haul_field(op, key)
            .map(|f| f.value)
            .ok_or_else(|| format!("{key} isn't on this operation"))
    }

    /// Start a haul with `button` at `pos`, or `None` when the live tool's
    /// row doesn't list that button (DL-09).
    pub(crate) fn begin_haul(
        &mut self,
        button: Button,
        pos: Pos2,
        r: Rect,
        click: HaulClick,
    ) -> Option<input::GestureKind> {
        let owner = self.haul_owner(button)?;
        let pair = owner.row()?.pair(button);
        Some(input::GestureKind::Haul(Box::new(HaulDrag {
            owner,
            button,
            tracker: Tracker::new([pos.x, pos.y], pair),
            click,
            locked: None,
            metres_per_pixel: self.camera.distance / (r.height() as f64 * 0.9),
            started: false,
        })))
    }

    /// What a press with `button` that never moves does.
    pub(crate) fn haul_click(&self, button: Button, _mods: Modifiers) -> HaulClick {
        let tool = self.editor.session.live.as_ref().map(|l| l.tool);
        match (tool, button) {
            (Some(t), Button::Middle) if t.is_smooth_shift() => HaulClick::CloneSmoothShift,
            _ => HaulClick::Nothing,
        }
    }

    /// Whether `pos` is on the live tool's own on-screen control, which
    /// takes the press before the drag layer (DL-01).
    pub(crate) fn haul_on_tool_control(&self, pos: Pos2, r: Rect) -> bool {
        match self.editor.session.live.as_ref() {
            Some(l) if l.tool == Tool::LoopSlice => self.loop_slice_bar_hit(&l.params, pos, r),
            _ => false,
        }
    }

    /// How fast field `key` changes per pixel (provisional; `haul::feel`).
    fn haul_rate(&self, key: &str, unit: Unit, mpp: f64) -> Rate {
        if key == "loop_slice_position" {
            // LS-Q02: one bar pixel per screen pixel; the field is percent.
            let length = self
                .editor
                .session
                .live
                .as_ref()
                .map_or(200., |l| l.params.slice.loop_slice.length.max(1.));
            return Rate::PerPixel(100. / length * feel::LOOP_SLICE_BAR_PX_PER_PX);
        }
        match unit {
            Unit::Count => Rate::Whole,
            Unit::Distance => Rate::PerPixel(mpp),
            Unit::Angle => Rate::PerPixel(feel::DEGREES_PER_PX),
            Unit::Percent => Rate::PerPixel(feel::PERCENT_PER_PX),
            Unit::Factor | Unit::Scalar | Unit::Suffix(_) => Rate::PerPixel(feel::SCALAR_PER_PX),
        }
    }

    /// The pointer moved to `pos` during a haul.
    pub(crate) fn haul_move(&mut self, drag: &mut HaulDrag, pos: Pos2) {
        let Some((axis, distance)) = drag.tracker.update([pos.x, pos.y]) else {
            return;
        };
        drag.started = true;
        if drag.locked.is_none() {
            let Some(key) = drag
                .owner
                .row()
                .and_then(|r| r.pair(drag.button).get(axis))
                .map(str::to_owned)
            else {
                return;
            };
            let Some(info) = self.haul_field(drag.owner, &key) else {
                self.status = format!("Drag control {key} isn't on this tool's panel");
                return;
            };
            if let Some(why) = info.greyed {
                self.status = format!("{}: {why}", info.label);
                // Lock onto nothing: the rest of this drag changes nothing.
                drag.tracker.axis = Some(axis);
                drag.locked = Some(Locked {
                    key: String::new(),
                    label: info.label,
                    start: info.value,
                    rate: Rate::Whole,
                    shown: info.value,
                });
                return;
            }
            drag.locked = Some(Locked {
                rate: self.haul_rate(&key, info.unit, drag.metres_per_pixel),
                key,
                label: info.label,
                start: info.value,
                shown: info.value,
            });
        }
        let Some(locked) = drag.locked.as_mut() else {
            return;
        };
        if locked.key.is_empty() {
            return;
        }
        let value = locked.start + haul::change(distance, locked.rate);
        if (value - locked.shown).abs() < 1e-12 {
            return;
        }
        let (owner, key, label) = (drag.owner, locked.key.clone(), locked.label.clone());
        let mut result = self.haul_set(owner, &key, value);
        // A whole number dragged fast past its limit (say below Segments 0
        // in one mouse move) steps back toward the last value taken until
        // one is, so it lands on the limit instead of short of it.
        if result.is_err() && locked.rate == Rate::Whole {
            let dir = (locked.shown - value).signum();
            let mut v = value + dir;
            while (locked.shown - v) * dir > 0.5 {
                if let Ok(shown) = self.haul_set(owner, &key, v) {
                    result = Ok(shown);
                    break;
                }
                v += dir;
            }
        }
        match result {
            Ok(shown) => {
                locked.shown = shown;
                self.clear_fields();
                self.status = format!("{label}: {}", haul_text(shown));
            }
            // A value the tool refuses (past a limit) stops at the last
            // one it took, like the mini-slider scrub.
            Err(e) => self.status = format!("{label}: {e}"),
        }
    }

    /// The button came up at `pos`. A press that never moved does its
    /// click instead.
    pub(crate) fn haul_release(&mut self, drag: &HaulDrag, pos: Pos2, r: Rect) {
        if drag.started || drag.tracker.moved([pos.x, pos.y]) {
            return;
        }
        match drag.click {
            HaulClick::Nothing => {}
            HaulClick::LoopSliceRing(mods) => {
                if let Some(p) = self.slice_params() {
                    self.loop_slice_pick_ring(p, drag.tracker.start_pos(), r, mods);
                }
            }
            HaulClick::CloneSmoothShift => {
                self.run(Command::CloneSmoothShift);
                self.clear_fields();
            }
        }
    }
}

/// A value for the status line: whole numbers without decimals.
fn haul_text(v: f64) -> String {
    if v.fract() == 0. {
        format!("{v:.0}")
    } else {
        format!("{v:.3}")
    }
}

trait StartPos {
    fn start_pos(&self) -> Pos2;
}
impl StartPos for Tracker {
    fn start_pos(&self) -> Pos2 {
        pos2(self.start[0], self.start[1])
    }
}
