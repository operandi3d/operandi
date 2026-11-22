//! Light properties Tier 1 in the app: panel tabs and row order (LP-01,
//! LP-23, LP-PO), Create menu entries, and the shared Display/Assembly
//! rules the viewport follows (CP-D01, CP-A01, CP-A02).
use super::tests::{box_app, draw};
use super::*;
use operandi_core::camera_light::{LightEdit, LightKind};
use operandi_core::item_common::{Choice, DisplayVisible, ItemCommonEdit};

// Tall enough for every light row to fit in the Properties panel.
const SIZE: Vec2 = vec2(1440., 2600.);

fn view_rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

/// Painted text `t` shows `label`: whole, or cut with "…" because the
/// panel is narrow (UIC-111/115 put the full name in the tooltip).
fn shows(t: &str, label: &str) -> bool {
    t == label
        || t.strip_suffix('…')
            .is_some_and(|start| !start.is_empty() && label.starts_with(start))
}

fn texts(out: &egui::FullOutput) -> Vec<(String, f32)> {
    let mut v = Vec::new();
    for s in &out.shapes {
        if let egui::Shape::Text(t) = &s.shape {
            v.push((t.galley.job.text.clone(), t.pos.y));
        }
    }
    v
}

/// Draws the app twice (layout settles) with the light tab open.
fn properties_texts(a: &mut Operandi) -> Vec<(String, f32)> {
    let ctx = Context::default();
    a.upper_frame.show_page(0);
    draw(a, &ctx, SIZE, vec![]);
    texts(&draw(a, &ctx, SIZE, vec![]))
}

fn y_of(texts: &[(String, f32)], label: &str) -> f32 {
    texts
        .iter()
        .find(|(t, _)| shows(t, label))
        .unwrap_or_else(|| panic!("no {label:?} in {texts:?}"))
        .1
}

fn new_light(a: &mut Operandi, kind: LightKind) -> u64 {
    a.add_item(operandi_core::item_list::AddItemKind::Light(kind));
    a.editor.scene.selection.item.unwrap()
}

#[test]
fn every_item_kind_gets_display_and_assembly_tabs() {
    let mut a = box_app();
    assert_eq!(
        a.property_tabs(),
        [
            "Mesh",
            "Surface",
            "Display",
            "Assembly",
            "User Channels",
            "Tags"
        ]
    );
    new_light(&mut a, LightKind::Directional);
    assert_eq!(
        a.property_tabs(),
        [
            "Directional Light",
            "Display",
            "Assembly",
            "User Channels",
            "Tags"
        ]
    );
    new_light(&mut a, LightKind::Portal);
    assert_eq!(
        a.property_tabs(),
        ["Portal", "Display", "Assembly", "User Channels", "Tags"]
    );
    a.add_item(operandi_core::item_list::AddItemKind::Camera);
    assert_eq!(
        a.property_tabs(),
        [
            "Camera View",
            "Camera Effects",
            "Display",
            "Assembly",
            "User Channels",
            "Tags"
        ]
    );
}

#[test]
fn light_rows_follow_the_panel_order() {
    // LP-23: Render, Dissolve, intensity, Light Color, Falloff, Shadow;
    // size rows; Visible ×3; spot cone rows last.
    let mut a = box_app();
    new_light(&mut a, LightKind::Spot);
    a.properties_tab = "Spot Light".into();
    let t = properties_texts(&mut a);
    let order = [
        "Render",
        "Dissolve",
        "Radiant Intensity",
        "Light Color",
        "Falloff Type",
        "Shadow Type",
        "Radius",
        "Visible to Camera",
        "Visible to Reflection Rays",
        "Visible to Refraction Rays",
        "Cone Angle",
        "Soft Edge",
    ];
    for pair in order.windows(2) {
        assert!(
            y_of(&t, pair[0]) < y_of(&t, pair[1]),
            "{} before {}",
            pair[0],
            pair[1]
        );
    }
    // The light material follows with the Affect switches and Set
    // Temperature.
    for label in [
        "Affect Diffuse",
        "Affect Specular",
        "Set Temperature (Kelvins)...",
    ] {
        assert!(y_of(&t, label) > y_of(&t, "Soft Edge"), "{label}");
    }
}

