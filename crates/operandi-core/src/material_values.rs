//! Material-value render outputs: Render Outputs that write a material's
//! own values (Diffuse Amount, Roughness, ...) at the first camera hit,
//! rather than light. Provisional readings are MV-Q01..Q08 in
//! docs/SPECIFICATION_QUESTIONS.md.
//!
//! One mechanism serves all of them: each value is an [`Expr`] built from
//! the surface's folded channels (so texture layers count), which the
//! renderer writes into a colour AOV named `operandi_value_<name>`. The two
//! geometry values (dPdu, dPdv) aren't material channels; the renderer
//! writes them for every surface from the mesh's first UV map.
//!
//! Master plan A31 (materials become MaterialX graphs, the Shader Tree a
//! view of them): a future MaterialX surface only has to write the same
//! named AOVs. Each value's nearest OpenPBR / MaterialX meaning:
//!
//! | Value                       | OpenPBR / MaterialX                         |
//! |-----------------------------|---------------------------------------------|
//! | diffuse_amount              | base_weight                                 |
//! | diffuse_coefficient         | base_color × base_weight                    |
//! | diffuse_energy_conservation | base_color × base_weight × (1 − transmission_weight) × (1 − subsurface_weight) × (1 − base_metalness) |
//! | diffuse_roughness           | base_diffuse_roughness                      |
//! | specular_coefficient        | specular_weight × specular_color            |
//! | reflection_coefficient      | specular_weight × specular_color            |
//! | roughness                   | specular_roughness                          |
//! | subsurface_amount           | subsurface_weight                           |
//! | subsurface_color            | subsurface_color                            |
//! | transparent_amount          | transmission_weight                         |
//! | transparent_color           | transmission_color                          |
//! | dpdu                        | geomprop `tangent` (world space)            |
//! | dpdv                        | geomprop `bitangent` (world space)          |
use crate::shader_eval::{Ch, Expr, Surface};

/// A value a material-value Render Output writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MaterialValue {
    DiffuseAmount,
    DiffuseCoefficient,
    DiffuseEnergyConservation,
    DiffuseRoughness,
    SpecularCoefficient,
    ReflectionCoefficient,
    Roughness,
    SubsurfaceAmount,
    SubsurfaceColor,
    TransparentAmount,
    TransparentColor,
    Dpdu,
    Dpdv,
}

impl MaterialValue {
    /// How many there are (`OC_VALUE_COUNT` in the C bridge).
    pub const COUNT: usize = 13;
    /// Every value, in `OC_VALUE_*` order.
    pub const ALL: [MaterialValue; Self::COUNT] = {
        use MaterialValue::*;
        [
            DiffuseAmount,
            DiffuseCoefficient,
            DiffuseEnergyConservation,
            DiffuseRoughness,
            SpecularCoefficient,
            ReflectionCoefficient,
            Roughness,
            SubsurfaceAmount,
            SubsurfaceColor,
            TransparentAmount,
            TransparentColor,
            Dpdu,
            Dpdv,
        ]
    };

    /// The stable name: the AOV is `operandi_value_<name>` (MV-Q08).
    pub fn name(self) -> &'static str {
        use MaterialValue::*;
        match self {
            DiffuseAmount => "diffuse_amount",
            DiffuseCoefficient => "diffuse_coefficient",
            DiffuseEnergyConservation => "diffuse_energy_conservation",
            DiffuseRoughness => "diffuse_roughness",
            SpecularCoefficient => "specular_coefficient",
            ReflectionCoefficient => "reflection_coefficient",
            Roughness => "roughness",
            SubsurfaceAmount => "subsurface_amount",
            SubsurfaceColor => "subsurface_color",
            TransparentAmount => "transparent_amount",
            TransparentColor => "transparent_color",
            Dpdu => "dpdu",
            Dpdv => "dpdv",
        }
    }

    /// The renderer's AOV name for this value.
    pub fn aov_name(self) -> String {
        format!("operandi_value_{}", self.name())
    }

    /// Colour values carry three numbers; the rest one (shown as grey).
    pub fn is_color(self) -> bool {
        use MaterialValue::*;
        matches!(
            self,
            DiffuseCoefficient
                | DiffuseEnergyConservation
                | SpecularCoefficient
                | ReflectionCoefficient
                | SubsurfaceColor
                | TransparentColor
                | Dpdu
                | Dpdv
        )
    }

    /// Values of the mesh's geometry, not of its material.
    pub fn is_geometry(self) -> bool {
        matches!(self, MaterialValue::Dpdu | MaterialValue::Dpdv)
    }

    /// Position in `ALL` (the `OC_VALUE_*` code).
    pub fn index(self) -> usize {
        self as usize
    }
}

