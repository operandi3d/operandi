//! Tool panels for Torus, Capsule, Arc and Tube (NPR). Numeric fields use
//! the shared draft text (confirmed on Enter or leaving the field); linked
//! sizes are resolved by the parameter setters in operandi-core, and every
//! field is refreshed from the accepted values so links show at once.
use super::*;

impl Operandi {
    /// Start (or, on the same tool, drop) one of the four primitives.
    pub(super) fn arm_shape(&mut self, kind: ShapeKind) {
        let before = self.editor.session.shape.as_ref().map(|s| s.kind());
        self.run(Command::ArmShape(kind));
        // A new Tube path has no current point yet (DPM-08).
        self.point_modes.tube_current = None;
        if self.editor.session.shape.as_ref().map(|s| s.kind()) != before {
            self.clear_fields();
        }
    }
}

use crate::form::Unit;
use crate::primitive_panel::{plain, plain_vector};
use crate::tool_form::{self as tf, Button, ToolFields, ToolForm};

type P = ShapeParameters;

/// Field keys: one prefix per tool so drafts never cross tools.
fn prefix(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Torus => "tor_",
        ShapeKind::Capsule => "cap_",
        ShapeKind::Arc => "arc_",
        ShapeKind::Tube => "tub_",
        ShapeKind::Text => "txt_",
        ShapeKind::Teapot => "tea_",
    }
}

/// Reads one shape's value (the default for other shapes).
macro_rules! get {
    ($shape:ident, |$t:ident| $e:expr) => {
        |p: &P| match p {
            ShapeParameters::$shape($t) => $e,
            _ => Default::default(),
        }
    };
}
/// Writes one shape's value.
macro_rules! put {
    ($shape:ident, |$t:ident, $v:ident| $e:expr) => {
        |p: &mut P, $v| {
            if let ShapeParameters::$shape($t) = p {
                $e;
            }
        }
    };
    ($shape:ident, |$t:ident, $a:ident, $v:ident| $e:expr) => {
        |p: &mut P, $a: usize, $v: f64| {
            if let ShapeParameters::$shape($t) = p {
                $e;
            }
        }
    };
}

/// A whole number of zero or more.
fn whole(
    key: &str,
    label: &str,
    get: impl Fn(&P) -> u32 + 'static,
    set: impl Fn(&mut P, u32) + 'static,
) -> tf::Field<P> {
    tf::count(
        key,
        label,
        (0., f64::from(u32::MAX)),
        "Enter a whole number",
        move |p| f64::from(get(p)),
        set,
    )
}

fn make_uvs(key: &str) -> tf::Field<P> {
    tf::check(
        key,
        "Make UVs",
        |p: &P| match p {
            ShapeParameters::Torus(t) => t.make_uvs,
            ShapeParameters::Capsule(c) => c.make_uvs,
            ShapeParameters::Tube(t) => t.make_uvs,
            ShapeParameters::Text(t) => t.make_uvs,
            ShapeParameters::Teapot(t) => t.make_uvs,
            ShapeParameters::Arc(_) => false,
        },
        |p, on| match p {
            ShapeParameters::Torus(t) => t.make_uvs = on,
            ShapeParameters::Capsule(c) => c.make_uvs = on,
            ShapeParameters::Tube(t) => t.make_uvs = on,
            ShapeParameters::Text(t) => t.make_uvs = on,
            ShapeParameters::Teapot(t) => t.make_uvs = on,
            ShapeParameters::Arc(_) => {}
        },
    )
}

fn axis() -> tf::Field<P> {
    tf::axis(
        |p: &P| match p {
            ShapeParameters::Torus(t) => t.axis,
            ShapeParameters::Capsule(c) => c.axis,
            ShapeParameters::Text(t) => t.axis,
            ShapeParameters::Teapot(t) => t.axis,
            _ => 1,
        },
        |p, a| match p {
            ShapeParameters::Torus(t) => t.axis = a,
            ShapeParameters::Capsule(c) => c.axis = a,
            ShapeParameters::Text(t) => t.axis = a,
            ShapeParameters::Teapot(t) => t.axis = a,
            _ => {}
        },
    )
}

