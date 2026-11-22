//! UV Unwrap... and Pack UVs... (spec `uv-unwrap-pack-relax` v1.1), on the
//! Edit tab's UV group and in the Geometry menu, built with the shared dialog
//! (UIC-80..99). OK runs `Command::UvUnwrap` / `Command::PackUvs` once (one
//! undo step, UIC-96); the values are remembered for next time (UIC-95).
//!
//! Provisional: the spec makes UV Unwrap an interactive tool with live Tool
//! Properties. Operandi has no UV viewport yet to watch it in, so for now
//! it is a dialog with the same fields, in the spec's order (UV-Q20).
use super::*;
use crate::toolbox::row_height;
use operandi_core::uv_tools::{
    Axis3, InitialProjection, PackDirection, PackOptions, PackRegion, PackTag, SeamSource,
    UnwrapOptions, UnwrapSolver, UvAxis,
};

const PROJECTIONS: [&str; 5] = [
    "Planar",
    "Cylindrical",
    "Spherical",
    "Use Current Map",
    "Group Normal",
];
const PROJECTION_VALUES: [InitialProjection; 5] = [
    InitialProjection::Planar,
    InitialProjection::Cylindrical,
    InitialProjection::Spherical,
    InitialProjection::UseCurrentMap,
    InitialProjection::GroupNormal,
];
const AXES: [&str; 3] = ["X", "Y", "Z"];
const AXIS_VALUES: [Axis3; 3] = [Axis3::X, Axis3::Y, Axis3::Z];
const UV_AXES: [&str; 2] = ["U", "V"];
const METHODS: [&str; 2] = ["Conformal", "Angle Based"];
const SEAM_TYPES: [&str; 3] = ["Selected Edges", "UV Seam Map", "Selection Set"];
const DIRECTIONS: [&str; 3] = ["Horizontal", "Vertical", "Auto"];
const REGIONS: [&str; 7] = [
    "Normalized",
    "Nearest",
    "UDIM",
    "Manual",
    "Tiles",
    "Polygon Tag",
    "By Item",
];
const REGION_VALUES: [PackRegion; 7] = [
    PackRegion::Normalized,
    PackRegion::Nearest,
    PackRegion::Udim,
    PackRegion::Manual,
    PackRegion::Tiles,
    PackRegion::PolygonTag,
    PackRegion::ByItem,
];

fn index_of<T: PartialEq>(values: &[T], v: &T) -> usize {
    values.iter().position(|x| x == v).unwrap_or(0)
}

