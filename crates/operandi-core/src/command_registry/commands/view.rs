//! View phrases (one section of `COMMANDS`, in table order).
use super::*;

pub(super) const COMMANDS: &[Entry] = &[
    // ---------------------------------------------------------- view
    action("Top View", View, "Top view (again: bottom).")
        .operandi().ui(),
    action("Front View", View, "Front view (again: back).")
        .operandi().ui(),
    action("Right View", View, "Right view (again: left).")
        .operandi().ui(),
    action("Perspective View", View, "Perspective view.")
        .operandi().ui(),
    action("View", View, "Sets the active view's direction.")
        .operandi()
        .args(&[arg(
            "direction",
            ArgType::Choice(&[
                "top",
                "bottom",
                "front",
                "back",
                "right",
                "left",
                "perspective",
                "camera",
            ]),
            N,
        )])
        .example("view top").ui(),
    script("viewport.fit", View, "Frames everything (selected:true frames the selection).")
        .args(&[opt("selected", BOOL, N, Some("false"))])
        .spelled("Frame All").ui(),
    script("viewport.fitSelected", View, "Frames the selection.").spelled("Frame Selection").ui(),
    action("Zoom In", View, "Zooms in a step.")
        .operandi().ui(),
    action("Zoom Out", View, "Zooms out a step.")
        .operandi().ui(),
    action("Re-centre", View, "Moves the view centre to the point under the pointer.")
        .operandi()
        .aliases(&["Recentre"]).ui(),
    action("Turn", View, "Toggles turntable orbiting.")
        .operandi().ui(),
    action("Turn Upright", View, "Turns the view upright.")
        .operandi().ui(),
    action("Align Selected", View, "Aligns the view to the selection.")
        .operandi().ui(),
    action("Fit and Align Selected", View, "Fits and aligns the view to the selection.")
        .operandi().ui(),
    script("viewport.maximize", View, "Maximises the view (or restores it).")
        .aliases(&["Maximise Viewport (Alternate)"])
        .spelled("Maximise Viewport")
        .ui(),
    // Menus as forms: View > Split View and Tear Off View (name map
    // names; direction:, keep: and fromMouse: aren't read yet).
    script("viewport.split", View, "Splits the active view in two, side by side (View > Split View).")
        .ui(),
    script("viewport.detach", View, "Tears the active view off into a window of its own (View > Tear Off View); a window's only view stays.")
        .ui(),
    action("Viewport Properties", ViewportOptions, "Opens the view's Properties popover.")
        .operandi().ui(),
    action("Shaded View", ViewportOptions, "Shaded drawing style.")
        .operandi().ui(),
    action("Wireframe View", ViewportOptions, "Wireframe drawing style.")
        .operandi().ui(),
    action("Wireframe Overlay None", ViewportOptions, "No wireframe over active meshes.")
        .operandi().ui(),
    action("Wireframe Overlay Uniform", ViewportOptions, "Uniform wireframe over active meshes.")
        .operandi().ui(),
    action("Wireframe Overlay Colored", ViewportOptions, "Coloured wireframe over active meshes.")
        .operandi().ui(),
    // XR-2: the 3D view header's Transparency buttons (Normal, Ghost,
    // X-Ray; Alt+click for inactive meshes) as Input Map phrases.
    action("Transparency Normal", ViewportOptions, "This view's active meshes drawn solid (Transparency Overrides off).")
        .modo("view3d.TransparencyOverride Normal").ui(),
    action("Transparency Ghost", ViewportOptions, "This view's active meshes in Ghost: polygons facing you see-through and tinted.")
        .modo("view3d.TransparencyOverride Ghost").ui(),
    action("Transparency X-Ray", ViewportOptions, "This view's active meshes in X-Ray: polygons facing you clear.")
        .modo("view3d.TransparencyOverride X-Ray").ui(),
    action("Inactive Transparency Normal", ViewportOptions, "This view's inactive meshes drawn solid.")
        .modo("view3d.TransparencyOverride Normal bgmesh:1").ui(),
    action("Inactive Transparency Ghost", ViewportOptions, "This view's inactive meshes in Ghost.")
        .modo("view3d.TransparencyOverride Ghost bgmesh:1").ui(),
    action("Inactive Transparency X-Ray", ViewportOptions, "This view's inactive meshes in X-Ray.")
        .modo("view3d.TransparencyOverride X-Ray bgmesh:1").ui(),
    script("operandi.view3d.toggleXRay", ViewportOptions, "Toggles this view's active meshes between Normal and X-Ray (Ghost goes to X-Ray). No default key.")
        .spelled("Toggle X-Ray")
        .operandi()
        .ui(),
    action("Shading", ViewportOptions, "Sets the active meshes' drawing style.")
        .operandi()
        .args(&[arg(
            "style",
            ArgType::Choice(&[
                "wireframe",
                "shaded",
                "advanced",
                "texture",
                "default",
                "gooch",
                "vertex map",
                "reflection",
                "solid",
                "cel",
            ]),
            N,
        )])
        .ui()
        .example("shading shaded"),
    action("Toggle", ViewportOptions, "Toggles a viewport display option.")
        .operandi()
        .args(&[arg(
            "option",
            ArgType::Choice(&[
                "lights",
                "cameras",
                "wireframe overlay",
                "show verts",
                "grid",
                "background gradient",
            ]),
            N,
        )])
        .ui()
        .example("toggle grid"),
    action("Snapping Toggle", Tools, "Toggles snapping.")
        .operandi().ui(),
    // SPEC-BACKGROUND-CONSTRAINT: the reference tool const.bg (registry
    // thread 2026-11-13); its options are tool.attr const.bg geometry,
    // bkgOffset, negative, dblSided, handle and target.
    action(
        "Constrain to Background",
        Tools,
        "Turns Constrain to Background on or off: points tools move or create land on the background meshes.",
    )
    .modo("tool.set const.bg on").ui(),
    action("Snapping Options", Tools, "Opens Snapping Options.")
        .operandi().ui(),
    // BEHAVIOR-MESH-CONSTRAINT MC-03: Edit > Snapping > Mesh Constraints.
    action("Mesh Constraints", Tools, "Opens the Mesh Constraint popover.")
        .operandi().ui(),
    action("Element Snap", Tools, "Starts the Element Snap tool (Drag Snap) with snapping on.")
        .operandi().ui(),
    action("Pie: View Direction", View, "Opens the View Direction pie.")
        .operandi().ui(),
    action("Pie: Selection & Transforms", Selection, "Opens the Selection & Transforms pie.")
        .operandi().ui(),
    action("Pie: Shading Style", ViewportOptions, "Opens the Shading Style pie.")
        .operandi().ui(),
    action("Pie: Viewport Toggles", ViewportOptions, "Opens the Viewport Toggles pie.")
        .operandi().ui(),
    action("Pie: Falloffs", Tools, "Opens the Falloffs pie.")
        .operandi().ui(),
    action("Pie: Action Centers", Tools, "Opens the Action Centers pie.")
        .operandi().ui(),
    action("Pie: Modeling", Tools, "Opens the Modeling pie.")
        .operandi().ui(),
    action("Pie: Panels", View, "Opens the Panels pie: pops out the Graph Editor, Items, Properties, the Timeline or the Toolbox on its own as a popover (Ctrl+3).")
        .operandi().ui(),
    // Layouts stage 2 (UIC-18, LAY-32): the real Ctrl key on a Mac.
    action("Pie: Layouts", View, "Opens the Layouts pie: each slot shows a layout (Ctrl+`).")
        .operandi().ui(),
    action("Layout Switcher", View, "The quick layout switcher while Ctrl+Shift are held: each Tab moves the outline on, releasing switches (UIC-18; no key by default since Ctrl+Shift+Tab pages the tool tabs).")
        .operandi().ui(),
    // Tool tab cycling (tool-tab-cycling v1.1): the real Ctrl key on a Mac.
    action("Next Tool Tab", View, "Opens the next tool tab to the right, wrapping to the first (Ctrl+Tab).")
        .operandi().ui(),
    action("Previous Tool Tab", View, "Opens the previous tool tab to the left, wrapping to the last (Ctrl+Shift+Tab).")
        .operandi().ui(),
    // Layouts stage 6 (LAY-53, UIC-206): the backtick key.
    // Animation keys (Items mode; the app's anim_ui.rs runs their lines).
    action("Play/Stop", Items, "Plays or stops playback (TK-60; / in the Animate layout).")
        .operandi().ui(),
    action("Play Backwards", Items, "Plays backwards through the current range (TK-60).")
        .operandi().ui(),
    action("Back One Frame", Items, "Steps back one frame (TK-63, Shift+Left).")
        .operandi().ui(),
    action("Forward One Frame", Items, "Steps forward one frame (TK-63, Shift+Right).")
        .operandi().ui(),
    action("Step Back", Items, "Steps back one frame (TK-63; comma in the Animate layout).")
        .operandi().ui(),
    action("Step Forward", Items, "Steps forward one frame (TK-63; period in the Animate layout).")
        .operandi().ui(),
    action("First Frame", Items, "Goes to the current range's first frame (TK-63, Shift+Up).")
        .operandi().ui(),
    action("Last Frame", Items, "Goes to the current range's last frame (TK-63, Shift+Down).")
        .operandi().ui(),
    action("Previous Key", Items, "Goes to the previous key of the selected items or channels (TK-63, Ctrl+Left).")
        .operandi().ui(),
    action("Next Key", Items, "Goes to the next key of the selected items or channels (TK-63, Ctrl+Right).")
        .operandi().ui(),
    action("First Key", Items, "Goes to the first key of the selected items or channels (TK-63, Ctrl+Up).")
        .operandi().ui(),
    action("Last Key", Items, "Goes to the last key of the selected items or channels (TK-63, Ctrl+Down).")
        .operandi().ui(),
    action("Key Selected Channels", Items, "Keys the selected channels, or the live transform tool's (TK-21, S).")
        .operandi().ui(),
    action("Key Selected Items", Items, "Keys every animated channel of the selected items (TK-23, Shift+S).")
        .operandi().ui(),
    action("Key Position", Items, "Keys the selected items' position (TK-22, Shift+W).")
        .operandi().ui(),
    action("Key Rotation", Items, "Keys the selected items' rotation (TK-22, Shift+E).")
        .operandi().ui(),
    action("Key Scale", Items, "Keys the selected items' scale (TK-22; Shift+R in the Animate layout).")
        .operandi().ui(),
    action("Key Transforms", Items, "Keys the selected items' position, rotation and scale (TK-22; Shift+Y in the Animate layout).")
        .operandi().ui(),
    action("Toggle Palettes", View, "Hides every open palette, or shows them all again (`, LAY-53).")
        .operandi().ui(),
    action("Node View", View, "Opens the node view of the selected Material Group, or the group the selected layer is in (Cmd+Shift+N).")
        .operandi().ui(),
    action("UI Build Mode", View, "Turns UI build mode on or off: drag pads on every panel (Ctrl+Shift+L).")
        .operandi().ui(),
];