/// Teapot (missing-tools-g2 MT6): Position is the bounding box's centre,
/// Size its longest side.
fn teapot(f: &mut ToolForm<P>) {
    f.fields_from(plain_vector(
        "tea_position",
        "Position",
        get!(Teapot, |t| t.position),
        put!(Teapot, |t, a, v| t.position[a] = v),
    ))
    .field(plain(
        "tea_size",
        "Size",
        Unit::Distance,
        get!(Teapot, |t| t.size),
        put!(Teapot, |t, v| t.size = v),
    ))
    .separator()
    .field(axis())
    .field(make_uvs("tea-uvs"));
}

fn torus(f: &mut ToolForm<P>) {
    f.fields_from(plain_vector(
        "tor_position",
        "Position",
        get!(Torus, |t| t.position),
        put!(Torus, |t, a, v| t.position[a] = v),
    ))
    .fields_from(plain_vector(
        "tor_radius",
        "Radius",
        get!(Torus, |t| t.radius()),
        put!(Torus, |t, a, v| t.set_radius(a, v)),
    ))
    .separator()
    .field(whole(
        "tor_sides",
        "Sides",
        get!(Torus, |t| t.sides),
        put!(Torus, |t, n| t.sides = n),
    ))
    .field(whole(
        "tor_segments",
        "Segments",
        get!(Torus, |t| t.segments),
        put!(Torus, |t, n| t.segments = n),
    ))
    .field(plain(
        "tor_ring",
        "Ring Radius",
        Unit::Distance,
        get!(Torus, |t| t.ring_radius),
        put!(Torus, |t, v| t.ring_radius = v),
    ))
    .field(plain(
        "tor_xsec",
        "Cross Section",
        Unit::Distance,
        get!(Torus, |t| t.cross_section),
        put!(Torus, |t, v| t.cross_section = v),
    ))
    .field(plain(
        "tor_height",
        "Height",
        Unit::Distance,
        get!(Torus, |t| t.height),
        put!(Torus, |t, v| t.height = v),
    ))
    .field(plain(
        "tor_hole",
        "Hole Size",
        Unit::Scalar,
        get!(Torus, |t| t.hole()),
        put!(Torus, |t, v| t.set_hole(v)),
    ))
    .separator()
    .field(plain(
        "tor_bulge_top",
        "Bulge Top",
        Unit::Scalar,
        get!(Torus, |t| t.bulge_top),
        put!(Torus, |t, v| t.bulge_top = v),
    ))
    .field(plain(
        "tor_bulge_side",
        "Side",
        Unit::Scalar,
        get!(Torus, |t| t.bulge_side),
        put!(Torus, |t, v| t.bulge_side = v),
    ))
    .separator()
    .field(axis())
    .field(make_uvs("tor-uvs"));
}

fn capsule(f: &mut ToolForm<P>) {
    f.fields_from(plain_vector(
        "cap_position",
        "Position",
        get!(Capsule, |c| c.position),
        put!(Capsule, |c, a, v| c.position[a] = v),
    ))
    .fields_from(plain_vector(
        "cap_radius",
        "Radius",
        get!(Capsule, |c| c.radius),
        put!(Capsule, |c, a, v| c.radius[a] = v),
    ))
    .separator()
    .field(whole(
        "cap_sides",
        "Sides",
        get!(Capsule, |c| c.sides),
        put!(Capsule, |c, n| c.sides = n),
    ))
    .field(whole(
        "cap_segments",
        "Segments",
        get!(Capsule, |c| c.segments),
        put!(Capsule, |c, n| c.segments = n),
    ))
    .field(whole(
        "cap_end_segments",
        "End Segments",
        get!(Capsule, |c| c.end_segments),
        put!(Capsule, |c, n| c.end_segments = n),
    ))
    .field(plain(
        "cap_end_size",
        "End Size",
        Unit::Distance,
        get!(Capsule, |c| c.end_size),
        put!(Capsule, |c, v| c.end_size = v),
    ))
    .field(
        tf::choice(
            "cap-ends",
            "Ends",
            &[
                (CapsuleEnds::Polar, "Polar"),
                (CapsuleEnds::QuadGrid, "Quad Grid"),
            ],
            |p: &P| match p {
                ShapeParameters::Capsule(c) => c.ends,
                _ => CapsuleEnds::Polar,
            },
            put!(Capsule, |c, e| c.ends = e),
        )
        // Spec: NPR-Q04
        .tip("Quad Grid needs Sides in multiples of 4; other counts use Polar ends for now"),
    )
    .field(plain(
        "cap_bulge",
        "Bulge",
        Unit::Scalar,
        get!(Capsule, |c| c.bulge),
        put!(Capsule, |c, v| c.bulge = v),
    ))
    .separator()
    .field(axis())
    .field(make_uvs("cap-uvs"));
}