/// UV Unwrap's rows, top to bottom (spec §1 table).
pub(super) fn unwrap_rows(o: &UnwrapOptions, edge_sets: Vec<String>) -> Vec<form::Row> {
    use form::{Row, Unit, When};
    let (seam_type, seam_name) = match &o.seams {
        SeamSource::SelectedEdges => (0, String::new()),
        SeamSource::SeamMap(n) => (1, n.clone()),
        SeamSource::SelectionSet(n) => (2, n.clone()),
    };
    vec![
        Row::number("iterations", "Iterations", Unit::Count, o.iterations as f64)
            .range(0., 100_000.)
            .tip("How far the solver runs; 0 keeps the Initial Projection"),
        Row::number(
            "relative_gap",
            "Relative Gap Size",
            Unit::Scalar,
            o.relative_gap,
        )
        .range(0., 100.)
        .tip("Space left between islands and at the tile edge"),
        Row::check(
            "segment_material",
            "Segment by Material",
            o.segment_by_material,
        )
        .tip("Cut between polygons with different materials"),
        Row::check("segment_part", "Segment by Part", o.segment_by_part)
            .tip("Cut between polygons in different parts"),
        Row::choice(
            "projection",
            "Initial Projection",
            &PROJECTIONS,
            index_of(&PROJECTION_VALUES, &o.initial_projection),
        )
        .tip("Starting layout before solving"),
        Row::choice(
            "pinning_axis",
            "Pinning Axis",
            &UV_AXES,
            index_of(&[UvAxis::U, UvAxis::V], &o.pinning_axis),
        )
        .tip("Islands' longer side runs along this axis"),
        Row::choice(
            "projection_axis",
            "Projection Axis",
            &AXES,
            index_of(&AXIS_VALUES, &o.projection_axis),
        )
        .when(When::ChoiceIn("projection", vec![0, 1, 2]))
        .tip("Applies only to Planar, Cylindrical or Spherical"),
        Row::number("offset_angle", "Offset Angle", Unit::Angle, o.offset_angle)
            .tip("Rotates the unwrapped islands"),
        Row::check("seal_holes", "Seal Holes", o.seal_holes)
            .tip("Solve as if holes were capped, so they keep their shape"),
        Row::check("update_layout", "Update Layout", o.update_layout)
            .tip("Keep the layout inside the 0-1 tile"),
        Row::choice(
            "method",
            "Method",
            &METHODS,
            index_of(
                &[UnwrapSolver::Conformal, UnwrapSolver::AngleBased],
                &o.method,
            ),
        )
        .tip("Conformal: least squares conformal maps. Angle Based: angle based flattening"),
        Row::check("symmetrize", "Symmetrize", false)
            .when(When::Never)
            .tip("Symmetrize isn't built yet"),
        Row::choice(
            "symmetry_axis",
            "Symmetry Axis",
            &AXES,
            index_of(&AXIS_VALUES, &o.symmetry_axis),
        )
        .when(When::Checked("symmetrize"))
        .tip("Axis the mesh is symmetric across"),
        Row::choice(
            "mirror_axis",
            "Mirror Axis",
            &UV_AXES,
            index_of(&[UvAxis::U, UvAxis::V], &o.mirror_axis),
        )
        .when(When::Checked("symmetrize"))
        .tip("UV axis the result mirrors across"),
        Row::choice("seam_type", "UV Seam Type", &SEAM_TYPES, seam_type)
            .tip("Where seams come from"),
        Row::text(
            "seam_map",
            "UV Seam",
            if seam_type == 1 { &seam_name } else { "" },
        )
        .when(When::ChoiceIn("seam_type", vec![1]))
        .tip("Operandi has no seam maps yet, so this cuts nothing"),
        Row::text(
            "seam_set",
            "Selection Set",
            if seam_type == 2 { &seam_name } else { "" },
        )
        .when(When::ChoiceIn("seam_type", vec![2]))
        .suggestions(edge_sets)
        .tip("An edge selection set whose edges are seams"),
    ]
    .into_iter()
    .map(form::Row::fresh)
    .collect()
}

pub(super) fn unwrap_options(v: &form::Values, base: &UnwrapOptions) -> UnwrapOptions {
    let uv_axis = |k| {
        if v.choice(k) == 1 {
            UvAxis::V
        } else {
            UvAxis::U
        }
    };
    UnwrapOptions {
        iterations: v.number("iterations").round().max(0.) as u32,
        relative_gap: v.number("relative_gap"),
        segment_by_material: v.bool("segment_material"),
        segment_by_part: v.bool("segment_part"),
        initial_projection: PROJECTION_VALUES[v.choice("projection").min(4)],
        pinning_axis: uv_axis("pinning_axis"),
        projection_axis: AXIS_VALUES[v.choice("projection_axis").min(2)],
        offset_angle: v.number("offset_angle"),
        seal_holes: v.bool("seal_holes"),
        update_layout: v.bool("update_layout"),
        method: if v.choice("method") == 1 {
            UnwrapSolver::AngleBased
        } else {
            UnwrapSolver::Conformal
        },
        symmetrize: false,
        symmetry_axis: AXIS_VALUES[v.choice("symmetry_axis").min(2)],
        mirror_axis: uv_axis("mirror_axis"),
        seams: match v.choice("seam_type") {
            1 => SeamSource::SeamMap(v.text("seam_map").trim().to_owned()),
            2 => SeamSource::SelectionSet(v.text("seam_set").trim().to_owned()),
            _ => SeamSource::SelectedEdges,
        },
        ..base.clone()
    }
}

