//! Falloffs in the UI (specifications/falloff-2026-11-01, falloff spec
//! 2026-11-15 batch 1). The context-bar Falloff menu only chooses, adds and
//! clears (FO-15, FO-20..23, FO-Q10); every option lives in Tool Properties,
//! one section per layer under the active tool (FO-10..14, `FalloffFields`).
//! Every change goes through the falloff commands, so a live tool re-applies.
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use crate::unavailable::Gate;
use operandi_core::{
    AirbrushFalloff, CoplanarFalloff, CurvatureFalloff, CylinderFalloff, ElementFalloff,
    FalloffShape, FalloffSymmetric, ImageFalloff, LassoFalloff, LassoStyle, MixMode, NoiseFalloff,
    PathFalloff, ScreenFalloff, SelectionFalloff, SoftSelectionFalloff, TweakConnect, TweakMode,
    VertexMapFalloff, VertexMapMode,
};

/// FO-30: the Shape choices, in menu order.
const SHAPES: [FalloffShape; 5] = [
    FalloffShape::Linear,
    FalloffShape::EaseIn,
    FalloffShape::EaseOut,
    FalloffShape::Smooth,
    FalloffShape::CUSTOM,
];

/// Why In and Out are greyed (FO-12, UIC-117).
pub(crate) const IN_OUT_UNUSED: &str = "Only used when Shape is Custom";

/// FO-30a: the factory In and Out, shown greyed under other shapes.
/// Picking Custom starts from `FalloffShape::CUSTOM` (FAL-M-03).
const FACTORY_IN_OUT: (f64, f64) = (0., 0.);

/// FO-13 / FO-Q09: one label, "Shape", on every falloff (and Tweak's
/// Range). Approved bible deviation (Jason, 2026-11-15); the reference
/// says "Shape Preset" on the geometric falloffs.
pub(crate) const SHAPE: &str = "Shape";

/// `label` (FO-13), then In and Out, as Tool Properties rows (FO-12,
/// FAL-L-03). In and Out are always shown and greyed unless the shape is
/// Custom; greyed, they show the factory values. `shown` is the shape the
/// form is built for.
pub(crate) fn shape_fields<P: 'static>(
    key: &str,
    label: &str,
    shown: FalloffShape,
    get: impl Fn(&P) -> FalloffShape + 'static,
    set: impl Fn(&mut P, FalloffShape) + 'static,
) -> Vec<tf::Field<P>> {
    let get = std::rc::Rc::new(get);
    let set = std::rc::Rc::new(set);
    // Custom carries its In/Out values, so entries match by kind, not value.
    let index = |s: FalloffShape| {
        SHAPES
            .iter()
            .position(|c| std::mem::discriminant(c) == std::mem::discriminant(&s))
            .unwrap_or(0)
    };
    let names: Vec<(usize, &str)> = SHAPES
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.label()))
        .collect();
    let (g, g1, s) = (get.clone(), get.clone(), set.clone());
    let mut fields = vec![
        tf::choice(
            &format!("{key}-preset"),
            label,
            &names,
            move |p| index(g(p)),
            move |p, i| {
                if index(g1(p)) != i {
                    s(p, SHAPES[i]);
                }
            },
        )
        .tip("How the weight ramps from full to none"),
    ];
    let custom = matches!(shown, FalloffShape::Custom { .. });
    for (suffix, label) in [("in", "In"), ("out", "Out")] {
        let (g, s) = (get.clone(), set.clone());
        let read = move |p: &P| {
            let (ease_in, ease_out) = match g(p) {
                FalloffShape::Custom { ease_in, ease_out } => (ease_in, ease_out),
                _ => FACTORY_IN_OUT,
            };
            if suffix == "in" { ease_in } else { ease_out }
        };
        let g2 = get.clone();
        let tip = if suffix == "in" {
            "Strength near the start of the ramp. Which value is In and which is Out is provisional (FAL-Q02)"
        } else {
            "Strength near the end of the ramp. Which value is In and which is Out is provisional (FAL-Q02)"
        };
        fields.push(
            tf::number(
                &format!("{key}_{suffix}"),
                label,
                Unit::Scalar,
                read,
                move |p, v| {
                    if let FalloffShape::Custom {
                        mut ease_in,
                        mut ease_out,
                    } = g2(p)
                    {
                        if suffix == "in" {
                            ease_in = v;
                        } else {
                            ease_out = v;
                        }
                        s(p, FalloffShape::Custom { ease_in, ease_out });
                    }
                    Ok(())
                },
            )
            .suffix("")
            .tip(tip)
            .enabled_if(custom, IN_OUT_UNUSED),
        );
    }
    fields
}

/// The falloff stack as Tool Properties edits it: layer 0 is the active
/// falloff with its own Mix Mode, then the layers added on top.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FalloffP {
    pub(super) layers: Vec<(Falloff, MixMode)>,
}

impl FalloffP {
    pub(super) fn of(editor: &operandi_core::Editor) -> Option<Self> {
        let first = editor.spatial_falloff?;
        let mut layers = vec![(first, editor.spatial_falloff_mix)];
        layers.extend(editor.added_falloffs.iter().copied());
        Some(Self { layers })
    }
}

/// The section title of a layer (FO-10: `<Type> Falloff`). Sections keep
/// their fold state per title, so layers of one type fold together.
pub(super) fn section_name(f: Falloff) -> &'static str {
    match f {
        Falloff::Linear(_) => "Linear Falloff",
        Falloff::Radial(_) => "Radial Falloff",
        Falloff::Cylinder(_) => "Cylinder Falloff",
        Falloff::VertexMap(_) => "Vertex Map Falloff",
        Falloff::Selection(_) => "Selection Falloff",
        Falloff::SoftSelection(_) => "Soft Selection Falloff",
        Falloff::Element(_) => "Element Falloff",
        Falloff::Coplanar(_) => "Coplanar Falloff",
        Falloff::Screen(_) => "Screen Falloff",
        Falloff::Lasso(_) => "Lasso Falloff",
        Falloff::Airbrush(_) => "Airbrush Falloff",
        Falloff::Noise(_) => "Noise Falloff",
        Falloff::Image(_) => "Image Falloff",
        Falloff::Path(_) => "Path Falloff",
        Falloff::Curvature(_) => "Curvature Falloff",
        Falloff::Invert => "Invert Falloff",
    }
}

/// Layer `i` of the stack (it exists while the form built for it is shown).
fn layer(p: &FalloffP, i: usize) -> Falloff {
    p.layers.get(i).map_or(Falloff::Invert, |l| l.0)
}
fn layer_mut(p: &mut FalloffP, i: usize) -> Option<&mut Falloff> {
    p.layers.get_mut(i).map(|l| &mut l.0)
}

