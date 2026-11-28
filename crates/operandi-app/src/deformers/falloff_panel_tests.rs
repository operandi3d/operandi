//! Falloff batch 1 (falloff spec 2026-11-15 v1.1): falloff sections under
//! the tool in Tool Properties, In/Out greyed, Mix Mode on every layer, Esc
//! clears, right-click Steps, the Falloff menu's lists, Show Falloffs.
use super::tests::{box_app, draw, key};
use super::*;
use crate::falloff_panel::{
    FalloffFields, FalloffP, IN_OUT_UNUSED, fitted_on_choose, selection_layer, set_selection_steps,
    set_soft_selection_radius, soft_selection_layer,
};
use crate::tool_form::{Bind, ButtonAction, Item, ToolFields, ToolForm};
use glam::DVec2;
use operandi_core::{
    AirbrushFalloff, CoplanarFalloff, CurvatureFalloff, CylinderFalloff, ElementFalloff,
    FalloffShape, FalloffSymmetric, ImageFalloff, LassoFalloff, LassoStyle, LinearFalloff, MixMode,
    NoiseFalloff, PathFalloff, RadialFalloff, ScreenFalloff, SelectionFalloff,
    SoftSelectionFalloff, TweakConnect, TweakMode, VertexMapFalloff, VertexMapMode,
};

const SIZE: Vec2 = vec2(1440., 900.);

/// A box in Vertex mode, every point selected, with `tool` open.
fn app_with(tool: Tool) -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Vertex));
    let n = a.editor.scene.active().unwrap().mesh.vertices.len() as u32;
    a.editor.scene.selection.vertices = (0..n).collect();
    a.activate(tool);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(tool),
        "{}",
        a.status
    );
    a
}

fn form(a: &Operandi) -> ToolForm<FalloffP> {
    let p = FalloffFields::params(a).expect("a falloff is active");
    FalloffFields::form(a, &p)
}