fn arc(f: &mut ToolForm<P>) {
    f.fields_from(plain_vector(
        "arc_center",
        "Center",
        get!(Arc, |a| a.center),
        put!(Arc, |arc, a, v| {
            let mut c = arc.center;
            c[a] = v;
            arc.set_center(c)
        }),
    ))
    .fields_from(plain_vector(
        "arc_start",
        "Start",
        get!(Arc, |a| a.start()),
        put!(Arc, |arc, a, v| {
            let mut s = arc.start();
            s[a] = v;
            arc.set_start(s)
        }),
    ))
    .fields_from(plain_vector(
        "arc_end",
        "End",
        get!(Arc, |a| a.end()),
        put!(Arc, |arc, a, v| {
            let mut e = arc.end();
            e[a] = v;
            arc.set_end(e)
        }),
    ))
    .separator()
    .field(plain(
        "arc_radius",
        "Radius",
        Unit::Distance,
        get!(Arc, |a| a.radius),
        put!(Arc, |a, v| a.radius = v),
    ))
    .field(plain(
        "arc_angle",
        "Angle",
        Unit::Angle,
        get!(Arc, |a| a.angle),
        put!(Arc, |a, v| a.set_angle(v)),
    ))
    .field(whole(
        "arc_segments",
        "Segments",
        get!(Arc, |a| a.segments),
        put!(Arc, |a, n| a.segments = n),
    ))
    .field(tf::check(
        "arc-reverse",
        "Reverse",
        get!(Arc, |a| a.reverse),
        put!(Arc, |a, b| a.reverse = b),
    ));
}

fn text(f: &mut ToolForm<P>) {
    let fonts = operandi_core::text::font_names();
    let options: Vec<(String, &str)> = fonts.iter().map(|n| (n.clone(), n.as_str())).collect();
    f.field(tf::text(
        "txt_text",
        "Text",
        get!(Text, |t| t.text.clone()),
        put!(Text, |t, s| t.text = s),
    ))
    // Font: shipped fonts first, then the installed ones.
    .field(
        tf::choice(
            "txt-font",
            "Font",
            &options,
            get!(Text, |t| t.font.clone()),
            put!(Text, |t, s| t.font = s),
        )
        .otherwise(""),
    )
    .field(plain(
        "txt_size",
        "Size",
        Unit::Distance,
        get!(Text, |t| t.size),
        put!(Text, |t, v| t.size = v),
    ))
    .field(plain(
        "txt_tracking",
        "Tracking",
        Unit::Distance,
        get!(Text, |t| t.tracking),
        put!(Text, |t, v| t.tracking = v),
    ))
    .fields_from(plain_vector(
        "txt_position",
        "Position",
        get!(Text, |t| t.position),
        put!(Text, |t, a, v| t.position[a] = v),
    ))
    .separator()
    .field(tf::segments(
        "txt-location",
        "Location",
        &[
            (TextLocation::Top, "Top"),
            (TextLocation::Middle, "Middle"),
            (TextLocation::Bottom, "Bottom"),
        ],
        |p: &P| match p {
            ShapeParameters::Text(t) => t.location,
            _ => TextLocation::Middle,
        },
        put!(Text, |t, l| t.location = l),
    ))
    .field(tf::segments(
        "txt-justification",
        "Justification",
        &[
            (TextJustification::Left, "Left"),
            (TextJustification::Center, "Center"),
            (TextJustification::Right, "Right"),
        ],
        |p: &P| match p {
            ShapeParameters::Text(t) => t.justification,
            _ => TextJustification::Left,
        },
        put!(Text, |t, j| t.justification = j),
    ))
    .field(axis())
    .field(tf::check(
        "txt-flip",
        "Flip",
        get!(Text, |t| t.flip),
        put!(Text, |t, b| t.flip = b),
    ))
    .field(make_uvs("txt-uvs"));
}