fn shape_of(f: Falloff) -> FalloffShape {
    match f {
        Falloff::Linear(l) => l.shape,
        Falloff::Radial(r) => r.shape,
        Falloff::Cylinder(c) => c.shape,
        Falloff::Selection(s) => s.shape,
        Falloff::SoftSelection(s) => s.shape,
        Falloff::Element(e) => e.shape,
        Falloff::Coplanar(c) => c.shape,
        Falloff::VertexMap(_)
        | Falloff::Invert
        | Falloff::Screen(_)
        | Falloff::Lasso(_)
        | Falloff::Airbrush(_)
        | Falloff::Noise(_)
        | Falloff::Image(_)
        | Falloff::Path(_)
        | Falloff::Curvature(_) => FalloffShape::default(),
    }
}
fn set_shape_of(f: &mut Falloff, shape: FalloffShape) {
    match f {
        Falloff::Linear(l) => l.shape = shape,
        Falloff::Radial(r) => r.shape = shape,
        Falloff::Cylinder(c) => c.shape = shape,
        Falloff::Selection(s) => s.shape = shape,
        Falloff::SoftSelection(s) => s.shape = shape,
        Falloff::Element(e) => e.shape = shape,
        Falloff::Coplanar(c) => c.shape = shape,
        Falloff::VertexMap(_)
        | Falloff::Invert
        | Falloff::Screen(_)
        | Falloff::Lasso(_)
        | Falloff::Airbrush(_)
        | Falloff::Noise(_)
        | Falloff::Image(_)
        | Falloff::Path(_)
        | Falloff::Curvature(_) => {}
    }
}

/// A vector row of layer `i`'s falloff.
fn layer_vector(
    i: usize,
    key: &str,
    label: &'static str,
    get: fn(&Falloff) -> Option<DVec3>,
    set: fn(&mut Falloff, usize, f64),
) -> Vec<tf::Field<FalloffP>> {
    tf::vector(
        &format!("falloff{i}_{key}"),
        label,
        Unit::Distance,
        move |p: &FalloffP| get(&layer(p, i)).unwrap_or_default(),
        move |p, a, v| {
            if !v.is_finite() {
                return Err(format!("{label} must be a number"));
            }
            if let Some(f) = layer_mut(p, i) {
                set(f, a, v);
            }
            Ok(())
        },
    )
}

/// Layer `i`'s shape rows.
fn layer_shape(i: usize, label: &str, shown: FalloffShape) -> Vec<tf::Field<FalloffP>> {
    shape_fields(
        &format!("falloff{i}_shape"),
        label,
        shown,
        move |p: &FalloffP| shape_of(layer(p, i)),
        move |p, s| {
            if let Some(f) = layer_mut(p, i) {
                set_shape_of(f, s);
            }
        },
    )
}

/// Show Vertex and Show Edge of a Soft Selection or Coplanar layer
/// (FO-47, FO-48): they draw on the first layer.
fn show_checks(i: usize) -> Vec<tf::Field<FalloffP>> {
    fn flags(f: &mut Falloff) -> Option<(&mut bool, &mut bool)> {
        match f {
            Falloff::SoftSelection(s) => Some((&mut s.show_vertex, &mut s.show_edge)),
            Falloff::Coplanar(c) => Some((&mut c.show_vertex, &mut c.show_edge)),
            _ => None,
        }
    }
    let read = move |p: &FalloffP, edge: bool| {
        let mut f = layer(p, i);
        flags(&mut f).is_some_and(|(v, e)| if edge { *e } else { *v })
    };
    let first = i == 0;
    let why = "Shown for the first falloff only";
    vec![
        tf::check(
            &format!("falloff{i}_show_vertex"),
            "Show Vertex",
            move |p: &FalloffP| read(p, false),
            move |p, on| {
                if let Some((v, _)) = layer_mut(p, i).and_then(flags) {
                    *v = on;
                }
            },
        )
        .tip("Tint the affected points by their weight")
        .enabled_if(first, why),
        tf::check(
            &format!("falloff{i}_show_edge"),
            "Show Edge",
            move |p: &FalloffP| read(p, true),
            move |p, on| {
                if let Some((_, e)) = layer_mut(p, i).and_then(flags) {
                    *e = on;
                }
            },
        )
        .tip("Tint the affected edges by their weight")
        .enabled_if(first, why),
    ]
}

/// Swap layer `i`'s falloff for `next` and publish the stack.
fn set_layer(app: &mut Operandi, i: usize, next: Falloff) {
    let Some(mut p) = FalloffP::of(&app.editor) else {
        return;
    };
    if let Some(f) = layer_mut(&mut p, i) {
        *f = next;
    }
    match FalloffFields::publish(app, p) {
        Ok(()) => app.clear_fields(),
        Err(e) => app.status = e,
    }
}

