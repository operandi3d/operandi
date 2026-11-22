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

/// One haul in progress.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HaulDrag {
    pub(crate) tool: Tool,
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

/// Which panel serves `tool`'s fields.
#[derive(Clone, Copy)]
enum Panel {
    Live,
    Primitive,
    Surface,
}
fn panel(tool: Tool) -> Panel {
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
    fn haul_field(&self, tool: Tool, key: &str) -> Option<FieldInfo> {
        match panel(tool) {
            Panel::Live => field_info::<crate::live_fields::LiveFields>(self, key),
            Panel::Primitive => field_info::<crate::primitive_panel::PrimitiveFields>(self, key),
            Panel::Surface => field_info::<crate::smooth_shift_panel::SurfaceFields>(self, key),
        }
    }
    fn haul_set(&mut self, tool: Tool, key: &str, value: f64) -> Result<f64> {
        match panel(tool) {
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
        self.haul_field(tool, key).is_some()
    }

    /// The live tool, when the layer claims `button` for it.
    pub(crate) fn haul_tool(&self, button: Button) -> Option<Tool> {
        let tool = self.editor.live.as_ref()?.tool;
        haul::claims(tool, button).then_some(tool)
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
        let tool = self.haul_tool(button)?;
        let pair = haul::row(tool)?.pair(button);
        Some(input::GestureKind::Haul(Box::new(HaulDrag {
            tool,
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
        let tool = self.editor.live.as_ref().map(|l| l.tool);
        match (tool, button) {
            (Some(t), Button::Middle) if t.is_smooth_shift() => HaulClick::CloneSmoothShift,
            _ => HaulClick::Nothing,
        }
    }

    /// Whether `pos` is on the live tool's own on-screen control, which
    /// takes the press before the drag layer (DL-01).
    pub(crate) fn haul_on_tool_control(&self, pos: Pos2, r: Rect) -> bool {
        match self.editor.live.as_ref() {
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
            let Some(key) = haul::row(drag.tool)
                .and_then(|r| r.pair(drag.button).get(axis))
                .map(str::to_owned)
            else {
                return;
            };
            let Some(info) = self.haul_field(drag.tool, &key) else {
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
        let (tool, key, label) = (drag.tool, locked.key.clone(), locked.label.clone());
        match self.haul_set(tool, &key, value) {
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