#[test]
fn portal_shows_only_its_rows() {
    // LP-PO: Render, Shape, Width, Height, Environment Only; no intensity,
    // colour, falloff, shadow, Dissolve, Visible or material rows.
    let mut a = box_app();
    new_light(&mut a, LightKind::Portal);
    a.properties_tab = "Portal".into();
    let t = properties_texts(&mut a);
    for label in ["Render", "Shape", "Width", "Height", "Environment Only"] {
        y_of(&t, label);
    }
    for label in [
        "Dissolve",
        "Radiance",
        "Light Color",
        "Falloff Type",
        "Shadow Type",
        "Visible to Camera",
        "Light Material",
        "Affect Diffuse",
    ] {
        assert!(t.iter().all(|(s, _)| s != label), "{label} shown");
    }
}

#[test]
fn intensity_label_follows_the_kind() {
    // LP-12.
    for (kind, label) in [
        (LightKind::Directional, "Radiant Exitance"),
        (LightKind::Point, "Radiant Intensity"),
        (LightKind::Area, "Radiance"),
    ] {
        let mut a = box_app();
        new_light(&mut a, kind);
        a.properties_tab = kind.tab_name().into();
        y_of(&properties_texts(&mut a), label);
    }
}

#[test]
fn create_light_makes_a_portal_with_one_undo_step() {
    let mut a = box_app();
    let steps = a.editor.undo.len();
    let id = new_light(&mut a, LightKind::Portal);
    assert_eq!(a.editor.undo.len(), steps + 1);
    let item = a.editor.scene.item(id).unwrap();
    assert_eq!(item.name, "Portal");
    assert_eq!(item.light.as_ref().unwrap().kind, LightKind::Portal);
    a.run(Command::Undo);
    assert!(a.editor.scene.items.iter().all(|i| i.id != id));
}

#[test]
fn select_no_isnt_picked_in_item_mode() {
    // CP-A01: Item-mode clicks pass over it; the item list still selects.
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    a.run(Command::Mode(Mode::Item));
    let r = view_rect();
    let at = a.camera.project(DVec3::ZERO, r).unwrap().0;
    assert_eq!(a.pick_component(at, r).map(|h| h.0), Some(mesh));
    a.run(Command::EditItemCommon {
        items: vec![mesh],
        edit: ItemCommonEdit::Select(Choice::No),
    });
    assert!(a.pick_component(at, r).is_none());
    a.run(Command::SelectItem(Some(mesh)));
    assert_eq!(a.editor.scene.selection.item, Some(mesh));
}

#[test]
fn display_visible_no_hides_a_mesh_from_picking() {
    // CP-D01: not drawn, so not picked and hides nothing behind it.
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    a.run(Command::Mode(Mode::Polygon));
    let r = view_rect();
    let at = a.camera.project(DVec3::ZERO, r).unwrap().0;
    assert!(a.pick_component(at, r).is_some());
    a.run(Command::EditItemCommon {
        items: vec![mesh],
        edit: ItemCommonEdit::Visible(DisplayVisible::No),
    });
    assert!(a.pick_component(at, r).is_none());
    a.run(Command::Undo);
    assert!(a.pick_component(at, r).is_some());
}

#[test]
fn locked_items_show_no_transform_handles() {
    // CP-A02: transform tools show nothing on a locked item.
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    a.run(Command::Mode(Mode::Item));
    a.run(Command::SelectItem(Some(mesh)));
    a.activate(Tool::Move);
    let r = view_rect();
    assert!(!a.handles(r).is_empty());
    a.run(Command::Drop);
    a.run(Command::EditItemCommon {
        items: vec![mesh],
        edit: ItemCommonEdit::Lock(Choice::Yes),
    });
    a.activate(Tool::Move);
    assert!(a.handles(r).is_empty());
    a.run(Command::Drop);
    // A locked area light shows no size handles either.
    let light = new_light(&mut a, LightKind::Area);
    a.run(Command::Mode(Mode::Item));
    a.run(Command::SelectItem(Some(light)));
    a.activate(Tool::Move);
    assert!(a.item_handles(r).is_some());
    a.run(Command::Drop);
    a.run(Command::EditItemCommon {
        items: vec![light],
        edit: ItemCommonEdit::Lock(Choice::Yes),
    });
    a.activate(Tool::Move);
    assert!(a.item_handles(r).is_none());
}