/// One layer's section rows, in the spec's order with Mix Mode last
/// (FO-11, §4).
fn layer_items(app: &Operandi, i: usize, f: Falloff) -> Vec<tf::Item<FalloffP>> {
    let mut fields: Vec<tf::Field<FalloffP>> = Vec::new();
    let mut buttons: Vec<tf::Button<FalloffP>> = Vec::new();
    let mut items_note: Option<String> = None;
    // FO-35.
    let world = || {
        tf::check(
            &format!("falloff{i}_world"),
            "Use World Transforms",
            move |p: &FalloffP| match layer(p, i) {
                Falloff::Image(m) => m.world,
                f => f.uses_world(),
            },
            move |p, on| match layer_mut(p, i) {
                Some(Falloff::Linear(l)) => l.world = on,
                Some(Falloff::Radial(r)) => r.world = on,
                Some(Falloff::Cylinder(c)) => c.world = on,
                Some(Falloff::Noise(n)) => n.world = on,
                Some(Falloff::Path(n)) => n.world = on,
                Some(Falloff::Image(m)) => m.world = on,
                _ => {}
            },
        )
        .tip("On: positions and weights are in world space, so touching meshes deform as one. Off: each mesh uses its own space, so copies deform alike")
    };
    // FO-05: the next press in the view draws this falloff.
    let draw = || {
        tf::Button::run("Draw in View", |app: &mut Operandi| {
            app.falloff_draw_armed = true;
            app.status = "Press and drag in the view to draw the falloff".into();
        })
        .tip("The next press in the view sets the falloff's origin; drag to size it")
    };
    match f {
        // FO-40.
        Falloff::Linear(l) => {
            fields.extend(layer_vector(
                i,
                "start",
                "Start",
                |x| match x {
                    Falloff::Linear(l) => Some(l.start),
                    _ => None,
                },
                |x, a, v| {
                    if let Falloff::Linear(l) = x {
                        l.start[a] = v;
                    }
                },
            ));
            fields.extend(layer_vector(
                i,
                "end",
                "End",
                |x| match x {
                    Falloff::Linear(l) => Some(l.end),
                    _ => None,
                },
                |x, a, v| {
                    if let Falloff::Linear(l) = x {
                        l.end[a] = v;
                    }
                },
            ));
            for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate() {
                buttons.push(
                    tf::Button::run(format!("Auto Size {name}"), move |app: &mut Operandi| {
                        let Some(Falloff::Linear(l)) =
                            FalloffP::of(&app.editor).map(|p| layer(&p, i))
                        else {
                            return;
                        };
                        match l.auto_size(&app.editor.scene, axis) {
                            Ok(sized) => set_layer(app, i, Falloff::Linear(sized)),
                            Err(e) => app.status = e,
                        }
                    })
                    .tip(format!(
                        "Fit Start and End to the selection's bounds along {name}"
                    )),
                );
            }
            buttons.push(
                tf::Button::change("Reverse", move |p: &mut FalloffP| {
                    if let Some(Falloff::Linear(l)) = layer_mut(p, i) {
                        *l = l.reversed();
                    }
                })
                .tip("Swap Start and End"),
            );
            fields.push(
                tf::choice(
                    &format!("falloff{i}_symmetric"),
                    "Symmetric",
                    &[
                        (FalloffSymmetric::None, "None"),
                        (FalloffSymmetric::Start, "Start"),
                        (FalloffSymmetric::End, "End"),
                    ],
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Linear(l) => l.symmetric,
                        _ => FalloffSymmetric::None,
                    },
                    move |p, s| {
                        if let Some(Falloff::Linear(l)) = layer_mut(p, i) {
                            l.symmetric = s;
                        }
                    },
                )
                .tip("Mirror the ramp about Start or End"),
            );
            fields.push(world());
            if i == 0 {
                buttons.push(draw());
            }
            fields.extend(layer_shape(i, SHAPE, l.shape));
        }
        // FO-41.
        Falloff::Radial(r) => {
            fields.extend(layer_vector(
                i,
                "center",
                "Center",
                |x| match x {
                    Falloff::Radial(r) => Some(r.center),
                    _ => None,
                },
                |x, a, v| {
                    if let Falloff::Radial(r) = x {
                        r.center[a] = v;
                    }
                },
            ));
            fields.extend(layer_vector(
                i,
                "size",
                "Size",
                |x| match x {
                    Falloff::Radial(r) => Some(r.size),
                    _ => None,
                },
                |x, a, v| {
                    if let Falloff::Radial(r) = x {
                        r.size[a] = v;
                    }
                },
            ));
            buttons.push(
                tf::Button::run("Auto Size", move |app: &mut Operandi| {
                    let Some(Falloff::Radial(r)) = FalloffP::of(&app.editor).map(|p| layer(&p, i))
                    else {
                        return;
                    };
                    match r.auto_size(&app.editor.scene) {
                        Ok(sized) => set_layer(app, i, Falloff::Radial(sized)),
                        Err(e) => app.status = e,
                    }
                })
                .tip("Fit the falloff to the selection's bounds"),
            );
            fields.push(world());
            if i == 0 {
                buttons.push(draw());
            }
            fields.extend(layer_shape(i, SHAPE, r.shape));
        }
        // FO-42.
        Falloff::Cylinder(c) => {
            fields.extend(layer_vector(
                i,
                "center",
                "Center",
                |x| match x {
                    Falloff::Cylinder(c) => Some(c.center),
                    _ => None,
                },
                |x, a, v| {
                    if let Falloff::Cylinder(c) = x {
                        c.center[a] = v;
                    }
                },
            ));
            fields.extend(layer_vector(
                i,
                "size",
                "Size",
                |x| match x {
                    Falloff::Cylinder(c) => Some(c.size),
                    _ => None,
                },
                |x, a, v| {
                    if let Falloff::Cylinder(c) = x {
                        c.size[a] = v;
                    }
                },
            ));
            fields.push(
                tf::choice(
                    &format!("falloff{i}_axis"),
                    "Axis",
                    &[(0usize, "X"), (1, "Y"), (2, "Z")],
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Cylinder(c) => c.axis,
                        _ => 1,
                    },
                    move |p, a| {
                        if let Some(Falloff::Cylinder(c)) = layer_mut(p, i) {
                            c.axis = a;
                        }
                    },
                )
                .tip("The direction the tube runs; Size along it is unused"),
            );
            buttons.push(
                tf::Button::run("Auto Size", move |app: &mut Operandi| {
                    let Some(Falloff::Cylinder(c)) =
                        FalloffP::of(&app.editor).map(|p| layer(&p, i))
                    else {
                        return;
                    };
                    match c.auto_size(&app.editor.scene) {
                        Ok(sized) => set_layer(app, i, Falloff::Cylinder(sized)),
                        Err(e) => app.status = e,
                    }
                })
                .tip("Fit the falloff to the selection's bounds"),
            );
            fields.push(world());
            if i == 0 {
                buttons.push(draw());
            }
            fields.extend(layer_shape(i, SHAPE, c.shape));
        }
        // FO-49 (weight maps; colour and morph maps are batch 5).
        Falloff::VertexMap(v) => {
            fields.push(
                tf::choice(
                    &format!("falloff{i}_vm_mode"),
                    "Mode",
                    &[
                        (VertexMapMode::Magnitude, "Magnitude"),
                        (VertexMapMode::Component, "Component"),
                    ],
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::VertexMap(v) => v.mode,
                        _ => VertexMapMode::Magnitude,
                    },
                    move |p, mode| {
                        if let Some(Falloff::VertexMap(v)) = layer_mut(p, i) {
                            v.mode = mode;
                            if mode == VertexMapMode::Magnitude {
                                v.index = 0;
                            }
                        }
                    },
                )
                .tip("Reads the selected weight map (Weight Map in the Falloff menu)"),
            );
            fields.push(
                tf::count(
                    &format!("falloff{i}_vm_index"),
                    "Index",
                    (0., 3.),
                    "Index must be a whole number from 0 to 3",
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::VertexMap(v) => v.index as f64,
                        _ => 0.,
                    },
                    move |p, n| {
                        if let Some(Falloff::VertexMap(v)) = layer_mut(p, i) {
                            v.index = n;
                        }
                    },
                )
                .tip("Which channel Component mode reads")
                .enabled_if(
                    v.mode == VertexMapMode::Component,
                    "Index applies in Component mode",
                ),
            );
        }
        // FO-46.
        Falloff::Selection(s) => {
            fields.push(
                tf::count(
                    &format!("falloff{i}_steps"),
                    "Steps",
                    (1., operandi_core::falloff::MAX_SELECTION_STEPS as f64),
                    "Steps must be a whole number from 1 to 1000",
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Selection(s) => s.steps as f64,
                        _ => 1.,
                    },
                    move |p, n| {
                        if let Some(Falloff::Selection(s)) = layer_mut(p, i) {
                            s.steps = n.max(1);
                        }
                    },
                )
                .tip("Rings the weight takes to rise from 0 on the selection's border to full. With a tool open, right-click in the view adds one; right-drag right for more, left for fewer"),
            );
            fields.extend(layer_shape(i, SHAPE, s.shape));
        }
        // FO-47.
        Falloff::SoftSelection(s) => {
            fields.push(
                tf::number(
                    &format!("falloff{i}_soft_radius"),
                    "Radius",
                    Unit::Distance,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::SoftSelection(s) => s.radius,
                        _ => 0.,
                    },
                    move |p, v| {
                        if !(v.is_finite() && v >= 0.) {
                            return Err("Radius must be zero or more".into());
                        }
                        if let Some(Falloff::SoftSelection(s)) = layer_mut(p, i) {
                            s.radius = v;
                        }
                        Ok(())
                    },
                )
                .tip("How far beyond the selection the effect reaches. With a tool open, right-drag in the view: right for more, left for less"),
            );
            fields.push(
                tf::check(
                    &format!("falloff{i}_soft_connect"),
                    "Use Connectivity",
                    move |p: &FalloffP| matches!(layer(p, i), Falloff::SoftSelection(s) if s.connectivity),
                    move |p, on| {
                        if let Some(Falloff::SoftSelection(s)) = layer_mut(p, i) {
                            s.connectivity = on;
                        }
                    },
                )
                .tip("Only points joined to the selection count, measured along the surface; off: straight-line distance"),
            );
            fields.extend(layer_shape(i, SHAPE, s.shape));
            fields.extend(show_checks(i));
        }
        // FO-45.
        Falloff::Element(e) => {
            let modes: Vec<(TweakMode, &str)> =
                TweakMode::ALL.iter().map(|&m| (m, m.label())).collect();
            fields.push(
                tf::choice(
                    &format!("falloff{i}_el_mode"),
                    "Element Mode",
                    &modes,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Element(e) => e.mode,
                        _ => TweakMode::Auto,
                    },
                    move |p, m| {
                        if let Some(Falloff::Element(e)) = layer_mut(p, i) {
                            e.mode = m;
                        }
                    },
                )
                .tip("Which kind of element a press picks; Auto takes the nearest"),
            );
            fields.push(
                tf::number(
                    &format!("falloff{i}_el_range"),
                    "Range",
                    Unit::Distance,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Element(e) => e.range,
                        _ => 0.,
                    },
                    move |p, v| {
                        if !(v.is_finite() && v >= 0.) {
                            return Err("Range must be zero or more".into());
                        }
                        if let Some(Falloff::Element(e)) = layer_mut(p, i) {
                            e.range = v;
                        }
                        Ok(())
                    },
                )
                .tip("Points within this distance of the picked element move too, less the farther they are")
                .enabled_if(
                    e.connect.uses_range(),
                    "Rigid Connections moves the whole piece; Range is unused",
                ),
            );
            let connects: Vec<(TweakConnect, &str)> =
                TweakConnect::ALL.iter().map(|&c| (c, c.label())).collect();
            fields.push(
                tf::choice(
                    &format!("falloff{i}_el_connect"),
                    "Connected Elements",
                    &connects,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Element(e) => e.connect,
                        _ => TweakConnect::Ignore,
                    },
                    move |p, c| {
                        if let Some(Falloff::Element(e)) = layer_mut(p, i) {
                            e.connect = c;
                        }
                    },
                )
                .tip("Ignore: by distance only. Use Connectivity: only the picked piece. Rigid Connections: the whole piece. Edge Loops: the loop through a picked edge"),
            );
            fields.push(
                tf::check(
                    &format!("falloff{i}_el_raycast"),
                    "Always Raycast",
                    move |p: &FalloffP| matches!(layer(p, i), Falloff::Element(e) if e.always_raycast),
                    move |p, on| {
                        if let Some(Falloff::Element(e)) = layer_mut(p, i) {
                            e.always_raycast = on;
                        }
                    },
                )
                .tip("Pick on the surface even in wireframe views. Operandi always picks on the surface it draws, so this is stored only (provisional)"),
            );
            fields.extend(layer_shape(i, SHAPE, e.shape));
        }
        // FO-48.
        Falloff::Coplanar(c) => {
            fields.push(
                tf::number(
                    &format!("falloff{i}_cop_angle"),
                    "Angle",
                    Unit::Angle,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Coplanar(c) => c.angle,
                        _ => 0.,
                    },
                    move |p, v| {
                        if !(v.is_finite() && (0. ..=180.).contains(&v)) {
                            return Err("Angle must be from 0° to 180°".into());
                        }
                        if let Some(Falloff::Coplanar(c)) = layer_mut(p, i) {
                            c.angle = v;
                        }
                        Ok(())
                    },
                )
                .tip("Neighbouring polygons facing within this angle of the selection take part; 0° takes only those facing exactly the same way"),
            );
            fields.push(
                tf::check(
                    &format!("falloff{i}_cop_connect"),
                    "Use Connectivity",
                    move |p: &FalloffP| matches!(layer(p, i), Falloff::Coplanar(c) if c.connectivity),
                    move |p, on| {
                        if let Some(Falloff::Coplanar(c)) = layer_mut(p, i) {
                            c.connectivity = on;
                        }
                    },
                )
                .tip("Spread only across polygons joined to the selection; off: any polygon facing the same way"),
            );
            fields.extend(layer_shape(i, SHAPE, c.shape));
            fields.extend(show_checks(i));
        }
        // FO-43.
        Falloff::Screen(_) => {
            let px = |key: &'static str,
                      label: &'static str,
                      get: fn(ScreenFalloff) -> f64,
                      set: fn(&mut ScreenFalloff, f64)| {
                tf::number(
                    &format!("falloff{i}_{key}"),
                    label,
                    Unit::Suffix(" px"),
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Screen(s) => get(s),
                        _ => 0.,
                    },
                    move |p, v| {
                        if !v.is_finite() {
                            return Err(format!("{label} must be a number"));
                        }
                        if let Some(Falloff::Screen(s)) = layer_mut(p, i) {
                            set(s, v);
                        }
                        Ok(())
                    },
                )
            };
            fields.push(
                px("scr_cx", "Center X", |s| s.center.x, |s, v| s.center.x = v)
                    .tip("Pixels from the view's left edge"),
            );
            fields.push(
                px("scr_cy", "Center Y", |s| s.center.y, |s, v| s.center.y = v)
                    .tip("Pixels from the view's top edge"),
            );
            fields.push(
                px("scr_size", "Size", |s| s.size, |s, v| s.size = v.max(0.))
                    .tip("Radius of the circle on screen"),
            );
            fields.push(
                tf::check(
                    &format!("falloff{i}_scr_transparent"),
                    "Transparent",
                    move |p: &FalloffP| matches!(layer(p, i), Falloff::Screen(s) if s.transparent),
                    move |p, on| {
                        if let Some(Falloff::Screen(s)) = layer_mut(p, i) {
                            s.transparent = on;
                        }
                    },
                )
                .tip("Off: only points on polygons facing the view are affected"),
            );
            buttons.push(draw());
        }
        // FO-52.
        Falloff::Lasso(_) => {
            let styles: Vec<(LassoStyle, &str)> =
                LassoStyle::ALL.iter().map(|&s| (s, s.label())).collect();
            fields.push(
                tf::choice(
                    &format!("falloff{i}_lasso_style"),
                    "Style",
                    &styles,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Lasso(l) => l.style,
                        _ => LassoStyle::Rectangle,
                    },
                    move |p, s| {
                        if let Some(Falloff::Lasso(l)) = layer_mut(p, i) {
                            l.style = s;
                        }
                    },
                )
                .tip("The shape a drag draws: freehand, corner to corner, from the centre, or an ellipse in a box"),
            );
            fields.push(
                tf::number(
                    &format!("falloff{i}_lasso_soft"),
                    "Soft Border",
                    Unit::Suffix(" px"),
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Lasso(l) => l.soft_border,
                        _ => 0.,
                    },
                    move |p, v| {
                        if !(v.is_finite() && v >= 0.) {
                            return Err("Soft Border must be zero or more".into());
                        }
                        if let Some(Falloff::Lasso(l)) = layer_mut(p, i) {
                            l.soft_border = v;
                        }
                        Ok(())
                    },
                )
                .tip("Pixels over which the weight fades outside the shape"),
            );
            fields.push(
                tf::check(
                    &format!("falloff{i}_world"),
                    "Use World Transforms",
                    move |p: &FalloffP| matches!(layer(p, i), Falloff::Lasso(l) if l.world),
                    move |p, on| {
                        if let Some(Falloff::Lasso(l)) = layer_mut(p, i) {
                            l.world = on;
                        }
                    },
                )
                .tip("Stored. The shape's weights are fixed when it is drawn, so moving meshes afterwards changes nothing (provisional)"),
            );
            buttons.push(draw());
        }
        // FO-44.
        Falloff::Airbrush(_) => {
            fields.push(
                tf::number(
                    &format!("falloff{i}_air_size"),
                    "Size",
                    Unit::Suffix(" px"),
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Airbrush(a) => a.size,
                        _ => 0.,
                    },
                    move |p, v| {
                        if !(v.is_finite() && v >= 0.) {
                            return Err("Size must be zero or more".into());
                        }
                        if let Some(Falloff::Airbrush(a)) = layer_mut(p, i) {
                            a.size = v;
                        }
                        Ok(())
                    },
                )
                .tip(
                    "Brush radius on screen. Right-drag in the view: right for more, left for less",
                ),
            );
            fields.push(
                tf::number(
                    &format!("falloff{i}_air_strength"),
                    "Strength",
                    Unit::Percent,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Airbrush(a) => a.strength * 100.,
                        _ => 0.,
                    },
                    move |p, v| {
                        if !(v.is_finite() && (0. ..=100.).contains(&v)) {
                            return Err("Strength must be from 0 % to 100 %".into());
                        }
                        if let Some(Falloff::Airbrush(a)) = layer_mut(p, i) {
                            a.strength = v / 100.;
                        }
                        Ok(())
                    },
                )
                .tip("Weight each brush pass adds"),
            );
            fields.push(
                tf::check(
                    &format!("falloff{i}_air_transparent"),
                    "Transparent",
                    move |p: &FalloffP| matches!(layer(p, i), Falloff::Airbrush(a) if a.transparent),
                    move |p, on| {
                        if let Some(Falloff::Airbrush(a)) = layer_mut(p, i) {
                            a.transparent = on;
                        }
                    },
                )
                .tip("Off: only points on polygons facing the view are painted"),
            );
            buttons.push(
                tf::Button::run("Clear Paint", move |app: &mut Operandi| {
                    let kept = app
                        .editor
                        .falloff_stamps
                        .iter()
                        .filter(|((l, _), _)| *l != i)
                        .map(|(k, v)| (*k, v.as_ref().clone()))
                        .collect();
                    app.run(Command::FalloffStamps(kept));
                })
                .tip("Start the painted weights again from nothing"),
            );
        }
        // FO-50.
        Falloff::Noise(_) => {
            fields.push(
                tf::number(
                    &format!("falloff{i}_noise_scale"),
                    "Scale",
                    Unit::Distance,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Noise(n) => n.scale,
                        _ => 1.,
                    },
                    move |p, v| {
                        if !(v.is_finite() && v != 0.) {
                            return Err("Scale must be a number other than 0".into());
                        }
                        if let Some(Falloff::Noise(n)) = layer_mut(p, i) {
                            n.scale = v;
                        }
                        Ok(())
                    },
                )
                .tip("Size of the noise blobs"),
            );
            fields.push(world());
        }
        // FO-54.
        Falloff::Image(m) => {
            let name = app
                .falloff_image
                .as_ref()
                .and_then(|(p, _)| std::path::Path::new(p).file_name())
                .map_or("(none)".to_owned(), |n| n.to_string_lossy().into_owned());
            fields.push(
                tf::value(&format!("falloff{i}_image_name"), "Image", name)
                    .tip("The picture whose brightness is the weight"),
            );
            buttons.push(
                tf::Button::run("Browse…", |app: &mut Operandi| app.choose_falloff_image())
                    .tip("Choose the picture"),
            );
            let flag = |key: &'static str,
                        label: &'static str,
                        tip: &'static str,
                        get: fn(ImageFalloff) -> bool,
                        set: fn(&mut ImageFalloff, bool)| {
                tf::check(
                    &format!("falloff{i}_{key}"),
                    label,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Image(m) => get(m),
                        _ => false,
                    },
                    move |p, on| {
                        if let Some(Falloff::Image(m)) = layer_mut(p, i) {
                            set(m, on);
                        }
                    },
                )
                .tip(tip)
            };
            let num = |key: &'static str,
                       label: &'static str,
                       unit: Unit,
                       tip: &'static str,
                       get: fn(ImageFalloff) -> f64,
                       set: fn(&mut ImageFalloff, f64)| {
                tf::number(
                    &format!("falloff{i}_{key}"),
                    label,
                    unit,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Image(m) => get(m),
                        _ => 0.,
                    },
                    move |p, v| {
                        if !v.is_finite() {
                            return Err(format!("{label} must be a number"));
                        }
                        if let Some(Falloff::Image(m)) = layer_mut(p, i) {
                            set(m, v);
                        }
                        Ok(())
                    },
                )
                .tip(tip)
            };
            fields.push(
                flag(
                    "img_repeat",
                    "Repeat",
                    "Tile the picture; off, outside it weighs 0",
                    |m| m.repeat,
                    |m, v| m.repeat = v,
                )
                .enabled_if(!m.use_uv, "Repeat has no effect through a UV map"),
            );
            fields.push(num(
                "img_soft",
                "Soft Border",
                Unit::Percent,
                "Fade the picture's edges toward its centre",
                |m| m.soft_border,
                |m, v| m.soft_border = v.clamp(0., 100.),
            ));
            fields.push(flag("img_uv", "Use Selected UV Map", "Look the picture up through the mesh's selected UV map instead of laying it over the view", |m| m.use_uv, |m, v| m.use_uv = v));
            fields.push(num(
                "img_scale",
                "Scale",
                Unit::Percent,
                "Size of the picture",
                |m| m.scale,
                |m, v| m.scale = v.max(0.01),
            ));
            fields.push(num(
                "img_px",
                "Pos X",
                Unit::Scalar,
                "Offset across, in picture widths",
                |m| m.pos.x,
                |m, v| m.pos.x = v,
            ));
            fields.push(num(
                "img_py",
                "Pos Y",
                Unit::Scalar,
                "Offset down, in picture heights",
                |m| m.pos.y,
                |m, v| m.pos.y = v,
            ));
            fields.push(num(
                "img_rot",
                "Rotation",
                Unit::Angle,
                "Turn the picture",
                |m| m.rotation,
                |m, v| m.rotation = v,
            ));
            fields.push(world());
        }
        // FO-53.
        Falloff::Path(_) => {
            fields.push(
                tf::number(
                    &format!("falloff{i}_path_size"),
                    "Size",
                    Unit::Distance,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Path(f) => f.size,
                        _ => 1.,
                    },
                    move |p, v| {
                        if !(v.is_finite() && v >= 0.) {
                            return Err("Size must be zero or more".into());
                        }
                        if let Some(Falloff::Path(f)) = layer_mut(p, i) {
                            f.size = v;
                        }
                        Ok(())
                    },
                )
                .tip("Radius of the tube around the curve"),
            );
            fields.push(world());
            let editing = app.falloff_path_editing;
            buttons.push(
                tf::Button::run(
                    if editing {
                        "Stop Adding Points"
                    } else {
                        "Add Points"
                    },
                    |app: &mut Operandi| {
                        app.falloff_path_editing = !app.falloff_path_editing;
                        app.status = if app.falloff_path_editing {
                            "Click in the view to add path points; drag a point to move it".into()
                        } else {
                            "Path editing off".into()
                        };
                    },
                )
                .tip("While on, each click in the view adds a point to the curve"),
            );
            buttons.push(
                tf::Button::run("Clear Path", |app: &mut Operandi| {
                    app.run(Command::FalloffPath(Vec::new()))
                })
                .tip("Remove every path point"),
            );
            items_note = Some(format!(
                "{} path point(s). Drag a point to move it.",
                app.editor.falloff_path.len()
            ));
        }
        // FO-51.
        Falloff::Curvature(_) => {
            fields.push(
                tf::number(
                    &format!("falloff{i}_curv_scale"),
                    "Scale",
                    Unit::Scalar,
                    move |p: &FalloffP| match layer(p, i) {
                        Falloff::Curvature(f) => f.scale,
                        _ => 1.,
                    },
                    move |p, v| {
                        if !v.is_finite() {
                            return Err("Scale must be a number".into());
                        }
                        if let Some(Falloff::Curvature(f)) = layer_mut(p, i) {
                            f.scale = v;
                        }
                        Ok(())
                    },
                )
                .tip("How strongly bumps (up) and dips (down) move away from half weight; negative swaps them"),
            );
        }
        // FO-33: no options and no Mix Mode.
        Falloff::Invert => {}
    }
    if !matches!(f, Falloff::Invert) {
        let tip = if i == 0 {
            "How this falloff combines with the ones below it. On the first falloff it has no effect (FO-32)"
        } else {
            "How this falloff combines with the result of the ones before it"
        };
        let mixes: Vec<(MixMode, &str)> = MixMode::ALL.iter().map(|&m| (m, m.label())).collect();
        fields.push(
            tf::choice(
                &format!("falloff{i}_mix"),
                "Mix Mode",
                &mixes,
                move |p: &FalloffP| p.layers.get(i).map_or(MixMode::Multiply, |l| l.1),
                move |p, m| {
                    if let Some(l) = p.layers.get_mut(i) {
                        l.1 = m;
                    }
                },
            )
            .tip(tip),
        );
    }
    let mut items: Vec<tf::Item<FalloffP>> = fields
        .into_iter()
        .map(|f| tf::Item::Field(Box::new(f)))
        .collect();
    // UIC-116: panel buttons fill the field column, one per row.
    items.extend(buttons.into_iter().map(tf::Item::Wide));
    // A deform preset's own falloff can be fitted again (DEFORM §2); a
    // Selection falloff already follows the selection.
    if i == 0
        && app
            .editor
            .session
            .preset
            .as_ref()
            .and_then(|p| p.row().falloff_kind())
            .is_some_and(|k| {
                use operandi_core::tool_preset::FalloffKind as K;
                matches!(k, K::Linear | K::Radial | K::Cylinder)
            })
    {
        items.push(tf::Item::Wide(
            tf::Button::run("Fit to Selection", |app: &mut Operandi| {
                let Some(row) = app.editor.session.preset.as_ref().map(|p| p.row()) else {
                    return;
                };
                match row.fitted_falloff(&app.editor.scene) {
                    Ok(Some(fitted)) => app.run(Command::SpatialFalloff(Some(fitted))),
                    Ok(None) => {}
                    Err(e) => app.status = e,
                }
            })
            .tip("Fit the falloff to the selection again"),
        ));
    }
    if let Some(note) = items_note {
        items.push(tf::Item::Note(note));
    }
    match f {
        Falloff::Invert => items.push(tf::Item::Note(
            "No options: inverts the weight of the falloffs before it.".into(),
        )),
        Falloff::VertexMap(_) => items.push(tf::Item::Note(
            "Reads the selected weight map; no handles.".into(),
        )),
        _ => {}
    }
    if i > 0 {
        items.push(tf::Item::Wide(
            tf::Button::change("Remove", move |p: &mut FalloffP| {
                if i < p.layers.len() {
                    p.layers.remove(i);
                }
            })
            .tip("Take this falloff off the stack"),
        ));
    }
    items
}