/// Each section's title and its rows' labels.
fn sections(f: &ToolForm<FalloffP>) -> Vec<(&'static str, Vec<String>)> {
    f.items
        .iter()
        .filter_map(|i| match i {
            Item::Section { name, items, .. } => Some((
                *name,
                items
                    .iter()
                    .filter_map(|i| match i {
                        Item::Field(f) => Some(f.row.label.clone()),
                        _ => None,
                    })
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

fn field<'a>(f: &'a ToolForm<FalloffP>, key: &str) -> &'a tool_form::Field<FalloffP> {
    f.fields()
        .into_iter()
        .find(|x| x.row.key == key)
        .unwrap_or_else(|| panic!("no field {key}"))
}

/// FO-10/11/12 (G01, G02, G03): Scale with a Selection falloff shows a
/// "Selection Falloff" section: Steps, Shape, In, Out (greyed), Mix Mode last.
#[test]
fn selection_falloff_shows_as_a_section_under_the_tool() {
    let mut a = app_with(Tool::Scale);
    a.run(Command::SpatialFalloff(Some(Falloff::Selection(
        SelectionFalloff::default(),
    ))));
    let f = form(&a);
    assert_eq!(
        sections(&f),
        vec![(
            "Selection Falloff",
            vec!["Steps", "Shape", "In", "Out", "Mix Mode"]
                .into_iter()
                .map(String::from)
                .collect()
        )]
    );
    for k in ["falloff0_shape_in", "falloff0_shape_out"] {
        assert_eq!(
            field(&f, k).disabled_tip.as_deref(),
            Some(IN_OUT_UNUSED),
            "{k}"
        );
    }
    assert!(field(&f, "falloff0_steps").disabled_tip.is_none());
    assert!(field(&f, "falloff0_mix").disabled_tip.is_none());
}

/// FO-13: Linear says Shape too (FO-Q09, Jason's OK);
/// Custom ungreys In and Out, which show the factory 0 and 0 before
/// (FO-30a).
#[test]
fn linear_rows_and_custom_in_out() {
    let mut a = app_with(Tool::Move);
    a.run(Command::SpatialFalloff(Some(Falloff::Linear(
        LinearFalloff::default(),
    ))));
    let f = form(&a);
    let (name, labels) = &sections(&f)[0];
    assert_eq!(*name, "Linear Falloff");
    assert_eq!(
        labels,
        &[
            "Start X",
            "Y",
            "Z",
            "End X",
            "Y",
            "Z",
            "Symmetric",
            "Use World Transforms",
            "Shape",
            "In",
            "Out",
            "Mix Mode"
        ]
    );
    let p = FalloffFields::params(&a).unwrap();
    let read = |f: &ToolForm<FalloffP>, k: &str, p: &FalloffP| match &field(f, k).bind {
        tool_form::Bind::Number { get, .. } => get(p),
        _ => panic!("{k} is a number"),
    };
    assert_eq!(read(&f, "falloff0_shape_in", &p), 0.);
    assert_eq!(read(&f, "falloff0_shape_out", &p), 0.);
    // Picking Custom: In and Out apply.
    let mut custom = p.clone();
    if let Falloff::Linear(l) = &mut custom.layers[0].0 {
        l.shape = FalloffShape::CUSTOM;
    }
    FalloffFields::publish(&mut a, custom).unwrap();
    let f = form(&a);
    assert!(field(&f, "falloff0_shape_in").disabled_tip.is_none());
    // Editing a row re-applies through the falloff command (FO-14).
    let mut moved = FalloffFields::params(&a).unwrap();
    if let Falloff::Linear(l) = &mut moved.layers[0].0 {
        l.end = DVec3::new(0., 3., 0.);
    }
    FalloffFields::publish(&mut a, moved).unwrap();
    assert!(matches!(
        a.editor.spatial_falloff,
        Some(Falloff::Linear(l)) if l.end == DVec3::new(0., 3., 0.)
    ));
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
}

/// FO-10/11/32: every layer gets its own section with Mix Mode; the first
/// layer's Mix Mode is kept but changes nothing; Remove takes a layer off.
#[test]
fn every_layer_has_a_section_and_mix_mode() {
    let mut a = app_with(Tool::Move);
    a.run(Command::SpatialFalloff(Some(Falloff::Radial(
        RadialFalloff::default(),
    ))));
    a.run(Command::AddedFalloffs(vec![
        (
            Falloff::VertexMap(VertexMapFalloff::default()),
            MixMode::Add,
        ),
        (Falloff::Invert, MixMode::Multiply),
    ]));
    let f = form(&a);
    let s = sections(&f);
    let names: Vec<_> = s.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        ["Radial Falloff", "Vertex Map Falloff", "Invert Falloff"]
    );
    assert_eq!(s[0].1.last().map(String::as_str), Some("Mix Mode"));
    assert_eq!(s[1].1, ["Mode", "Index", "Mix Mode"]);
    // FO-33: Invert has no options and no Mix Mode.
    assert!(s[2].1.is_empty());
    // Index is greyed outside Component mode (VMF-01).
    assert!(field(&f, "falloff1_vm_index").disabled_tip.is_some());
    // The first layer's Mix Mode: stored, no effect (FO-32).
    let weights = |a: &Operandi| {
        let stack = a.editor.active_falloff().unwrap();
        let inputs = operandi_core::falloff::PointInputs {
            map: None,
            rings: None,
            layers: &[],
        };
        [0., 0.5, 0.9].map(|x| stack.weight_at(DVec3::new(x, 0., 0.), 0, inputs))
    };
    let before = weights(&a);
    let mut p = FalloffFields::params(&a).unwrap();
    p.layers[0].1 = MixMode::Subtract;
    FalloffFields::publish(&mut a, p).unwrap();
    assert_eq!(a.editor.spatial_falloff_mix, MixMode::Subtract);
    assert_eq!(weights(&a), before);
    // Remove the Vertex Map layer.
    let mut p = FalloffFields::params(&a).unwrap();
    p.layers.remove(1);
    FalloffFields::publish(&mut a, p).unwrap();
    assert_eq!(
        a.editor.added_falloffs,
        vec![(Falloff::Invert, MixMode::Multiply)]
    );
    // Choosing another type replaces the stack and resets Mix Mode.
    a.run(Command::SpatialFalloff(Some(Falloff::Selection(
        SelectionFalloff::default(),
    ))));
    assert!(a.editor.added_falloffs.is_empty());
    assert_eq!(a.editor.spatial_falloff_mix, MixMode::Multiply);
}

/// FO-16: with no tool open the falloff's sections still show.
#[test]
fn falloff_sections_show_with_no_tool() {
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(Falloff::Selection(
        SelectionFalloff::default(),
    ))));
    assert!(a.editor.session.live.is_none());
    assert_eq!(sections(&form(&a))[0].0, "Selection Falloff");
    // Nothing to show with no falloff.
    a.run(Command::SpatialFalloff(None));
    assert!(FalloffFields::params(&a).is_none());
}