/// Pack UVs' rows, top to bottom (spec §2 table).
pub(super) fn pack_rows(o: &PackOptions) -> Vec<form::Row> {
    use form::{Row, Unit, When};
    vec![
        Row::check("pack", "Pack", o.pack).tip("Islands may move"),
        // Keep Size turns Stretch off, but sits below it in the spec's
        // order, so Stretch can't grey out (UIC-93); the tip says so.
        Row::check("stretch", "Stretch", o.stretch)
            .tip("First give each island the UV size its 3D size calls for (not with Keep Size)"),
        Row::check("orient", "Orient", o.orient).tip("Islands may rotate while packing"),
        Row::check("keep_size", "Keep Size", o.keep_size).tip("Islands keep their size"),
        Row::choice(
            "direction",
            "Direction",
            &DIRECTIONS,
            index_of(
                &[
                    PackDirection::Horizontal,
                    PackDirection::Vertical,
                    PackDirection::Auto,
                ],
                &o.direction,
            ),
        )
        .when(When::Checked("orient"))
        .tip("Preferred proportion of each island"),
        Row::number("gaps", "Gaps", Unit::Scalar, o.gaps)
            .range(0., 100.)
            .tip("Space between islands, relative to the region (not with Set Gaps by Pixel)"),
        Row::check("gaps_by_pixel", "Set Gaps by Pixel", o.gaps_by_pixel)
            .tip("Use Pixel Size instead of Gaps"),
        Row::number(
            "pixel_size",
            "Pixel Size",
            Unit::Suffix(" px"),
            o.pixel_size,
        )
        .range(0., 100_000.)
        .when(When::Checked("gaps_by_pixel"))
        .tip("Gap in pixels of a texture of Image Size"),
        Row::number(
            "image_size",
            "Image Size",
            Unit::Suffix(" px"),
            o.image_size,
        )
        .range(1., 1_000_000.)
        .when(When::Checked("gaps_by_pixel"))
        .tip("The texture size Pixel Size is measured on"),
        Row::check("bounding_box", "Based on Bounding Box", o.bounding_box)
            .tip("Treat each island as its bounding rectangle (faster, looser)"),
        Row::check("lock_stacked", "Lock Stacked", o.lock_stacked)
            .tip("Islands lying exactly on top of each other move as one"),
        Row::choice(
            "region",
            "Pack Region",
            &REGIONS,
            index_of(&REGION_VALUES, &o.region),
        )
        .tip("Where the islands go: Nearest keeps each island's own tile, Polygon Tag and By Item give each group a tile"),
        Row::number("udim", "UDIM", Unit::Count, o.udim as f64)
            .range(1001., 9999.)
            .when(When::ChoiceIn("region", vec![2]))
            .tip("Target tile"),
        Row::number("region_u", "Region U", Unit::Scalar, o.region_uv.x)
            .when(When::ChoiceIn("region", vec![3]))
            .tip("Lower-left corner of the region"),
        Row::number("region_v", "Region V", Unit::Scalar, o.region_uv.y)
            .when(When::ChoiceIn("region", vec![3])),
        Row::number("width", "Width", Unit::Scalar, o.region_size.x)
            .range(0., 1000.)
            .when(When::ChoiceIn("region", vec![3])),
        Row::number("height", "Height", Unit::Scalar, o.region_size.y)
            .range(0., 1000.)
            .when(When::ChoiceIn("region", vec![3])),
        Row::number("tile_u", "Tile U", Unit::Count, o.tiles[0] as f64)
            .range(1., 100.)
            .when(When::ChoiceIn("region", vec![4])),
        Row::number("tile_v", "Tile V", Unit::Count, o.tiles[1] as f64)
            .range(1., 100.)
            .when(When::ChoiceIn("region", vec![4])),
        Row::choice(
            "polygon_tag",
            "Polygon Tag",
            &["Material", "Part", "Smoothing Group", "Selection Set"],
            index_of(
                &[
                    PackTag::Material,
                    PackTag::Part,
                    PackTag::SmoothingGroup,
                    PackTag::SelectionSet,
                ],
                &o.polygon_tag,
            ),
        )
        .when(When::ChoiceIn("region", vec![5]))
        .tip("Islands are grouped by this tag; each group gets its own tile"),
        Row::check("background", "Background Constraint", o.background)
            .tip("Other visible meshes' islands in this map stay put; pack round them"),
        Row::check("write_new", "Pack UVs to New Map", o.write_new)
            .tip("Write the result to a new map and keep the original"),
        Row::text("new_name", "New Texture", &o.new_name)
            .when(When::Checked("write_new"))
            .tip("The new map's name; empty picks Texture 2, Texture 3... (By Item: the item's name)"),
        Row::check(
            "best_box_rotation",
            "Find Best Box Rotation",
            o.find_best_box_rotation,
        )
        .when(When::Checked("orient"))
        .tip("With Orient and Auto, any angle may be used; off, quarter turns only"),
    ]
    .into_iter()
    .map(form::Row::fresh)
    .collect()
}