/// Tool Properties: one section per falloff layer, in stack order, under
/// the active tool's rows, or alone when no tool is open (FO-10, FO-16).
pub(super) struct FalloffFields;

impl ToolFields for FalloffFields {
    type P = FalloffP;
    fn params(app: &Operandi) -> Option<FalloffP> {
        FalloffP::of(&app.editor)
    }
    fn form(app: &Operandi, p: &FalloffP) -> ToolForm<FalloffP> {
        let mut f = ToolForm::new();
        for (i, &(falloff, _)) in p.layers.iter().enumerate() {
            f.section(section_name(falloff), false, layer_items(app, i, falloff));
        }
        f.note("Esc in the view clears the falloff when no tool edit is in progress.");
        f
    }
    fn publish(app: &mut Operandi, p: FalloffP) -> Result<()> {
        let Some(&(first, mix)) = p.layers.first() else {
            return app.editor.dispatch(Command::SpatialFalloff(None));
        };
        for (f, _) in &p.layers {
            f.validate()?;
        }
        if app.editor.spatial_falloff != Some(first) {
            app.editor.dispatch(Command::SpatialFalloff(Some(first)))?;
        }
        if app.editor.spatial_falloff_mix != mix {
            app.editor.dispatch(Command::SpatialFalloffMix(mix))?;
        }
        let added = p.layers[1..].to_vec();
        if app.editor.added_falloffs != added {
            app.editor.dispatch(Command::AddedFalloffs(added))?;
        }
        Ok(())
    }
    fn published(app: &mut Operandi) {
        // FO-43/52: Screen and Lasso weights follow their options.
        app.restamp_screen_falloffs();
        app.status = "Falloff changed".into();
    }
}