fn tube(f: &mut ToolForm<P>, t: &TubeParameters, current: Option<usize>) {
    // DPM-01/08/11: Mode first, as on the curve tools.
    f.push(crate::draw_tools::app_mode_row(
        |m| m.tube,
        |m, mode| m.tube = mode,
    ));
    // #44 (Steve Hill): as on the curve tools, the current point's
    // position only (the one last clicked in the view, else the last).
    let count = t.points.len();
    f.note(format!("Number of Points {count}"));
    if let Some(c) = current.filter(|&c| c < count).or(count.checked_sub(1)) {
        f.note(format!(
            "Current Point {} · click a point in the view to edit another",
            c + 1
        ));
        f.fields_from(plain_vector(
            "tub_current",
            "Position",
            move |p: &P| match p {
                ShapeParameters::Tube(t) => t.points.get(c).copied().unwrap_or_default(),
                _ => DVec3::ZERO,
            },
            move |p, a, v| {
                if let ShapeParameters::Tube(t) = p
                    && let Some(point) = t.points.get_mut(c)
                {
                    point[a] = v;
                }
            },
        ));
    }
    f.buttons(vec![
        Button::change("Add Point", |p: &mut P| {
            if let ShapeParameters::Tube(t) = p {
                let next = match t.points.as_slice() {
                    [] => DVec3::ZERO,
                    [only] => *only + DVec3::Y,
                    [.., a, b] => *b + (*b - *a),
                };
                t.points.push(next);
            }
        })
        .tip("Add a point after the last one, continuing its direction"),
        Button::change("Remove Last", |p: &mut P| {
            if let ShapeParameters::Tube(t) = p {
                t.points.pop();
            }
        })
        .enabled_if(!t.points.is_empty(), "No points to remove"),
    ])
    .separator()
    .field(whole(
        "tub_sides",
        "Sides",
        get!(Tube, |t| t.sides),
        put!(Tube, |t, n| t.sides = n),
    ))
    .field(whole(
        "tub_segments",
        "Segments",
        get!(Tube, |t| t.segments),
        put!(Tube, |t, n| t.segments = n),
    ))
    .field(plain(
        "tub_radius",
        "Radius",
        Unit::Distance,
        get!(Tube, |t| t.radius),
        put!(Tube, |t, v| t.radius = v),
    ))
    .field(plain(
        "tub_twist",
        "Twist",
        Unit::Angle,
        get!(Tube, |t| t.twist),
        put!(Tube, |t, v| t.twist = v),
    ))
    .field(
        tf::check(
            "tub-caps",
            "Caps",
            get!(Tube, |t| t.caps),
            put!(Tube, |t, b| t.caps = b),
        )
        .enabled_if(!t.closed, "A closed tube has no ends to cap"),
    )
    .field(tf::check(
        "tub-closed",
        "Closed",
        get!(Tube, |t| t.closed),
        put!(Tube, |t, b| {
            // Turning Closed off brings Caps back on, as Reset has it.
            if t.closed && !b {
                t.caps = true;
            }
            t.closed = b
        }),
    ))
    .field(tf::check(
        "tub-by-length",
        "By Length",
        get!(Tube, |t| t.by_length),
        put!(Tube, |t, b| t.by_length = b),
    ))
    .field(make_uvs("tub-uvs"));
}

/// Torus, Capsule, Arc, Tube and Text Tool Properties (NPR).
pub(super) struct ShapeFields;