impl Surface {
    /// The value `v` of this folded surface, `None` for the geometry values
    /// (the renderer makes those). Provisional formulas MV-Q01..Q04.
    pub fn material_value(&self, v: MaterialValue) -> Option<Expr> {
        use MaterialValue::*;
        let c = |ch: Ch| self.get(ch).clone();
        // MV-Q01: every shading model.
        let diffuse = || Expr::times(c(Ch::DiffuseColor), c(Ch::DiffuseAmount));
        // MV-Q03.
        let specular = || Expr::times(c(Ch::SpecularColor), c(Ch::SpecularAmount));
        Some(match v {
            DiffuseAmount => c(Ch::DiffuseAmount),
            DiffuseCoefficient => diffuse(),
            // MV-Q02: the diffuse left after see-through, subsurface,
            // metal, specular and coat take their shares.
            DiffuseEnergyConservation => {
                // Specular and coat take their straight-on reflectance
                // (0.04 for the coat's IOR 1.5).
                let (metallic, coat) = self.metallic_and_coat();
                let coat = Expr::times(coat, Expr::scalar(0.04));
                let mut e = diffuse();
                for share in [
                    c(Ch::TransparentAmount),
                    c(Ch::SubsurfaceAmount),
                    metallic,
                    self.specular_f0(),
                    coat,
                ] {
                    if share != Expr::scalar(0.) {
                        e = Expr::times(e, Expr::one_minus(share));
                    }
                }
                e
            }
            DiffuseRoughness => c(Ch::DiffuseRoughness),
            SpecularCoefficient => specular(),
            // MV-Q04: with Match Specular on, reflections follow specular.
            ReflectionCoefficient => {
                if self.match_specular {
                    specular()
                } else {
                    Expr::times(c(Ch::ReflectionColor), c(Ch::ReflectionAmount))
                }
            }
            Roughness => c(Ch::Roughness),
            SubsurfaceAmount => c(Ch::SubsurfaceAmount),
            SubsurfaceColor => c(Ch::SubsurfaceColor),
            TransparentAmount => c(Ch::TransparentAmount),
            TransparentColor => c(Ch::TransparentColor),
            Dpdu | Dpdv => return None,
        })
    }