/// FO-02: Esc over a view with no tool edit in progress clears the falloff
/// (every layer) and leaves the tool open.
#[test]
fn escape_in_the_view_clears_the_falloff() {
    let ctx = Context::default();
    let mut a = app_with(Tool::Move);
    a.run(Command::SpatialFalloff(Some(Falloff::Radial(
        RadialFalloff::default(),
    ))));
    a.run(Command::AddedFalloffs(vec![(
        Falloff::Invert,
        MixMode::Multiply,
    )]));
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = a.viewport_rect.center();
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(at), key(Key::Escape)],
    );
    assert_eq!(a.editor.spatial_falloff, None, "{}", a.status);
    assert!(a.editor.added_falloffs.is_empty());
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
}

fn right_press(a: &mut Operandi, ctx: &Context, from: Pos2, by: Vec2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![button(from, true)]);
    if by != Vec2::ZERO {
        for step in 1..=4 {
            draw(
                a,
                ctx,
                SIZE,
                vec![Event::PointerMoved(from + by * (step as f32 / 4.))],
            );
        }
    }
    draw(a, ctx, SIZE, vec![button(from + by, false)]);
}

/// FO-46 / FO-Q04 default: a right-click adds one Step, a right-drag right
/// adds one per 20 px, also when Selection is an added layer.
#[test]
fn right_click_adds_a_step_and_drags_change_steps() {
    let ctx = Context::default();
    let mut a = app_with(Tool::Move);
    a.run(Command::SpatialFalloff(Some(Falloff::Radial(
        RadialFalloff::default(),
    ))));
    a.run(Command::AddedFalloffs(vec![(
        Falloff::Selection(SelectionFalloff::default()),
        MixMode::Multiply,
    )]));
    draw(&mut a, &ctx, SIZE, vec![]);
    let steps = |a: &Operandi| match a.editor.added_falloffs.first() {
        Some((Falloff::Selection(s), _)) => s.steps,
        other => panic!("{other:?}"),
    };
    assert_eq!(steps(&a), 2);
    let from = a.viewport_rect.left_top() + vec2(80., 80.);
    right_press(&mut a, &ctx, from, Vec2::ZERO);
    assert_eq!(steps(&a), 3, "{}", a.status);
    right_press(&mut a, &ctx, from, vec2(45., 0.));
    assert_eq!(steps(&a), 5, "{}", a.status);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
}

/// FO-20/21 (batch 1, built types): Add lists Invert, Linear, Radial,
/// Vertex Map, Selection; Show Falloffs starts on (FO-06).
#[test]
fn add_list_order_and_show_falloffs_default() {
    let a = box_app();
    let labels: Vec<&str> = a.add_choices().iter().map(|f| f.label()).collect();
    assert_eq!(
        labels,
        [
            "Invert",
            "Linear",
            "Cylinder",
            "Radial",
            "Airbrush",
            "Screen",
            "Element",
            "Noise",
            "Curvature",
            "Vertex Map",
            "Path",
            "Lasso",
            "Image",
            "Selection",
            "Soft Selection",
            "Coplanar"
        ]
    );
    assert!(a.show_falloffs);
}

// Exercise the same bindings Tool Properties uses, without creating a UI.
fn choose(form: &ToolForm<FalloffP>, p: &mut FalloffP, key: &str, label: &str) {
    let f = field(form, key);
    let (crate::form::Kind::Choice(labels), Bind::Choice { get, set }) = (&f.row.kind, &f.bind)
    else {
        panic!("{key} is a choice");
    };
    let index = labels.iter().position(|name| name == label).unwrap();
    set(p, index);
    assert_eq!(get(p), index, "{key}");
}

fn number(
    form: &ToolForm<FalloffP>,
    p: &mut FalloffP,
    key: &str,
    value: f64,
) -> std::result::Result<f64, String> {
    let Bind::Number { get, set } = &field(form, key).bind else {
        panic!("{key} is a number");
    };
    set(p, value)?;
    Ok(get(p))
}

fn check(form: &ToolForm<FalloffP>, p: &mut FalloffP, key: &str, on: bool) {
    let f = field(form, key);
    assert!(f.disabled_tip.is_none(), "{key}");
    let Bind::Check { get, set } = &f.bind else {
        panic!("{key} is a checkbox");
    };
    set(p, on);
    assert_eq!(get(p), on, "{key}");
}