#[test]
fn portal_has_area_handles() {
    let mut a = box_app();
    let portal = new_light(&mut a, LightKind::Portal);
    a.run(Command::Mode(Mode::Item));
    a.run(Command::SelectItem(Some(portal)));
    a.activate(Tool::Move);
    assert!(a.item_handles(view_rect()).is_some());
}

#[test]
fn affect_switches_store_zero_or_full() {
    // LPQ-07: on/off rows store 0 or 100 %.
    let mut a = box_app();
    let id = new_light(&mut a, LightKind::Point);
    a.run(Command::EditLight {
        item: id,
        edit: LightEdit::AffectDiffuse(0.0),
    });
    let l = a.editor.scene.item(id).unwrap();
    assert_eq!(l.light.as_ref().unwrap().affect_diffuse, 0.0);
}

// --- Tier 2 ----------------------------------------------------------------

#[test]
fn tier2_kinds_show_their_rows() {
    use operandi_core::camera_light::LightKind as K;
    for (kind, tab, want, absent) in [
        (
            K::Cylinder,
            "Cylinder Light",
            vec![
                "Targeting",
                "Radiance",
                "Radius",
                "Length",
                "Visible to Camera",
            ],
            vec!["Width", "Prototype", "Photometry File"],
        ),
        (
            K::Mesh,
            "Mesh Light",
            vec!["Radiance", "Prototype", "Use Prototype Transform"],
            vec!["Targeting", "Radius", "Length"],
        ),
        (
            K::Photometric,
            "Photometric Light",
            vec![
                "Targeting",
                "Radiant Intensity",
                "Width",
                "Height",
                "Cone Angle",
                "Soft Edge",
                "Photometry File",
                "Browse...",
            ],
            vec!["Prototype", "Length"],
        ),
        (
            K::Directional,
            "Directional Light",
            vec!["Targeting", "Spread Angle", "Physical Sun"],
            vec!["Radius"],
        ),
        (K::Point, "Point Light", vec!["Radius"], vec!["Targeting"]),
    ] {
        let mut a = box_app();
        new_light(&mut a, kind);
        a.properties_tab = tab.into();
        let t = properties_texts(&mut a);
        for label in want {
            y_of(&t, label);
        }
        for label in absent {
            assert!(t.iter().all(|(s, _)| s != label), "{kind:?}: {label} shown");
        }
    }
}

#[test]
fn targeting_rows_aim_and_grey_rotation() {
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    let spot = new_light(&mut a, LightKind::Spot);
    a.run(Command::EditItemCommon {
        items: vec![spot],
        edit: ItemCommonEdit::Target(Some(mesh)),
    });
    assert!(a.editor.scene.rotation_driven(spot));
    a.properties_tab = "Spot Light".into();
    let t = properties_texts(&mut a);
    for label in ["Remove Target", "Enable", "Roll"] {
        y_of(&t, label);
    }
    // The camera shares the rows.
    a.add_item(operandi_core::item_list::AddItemKind::Camera);
    a.properties_tab = "Camera View".into();
    y_of(&properties_texts(&mut a), "Targeting");
}

#[test]
fn photometry_chosen_loads_the_file() {
    let mut a = box_app();
    let id = new_light(&mut a, LightKind::Photometric);
    let path = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/photometry/operandi-downlight.ies"
    ));
    let steps = a.editor.undo.len();
    a.photometry_chosen(id, path.clone());
    assert_eq!(a.editor.undo.len(), steps + 1);
    let l = a.editor.scene.item(id).unwrap();
    let l = l.light.as_ref().unwrap();
    assert_eq!(l.photometry_file, path.to_string_lossy());
    assert_eq!((l.width, l.height), (0.3, 0.2));
    // LP-PH-05: the viewport shape follows the file (more marks than the
    // no-file star) and is longest straight down.
    let world = glam::DMat4::IDENTITY;
    let marks = crate::light_tier2_ui::photometric_marks(world, l);
    assert!(marks.len() > 50);
    let lowest = marks
        .iter()
        .filter_map(|m| match m {
            crate::camera_light_ui::Mark::Line([p, q]) => Some(p.z.min(q.z)),
            _ => None,
        })
        .fold(f64::MAX, f64::min);
    assert!((lowest + l.stored.display_size).abs() < 1e-9, "{lowest}");
}

