//! Set Material dialog (new or existing tag, with its settings) and the
//! shared command route, in the shared form dialog (UIC-80..99).
//!
//! Rows, top to bottom: Name (editable, with a list of the scene's
//! materials), Type, Color, Diffuse, Specular, then the Smoothing, Make
//! Default and Add to Library checkboxes. OK and Cancel sit on the right.
//! OK assigns the tag and stores the settings in one undo step. Every row
//! shows the named material's stored settings, so none is remembered
//! between openings (the material is the memory).
use super::*;
use operandi_core::{Material, MaterialKind};
pub(super) struct SetMaterialDialog {
    target: MaterialTarget,
    form: form::Dialog,
    /// The name whose stored settings the rows last showed, so editing the
    /// name to another existing material loads that material's settings.
    loaded: String,
}
impl SetMaterialDialog {
    /// The Name field's text.
    pub(super) fn name(&self) -> String {
        self.form.values.text("name").to_owned()
    }
    /// The settings the rows show.
    pub(super) fn material(&self) -> Material {
        let v = &self.form.values;
        Material {
            kind: MaterialKind::ALL[v.choice("type").min(MaterialKind::ALL.len() - 1)],
            color: v.colour("color"),
            diffuse: v.number("diffuse"),
            specular: v.number("specular"),
            smoothing: v.bool("smoothing"),
            make_default: v.bool("make_default"),
            add_to_library: v.bool("add_to_library"),
        }
    }
    /// Shows `material`'s settings in the rows.
    pub(super) fn show_material(&mut self, material: &Material) {
        use form::Value;
        let v = &mut self.form.values;
        let kind = MaterialKind::ALL
            .iter()
            .position(|k| *k == material.kind)
            .unwrap_or(0);
        v.set("type", Value::Choice(kind));
        v.set("color", Value::Colour(material.color));
        v.set("diffuse", Value::Number(material.diffuse));
        v.set("specular", Value::Number(material.specular));
        v.set("smoothing", Value::Bool(material.smoothing));
        v.set("make_default", Value::Bool(material.make_default));
        v.set("add_to_library", Value::Bool(material.add_to_library));
    }
}
fn rows(name: &str, names: Vec<String>) -> Vec<form::Row> {
    use form::{Row, Unit};
    let kinds: Vec<&str> = MaterialKind::ALL.iter().map(|k| k.label()).collect();
    vec![
        Row::text("name", "Name", name)
            .suggestions(names)
            .tip("A new material name, or ▾ to pick one of the scene's materials"),
        Row::choice("type", "Type", &kinds, 0),
        Row::colour("color", "Color", [0.6; 3]),
        Row::number("diffuse", "Diffuse", Unit::Factor, 0.8).range(0., 1.),
        Row::number("specular", "Specular", Unit::Factor, 0.5).range(0., 1.),
        Row::check("smoothing", "Smoothing", true).tip("Off shades this material's polygons flat"),
        Row::check("make_default", "Make Default", false)
            .tip("Stored with the material; not used yet"),
        Row::check("add_to_library", "Add to Library", false)
            .tip("Stored with the material; not used yet"),
    ]
    .into_iter()
    .map(form::Row::fresh)
    .collect()
}
impl Operandi {
    pub(super) fn open_set_material(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Set Material".into();
            return;
        }
        match self.editor.set_material_target() {
            Ok(target) => {
                // Mixed or untagged polygons start with Preferences >
                // Application > Default Material.
                let name = Editor::common_material(&target)
                    .filter(|n| !operandi_core::material::is_default_name(n))
                    .unwrap_or_else(|| {
                        let preferred = self.defaults_preferences.default_material.trim();
                        if preferred.is_empty() {
                            operandi_core::material::DEFAULT_MATERIAL.into()
                        } else {
                            preferred.to_owned()
                        }
                    });
                let material = self.editor.scene.material_or_default(&name);
                let rows = rows(&name, self.editor.scene.material_names());
                let mut dialog = SetMaterialDialog {
                    form: self.form_dialog("Set Material", rows).focus("name"),
                    loaded: name,
                    target,
                };
                dialog.show_material(&material);
                self.set_material = Some(dialog);
            }
            Err(e) => self.status = e,
        }
    }
    /// Assign by name only (`poly.setMaterial {Name}`): stored settings stay.
    pub(super) fn assign_material(
        &mut self,
        name: String,
        expected: Option<MaterialTarget>,
    ) -> bool {
        self.assign_material_with(name, expected, None)
    }
    fn assign_material_with(
        &mut self,
        name: String,
        expected: Option<MaterialTarget>,
        material: Option<Material>,
    ) -> bool {
        if self.has_modal() {
            self.status = "Close the open dialog before Set Material".into();
            return false;
        }
        self.run_ok(Command::SetMaterial(MaterialAssignment {
            name,
            expected,
            material,
        }))
    }
    pub(super) fn set_material_dialog(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.set_material.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "Set Material closed; nothing assigned".into();
            return;
        }
        let outcome = self.show_form(ctx, &mut dialog.form);
        // Editing the name to another stored material shows its settings;
        // a new name keeps the current ones.
        let name = dialog.name();
        if name != dialog.loaded {
            if let Some(stored) = self.editor.scene.material(&name).cloned() {
                dialog.show_material(&stored);
            }
            dialog.loaded = name;
        }
        match outcome {
            form::Outcome::Cancel => self.status = "Set Material canceled".into(),
            form::Outcome::Ok => {
                let material = dialog.material();
                if !self.assign_material_with(
                    dialog.name(),
                    Some(dialog.target.clone()),
                    Some(material),
                ) {
                    dialog.form.error = Some(self.status.clone());
                    self.set_material = Some(dialog);
                }
            }
            _ => self.set_material = Some(dialog),
        }
    }
}

/// Material colors are linear; the system picker works in sRGB.
#[cfg(test)]
fn srgb_from_linear(c: f32) -> f32 {
    let c = c.clamp(0., 1.);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    }
}
#[cfg(test)]
fn linear_from_srgb(c: f32) -> f32 {
    let c = c.clamp(0., 1.);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srgb_round_trip() {
        for c in [0., 0.001, 0.2, 0.5, 0.8, 1.] {
            assert!((linear_from_srgb(srgb_from_linear(c)) - c).abs() < 1e-5);
        }
    }
}
