//! Light properties Tier 3 (specifications/light-properties-2026-11-06),
//! light side: Volumetrics (LP-40..44, LM-30..33), LPE Label (LP-22),
//! Importance (LP-18), Dome Light (LP-DM-01), the light material's Enable,
//! Blend Mode, Opacity and image layers (LM-01..04), Affect Subsurface
//! (LM-15) and Physical Sun Gamma / stored places (LP-SUN-07, -13).
use operandi_core::camera_light::*;
use operandi_core::physical_sun::SunEdit;
use operandi_core::*;

fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e
}

fn new_light(e: &mut Editor, kind: LightKind) -> u64 {
    e.dispatch(Command::NewLight(kind)).unwrap();
    e.scene.selection.item.unwrap()
}

fn light(e: &Editor, id: u64) -> &LightSettings {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .light
        .as_ref()
        .unwrap()
}

fn edit(e: &mut Editor, id: u64, edit: LightEdit) -> Result<()> {
    e.dispatch(Command::EditLight { item: id, edit })
}

fn last_label(e: &Editor) -> &str {
    &e.undo.last().unwrap().label
}

fn round_trip(scene: &Scene) -> Scene {
    let bytes = persistence::encode(scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 33"));
    persistence::decode(&bytes).unwrap()
}

#[test]
fn dome_light_defaults_and_rows() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Dome);
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    assert_eq!(item.name, "Dome Light");
    let k = LightKind::Dome;
    assert_eq!(k.intensity_label(), "Radiance");
    assert!(k.emits() && !k.has_targeting() && !k.has_visible_rows() && !k.has_volumetrics());
    assert_eq!(light(&e, id).intensity, 3.0);
    edit(&mut e, id, LightEdit::Dissolve(0.5)).unwrap();
    assert_eq!(round_trip(&e.scene), e.scene);
}

#[test]
fn volumetrics_defaults_by_kind() {
    // LP-40..44, LM-30..33 observed defaults.
    let d = Volumetrics::new(LightKind::Directional);
    assert!(!d.enabled);
    assert_eq!((d.samples, d.height, d.radius), (40, 10.0, 0.5));
    assert_eq!(Volumetrics::new(LightKind::Spot).radius, 0.0, "spot Base");
    assert_eq!(Volumetrics::new(LightKind::Point).sphere_radius, 1.0);
    assert_eq!(
        (d.color, d.density, d.attenuation, d.shift),
        ([1.0; 3], 0.5, 0.1, 0.0)
    );
    for kind in LightKind::ALL {
        assert_eq!(
            kind.has_volumetrics(),
            matches!(
                kind,
                LightKind::Directional
                    | LightKind::Point
                    | LightKind::Spot
                    | LightKind::Photometric
            ),
            "{kind:?}"
        );
    }
}

#[test]
fn volumetric_rows_are_named_steps_and_save() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Spot);
    assert!(light(&e, id).volumetrics.is_none());
    for (row, name) in [
        (VolumetricEdit::Enabled(true), "Volumetrics"),
        (VolumetricEdit::Height(4.0), "Volumetric Height"),
        (VolumetricEdit::Radius(0.2), "Volumetric Radius"),
        (VolumetricEdit::Color([1.0, 0.5, 0.2]), "Scattering Color"),
        (VolumetricEdit::Density(0.8), "Density"),
        (VolumetricEdit::Attenuation(0.3), "Light Attenuation"),
        (VolumetricEdit::Shift(-0.4), "Light Shift"),
    ] {
        edit(&mut e, id, LightEdit::Volumetric(row)).unwrap();
        assert_eq!(last_label(&e), name);
    }
    let v = light(&e, id).volumetrics.clone().unwrap();
    assert!(v.enabled && v.height == 4.0 && v.shift == -0.4);
    assert_eq!(round_trip(&e.scene), e.scene);
    for bad in [
        VolumetricEdit::Height(0.0),
        VolumetricEdit::Attenuation(1.5),
        VolumetricEdit::Shift(2.0),
        VolumetricEdit::Samples(0),
    ] {
        assert!(edit(&mut e, id, LightEdit::Volumetric(bad)).is_err());
    }
    for _ in 0..7 {
        e.dispatch(Command::Undo).unwrap();
    }
    assert!(light(&e, id).volumetrics.is_none(), "back to defaults");
    // Not on every kind.
    let area = new_light(&mut e, LightKind::Area);
    assert!(
        edit(
            &mut e,
            area,
            LightEdit::Volumetric(VolumetricEdit::Enabled(true))
        )
        .is_err()
    );
}

#[test]
fn lpe_label_importance_and_subsurface_store() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Point);
    assert_eq!(light(&e, id).stored.importance, 1.0);
    edit(&mut e, id, LightEdit::LpeLabel("  Key ".into())).unwrap();
    assert_eq!(last_label(&e), "LPE Label");
    assert_eq!(light(&e, id).lpe_label, "Key");
    edit(&mut e, id, LightEdit::Importance(2.5)).unwrap();
    assert_eq!(last_label(&e), "Importance");
    edit(&mut e, id, LightEdit::AffectSubsurface(0.25)).unwrap();
    assert_eq!(last_label(&e), "Affect Subsurface");
    let l = light(&e, id);
    assert_eq!(
        (l.stored.importance, l.stored.affect_subsurface),
        (2.5, 0.25)
    );
    assert!(edit(&mut e, id, LightEdit::Importance(-1.0)).is_err());
    assert!(edit(&mut e, id, LightEdit::LpeLabel("x".repeat(300))).is_err());
    assert_eq!(round_trip(&e.scene), e.scene);
}