fn change(form: &ToolForm<FalloffP>, p: &mut FalloffP, layer: usize, label: &str) {
    let Item::Section { items, .. } = &form.items[layer] else {
        panic!("expected a falloff section at {layer}");
    };
    let action = items.iter().find_map(|item| match item {
        Item::Wide(button) if button.label == label => Some(&button.on),
        _ => None,
    });
    let Some(ButtonAction::Change(edit)) = action else {
        panic!("missing {label} on layer {layer}");
    };
    edit(p, ());
}

#[test]
fn stack_buttons_and_mix_choices_publish_to_the_right_layers() {
    let mut app = box_app();
    let linear = LinearFalloff {
        start: DVec3::new(1., 2., 3.),
        end: DVec3::new(4., 5., 6.),
        ..Default::default()
    };
    let radial = RadialFalloff::default().into();
    let mut p = FalloffP {
        layers: vec![
            (linear.into(), MixMode::Multiply),
            (radial, MixMode::Add),
            (Falloff::Invert, MixMode::Multiply),
        ],
    };
    FalloffFields::publish(&mut app, p.clone()).unwrap();
    assert_eq!(FalloffFields::params(&app), Some(p.clone()));

    let fields = form(&app);
    choose(&fields, &mut p, "falloff0_mix", "Subtract");
    choose(&fields, &mut p, "falloff1_mix", "Max");
    change(&fields, &mut p, 0, "Reverse");
    FalloffFields::publish(&mut app, p).unwrap();
    let reversed = LinearFalloff {
        start: linear.end,
        end: linear.start,
        ..linear
    };
    assert_eq!(app.editor.spatial_falloff, Some(reversed.into()));
    assert_eq!(app.editor.spatial_falloff_mix, MixMode::Subtract);
    assert_eq!(
        app.editor.added_falloffs,
        [(radial, MixMode::Max), (Falloff::Invert, MixMode::Multiply)]
    );

    let fields = form(&app);
    let mut p = FalloffFields::params(&app).unwrap();
    change(&fields, &mut p, 1, "Remove");
    FalloffFields::publish(&mut app, p).unwrap();
    assert_eq!(app.editor.spatial_falloff, Some(reversed.into()));
    assert_eq!(
        app.editor.added_falloffs,
        [(Falloff::Invert, MixMode::Multiply)]
    );

    FalloffFields::publish(&mut app, FalloffP { layers: vec![] }).unwrap();
    assert!(FalloffFields::params(&app).is_none());
    assert!(app.editor.added_falloffs.is_empty());
    assert_eq!(app.editor.spatial_falloff_mix, MixMode::Multiply);
}

#[test]
fn invalid_added_layer_leaves_the_published_stack_intact() {
    let mut app = box_app();
    let before = FalloffP {
        layers: vec![
            (LinearFalloff::default().into(), MixMode::Add),
            (Falloff::Invert, MixMode::Multiply),
        ],
    };
    FalloffFields::publish(&mut app, before.clone()).unwrap();
    let rejected = FalloffP {
        layers: vec![
            (RadialFalloff::default().into(), MixMode::Subtract),
            (
                Falloff::Selection(SelectionFalloff {
                    steps: 0,
                    ..Default::default()
                }),
                MixMode::Multiply,
            ),
        ],
    };
    assert!(FalloffFields::publish(&mut app, rejected).is_err());
    assert_eq!(FalloffFields::params(&app), Some(before));
}