impl ToolFields for ShapeFields {
    type P = ShapeParameters;
    fn params(app: &Operandi) -> Option<P> {
        app.editor
            .session
            .shape
            .as_ref()
            .map(|s| s.requested.clone())
    }
    fn form(app: &Operandi, p: &P) -> ToolForm<P> {
        let mut f = ToolForm::new();
        let kind = p.kind();
        f.title(format!("{} · numeric construction", kind.label()));
        match p {
            ShapeParameters::Torus(_) => torus(&mut f),
            ShapeParameters::Capsule(_) => capsule(&mut f),
            ShapeParameters::Arc(_) => arc(&mut f),
            ShapeParameters::Tube(t) => tube(&mut f, t, app.point_modes.tube_current),
            ShapeParameters::Text(_) => text(&mut f),
            ShapeParameters::Teapot(_) => teapot(&mut f),
        }
        // #35: the falloff shapes the Torus and Capsule as they're made.
        if matches!(kind, ShapeKind::Torus | ShapeKind::Capsule) {
            crate::primitive_panel::shape_falloff_section(
                &mut f,
                app,
                |p: &P| match p {
                    ShapeParameters::Torus(t) => t.falloff,
                    ShapeParameters::Capsule(c) => c.falloff,
                    _ => Default::default(),
                },
                |p| match p {
                    ShapeParameters::Torus(t) => Some(&mut t.falloff),
                    ShapeParameters::Capsule(c) => Some(&mut c.falloff),
                    _ => None,
                },
                Some("Moving these rings would change the shape's ends"),
                Vec::new(),
                None,
            );
        }
        let session = app.editor.session.shape.as_ref();
        let ready = app.field_errors.is_empty()
            && session.is_some_and(|s| s.error.is_none() && s.preview.is_some());
        f.separator()
            .wide(
                Button::command("Apply", Command::Apply)
                    .tip("Add this shape to the mesh and keep the tool for another")
                    .enabled_if(
                        ready,
                        if app.field_errors.is_empty() {
                            session
                                .and_then(|s| s.error.clone())
                                .unwrap_or_else(|| "Nothing to apply yet".into())
                        } else {
                            crate::unavailable::FIELD_ERRORS.into()
                        },
                    ),
            )
            .buttons(vec![
                Button::command("Drop", Command::Drop),
                Button::command("Cancel", Command::Cancel),
                Button::command("Reset", Command::ResetShape),
            ]);
        if let Some(e) = session.and_then(|s| s.error.as_ref()) {
            f.error(e);
        }
        if let Some(m) = session.and_then(|s| s.preview.as_ref()) {
            f.note(format!(
                "Preview: {} vertices / {} polygons",
                m.vertices.len(),
                m.faces.len()
            ));
        }
        // Spec: NPR-Q01, NPR-Q02
        f.note(match kind {
            ShapeKind::Torus | ShapeKind::Capsule => "Set the values here; drawing and handles in the viewport, Polygon Type, Flip, Distortion Compensation and Gaps aren't available yet.",
            ShapeKind::Arc => "Set the values here; Arc's viewport handles aren't available yet.",
            ShapeKind::Text => "Type the text, then Drop to keep it as polygons. Drag in the viewport to move it. Dropped text can't be edited as text again yet.",
            // MT6-06: the shape's credit, as the data asks.
            ShapeKind::Teapot => "The Utah Teapot (1987 version, Frank Crow; University of Utah). Set the values here, or drag in the viewport to move it; drawing its size isn't available.",
            ShapeKind::Tube => "Click in the viewport to add a point after the current one; drag a point to move it; middle-click a point to delete it (Shift: and every later one). Tube profiles aren't available yet.",
        });
        f
    }
    fn publish(app: &mut Operandi, p: P) -> Result<()> {
        app.editor.dispatch(Command::ConfigureShape(p))
    }
    fn published(app: &mut Operandi) {
        if let Some(s) = &app.editor.session.shape {
            app.status = format!(
                "{} preview · Apply adds it and keeps the tool; Drop adds it and closes; Cancel discards it",
                s.kind().label()
            );
        }
    }
    fn settled(app: &mut Operandi) -> bool {
        app.editor
            .session
            .shape
            .as_ref()
            .is_none_or(|s| s.error.is_none())
    }
    fn option_error_key(app: &Operandi, _: &str) -> Option<String> {
        app.editor
            .session
            .shape
            .as_ref()
            .map(|s| format!("{}error", prefix(s.kind())))
    }
    fn after_option(app: &mut Operandi) {
        let Some(ShapeParameters::Tube(t)) = app
            .editor
            .session
            .shape
            .as_ref()
            .map(|s| s.requested.clone())
        else {
            return;
        };
        // Adding or removing a point can change which point the Position
        // rows show (#44): they read it again.
        let gone = |key: &String| key.starts_with("tub_current");
        app.fields.retain(|k, _| !gone(k));
        app.confirmed_fields.retain(|k, _| !gone(k));
        app.field_errors.retain(|k, _| !gone(k));
        // A Tube with too few points has no preview yet; say what's needed
        // instead of showing a field error.
        if t.points.len() < 2 {
            app.field_errors.retain(|k, _| !k.starts_with("tub_"));
            if let Some(s) = app.editor.session.shape.as_mut() {
                s.error = None;
            }
            app.status = "Tube: add at least 2 control points".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(kind: &str) -> Operandi {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.execute(kind);
        a
    }
    fn enter(a: &mut Operandi, key: &str, text: &str) {
        a.fields.insert(key.into(), text.into());
        assert!(a.confirm_fields(), "{key} = {text}: {}", a.status);
    }
    /// Arming draws nothing; place the remembered shape like a drag would.
    fn place(a: &mut Operandi) {
        let p = a.editor.session.shape.as_ref().unwrap().requested.clone();
        a.run(Command::ConfigureShape(p));
    }
    fn counts(a: &Operandi) -> (usize, usize) {
        let m = &a.editor.scene.active().unwrap().mesh;
        (m.vertices.len(), m.faces.len())
    }

    #[test]
    fn torus_fields_update_linked_sizes_and_drop_adds_it() {
        let mut a = app("Torus");
        assert!(a.editor.session.shape.is_some());
        // Draw once so the fields exist, as the panel does.
        a.sync_form::<ShapeFields>(true);
        enter(&mut a, "tor_sides", "8");
        enter(&mut a, "tor_segments", "4");
        enter(&mut a, "tor_hole", "1");
        // Hole 1 keeps Cross Section and closes the ring onto the axis.
        assert_eq!(a.fields["tor_ring"], workspace::number_text(0.25, " m"));
        enter(&mut a, "tor_radius0", "1 m");
        assert_eq!(a.fields["tor_xsec"], workspace::number_text(1., " m"));
        assert_eq!(a.fields["tor_ring"], workspace::number_text(1., " m"));
        a.run(Command::Drop);
        assert_eq!(counts(&a), (32, 32));
        assert!(a.editor.session.shape.is_none());
    }

    #[test]
    fn bad_text_marks_the_field_and_blocks_apply() {
        let mut a = app("Capsule");
        a.sync_form::<ShapeFields>(true);
        a.fields.insert("cap_sides".into(), "2.5".into());
        assert!(!a.confirm_fields());
        assert!(a.field_errors.contains_key("cap_sides"));
        a.fields.insert("cap_sides".into(), "8".into());
        assert!(a.confirm_fields());
        assert!(a.field_errors.is_empty());
    }

    #[test]
    fn arc_angle_takes_degrees_and_caps_at_180() {
        let mut a = app("Arc");
        a.sync_form::<ShapeFields>(true);
        enter(&mut a, "arc_angle", "270");
        assert_eq!(a.fields["arc_angle"], workspace::number_text(180., "°"));
        enter(&mut a, "arc_segments", "6");
        a.run(Command::Apply);
        assert_eq!(counts(&a), (8, 1));
        assert!(a.editor.session.shape.is_some());
    }

    #[test]
    fn tube_points_from_the_panel() {
        let mut a = app("Tube");
        a.form_change::<ShapeFields>(|p| {
            if let ShapeParameters::Tube(t) = p {
                t.points = vec![DVec3::ZERO, DVec3::Y];
                t.sides = 4;
                t.segments = 1;
            }
        });
        a.run(Command::Drop);
        assert_eq!(counts(&a), (8, 6));
    }

    #[test]
    fn tube_panel_shows_the_current_point_only() {
        // #44 (Steve Hill): like the curve tools, Tool Properties shows the
        // current point's position, not a row per point; with none picked,
        // the last one.
        use crate::tool_form::Item;
        let mut a = app("Tube");
        a.form_change::<ShapeFields>(|p| {
            if let ShapeParameters::Tube(t) = p {
                t.points = vec![DVec3::ZERO, DVec3::Y, DVec3::new(1., 2., 0.)];
            }
        });
        let labels = |a: &Operandi| -> Vec<String> {
            let p = ShapeFields::params(a).unwrap();
            ShapeFields::form(a, &p)
                .items
                .iter()
                .filter_map(|i| match i {
                    Item::Field(f) => Some(f.row.label.clone()),
                    _ => None,
                })
                .collect()
        };
        let shown = labels(&a);
        assert!(!shown.iter().any(|l| l.starts_with("Point ")), "{shown:?}");
        assert!(shown.contains(&"Position X".to_owned()), "{shown:?}");
        let position = |a: &Operandi| {
            let p = ShapeFields::params(a).unwrap();
            let form = ShapeFields::form(a, &p);
            form.items
                .iter()
                .find_map(|i| match i {
                    Item::Field(f) if f.row.label == "Position X" => Some(f.row.key.clone()),
                    _ => None,
                })
                .unwrap()
        };
        // None picked: the last point. Picking point 2 shows it, and the
        // fields edit it.
        assert_eq!(position(&a), "tub_current0");
        a.point_modes.tube_current = Some(1);
        a.sync_form::<ShapeFields>(true);
        assert_eq!(a.fields["tub_current1"], workspace::number_text(1., " m"));
        enter(&mut a, "tub_current0", "0.5 m");
        let ShapeParameters::Tube(t) = &a.editor.session.shape.as_ref().unwrap().requested else {
            panic!("tube")
        };
        assert_eq!(t.points[1], DVec3::new(0.5, 1., 0.), "{}", a.status);
        assert_eq!(t.points[2], DVec3::new(1., 2., 0.));
    }

    #[test]
    fn command_line_and_menu_names() {
        let mut a = app("Torus");
        assert_eq!(a.command_active(), command::id("prim.toroid"));
        place(&mut a);
        a.execute("tool.set prim.capsule on");
        assert!(matches!(
            a.editor.session.shape.as_ref().map(|s| s.kind()),
            Some(ShapeKind::Capsule)
        ));
        // Switching tools dropped the Torus into the mesh.
        assert_eq!(counts(&a), (288, 288));
        a.execute("tool.reset");
        place(&mut a);
        a.execute("Capsule");
        assert!(a.editor.session.shape.is_none());
        assert_eq!(counts(&a), (288 + 242, 288 + 264));
    }

    #[test]
    fn text_typed_in_the_panel_drops_as_polygons() {
        let mut a = app("Text");
        assert!(a.editor.session.shape.is_some(), "{}", a.status);
        // Nothing to show until there are letters.
        assert!(a.editor.session.shape.as_ref().unwrap().preview.is_none());
        a.sync_form::<ShapeFields>(true);
        enter(&mut a, "txt_text", "Hi");
        enter(&mut a, "txt_size", "2 m");
        let preview = a
            .editor
            .session
            .shape
            .as_ref()
            .unwrap()
            .preview
            .clone()
            .unwrap();
        assert!(!preview.faces.is_empty());
        let undo = a.editor.history.undo.len();
        a.run(Command::Drop);
        assert!(a.editor.session.shape.is_none());
        assert_eq!(counts(&a), (preview.vertices.len(), preview.faces.len()));
        a.run(Command::Undo);
        assert_eq!(counts(&a), (0, 0));
        assert!(a.editor.history.undo.len() <= undo + 1);
        // The next Text starts from the last settings.
        a.execute("Text");
        let ShapeParameters::Text(t) = &a.editor.session.shape.as_ref().unwrap().requested else {
            panic!("text")
        };
        assert_eq!((t.text.as_str(), t.size), ("Hi", 2.));
    }

    #[test]
    fn dropping_empty_text_adds_nothing() {
        let mut a = app("Text");
        a.run(Command::Drop);
        assert!(a.editor.session.shape.is_none(), "{}", a.status);
        assert_eq!(counts(&a), (0, 0));
    }
}