pub(super) fn pack_options(v: &form::Values) -> PackOptions {
    let count = |k| v.number(k).round().max(0.) as u32;
    PackOptions {
        pack: v.bool("pack"),
        stretch: v.bool("stretch"),
        orient: v.bool("orient"),
        keep_size: v.bool("keep_size"),
        direction: [
            PackDirection::Horizontal,
            PackDirection::Vertical,
            PackDirection::Auto,
        ][v.choice("direction").min(2)],
        gaps: v.number("gaps"),
        gaps_by_pixel: v.bool("gaps_by_pixel"),
        pixel_size: v.number("pixel_size"),
        image_size: v.number("image_size"),
        bounding_box: v.bool("bounding_box"),
        lock_stacked: v.bool("lock_stacked"),
        region: REGION_VALUES[v.choice("region").min(6)],
        udim: count("udim"),
        region_uv: glam::DVec2::new(v.number("region_u"), v.number("region_v")),
        region_size: glam::DVec2::new(v.number("width"), v.number("height")),
        tiles: [count("tile_u"), count("tile_v")],
        find_best_box_rotation: v.bool("best_box_rotation"),
        polygon_tag: [
            PackTag::Material,
            PackTag::Part,
            PackTag::SmoothingGroup,
            PackTag::SelectionSet,
        ][v.choice("polygon_tag").min(3)],
        background: v.bool("background"),
        write_new: v.bool("write_new"),
        new_name: v.text("new_name").trim().to_owned(),
    }
}

/// Which UV dialog is open.
pub(super) enum UvDialog {
    Unwrap(form::Dialog),
    Pack(form::Dialog),
    /// An E7 rule's options (uv_edit_ui.rs; UVT-08 Ctrl/Cmd+click).
    Island(form::Dialog, operandi_core::uv_edit::RuleKind),
    /// A stage 3 button's options (uv_ops_ui.rs).
    Op(form::Dialog, crate::uv_ops_ui::OpDialog),
}