#[test]
fn choosing_a_type_fits_cylinder_only_with_an_open_tool_and_keeps_presets() {
    let mut app = app_with(Tool::Move);
    // One side of the box gives the fit a nonzero center.
    let mesh = &app.editor.scene.active().unwrap().mesh;
    app.editor.scene.selection.vertices = mesh
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, point)| point.x > 0.)
        .map(|(index, _)| index as u32)
        .collect();
    let linear = LinearFalloff {
        end: DVec3::new(3., 4., 5.),
        ..Default::default()
    };
    let radial = RadialFalloff {
        center: DVec3::splat(4.),
        ..Default::default()
    };
    for f in [linear.into(), radial.into()] {
        app.editor
            .dispatch(Command::SpatialFalloff(Some(f)))
            .unwrap();
    }
    app.editor
        .dispatch(Command::AddedFalloffs(vec![(
            Falloff::Invert,
            MixMode::Add,
        )]))
        .unwrap();

    let choices = app.add_choices();
    assert!(choices.contains(&linear.into()));
    assert!(choices.contains(&radial.into()));
    for f in [linear.into(), radial.into()] {
        assert_eq!(fitted_on_choose(&app.editor, f), f);
    }
    let cylinder = CylinderFalloff::default();
    let choice = Falloff::Cylinder(cylinder);
    let fitted = fitted_on_choose(&app.editor, choice);
    assert_ne!(fitted, choice);
    assert_eq!(
        fitted,
        Falloff::Cylinder(cylinder.auto_size(&app.editor.scene).unwrap())
    );
    app.editor
        .dispatch(Command::SpatialFalloff(Some(fitted)))
        .unwrap();
    assert!(app.editor.added_falloffs.is_empty());
    assert_eq!(app.editor.spatial_falloff, Some(fitted));

    app.editor.dispatch(Command::Drop).unwrap();
    assert_eq!(fitted_on_choose(&app.editor, choice), choice);
}

#[test]
fn shape_choices_keep_custom_values_when_reselected_and_restore_factory_values() {
    let mut app = box_app();
    let mut p = FalloffP {
        layers: vec![(LinearFalloff::default().into(), MixMode::Multiply)],
    };
    let fields = FalloffFields::form(&app, &p);
    for (label, shape) in [
        ("Ease-In", FalloffShape::EaseIn),
        ("Ease-Out", FalloffShape::EaseOut),
        ("Smooth", FalloffShape::Smooth),
        ("Linear", FalloffShape::Linear),
        ("Custom", FalloffShape::CUSTOM),
    ] {
        choose(&fields, &mut p, "falloff0_shape-preset", label);
        assert!(matches!(p.layers[0].0, Falloff::Linear(l) if l.shape == shape));
    }
    let fields = FalloffFields::form(&app, &p);
    for (key, value) in [("falloff0_shape_in", 0.25), ("falloff0_shape_out", 0.75)] {
        assert!(field(&fields, key).disabled_tip.is_none());
        assert_eq!(number(&fields, &mut p, key, value), Ok(value));
    }
    choose(&fields, &mut p, "falloff0_shape-preset", "Custom");
    FalloffFields::publish(&mut app, p.clone()).unwrap();
    assert!(
        matches!(app.editor.spatial_falloff, Some(Falloff::Linear(l))
        if l.shape == FalloffShape::Custom { ease_in: 0.25, ease_out: 0.75 })
    );

    choose(&fields, &mut p, "falloff0_shape-preset", "Linear");
    let fields = FalloffFields::form(&app, &p);
    for key in ["falloff0_shape_in", "falloff0_shape_out"] {
        assert_eq!(
            field(&fields, key).disabled_tip.as_deref(),
            Some(IN_OUT_UNUSED)
        );
        let before = p.clone();
        assert_eq!(number(&fields, &mut p, key, 0.5), Ok(0.));
        assert_eq!(p, before);
    }
    choose(&fields, &mut p, "falloff0_shape-preset", "Custom");
    assert!(matches!(p.layers[0].0, Falloff::Linear(l) if l.shape == FalloffShape::CUSTOM));
}

#[test]
fn vertex_map_index_enables_in_component_mode_and_resets_in_magnitude_mode() {
    let mut app = box_app();
    let mut p = FalloffP {
        layers: vec![(
            Falloff::VertexMap(VertexMapFalloff::default()),
            MixMode::Multiply,
        )],
    };
    let fields = FalloffFields::form(&app, &p);
    choose(&fields, &mut p, "falloff0_vm_mode", "Component");
    let fields = FalloffFields::form(&app, &p);
    let key = "falloff0_vm_index";
    assert!(field(&fields, key).disabled_tip.is_none());
    assert_eq!(number(&fields, &mut p, key, 3.), Ok(3.));
    let before = p.clone();
    for invalid in [-1., 0.5, 4., f64::NAN] {
        assert!(number(&fields, &mut p, key, invalid).is_err());
        assert_eq!(p, before);
    }
    FalloffFields::publish(&mut app, p.clone()).unwrap();
    assert_eq!(
        app.editor.spatial_falloff,
        Some(Falloff::VertexMap(VertexMapFalloff {
            mode: VertexMapMode::Component,
            index: 3,
        }))
    );
    choose(&fields, &mut p, "falloff0_vm_mode", "Magnitude");
    let fields = FalloffFields::form(&app, &p);
    assert_eq!(
        field(&fields, key).disabled_tip.as_deref(),
        Some("Index applies in Component mode")
    );
    assert_eq!(
        p.layers[0].0,
        Falloff::VertexMap(VertexMapFalloff {
            mode: VertexMapMode::Magnitude,
            index: 0,
        })
    );
}