#[test]
fn light_material_enable_blend_opacity() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Spot);
    let l = light(&e, id);
    assert!(l.stored.material_enabled && l.stored.opacity == 1.0);
    assert_eq!(l.blend_mode, LightBlend::Normal);
    edit(&mut e, id, LightEdit::MaterialEnabled(false)).unwrap();
    assert_eq!(last_label(&e), "Material Enable");
    edit(&mut e, id, LightEdit::BlendMode(LightBlend::Screen)).unwrap();
    assert_eq!(last_label(&e), "Blend Mode");
    edit(&mut e, id, LightEdit::Opacity(0.4)).unwrap();
    assert_eq!(last_label(&e), "Opacity");
    assert!(edit(&mut e, id, LightEdit::Opacity(1.5)).is_err());
    assert_eq!(round_trip(&e.scene), e.scene);
}

#[test]
fn image_layers_add_edit_remove() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Spot);
    edit(
        &mut e,
        id,
        LightEdit::Layer(LayerEdit::Add("/tmp/gobo.png".into())),
    )
    .unwrap();
    assert_eq!(last_label(&e), "Add Image Layer");
    let layer = &light(&e, id).layers[0];
    // A new layer multiplies the light by the picture, fully.
    assert_eq!(
        (layer.blend, layer.opacity, layer.enabled),
        (LightBlend::Multiply, 1.0, true)
    );
    edit(
        &mut e,
        id,
        LightEdit::Layer(LayerEdit::Add("/tmp/b.png".into())),
    )
    .unwrap();
    edit(&mut e, id, LightEdit::Layer(LayerEdit::Opacity(1, 0.5))).unwrap();
    assert_eq!(last_label(&e), "Layer Opacity");
    edit(
        &mut e,
        id,
        LightEdit::Layer(LayerEdit::Blend(1, LightBlend::Add)),
    )
    .unwrap();
    edit(&mut e, id, LightEdit::Layer(LayerEdit::Enabled(0, false))).unwrap();
    assert_eq!(round_trip(&e.scene), e.scene);
    edit(&mut e, id, LightEdit::Layer(LayerEdit::Remove(0))).unwrap();
    assert_eq!(last_label(&e), "Remove Layer");
    assert_eq!(light(&e, id).layers.len(), 1);
    assert_eq!(light(&e, id).layers[0].image, "/tmp/b.png");
    assert!(edit(&mut e, id, LightEdit::Layer(LayerEdit::Remove(5))).is_err());
    assert!(edit(&mut e, id, LightEdit::Layer(LayerEdit::Opacity(0, 2.0))).is_err());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(light(&e, id).layers.len(), 2);
}

#[test]
fn blend_modes_mix_like_cycles() {
    let (b, c) = ([0.5, 0.2, 0.8], [0.4, 0.6, 1.0]);
    let close = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12);
    assert!(close(LightBlend::Normal.mix(b, c, 1.0), c));
    assert!(close(LightBlend::Normal.mix(b, c, 0.0), b));
    assert!(close(LightBlend::Multiply.mix(b, c, 1.0), [0.2, 0.12, 0.8]));
    assert!(close(LightBlend::Add.mix(b, c, 0.5), [0.7, 0.5, 1.3]));
    assert!(close(
        LightBlend::Subtract.mix(b, c, 1.0),
        [0.1, -0.4, -0.2]
    ));
    assert!(close(LightBlend::Screen.mix(b, c, 1.0), [0.7, 0.68, 1.0]));
    let o = LightBlend::Overlay.mix(b, c, 1.0);
    assert!(close(o, [1.0 - 2.0 * 0.5 * 0.6, 2.0 * 0.2 * 0.6, 1.0]));
}

#[test]
fn sun_gamma_and_stored_places() {
    let mut e = new_scene();
    let id = e.scene.items[2].id;
    assert_eq!(light(&e, id).sun.gamma, 2.2);
    edit(&mut e, id, LightEdit::Sun(SunEdit::Gamma(1.8))).unwrap();
    assert_eq!(last_label(&e), "Gamma");
    assert!(edit(&mut e, id, LightEdit::Sun(SunEdit::Gamma(0.0))).is_err());
    // A stored place fills the numbers like a city (one "Location" step).
    edit(
        &mut e,
        id,
        LightEdit::Sun(SunEdit::Place(64.15, -21.94, 0.0)),
    )
    .unwrap();
    assert_eq!(last_label(&e), "Location");
    let s = &light(&e, id).sun;
    assert_eq!((s.latitude, s.longitude, s.time_zone), (64.15, -21.94, 0.0));
    assert_eq!(s.location(), None);
    assert!(edit(&mut e, id, LightEdit::Sun(SunEdit::Place(91.0, 0.0, 0.0))).is_err());
    assert_eq!(round_trip(&e.scene), e.scene);
}

#[test]
fn tier3_settings_need_version_16() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Point);
    edit(&mut e, id, LightEdit::LpeLabel("Fill".into())).unwrap();
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    let old = text.replacen("\"version\": 33", "\"version\": 15", 1);
    assert!(persistence::decode(old.as_bytes()).is_err());
}