#[test]
fn every_kind_is_in_the_create_light_menu() {
    use operandi_core::camera_light::LightKind as K;
    assert_eq!(K::ALL.len(), 9);
    for kind in K::ALL {
        let mut a = box_app();
        let id = new_light(&mut a, kind);
        let item = a.editor.scene.item(id).unwrap();
        assert_eq!(item.light.as_ref().unwrap().kind, kind);
    }
}

// --- Tier 3 (lights) -------------------------------------------------------

#[test]
fn tier3_light_rows() {
    use operandi_core::camera_light::LightKind as K;
    for (kind, tab, want, absent) in [
        (
            K::Spot,
            "Spot Light",
            vec![
                "Importance",
                "LPE Label",
                "Volumetrics",
                "Light Material",
                "Enable",
                "Blend Mode",
                "Opacity",
                "Affect Caustics",
                "Affect Subsurface",
                "Add Image Layer...",
            ],
            vec![],
        ),
        (
            K::Dome,
            "Dome Light",
            vec![
                "Render",
                "Dissolve",
                "Radiance",
                "Light Color",
                "Shadow Type",
            ],
            vec![
                "Falloff Type",
                "Visible to Camera",
                "LPE Label",
                "Targeting",
                "Volumetrics",
            ],
        ),
        (
            K::Area,
            "Area Light",
            vec!["LPE Label"],
            vec!["Volumetrics"],
        ),
    ] {
        let mut a = box_app();
        new_light(&mut a, kind);
        a.properties_tab = tab.into();
        let t = properties_texts(&mut a);
        for label in want {
            y_of(&t, label);
        }
        for label in absent {
            assert!(t.iter().all(|(s, _)| s != label), "{kind:?}: {label} shown");
        }
    }
}