#[test]
fn element_choices_grey_range_in_rigid_mode_and_preserve_its_value() {
    let mut app = box_app();
    let mut p = FalloffP {
        layers: vec![(
            Falloff::Element(ElementFalloff::default()),
            MixMode::Multiply,
        )],
    };
    let fields = FalloffFields::form(&app, &p);
    choose(&fields, &mut p, "falloff0_el_mode", "Edge Center");
    assert_eq!(number(&fields, &mut p, "falloff0_el_range", 2.5), Ok(2.5));
    check(&fields, &mut p, "falloff0_el_raycast", true);
    choose(&fields, &mut p, "falloff0_el_connect", "Rigid Connections");
    let fields = FalloffFields::form(&app, &p);
    assert!(field(&fields, "falloff0_el_range").disabled_tip.is_some());
    choose(&fields, &mut p, "falloff0_el_connect", "Edge Loops");
    let fields = FalloffFields::form(&app, &p);
    assert!(field(&fields, "falloff0_el_range").disabled_tip.is_none());
    FalloffFields::publish(&mut app, p).unwrap();
    assert_eq!(
        app.element_falloff(),
        Some(ElementFalloff {
            mode: TweakMode::EdgeCenter,
            range: 2.5,
            connect: TweakConnect::EdgeLoops,
            always_raycast: true,
            ..Default::default()
        })
    );
}

#[test]
fn show_vertex_and_edge_controls_are_enabled_only_on_the_first_layer() {
    let app = box_app();
    let soft = Falloff::SoftSelection(SoftSelectionFalloff::default());
    let coplanar = Falloff::Coplanar(CoplanarFalloff::default());
    for (first, second) in [(soft, coplanar), (coplanar, soft)] {
        let mut p = FalloffP {
            layers: vec![(first, MixMode::Multiply), (second, MixMode::Add)],
        };
        let fields = FalloffFields::form(&app, &p);
        for name in ["show_vertex", "show_edge"] {
            let key = format!("falloff0_{name}");
            check(&fields, &mut p, &key, false);
            check(&fields, &mut p, &key, true);
            assert_eq!(
                field(&fields, &format!("falloff1_{name}"))
                    .disabled_tip
                    .as_deref(),
                Some("Shown for the first falloff only"),
            );
        }
        let (vertex, edge) = match p.layers[0].0 {
            Falloff::SoftSelection(s) => (s.show_vertex, s.show_edge),
            Falloff::Coplanar(c) => (c.show_vertex, c.show_edge),
            other => panic!("{other:?}"),
        };
        assert!(vertex && edge);
        assert_eq!(p.layers[1], (second, MixMode::Add));
    }
}

#[test]
fn image_controls_clamp_values_and_grey_repeat_through_uvs() {
    let mut app = box_app();
    let mut p = FalloffP {
        layers: vec![(Falloff::Image(ImageFalloff::default()), MixMode::Multiply)],
    };
    let fields = FalloffFields::form(&app, &p);
    for (key, input, expected) in [
        ("falloff0_img_soft", -4., 0.),
        ("falloff0_img_soft", 120., 100.),
        ("falloff0_img_scale", 0., 0.01),
        ("falloff0_img_px", -0.25, -0.25),
        ("falloff0_img_py", 0.75, 0.75),
        ("falloff0_img_rot", 45., 45.),
    ] {
        assert_eq!(number(&fields, &mut p, key, input), Ok(expected), "{key}");
    }
    check(&fields, &mut p, "falloff0_img_repeat", false);
    check(&fields, &mut p, "falloff0_img_uv", true);
    check(&fields, &mut p, "falloff0_world", true);
    let fields = FalloffFields::form(&app, &p);
    assert_eq!(
        field(&fields, "falloff0_img_repeat")
            .disabled_tip
            .as_deref(),
        Some("Repeat has no effect through a UV map")
    );
    FalloffFields::publish(&mut app, p.clone()).unwrap();
    assert_eq!(
        app.editor.spatial_falloff,
        Some(Falloff::Image(ImageFalloff {
            repeat: false,
            soft_border: 100.,
            use_uv: true,
            scale: 0.01,
            pos: DVec2::new(-0.25, 0.75),
            rotation: 45.,
            world: true,
        }))
    );
    check(&fields, &mut p, "falloff0_img_uv", false);
    let fields = FalloffFields::form(&app, &p);
    assert!(field(&fields, "falloff0_img_repeat").disabled_tip.is_none());
}