impl Operandi {
    /// UV Unwrap... opens its dialog (Edit tab, Geometry menu, command line).
    pub(super) fn open_uv_unwrap(&mut self) {
        if self.uv_dialog.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before UV Unwrap".into();
            return;
        }
        let sets = self
            .editor
            .scene
            .active()
            .map(|i| {
                operandi_core::selection_sets::set_names(
                    &i.mesh,
                    operandi_core::selection_sets::SetKind::Edge,
                )
            })
            .unwrap_or_default();
        let rows = unwrap_rows(&self.editor.uv_unwrap_preset, sets);
        self.uv_dialog = Some(UvDialog::Unwrap(self.form_dialog("UV Unwrap", rows)));
    }

    /// Pack UVs... opens its dialog.
    pub(super) fn open_pack_uvs(&mut self) {
        if self.uv_dialog.is_some() {
            return;
        }
        if self.has_modal() {
            self.status = "Close the open dialog before Pack UVs".into();
            return;
        }
        let rows = pack_rows(&self.editor.uv_pack_preset);
        self.uv_dialog = Some(UvDialog::Pack(self.form_dialog("Pack UVs", rows)));
    }

    /// The open UV dialog, once per frame.
    pub(super) fn uv_dialogs(&mut self, ctx: &Context) {
        let Some(mut dialog) = self.uv_dialog.take() else {
            return;
        };
        if self.has_modal() {
            self.status = "UV dialog closed; nothing changed".into();
            return;
        }
        if let UvDialog::Island(form, kind) = &mut dialog {
            let kind = *kind;
            match self.show_form(ctx, form) {
                form::Outcome::Cancel => self.status = "Canceled; nothing changed".into(),
                form::Outcome::Ok => {
                    let values = form.values.clone();
                    if let Err(e) = self.island_dialog_ok(kind, &values) {
                        self.status = e.clone();
                        form.error = Some(e);
                        self.uv_dialog = Some(dialog);
                    }
                }
                _ => self.uv_dialog = Some(dialog),
            }
            return;
        }
        if let UvDialog::Op(form, kind) = &mut dialog {
            let kind = kind.clone();
            match self.show_form(ctx, form) {
                form::Outcome::Cancel => self.status = "Canceled; nothing changed".into(),
                form::Outcome::Ok => {
                    let values = form.values.clone();
                    if let Err(e) = self.uv_op_dialog_ok(&kind, &values) {
                        self.status = e.clone();
                        form.error = Some(e);
                        self.uv_dialog = Some(dialog);
                    }
                }
                _ => self.uv_dialog = Some(dialog),
            }
            return;
        }
        let (form, name) = match &mut dialog {
            UvDialog::Unwrap(f) => (f, "UV Unwrap"),
            UvDialog::Pack(f) => (f, "Pack UVs"),
            UvDialog::Island(..) | UvDialog::Op(..) => unreachable!("handled above"),
        };
        match self.show_form(ctx, form) {
            form::Outcome::Cancel => {
                self.status = format!("{name} canceled; nothing changed");
            }
            form::Outcome::Ok => {
                let (command, check) = match &dialog {
                    UvDialog::Unwrap(f) => {
                        let o = unwrap_options(&f.values, &self.editor.uv_unwrap_preset);
                        let check = o.validate();
                        (Command::UvUnwrap(o), check)
                    }
                    UvDialog::Pack(f) => {
                        let o = pack_options(&f.values);
                        let check = o.validate();
                        (Command::PackUvs(o), check)
                    }
                    UvDialog::Island(..) | UvDialog::Op(..) => unreachable!("handled above"),
                };
                match check {
                    Ok(()) => {
                        if self.run_ok(command) {
                            self.status = format!("{name} done");
                        }
                    }
                    Err(e) => {
                        self.status = e.clone();
                        match &mut dialog {
                            UvDialog::Unwrap(f)
                            | UvDialog::Pack(f)
                            | UvDialog::Island(f, _)
                            | UvDialog::Op(f, _) => f.error = Some(e),
                        }
                        self.uv_dialog = Some(dialog);
                    }
                }
            }
            _ => self.uv_dialog = Some(dialog),
        }
    }

    /// The Edit tab's UV rows.
    pub(super) fn uv_panel(&mut self, ui: &mut Ui) {
        for (key, label, tip) in [
            (
                "UV Unwrap",
                "Unwrap...",
                "UV Unwrap... · flatten the visible polygons into the UV map, cut at seams · uv.unwrap",
            ),
            (
                "Pack UVs",
                "Pack UVs...",
                "Pack UVs... · lay the UV map's islands out in the tile · uv.pack",
            ),
        ] {
            let b = ui.add_sized(
                vec2(ui.available_width(), row_height()),
                egui::Button::new(format!("{label}    {}", self.shortcut(key))),
            );
            self.toolbox_rects.insert(key.into(), b.rect);
            if b.clicked() {
                match key {
                    "UV Unwrap" => self.open_uv_unwrap(),
                    _ => self.open_pack_uvs(),
                }
            }
            b.on_hover_text(tip);
        }
        // E7 island buttons (uv-editing-2026-11-16).
        self.uv_island_buttons(ui);
    }
}