    /// [`Surface::principled`] that also carries the material values a
    /// render's outputs want. Without any it is exactly `principled()`.
    pub fn principled_with_values(
        &self,
        wanted: &[MaterialValue],
    ) -> crate::shader_eval::Principled {
        let mut p = self.principled();
        for &v in wanted {
            if p.values.iter().any(|(w, _)| *w == v) {
                continue;
            }
            if let Some(e) = self.material_value(v) {
                p.values.push((v, e));
            }
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_eval::Context;
    use crate::shader_tree::{NewLayer, NodeKind, ShaderTree, TextureEffect, TextureSource};

    fn ctx() -> Context<'static> {
        Context {
            item: 1,
            ancestors: &[],
            material: "Default",
            part: None,
            sets: &[],
            locators: &[],
        }
    }
    /// The default tree with its Base Material changed by `f`.
    fn tree(f: impl FnOnce(&mut crate::shader_tree::MaterialChannels)) -> ShaderTree {
        let mut tree = ShaderTree::default();
        let id = tree
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Material(_)))
            .unwrap()
            .id;
        let mut node = tree.get(id).unwrap().clone();
        if let NodeKind::Material(m) = &mut node.kind {
            f(m);
        }
        tree.replace(node).unwrap();
        tree
    }
    fn value(tree: &ShaderTree, v: MaterialValue) -> [f64; 3] {
        tree.evaluate(&ctx())
            .material_value(v)
            .unwrap()
            .constant()
            .expect("constant")
    }
    fn near(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
    }

    #[test]
    fn names_and_order_are_stable() {
        assert_eq!(MaterialValue::ALL.len(), MaterialValue::COUNT);
        for (k, v) in MaterialValue::ALL.iter().enumerate() {
            assert_eq!(v.index(), k);
        }
        assert_eq!(MaterialValue::Dpdu.aov_name(), "operandi_value_dpdu");
        assert_eq!(
            MaterialValue::DiffuseEnergyConservation.name(),
            "diffuse_energy_conservation"
        );
        assert!(MaterialValue::Dpdv.is_geometry() && MaterialValue::Dpdv.is_color());
        assert!(!MaterialValue::Roughness.is_color());
    }

    #[test]
    fn diffuse_coefficient_is_colour_times_amount() {
        let t = tree(|m| {
            m.diffuse_color = [0.6, 0.3, 0.1];
            m.diffuse_amount = 0.5;
        });
        assert!(near(
            value(&t, MaterialValue::DiffuseCoefficient),
            [0.3, 0.15, 0.05]
        ));
        assert!(near(value(&t, MaterialValue::DiffuseAmount), [0.5; 3]));
    }

    #[test]
    fn energy_conservation_takes_off_the_other_shares() {
        // MV-Q02: transparent, subsurface, metal, specular (straight-on
        // 0.08 × Specular Amount) and coat (0.04 × weight).
        let t = tree(|m| {
            m.diffuse_color = [0.6, 0.3, 0.1];
            m.diffuse_amount = 0.5;
            m.transparent_amount = 0.25;
            m.subsurface_amount = 0.;
            m.metallic = 0.;
            m.specular_amount = 0.5;
            m.clearcoat_amount = 0.;
            m.match_specular = true;
            m.more.use_refractive_index = false;
        });
        let f = 0.75 * 0.96;
        assert!(near(
            value(&t, MaterialValue::DiffuseEnergyConservation),
            [0.3 * f, 0.15 * f, 0.05 * f]
        ));
        let t = tree(|m| {
            m.diffuse_color = [1.; 3];
            m.diffuse_amount = 1.;
            m.transparent_amount = 0.;
            m.subsurface_amount = 0.5;
            m.metallic = 0.5;
            m.specular_amount = 0.25;
            m.clearcoat_amount = 0.5;
            m.match_specular = true;
            m.more.use_refractive_index = false;
        });
        assert!(near(
            value(&t, MaterialValue::DiffuseEnergyConservation),
            [0.25 * 0.98 * 0.98; 3]
        ));
        // Use Refractive Index: ((n − 1) / (n + 1))², 1/9 at n = 2.
        let t = tree(|m| {
            m.diffuse_color = [1.; 3];
            m.diffuse_amount = 1.;
            m.transparent_amount = 0.;
            m.subsurface_amount = 0.;
            m.metallic = 0.;
            m.clearcoat_amount = 0.;
            m.match_specular = true;
            m.refractive_index = 2.;
            m.more.use_refractive_index = true;
        });
        assert!(near(
            value(&t, MaterialValue::DiffuseEnergyConservation),
            [8. / 9.; 3]
        ));
    }

    #[test]
    fn reflection_follows_specular_with_match_specular() {
        let set = |m: &mut crate::shader_tree::MaterialChannels| {
            m.specular_color = [1., 0.5, 0.];
            m.specular_amount = 0.4;
            m.reflection_color = [0., 1., 1.];
            m.reflection_amount = 0.2;
        };
        let on = tree(|m| {
            set(m);
            m.match_specular = true;
        });
        assert!(near(
            value(&on, MaterialValue::SpecularCoefficient),
            [0.4, 0.2, 0.]
        ));
        assert!(near(
            value(&on, MaterialValue::ReflectionCoefficient),
            [0.4, 0.2, 0.]
        ));
        let off = tree(|m| {
            set(m);
            m.match_specular = false;
        });
        assert!(near(
            value(&off, MaterialValue::ReflectionCoefficient),
            [0., 0.2, 0.2]
        ));
    }

    #[test]
    fn plain_channels_pass_through() {
        let t = tree(|m| {
            m.roughness = 0.42;
            m.diffuse_roughness = 0.3;
            m.subsurface_amount = 0.1;
            m.subsurface_color = [0.9, 0.2, 0.1];
            m.transparent_amount = 0.7;
            m.transparent_color = [0.2, 0.5, 0.8];
        });
        use MaterialValue::*;
        assert!(near(value(&t, Roughness), [0.42; 3]));
        assert!(near(value(&t, DiffuseRoughness), [0.3; 3]));
        assert!(near(value(&t, SubsurfaceAmount), [0.1; 3]));
        assert!(near(value(&t, SubsurfaceColor), [0.9, 0.2, 0.1]));
        assert!(near(value(&t, TransparentAmount), [0.7; 3]));
        assert!(near(value(&t, TransparentColor), [0.2, 0.5, 0.8]));
        let s = t.evaluate(&ctx());
        assert_eq!(s.material_value(Dpdu), None);
        assert_eq!(s.material_value(Dpdv), None);
    }

    #[test]
    fn a_texture_layer_drives_the_value() {
        let mut t = tree(|m| m.roughness = 0.42);
        // A Constant layer in the Base Material's group, over the material.
        let parent = t
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Material(_)))
            .and_then(|n| n.parent);
        let id = t.add(NewLayer::Constant, parent).unwrap();
        let mut node = t.get(id).unwrap().clone();
        if let NodeKind::Texture(tex) = &mut node.kind {
            tex.effect = TextureEffect::Roughness;
            tex.source = TextureSource::Constant {
                color: [1.; 3],
                value: 0.9,
            };
        }
        t.replace(node).unwrap();
        assert!(near(value(&t, MaterialValue::Roughness), [0.9; 3]));
    }

    #[test]
    fn values_are_only_carried_when_asked() {
        let s = ShaderTree::default().evaluate(&ctx());
        let plain = s.principled();
        assert!(plain.values.is_empty());
        assert_eq!(s.principled_with_values(&[]), plain);
        let p = s.principled_with_values(&[
            MaterialValue::Roughness,
            MaterialValue::Dpdu,
            MaterialValue::Roughness,
        ]);
        assert_eq!(p.values.len(), 1);
        assert_eq!(p.values[0].0, MaterialValue::Roughness);
        assert_eq!(p.normalized().values, p.values);
    }

    #[test]
    fn outputs_save_as_version_52_only_when_used() {
        use crate::shader_tree::OutputEffect;
        let version = |s: &crate::Scene| {
            let text = String::from_utf8(crate::persistence::encode(s).unwrap()).unwrap();
            text.contains("\"version\": 52")
        };
        let mut scene = crate::Scene::default();
        scene
            .shader_tree
            .add(NewLayer::RenderOutput(OutputEffect::Depth), None)
            .unwrap();
        assert!(!version(&scene), "a plain output keeps the older number");
        let dpdu = scene
            .shader_tree
            .add(NewLayer::RenderOutput(OutputEffect::Dpdu), None)
            .unwrap();
        let coef = scene
            .shader_tree
            .add(
                NewLayer::RenderOutput(OutputEffect::DiffuseCoefficient),
                None,
            )
            .unwrap();
        assert!(version(&scene));
        let bytes = crate::persistence::encode(&scene).unwrap();
        let back = crate::persistence::decode(&bytes).unwrap();
        assert_eq!(back.shader_tree, scene.shader_tree);
        for (id, effect) in [
            (dpdu, OutputEffect::Dpdu),
            (coef, OutputEffect::DiffuseCoefficient),
        ] {
            assert_eq!(
                back.shader_tree.get(id).unwrap().kind,
                NodeKind::RenderOutput(effect)
            );
        }
        // An older version number with one is refused cleanly.
        let old =
            String::from_utf8(bytes)
                .unwrap()
                .replacen("\"version\": 52", "\"version\": 51", 1);
        let err = crate::persistence::decode(old.as_bytes()).unwrap_err();
        assert!(err.contains("version 52"), "{err}");
        assert_eq!(OutputEffect::Dpdu.layer_name(), "dPdu Output");
        assert_eq!(
            OutputEffect::DiffuseCoefficient.material_value(),
            Some(MaterialValue::DiffuseCoefficient)
        );
        assert_eq!(OutputEffect::Depth.material_value(), None);
        assert_eq!(OutputEffect::MATERIAL_VALUES.len(), MaterialValue::COUNT);
        for e in OutputEffect::MATERIAL_VALUES {
            assert!(e.material_value().is_some() && e.material_value_tip().is_some());
        }
    }
}