#[test]
fn screen_airbrush_and_lasso_options_map_pixels_percentages_and_choices() {
    let mut app = box_app();
    let mut p = FalloffP {
        layers: vec![
            (Falloff::Screen(ScreenFalloff::default()), MixMode::Multiply),
            (Falloff::Airbrush(AirbrushFalloff::default()), MixMode::Add),
            (Falloff::Lasso(LassoFalloff::default()), MixMode::Max),
        ],
    };
    let fields = FalloffFields::form(&app, &p);
    for (key, input, expected) in [
        ("falloff0_scr_cx", -10., -10.),
        ("falloff0_scr_cy", 25., 25.),
        ("falloff0_scr_size", -1., 0.),
        ("falloff1_air_size", 11., 11.),
        ("falloff1_air_strength", 75., 75.),
        ("falloff2_lasso_soft", 6., 6.),
    ] {
        assert_eq!(number(&fields, &mut p, key, input), Ok(expected), "{key}");
    }
    check(&fields, &mut p, "falloff0_scr_transparent", true);
    check(&fields, &mut p, "falloff1_air_transparent", true);
    check(&fields, &mut p, "falloff2_world", false);
    choose(&fields, &mut p, "falloff2_lasso_style", "Ellipse");
    FalloffFields::publish(&mut app, p).unwrap();
    assert_eq!(
        app.editor.spatial_falloff,
        Some(Falloff::Screen(ScreenFalloff {
            center: DVec2::new(-10., 25.),
            size: 0.,
            transparent: true,
        }))
    );
    assert_eq!(
        app.editor.added_falloffs,
        [
            (
                Falloff::Airbrush(AirbrushFalloff {
                    size: 11.,
                    strength: 0.75,
                    transparent: true,
                    ..Default::default()
                }),
                MixMode::Add
            ),
            (
                Falloff::Lasso(LassoFalloff {
                    style: LassoStyle::Ellipse,
                    soft_border: 6.,
                    world: false,
                }),
                MixMode::Max
            ),
        ]
    );
}

#[test]
fn geometric_fields_edit_only_the_named_axis_or_option() {
    let mut app = box_app();
    let linear = LinearFalloff::default();
    let radial = RadialFalloff::default();
    let cylinder = CylinderFalloff::default();
    let mut p = FalloffP {
        layers: vec![
            (linear.into(), MixMode::Multiply),
            (radial.into(), MixMode::Add),
            (Falloff::Cylinder(cylinder), MixMode::Min),
        ],
    };
    let fields = FalloffFields::form(&app, &p);
    for (key, value) in [
        ("falloff0_start1", -3.),
        ("falloff0_end2", 6.),
        ("falloff1_center2", 8.),
        ("falloff1_size0", 4.),
        ("falloff2_center0", 5.),
        ("falloff2_size1", 7.),
    ] {
        assert_eq!(number(&fields, &mut p, key, value), Ok(value), "{key}");
    }
    choose(&fields, &mut p, "falloff0_symmetric", "End");
    choose(&fields, &mut p, "falloff2_axis", "Z");
    for key in ["falloff0_world", "falloff1_world", "falloff2_world"] {
        check(&fields, &mut p, key, true);
    }
    let before = p.clone();
    assert!(number(&fields, &mut p, "falloff0_start1", f64::INFINITY).is_err());
    assert_eq!(p, before);
    FalloffFields::publish(&mut app, p).unwrap();
    assert_eq!(
        app.editor.spatial_falloff,
        Some(
            LinearFalloff {
                start: DVec3::new(linear.start.x, -3., linear.start.z),
                end: DVec3::new(linear.end.x, linear.end.y, 6.),
                symmetric: FalloffSymmetric::End,
                world: true,
                ..linear
            }
            .into()
        )
    );
    assert_eq!(
        app.editor.added_falloffs,
        [
            (
                RadialFalloff {
                    center: DVec3::new(radial.center.x, radial.center.y, 8.),
                    size: DVec3::new(4., radial.size.y, radial.size.z),
                    world: true,
                    ..radial
                }
                .into(),
                MixMode::Add
            ),
            (
                Falloff::Cylinder(CylinderFalloff {
                    center: DVec3::new(5., cylinder.center.y, cylinder.center.z),
                    size: DVec3::new(cylinder.size.x, 7., cylinder.size.z),
                    axis: 2,
                    world: true,
                    ..cylinder
                }),
                MixMode::Min
            ),
        ]
    );
}

