//! The Bind dialog (BEHAVIOR-RIG-PHASE5 SK-20, SK-21) in the shared form
//! dialog (UIC-80..97): OK runs one `operandi.skin.bind` line, one undo
//! step (UIC-96); Cancel changes nothing. A failed Heat bind keeps the
//! dialog open with the reason and offers "Use Smooth (Distance) instead"
//! ([OP] in SK-21). The Rig tab's Bind button opens it; Shift+click
//! unbinds (SK-24, its tooltip says so, UIC-04).
use super::*;
use crate::command_handlers::Commands;
use operandi_core::command_registry::{self as table, Area};

const TYPES: [&str; 4] = ["Rigid", "Smooth (Distance)", "Smooth (Visible)", "Heat"];
const TYPE_KEYS: [&str; 4] = ["rigid", "distance", "visible", "heat"];
const FALLOFFS: [&str; 3] = ["Low (soft)", "Medium", "High (sharp)"];
const FALLOFF_KEYS: [&str; 3] = ["low", "medium", "high"];
const FALLBACK: &str = "Use Smooth (Distance) instead";

fn rows() -> Vec<form::Row> {
    use form::{Row, Unit};
    vec![
        Row::choice("type", "Bind Type", &TYPES, 3).tip(
            "Heat diffuses weights over the surface; it needs a closed mesh with the joints inside",
        ),
        Row::choice("falloff", "Falloff", &FALLOFFS, 1)
            .tip("How quickly a farther joint's share fades"),
        Row::check("segments", "Bind as Segments", true)
            .tip("Weight along each joint's line to its children; off, a radius round each joint"),
        Row::check("limit", "Limit Weights", false),
        Row::number("perVertex", "Weights per Vertex", Unit::Count, 4.).range(1., 8.),
        Row::number("minimum", "Minimum Weight", Unit::Percent, 0.).range(0., 100.),
        Row::check("dual", "Dual Quaternion", false)
            .tip("The skin keeps volume at bent joints (SK-32)"),
    ]
}

/// The `operandi.skin.bind` line the dialog's values make.
pub(crate) fn bind_line(v: &form::Values) -> String {
    format!(
        "operandi.skin.bind type:{} falloff:{} segments:{} limitWeights:{} weightsPerVertex:{} minimumWeight:{} dualQuaternion:{}",
        TYPE_KEYS[v.choice("type").min(3)],
        FALLOFF_KEYS[v.choice("falloff").min(2)],
        v.bool("segments"),
        v.bool("limit"),
        v.number("perVertex").round().clamp(1., 8.),
        (v.number("minimum") / 100.).clamp(0., 1.),
        v.bool("dual"),
    )
}

pub(crate) fn register(r: &mut Commands) {
    r.add_host_command(
        table::script(
            "operandi.skin.bindDialog",
            Area::Items,
            "Opens the Bind dialog (SK-20) for the selected meshes and root joint.",
        )
        .operandi()
        .ui(),
        |a, _| a.open_bind(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

impl Operandi {
    pub(super) fn open_bind(&mut self) {
        if !crate::anim_switch::enabled() {
            self.status = "Animation is off (Preferences > Experimental > Animation)".into();
            return;
        }
        // SK-02: refused before the dialog opens.
        if !self.editor.scene.animation.setup {
            self.status = operandi_core::skin_bind::NEEDS_SETUP.into();
            return;
        }
        if self.has_modal() || self.skeleton.bind.is_some() {
            self.status = "Close the open dialog before Bind".into();
            return;
        }
        self.skeleton.bind = Some(self.form_dialog("Bind", rows()));
    }

    /// Runs `line`; whether it made an undo step.
    fn bind_ran(&mut self, line: &str) -> bool {
        let mark = self.editor.history_mark();
        self.execute(line);
        self.editor.history_mark() != mark
    }

    pub(super) fn bind_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.skeleton.bind.take() else {
            return;
        };
        match self.show_form(ctx, &mut dialog) {
            form::Outcome::Cancel => self.status = "Bind canceled".into(),
            form::Outcome::Ok => {
                let line = bind_line(&dialog.values);
                if self.bind_ran(&line) {
                    self.remember_form(&dialog);
                } else {
                    dialog.error = Some(self.status.clone());
                    let heat = dialog.values.choice("type") == 3;
                    self.skeleton.bind = Some(if heat {
                        dialog.extras(&[FALLBACK])
                    } else {
                        dialog
                    });
                }
            }
            form::Outcome::Extra(0) => {
                dialog.values.set("type", form::Value::Choice(1));
                let line = bind_line(&dialog.values);
                if !self.bind_ran(&line) {
                    dialog.error = Some(self.status.clone());
                    self.skeleton.bind = Some(dialog);
                }
            }
            _ => self.skeleton.bind = Some(dialog),
        }
    }
}