#[test]
fn stored_sun_locations_save_and_list() {
    use crate::light_tier3_ui::{StoredLocation, SunLocations};
    let dir = std::env::temp_dir().join(format!("operandi-sun-{}", std::process::id()));
    let path = dir.join("sun-locations.json");
    let mut places = SunLocations::default();
    places.store(StoredLocation {
        name: "Studio".into(),
        latitude: 64.15,
        longitude: -21.94,
        time_zone: 0.0,
    });
    places.store(StoredLocation {
        name: "Studio".into(),
        latitude: 1.0,
        longitude: 2.0,
        time_zone: 3.0,
    });
    assert_eq!(places.places.len(), 1, "same name replaces");
    places.save(&path).unwrap();
    assert_eq!(SunLocations::load(&path), places);
    assert_eq!(
        SunLocations::load(&dir.join("missing.json")),
        SunLocations::default()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn light_group_passes_save_beside_the_render() {
    let p = std::path::Path::new("/renders/Render 3.png");
    assert_eq!(
        crate::render_ui::light_group_path(p, "Key"),
        std::path::Path::new("/renders/Render 3 (Key).png")
    );
    assert_eq!(
        crate::render_ui::light_group_path(p, "a/b:c"),
        std::path::Path::new("/renders/Render 3 (a_b_c).png")
    );
}

#[test]
fn dome_glyph_and_marks() {
    let l = operandi_core::camera_light::LightSettings::new(LightKind::Dome);
    let marks = crate::camera_light_ui::light_marks(glam::DMat4::IDENTITY, &l, false);
    assert!(marks.len() > 30);
    // Nothing below the ground plane.
    for m in &marks {
        if let crate::camera_light_ui::Mark::Line([a, b]) = m {
            assert!(a.y >= -1e-9 && b.y >= -1e-9);
        }
    }
}

// --- Tier 3 (shared tabs) --------------------------------------------------

fn extra(a: &mut Operandi, items: Vec<u64>, edit: operandi_core::item_extras::ExtraEdit) {
    a.run(Command::EditItemCommon {
        items,
        edit: ItemCommonEdit::Extra(edit),
    });
}

#[test]
fn shared_tabs_show_their_rows() {
    use operandi_core::item_extras::{ChannelType, ChannelValue, ExtraEdit, UserChannel};
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    a.run(Command::SelectItem(Some(mesh)));
    extra(&mut a, vec![mesh], ExtraEdit::Notes(true));
    for (tab, labels) in [
        (
            "Display",
            vec![
                "Name",
                "Pivot",
                "Center",
                "Add Draw Options",
                "Add Dimensions",
                "Remove Notes",
                "Line 9",
            ],
        ),
        (
            "Assembly",
            vec![
                "Redirect",
                "Pick Walk Up",
                "Pick Walk Right",
                "Tooltip",
                "User Tag",
                "Drop Action: Destination",
            ],
        ),
        // UIC-104: every tab starts with the item's Name row.
        ("User Channels", vec!["Name", "Add User Channel..."]),
        ("Tags", vec!["Name", "Add Tags", "Remove Tags"]),
    ] {
        a.properties_tab = tab.into();
        let t = properties_texts(&mut a);
        for label in labels {
            y_of(&t, label);
        }
    }
    extra(
        &mut a,
        vec![mesh],
        ExtraEdit::AddChannel(
            UserChannel::new(
                "Door",
                "Door Open",
                ChannelType::Float,
                ChannelValue::Number(1.0),
                None,
                None,
            )
            .unwrap(),
        ),
    );
    extra(
        &mut a,
        vec![mesh],
        ExtraEdit::SetTag("Comment".into(), "hello".into()),
    );
    a.properties_tab = "User Channels".into();
    y_of(&properties_texts(&mut a), "Door Open");
    a.properties_tab = "Tags".into();
    y_of(&properties_texts(&mut a), "Comment");
}

#[test]
fn redirect_selects_the_other_item_and_tooltip_shows() {
    use operandi_core::item_extras::ExtraEdit;
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    let light = new_light(&mut a, LightKind::Point);
    a.run(Command::Mode(Mode::Item));
    let r = view_rect();
    let at = a.camera.project(DVec3::ZERO, r).unwrap().0;
    extra(&mut a, vec![mesh], ExtraEdit::Redirect(Some(light)));
    a.select_at(at, r, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.item, Some(light));
    assert_eq!(a.item_tooltip(at, r), None);
    extra(
        &mut a,
        vec![mesh],
        ExtraEdit::Tooltip("The hero box".into()),
    );
    assert_eq!(a.item_tooltip(at, r).as_deref(), Some("The hero box"));
}

#[test]
fn draw_options_recolour_glyphs() {
    use operandi_core::item_extras::{ExtraEdit, WireColor};
    let mut a = box_app();
    let light = new_light(&mut a, LightKind::Point);
    let item = |a: &Operandi| a.editor.scene.item(light).unwrap().clone();
    assert_eq!(crate::item_extras_ui::draw_options_color(&item(&a)), None);
    extra(&mut a, vec![light], ExtraEdit::DrawOptions(true));
    extra(&mut a, vec![light], ExtraEdit::Wireframe(WireColor::User));
    extra(&mut a, vec![light], ExtraEdit::WireColor([1.0, 0.0, 0.0]));
    assert_eq!(
        crate::item_extras_ui::draw_options_color(&item(&a)),
        Some(Color32::from_rgb(255, 0, 0))
    );
    extra(&mut a, vec![light], ExtraEdit::DrawEnable(false));
    assert_eq!(crate::item_extras_ui::draw_options_color(&item(&a)), None);
}

// --- Item groups and Light Linking -----------------------------------------

#[test]
fn groups_tab_makes_and_selects_groups() {
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    let light = new_light(&mut a, LightKind::Point);
    a.run(Command::SelectItem(Some(mesh)));
    a.run(Command::ToggleItemSelection(light));
    let ctx = Context::default();
    a.new_group_from_selection(&ctx);
    let g = a.picked_group(&ctx).unwrap();
    assert_eq!(a.editor.scene.group_members(g), [mesh, light]);
    // The Groups tab lists it under the tool row (UIC-153) ...
    a.upper_frame.show_page(crate::groups_ui::GROUPS_PAGE);
    draw(&mut a, &ctx, SIZE, vec![]);
    let t = texts(&draw(&mut a, &ctx, SIZE, vec![]));
    for label in ["New Group", "Group"] {
        y_of(&t, label);
    }
    // ... and its right-click menu holds the group's actions.
    let row = a.groups_tree.rects[0].1.center();
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            egui::Event::PointerMoved(row),
            egui::Event::PointerButton {
                pos: row,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: row,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let t = texts(&draw(&mut a, &ctx, SIZE, vec![]));
    for label in ["Rename…", "Delete Group", "Add Selected", "Remove Selected"] {
        y_of(&t, label);
    }
    a.run(Command::SelectItem(None));
    a.run(Command::SelectGroup(g));
    assert_eq!(a.editor.scene.selection.item, Some(mesh));
    assert_eq!(a.editor.scene.selection.also, [light]);
}

#[test]
fn light_linking_rows_grey_until_control() {
    use operandi_core::camera_light::{LightEdit, LinkMode};
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    let spot = new_light(&mut a, LightKind::Spot);
    a.run(Command::NewGroup {
        name: Some("Hero".into()),
        members: vec![mesh],
    });
    let g = a.editor.scene.groups[0].id;
    a.run(Command::SelectItem(Some(spot)));
    a.properties_tab = "Spot Light".into();
    let t = properties_texts(&mut a);
    y_of(&t, "Light Linking");
    for (edit, ok) in [
        (LightEdit::LinkControl(true), true),
        (LightEdit::LinkGroup(Some(g)), true),
        (LightEdit::LinkMode(LinkMode::Include), true),
    ] {
        assert_eq!(a.run_ok(Command::EditLight { item: spot, edit }), ok);
    }
    let l = a.editor.scene.item(spot).unwrap();
    let link = l.light.as_ref().unwrap().linking.clone().unwrap();
    assert!(link.control && link.group == Some(g) && link.mode == LinkMode::Include);
}

#[test]
fn selected_lights_that_differ_show_mixed_until_edited() {
    // UIC-118: two point lights with different intensities: the field reads
    // "(mixed)"; Dissolve, the same on both, shows its value.
    let mut a = box_app();
    let first = new_light(&mut a, LightKind::Point);
    let second = new_light(&mut a, LightKind::Point);
    a.run(Command::EditLight {
        item: second,
        edit: LightEdit::Intensity(7.),
    });
    a.editor.scene.selection.item = Some(first);
    a.editor.scene.selection.also = vec![second];
    let texts = properties_texts(&mut a);
    assert!(
        texts.iter().any(|(t, _)| t == crate::form::rows::MIXED),
        "{texts:?}"
    );
    // One edit writes both lights as one undo step (LP-51).
    a.run(Command::EditLight {
        item: first,
        edit: LightEdit::Intensity(3.),
    });
    let intensity = |a: &Operandi, id: u64| {
        a.editor
            .scene
            .item(id)
            .unwrap()
            .light
            .as_ref()
            .unwrap()
            .intensity
    };
    assert_eq!((intensity(&a, first), intensity(&a, second)), (3., 3.));
    let texts = properties_texts(&mut a);
    assert!(
        !texts.iter().any(|(t, _)| t == crate::form::rows::MIXED),
        "{texts:?}"
    );
    a.run(Command::Undo);
    assert_eq!(intensity(&a, second), 7.);
}

#[test]
fn shared_tab_rows_show_mixed_where_selected_items_differ() {
    // UIC-118 on the Assembly tab: Select differs, Tooltip differs.
    use operandi_core::item_extras::ExtraEdit;
    let mut a = box_app();
    a.run(Command::NewMesh);
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    a.run(Command::EditItemCommon {
        items: vec![ids[1]],
        edit: ItemCommonEdit::Select(Choice::No),
    });
    extra(&mut a, vec![ids[1]], ExtraEdit::Tooltip("Door".into()));
    a.editor.scene.selection.item = Some(ids[0]);
    a.editor.scene.selection.also = vec![ids[1]];
    a.properties_tab = "Assembly".into();
    let texts = properties_texts(&mut a);
    let mixed = texts
        .iter()
        .filter(|(t, _)| t == crate::form::rows::MIXED)
        .count();
    assert!(mixed >= 2, "Select and Tooltip read (mixed): {texts:?}");
}