/// The first Selection layer of the stack and where it is (0 = the active
/// falloff), for the right-click Steps gesture (FO-46).
pub(super) fn selection_layer(editor: &operandi_core::Editor) -> Option<(usize, SelectionFalloff)> {
    let p = FalloffP::of(editor)?;
    p.layers.iter().enumerate().find_map(|(i, (f, _))| match f {
        Falloff::Selection(s) => Some((i, *s)),
        _ => None,
    })
}

/// The first Soft Selection layer of the stack and where it is, for the
/// right-drag Radius gesture (FO-47, FO-Q05).
pub(super) fn soft_selection_layer(
    editor: &operandi_core::Editor,
) -> Option<(usize, SoftSelectionFalloff)> {
    let p = FalloffP::of(editor)?;
    p.layers.iter().enumerate().find_map(|(i, (f, _))| match f {
        Falloff::SoftSelection(s) => Some((i, *s)),
        _ => None,
    })
}

/// Set Soft Selection layer `i`'s Radius (the right-drag gesture).
pub(super) fn set_soft_selection_radius(app: &mut Operandi, i: usize, radius: f64) {
    if let Some((_, mut s)) = soft_selection_layer(&app.editor).filter(|(at, _)| *at == i)
        && s.radius != radius
    {
        s.radius = radius;
        set_layer(app, i, Falloff::SoftSelection(s));
    }
}