#[test]
fn numeric_fields_reject_out_of_range_or_nonfinite_edits_without_changing_layers() {
    let app = box_app();
    let cases = [
        (
            Falloff::Selection(SelectionFalloff::default()),
            "steps",
            0.,
            1001.,
            5.,
        ),
        (
            Falloff::SoftSelection(SoftSelectionFalloff::default()),
            "soft_radius",
            -1.,
            f64::INFINITY,
            2.,
        ),
        (
            Falloff::Element(ElementFalloff::default()),
            "el_range",
            -1.,
            f64::INFINITY,
            2.,
        ),
        (
            Falloff::Coplanar(CoplanarFalloff::default()),
            "cop_angle",
            -1.,
            181.,
            180.,
        ),
        (
            Falloff::Lasso(LassoFalloff::default()),
            "lasso_soft",
            -1.,
            f64::INFINITY,
            2.,
        ),
        (
            Falloff::Airbrush(AirbrushFalloff::default()),
            "air_size",
            -1.,
            f64::INFINITY,
            2.,
        ),
        (
            Falloff::Airbrush(AirbrushFalloff::default()),
            "air_strength",
            -1.,
            101.,
            100.,
        ),
        (
            Falloff::Noise(NoiseFalloff::default()),
            "noise_scale",
            0.,
            f64::INFINITY,
            2.,
        ),
        (
            Falloff::Path(PathFalloff::default()),
            "path_size",
            -1.,
            f64::INFINITY,
            2.,
        ),
        (
            Falloff::Curvature(CurvatureFalloff::default()),
            "curv_scale",
            f64::NEG_INFINITY,
            f64::INFINITY,
            -2.,
        ),
    ];
    for (falloff, suffix, bad1, bad2, good) in cases {
        let mut p = FalloffP {
            layers: vec![(falloff, MixMode::Multiply)],
        };
        let fields = FalloffFields::form(&app, &p);
        let key = format!("falloff0_{suffix}");
        let before = p.clone();
        for invalid in [bad1, bad2, f64::NAN] {
            assert!(
                number(&fields, &mut p, &key, invalid).is_err(),
                "{key}: {invalid}"
            );
            assert_eq!(p, before, "{key}");
        }
        assert_eq!(number(&fields, &mut p, &key, good), Ok(good), "{key}");
        assert_ne!(p, before, "{key}");
    }
}

#[test]
fn selection_gesture_edits_target_only_the_first_matching_layer() {
    let mut app = box_app();
    assert!(selection_layer(&app.editor).is_none());
    assert!(soft_selection_layer(&app.editor).is_none());
    set_selection_steps(&mut app, 0, 9);
    set_soft_selection_radius(&mut app, 0, 2.5);
    assert!(FalloffFields::params(&app).is_none());
    let selection = SelectionFalloff::default();
    let soft = SoftSelectionFalloff::default();
    let before = FalloffP {
        layers: vec![
            (Falloff::Selection(selection), MixMode::Min),
            (Falloff::SoftSelection(soft), MixMode::Add),
            (Falloff::Selection(selection), MixMode::Subtract),
            (Falloff::SoftSelection(soft), MixMode::Max),
        ],
    };
    FalloffFields::publish(&mut app, before.clone()).unwrap();
    assert_eq!(selection_layer(&app.editor), Some((0, selection)));
    assert_eq!(soft_selection_layer(&app.editor), Some((1, soft)));
    set_selection_steps(&mut app, 2, 9);
    set_soft_selection_radius(&mut app, 3, 2.5);
    assert_eq!(FalloffFields::params(&app), Some(before.clone()));

    set_selection_steps(&mut app, 0, 9);
    set_soft_selection_radius(&mut app, 1, 2.5);
    let mut expected = before;
    expected.layers[0].0 = Falloff::Selection(SelectionFalloff {
        steps: 9,
        ..selection
    });
    expected.layers[1].0 = Falloff::SoftSelection(SoftSelectionFalloff {
        radius: 2.5,
        ..soft
    });
    assert_eq!(FalloffFields::params(&app), Some(expected));
}