/// Set Selection layer `i`'s Steps (the right-click gesture).
pub(super) fn set_selection_steps(app: &mut Operandi, i: usize, steps: u32) {
    if let Some((_, mut s)) = selection_layer(&app.editor).filter(|(at, _)| *at == i) {
        s.steps = steps;
        set_layer(app, i, Falloff::Selection(s));
    }
}

/// FO-04: Cylinder chosen while a tool is open fits the selection's
/// bounding box (Linear and Radial keep their last settings, FAL-M-03).
pub(super) fn fitted_on_choose(editor: &operandi_core::Editor, f: Falloff) -> Falloff {
    match f {
        Falloff::Cylinder(c) if editor.session.live.is_some() => {
            c.auto_size(&editor.scene).map_or(f, Falloff::Cylinder)
        }
        _ => f,
    }
}

impl Operandi {
    /// The context-bar Falloff menu: choose a type (replacing the stack),
    /// Add one on top, or Clear Falloffs (FO-15, FO-20..23). Options are
    /// in Tool Properties (FO-Q10).
    pub(super) fn falloff_menu(&mut self, ui: &mut Ui) {
        self.menu_owned_input = true;
        let enabled = self
            .editor
            .session
            .live
            .as_ref()
            .is_none_or(|l| l.tool.card().layers.falloff);
        if !enabled {
            ui.weak("Drop the current tool to change the falloff.");
        }
        ui.gated_ui(
            crate::unavailable::unless(enabled, "The active tool has no falloff; drop it first"),
            |ui| {
                let current = self.editor.spatial_falloff;
                let mut next = current;
                ui.set_min_width(180.);
                // FO-20, the built types (batch 1).
                if ui.selectable_label(current.is_none(), "(none)").clicked() {
                    next = None;
                }
                let kinds: [(Falloff, &str); 16] = [
                    (
                        self.editor.linear_falloff_preset.into(),
                        "A ramp from Start (full) to End (none)",
                    ),
                    (
                        Falloff::Cylinder(CylinderFalloff::default()),
                        "Full along a tube's axis, none at its wall",
                    ),
                    (
                        self.editor.radial_falloff_preset.into(),
                        "Full at the centre, none at the edge of an ellipsoid",
                    ),
                    (
                        Falloff::Airbrush(AirbrushFalloff::default()),
                        "Paint the falloff by dragging over the mesh; it builds up as you scrub",
                    ),
                    (
                        Falloff::Screen(ScreenFalloff::default()),
                        "A circle on screen: press for its centre, drag for its size",
                    ),
                    (
                        Falloff::Element(ElementFalloff::default()),
                        "Press on a vertex, edge or polygon and drag it; Range spreads the move around it",
                    ),
                    (
                        Falloff::Noise(NoiseFalloff::default()),
                        "Smooth 3D noise: roughs up a surface with Move",
                    ),
                    (
                        Falloff::Curvature(CurvatureFalloff::default()),
                        "Weights from how sharply the surface bends",
                    ),
                    (
                        Falloff::VertexMap(VertexMapFalloff::default()),
                        "Weights from the selected weight map (VMF-01)",
                    ),
                    (
                        Falloff::Selection(SelectionFalloff::default()),
                        "Weights from the component selection: 0 on its border, rising inward",
                    ),
                    (
                        Falloff::SoftSelection(SoftSelectionFalloff::default()),
                        "The selection at full weight, fading out over Radius beyond it",
                    ),
                    (
                        Falloff::Lasso(LassoFalloff::default()),
                        "A shape drawn on screen; inside is full, outside none",
                    ),
                    (
                        Falloff::Path(PathFalloff::default()),
                        "A tube around a curve: click in the view to add its points",
                    ),
                    (
                        Falloff::Image(ImageFalloff::default()),
                        "A picture's brightness, laid over the view or through the UV map",
                    ),
                    (
                        Falloff::Coplanar(CoplanarFalloff::default()),
                        "The selection plus neighbouring polygons facing within Angle of it",
                    ),
                    (
                        Falloff::Invert,
                        "1 − the weight of the falloffs before it (VMF-12)",
                    ),
                ];
                for (kind, tip) in kinds {
                    let on = current.is_some_and(|c| {
                        std::mem::discriminant(&c) == std::mem::discriminant(&kind)
                    });
                    if ui
                        .selectable_label(on, kind.label())
                        .on_hover_text(tip)
                        .clicked()
                        && !on
                    {
                        next = Some(kind);
                    }
                }
                if next != current {
                    // FO-04: a geometric falloff chosen with a tool open
                    // fits the selection; FO-05: the next press draws it.
                    let next = next.map(|f| fitted_on_choose(&self.editor, f));
                    self.run(Command::SpatialFalloff(next));
                    self.falloff_draw_armed = next.is_some_and(|f| {
                        f.has_handles() || matches!(f, Falloff::Screen(_) | Falloff::Lasso(_))
                    });
                    // FO-53: a Path starts in point-adding mode.
                    self.falloff_path_editing = matches!(next, Some(Falloff::Path(_)));
                    // FO-54: an Image falloff stamps at once.
                    self.restamp_screen_falloffs();
                }
                ui.separator();
                // FO-21, the built types in the spec's order.
                let has = current.is_some();
                let none = crate::unavailable::unless(
                    has,
                    "Choose a falloff first; added ones stack on top",
                );
                ui.gated_ui(none, |ui| {
                    ui.menu_button("Add", |ui| {
                        for f in self.add_choices() {
                            if ui.button(f.label()).clicked() {
                                let mut added = self.editor.added_falloffs.clone();
                                added.push((f, MixMode::Multiply));
                                self.run(Command::AddedFalloffs(added));
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Stack another falloff on top, with its own Mix Mode");
                });
                // FO-22.
                let clicked = ui
                    .gated(
                        crate::unavailable::unless(has, "No falloff is active"),
                        egui::Button::new("Clear Falloffs"),
                    )
                    .on_hover_text("Remove every falloff layer")
                    .clicked();
                if clicked {
                    self.run(Command::SpatialFalloff(None));
                    ui.close();
                }
                ui.separator();
                self.vertex_map_row(ui, true);
            },
        );
        ui.weak("Options are in Tool Properties.");
    }

    /// FO-21: what Add offers, built types only, in the spec's order.
    pub(super) fn add_choices(&self) -> [Falloff; 16] {
        [
            Falloff::Invert,
            Falloff::from(self.editor.linear_falloff_preset),
            Falloff::Cylinder(CylinderFalloff::default()),
            Falloff::from(self.editor.radial_falloff_preset),
            Falloff::Airbrush(AirbrushFalloff::default()),
            Falloff::Screen(ScreenFalloff::default()),
            Falloff::Element(ElementFalloff::default()),
            Falloff::Noise(NoiseFalloff::default()),
            Falloff::Curvature(CurvatureFalloff::default()),
            Falloff::VertexMap(VertexMapFalloff::default()),
            Falloff::Path(PathFalloff::default()),
            Falloff::Lasso(LassoFalloff::default()),
            Falloff::Image(ImageFalloff::default()),
            Falloff::Selection(SelectionFalloff::default()),
            // FO-21 (proposed): Soft Selection and Coplanar can stack too.
            Falloff::SoftSelection(SoftSelectionFalloff::default()),
            Falloff::Coplanar(CoplanarFalloff::default()),
        ]
    }

    /// The first Element layer of the stack (FO-45), whose options the
    /// press-to-pick uses.
    pub(super) fn element_falloff(&self) -> Option<ElementFalloff> {
        FalloffP::of(&self.editor)?
            .layers
            .iter()
            .find_map(|(f, _)| match f {
                Falloff::Element(e) => Some(*e),
                _ => None,
            })
    }

    /// Esc over a view with no tool edit in progress clears the falloff
    /// (FO-02), as it does Constrain to Background.
    pub(super) fn escape_clears_falloff(&self) -> bool {
        self.editor.spatial_falloff.is_some()
            && !self.editor.session.live.as_ref().is_some_and(|l| l.applied)
            && self
                .editor
                .session
                .pen_draft
                .as_ref()
                .is_none_or(|d| d.points().is_empty())
            && self.draw.is_none()
            && self.utility.is_none()
            && self.sculpt.brush.is_none()
    }

    /// VM-02: the selected weight map of the active mesh, which the Vertex
    /// Map falloff reads and the Vertex Map Tools paint.
    pub(super) fn weight_map_row(&mut self, ui: &mut Ui) {
        self.vertex_map_row(ui, false);
    }

    /// The map picker; `all_kinds` also lists morph and colour maps.
    fn vertex_map_row(&mut self, ui: &mut Ui, all_kinds: bool) {
        // FO-49: the Vertex Map falloff also reads morph and colour maps,
        // listed after the weight maps with their kind.
        let names: Vec<(String, &str)> = self
            .editor
            .scene
            .active()
            .map(|i| {
                let a = &i.mesh.attributes;
                a.point_weights
                    .keys()
                    .map(|n| (n.clone(), ""))
                    .chain(
                        a.relative_morphs
                            .keys()
                            .filter(|_| all_kinds)
                            .map(|n| (n.clone(), " (morph)")),
                    )
                    .chain(
                        a.corner_colors
                            .keys()
                            .filter(|_| all_kinds)
                            .map(|n| (n.clone(), " (colour)")),
                    )
                    .collect()
            })
            .unwrap_or_default();
        let current = self.editor.selected_weight_map.clone();
        let shown = current.clone().unwrap_or_else(|| "(none)".into());
        let mut next = current.clone();
        option_row::dropdown(ui, "weight-map", "Weight Map", &shown, |ui| {
            if ui.selectable_label(current.is_none(), "(none)").clicked() {
                next = None;
            }
            for (n, kind) in &names {
                if ui
                    .selectable_label(current.as_deref() == Some(n), format!("{n}{kind}"))
                    .clicked()
                {
                    next = Some(n.clone());
                }
            }
        });
        if next != current {
            self.run(Command::SelectWeightMap(next));
        }
    }
}
