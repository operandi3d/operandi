//! Preset browser, layouts, forms and menus (one section of `COMMANDS`, in table order).
use super::*;

pub(super) const COMMANDS: &[Entry] = &[
    // ------------------------------------------------------ preset browser
    // Names from the registry thread (2026-11-11), Stage 1 PB-100..122.
    // Layouts stage 6 (LAY-50, UIC-205): every palette.
    script("layout.window", Presets, "Opens or closes a palette (its own window): presetBrowser, uvEditor, uvView, toolbox, items, properties, itemsProperties, shading, groups, vertexMaps, commandHistory, vertexNormalTools, timeline or graphEditor. Bare open: toggles; open:true on an open one brings it forward (LAY-50).")
        .args(&[
            arg(
                "hash",
                ArgType::Choice(&[
                    "presetBrowser",
                    "uvEditor",
                    "uvView",
                    "toolbox",
                    "items",
                    "properties",
                    "shading",
                    "groups",
                    "vertexMaps",
                    "commandHistory",
                    "formEditor",
                    "vertexNormalTools",
                ]),
                N,
            ),
            opt("open", BOOL, N, None),
        ])
        .example("layout.window hash:uvEditor")
        .ui(),
    script("operandi.layout.togglePalettes", View, "Hides every open palette, or shows them all again (Layout > Windows/Palettes > Toggle Palettes, `; LAY-53).")
        .operandi()
        .ui(),
    // Layouts stage 1 (layouts-2026-11-16, LAY-30): every way of switching
    // layouts runs this.
    script("layout.restore", View, "Shows the layout called name: (Layout > Layouts lists them). The layout being left keeps its state; the scene, undo and selection don't change.")
        .args(&[arg("name", TEXT, N)])
        .example("layout.restore UV")
        .ui(),
    // Layouts stage 2 (layouts-2026-11-16 stage 2). Placeholders under
    // operandi.* until the registry thread names them (docs/NAMES-NEEDED.md).
    // The menu, the layout bar, the keys and the pie all run these (LAY-30).
    script("operandi.layout.create", View, "Makes a new layout called name: at the right-hand end of the layout bar and shows it. copy:true copies the current layout; without it the view area is one empty panel to build from (LAY-43). Without name: opens the New Layout dialog (CMD-12).")
        .operandi()
        .args(&[opt("name", TEXT, N, None), opt("copy", BOOL, N, Some("false"))])
        .example("operandi.layout.create Sketch")
        .ui(),
    script("operandi.layout.save", View, "Saves the current layout's file now (changes are also remembered automatically, UIC-199).")
        .operandi()
        .ui(),
    script("operandi.layout.saveAs", View, "Saves the current arrangement as a new layout called name: and shows it (LAY-44). Without name: opens the Save Layout As dialog (CMD-12).")
        .operandi()
        .args(&[opt("name", TEXT, N, None)])
        .example("operandi.layout.saveAs {My Model}")
        .ui(),
    script("operandi.layout.rename", View, "Renames layout: (the current one without it) to name:. Layouts you made only; shipped ones keep their names (LAY-45). Without name: opens the Rename Layout dialog (CMD-12).")
        .operandi()
        .args(&[opt("name", TEXT, N, None), opt("layout", TEXT, N, None)])
        .example("operandi.layout.rename Sculpting layout:Sketch")
        .ui(),
    script("operandi.layout.delete", View, "Deletes layout: (the current one without it). Layouts you made only; shipped ones can be reverted but not deleted (LAY-46). ask:true asks first (Layout > Delete Layout...).")
        .operandi()
        .args(&[opt("layout", TEXT, N, None), opt("ask", BOOL, N, Some("false"))])
        .ui(),
    script("operandi.layout.revert", View, "Puts layout: (the current one without it) back to its shipped arrangement and drops your copy (LAY-42). Its own keys stay unless keys:true (LAY-Q56). ask:true asks first (the tab's double-click, Layout > Reset Layout...).")
        .operandi()
        .aliases(&["operandi.layout.reset"])
        .args(&[opt("layout", TEXT, N, None), opt("keys", BOOL, N, Some("false")), opt("ask", BOOL, N, Some("false"))])
        .ui(),
    // Layouts stage 5 (LAY-20, UIC-201): each layout's key map context.
    script("operandi.layout.setKey", View, "Sets action:'s key (an Input Map action) in layout:'s own key map (the current layout without it): key:none unbinds it there, no key: gives it back to the global map. Other layouts keep the global key (LAY-20).")
        .operandi()
        .args(&[arg("action", TEXT, N), opt("key", TEXT, N, None), opt("layout", TEXT, N, None)])
        .example("operandi.layout.setKey action:Move key:Ctrl+M")
        .ui(),
    script("operandi.layout.clearKeys", View, "Clears layout:'s own keys (the current layout without it): it uses the global key map again (LAY-20).")
        .operandi()
        .args(&[opt("layout", TEXT, N, None)])
        .ui(),
    // Layouts stage 3 (LAY-87, UIC-216): a toolbox form back to shipped.
    script("operandi.form.revert", View, "Puts form: (a toolbox form id, e.g. toolbox.uv) back to its shipped version and drops your copy (LAY-87).")
        .operandi()
        .args(&[arg("form", TEXT, N)])
        .example("operandi.form.revert toolbox.uv")
        .ui(),
    // Layouts stage 4 (LAY-82, LAY-86, LAY-87, UIC-212): workbench edit
    // mode. form: defaults to the workbench being edited, else the
    // toolbox's (toolbox.workbench); index: counts the form's controls
    // from 0. Edits are the user's copy of the form (Revert drops it).
    script("operandi.workbench.edit", View, "Shows or hides the edit tools of workbench form: (Edit Workbench, LAY-82); bare toggles. Only forms marked as a workbench can be edited.")
        .operandi()
        .args(&[opt("on", BOOL, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.edit form:toolbox.workbench")
        .ui(),
    script("operandi.workbench.pick", View, "Pick to Add (LAY-82): while on, clicking a button, a Tool Properties row, a preset or a Command History line adds it to the workbench being edited instead of running it; bare toggles. Esc stops.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.workbench.add", View, "Adds a button running line: to the end of workbench form: (what Pick to Add does for a button).")
        .operandi()
        .args(&[arg("line", TEXT, N), opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.add {tool.set poly.bevel on} label:Bevel")
        .ui(),
    script("operandi.workbench.addProperty", View, "Adds a tool's Tool Properties row (property: <tool>/<key>, e.g. Bevel/inset) to workbench form:; it shows while that tool is open.")
        .operandi()
        .args(&[arg("property", TEXT, N), opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.addProperty Bevel/inset")
        .ui(),
    script("operandi.workbench.addDivider", View, "Adds a divider to workbench form:; with label: it starts a collapsible section (Add Divider with Label).")
        .operandi()
        .args(&[opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.addDivider label:Bevels")
        .ui(),
    script("operandi.workbench.addChain", View, "Adds the current tool chain (action centre, falloff, symmetry and the open tool) to workbench form: as a one-click custom tool (as:preset). as:macro and as:script aren't in Operandi yet.")
        .operandi()
        .aliases(&["operandi.workbench.addPipe"])
        .args(&[opt("as", ArgType::Choice(&["preset", "macro", "script"]), N, Some("preset")), opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.workbench.addChain label:{Soft Bevel}")
        .ui(),
    script("operandi.workbench.move", View, "Moves control from: of workbench form: to index to: (drag to reorder).")
        .operandi()
        .args(&[arg("from", INT, N), arg("to", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.workbench.move 3 0")
        .ui(),
    script("operandi.workbench.set", View, "Sets one property of control index: of workbench form: (its right-click menu): attr: label, tooltip, popup (default, popup, buttons), width (normal, wide, full), level (basic, standard, advanced), command, toggle (default, checkbox, toggleButton) or iconFile (a PNG or JPEG path; empty clears it).")
        .operandi()
        .args(&[
            arg("index", INT, N),
            arg("attr", ArgType::Choice(&["label", "tooltip", "popup", "width", "level", "command", "toggle", "iconFile"]), N),
            arg("value", TEXT, N),
            opt("form", TEXT, N, None),
        ])
        .example("operandi.workbench.set 0 width full")
        .ui(),
    script("operandi.workbench.chooseIcon", View, "Opens a file panel to pick a PNG or JPEG for control index: of workbench form: (Custom Icon...).")
        .operandi()
        .args(&[arg("index", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.workbench.chooseIcon 0")
        .ui(),
    script("operandi.workbench.duplicate", View, "Copies control index: of workbench form: just after it.")
        .operandi()
        .args(&[arg("index", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.workbench.duplicate 0")
        .ui(),
    script("operandi.workbench.delete", View, "Removes control index: from workbench form:.")
        .operandi()
        .args(&[arg("index", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.workbench.delete 0")
        .ui(),
    script("operandi.workbench.undo", View, "Takes back the last workbench edit of form: made this session (workbench edits aren't scene undo steps).")
        .operandi()
        .args(&[opt("form", TEXT, N, None)])
        .ui(),
    script("operandi.workbench.topBar", View, "Shows or hides the workbench row along the top of the window (toolbar.workbench); bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    // Layouts stage 7 (LAY-83, UIC-213): the Form Editor's edits. form:
    // defaults to the form selected in the Form Editor; index: counts the
    // form's controls from 0. Edits are the user's copy of the form
    // (operandi.form.revert drops it; operandi.workbench.undo takes one
    // back).
    script("operandi.form.newGroup", View, "Adds an empty group called name: to the Form Editor's tree (groups are for tidiness only).")
        .operandi()
        .args(&[arg("name", TEXT, N)])
        .example("operandi.form.newGroup {My Forms}")
        .ui(),
    script("operandi.form.new", View, "Makes a new, empty form called id: (your own form, kept in the Forms folder).")
        .operandi()
        .args(&[arg("id", TEXT, N), opt("label", TEXT, N, None), opt("style", TEXT, N, None), opt("group", TEXT, N, None)])
        .example("operandi.form.new my.tools label:{My Tools}")
        .ui(),
    script("operandi.form.add", View, "Adds a button running line: to form: (at index:, else the end).")
        .operandi()
        .args(&[arg("line", TEXT, N), opt("label", TEXT, N, None), opt("form", TEXT, N, None), opt("index", INT, N, None)])
        .example("operandi.form.add {tool.set poly.bevel on} form:my.tools")
        .ui(),
    script("operandi.form.addForm", View, "Places form sub: inside form: as a sub-form (at index:, else the end).")
        .operandi()
        .args(&[arg("sub", TEXT, N), opt("form", TEXT, N, None), opt("index", INT, N, None)])
        .example("operandi.form.addForm sub:toolbox.uv.ops form:toolbox.uv")
        .ui(),
    script("operandi.form.addDivider", View, "Adds a divider (a labelled one starts a section) at the end of form:.")
        .operandi()
        .args(&[opt("label", TEXT, N, None), opt("form", TEXT, N, None)])
        .ui(),
    script("operandi.form.move", View, "Moves control from: of form: to place to: (the Form Editor's drag).")
        .operandi()
        .args(&[arg("from", INT, N), arg("to", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.form.move from:0 to:1 form:toolbox.workbench")
        .ui(),
    script("operandi.form.set", View, "Sets property attr: to value: on control index: of form:, or on the form itself without index: (label, style, icons, iconSize, workbench, showWhen, slot, slotAt, slotSort, columns, group).")
        .operandi()
        .args(&[arg("attr", TEXT, N), opt("value", TEXT, N, None), opt("index", INT, N, None), opt("form", TEXT, N, None)])
        .example("operandi.form.set attr:workbench value:true form:my.tools")
        .ui(),
    script("operandi.form.delete", View, "Deletes control index: from form:.")
        .operandi()
        .args(&[arg("index", INT, N), opt("form", TEXT, N, None)])
        .example("operandi.form.delete index:0 form:toolbox.workbench")
        .ui(),
    script("operandi.form.select", View, "Opens the Form Editor on form: (and its control index:): Show in Form Editor.")
        .operandi()
        .args(&[arg("form", TEXT, N), opt("index", INT, N, None)])
        .example("operandi.form.select form:toolbox.uv")
        .ui(),
    script("operandi.form.find", View, "Find Form: the next button clicked shows its form in the Form Editor instead of running; bare toggles. Esc stops.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.form.preview", View, "Shows (or hides) the selected form, or form:, drawn in the Form Editor; bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None), opt("form", TEXT, N, None)])
        .ui(),
    script("operandi.form.export", View, "Saves form: to its own file: filename:, else the data folder's Saved Forms folder.")
        .operandi()
        .args(&[opt("form", TEXT, N, None), opt("filename", ArgType::Path, N, None)])
        .ui(),
    // Layouts stage 3 (UIC-221/223): a toolbox placeholder for a command
    // Operandi doesn't have yet. Its state answer greys it with that
    // reason, so the button needs no UI-side logic.
    script("operandi.mode.is", View, "Available only while the selection mode is one of these (item, vertex, edge, polygon); a form's showWhen names it. Running it says which mode is on.")
        .operandi()
        .args(&[arg("modes", TEXT, N)])
        .example("operandi.mode.is vertex edge")
        .quiet(),
    script("operandi.notBuilt", View, "A placeholder for a command Operandi doesn't have yet (name:, e.g. \"Shortest Path\"); does nothing but say so.")
        .operandi()
        .args(&[arg("name", TEXT, N)])
        .example("operandi.notBuilt \"Shortest Path\"")
        .quiet(),
    script("operandi.layout.clear", View, "Empties the current layout's view area to one empty panel (LAY-47).")
        .operandi()
        .ui(),
    script("operandi.layout.star", View, "Stars or unstars layout: (the current one without it); bare toggles (UIC-14).")
        .operandi()
        .args(&[opt("layout", TEXT, N, None), opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.layout.starredOnly", View, "Filters the Layouts pie to starred layouts and the current layout; bare toggles (UIC-14).")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.bar.collapse", View, "Collapses bar: (top, the default; left; right) to a line at its window edge, or brings it back; bare on: toggles. width: reopens a side bar that wide (UIC-16; the logo click, a double-click on a side bar's resize line, and the line's double-click or drag run it).")
        .operandi()
        .args(&[opt("bar", TEXT, N, None), opt("on", BOOL, N, None), opt("width", FLOAT, N, None)])
        .ui(),
    script("operandi.layout.move", View, "Moves layout: to place index: on the layout bar (0 is the left end, UIC-13).")
        .operandi()
        .args(&[arg("layout", TEXT, N), arg("index", INT, N)])
        .example("operandi.layout.move UV 0")
        .ui(),
    script("operandi.layout.switcher", View, "Opens the quick layout switcher: layouts in most-recently-used order (UIC-18). Layout > Recent Layouts runs it.")
        .operandi()
        .ui(),
    // Since 2026-11-20 panels move only in UI build mode, so this lock no
    // longer changes anything; kept so scripts using the name still run.
    script("layout.lockThumbs", View, "Lock Thumb Dragging (LAY-48), kept for scripts: panels now move only in UI build mode, so the lock has no effect; bare toggles.")
        .args(&[opt("lock", BOOL, N, None)])
        .ui(),
    script("layout.lockDividers", View, "Lock Divider Dragging for the current layout: dividers don't resize (LAY-48); bare toggles.")
        .args(&[opt("lock", BOOL, N, None)])
        .ui(),
    // UI build mode (Jason, 2026-11-20): the only time panels move.
    script("operandi.layout.buildMode", View, "UI build mode on:true or off; bare toggles. Panels, tabs and the mode bar move only in this mode, and every panel shows a drag pad to move or dock it by (Layout > UI Build Mode, Ctrl+Shift+L).")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.viewport.saveAs", View, "Saves the active view-area panel (group:false) or the whole view area (group:true) as a named piece under category:, reusable in any layout (Save Viewport As / Save Viewport Group As, LAY-72).")
        .operandi()
        .args(&[arg("name", TEXT, N), opt("category", TEXT, N, Some("User")), opt("group", BOOL, N, Some("false"))])
        .example("operandi.viewport.saveAs Front category:Views")
        .ui(),
    script("operandi.viewport.save", View, "Saves the piece the active panel (or the view area) shows again; every layout that uses it updates (LAY-72).")
        .operandi()
        .ui(),
    script("operandi.viewport.use", View, "Shows the saved piece name: in panel: (the active one without it): a viewport piece replaces the panel, a group piece fills its place (LAY-72).")
        .operandi()
        .args(&[arg("name", TEXT, N), opt("panel", INT, N, None)])
        .example("operandi.viewport.use Front")
        .ui(),
    script("operandi.viewport.revert", View, "Puts the piece the active panel (or the view area) shows back to its saved version (LAY-72).")
        .operandi()
        .ui(),
    script("operandi.viewport.delete", View, "Deletes the saved piece name:; panels showing it keep what they show (Delete Saved Viewport, LAY-72).")
        .operandi()
        .args(&[arg("name", TEXT, N)])
        .example("operandi.viewport.delete Front")
        .ui(),
    script("operandi.panel.kind", View, "Makes view-area panel: (the active one without it) show kind: (3D View, UV View or Empty), in the same place (the content arrow, LAY-73).")
        .operandi()
        .args(&[arg("kind", TEXT, N), opt("panel", INT, N, None)])
        .example("operandi.panel.kind {UV View}")
        .ui(),
    script("operandi.panel.show", View, "Brings the registered panel kind: to the front, or restores it on the right when missing.")
        .operandi()
        .args(&[arg("kind", TEXT, N)])
        .example("operandi.panel.show kind:{Shader Tree}")
        .ui(),
    script("operandi.toolbox.set", Tools, "Shows a toolbox set (Model, UV, Brush, Setup, Animation, Workbench or Game) without switching layouts or changing the views; paint is the old name of Brush.")
        .operandi()
        .args(&[arg("set", ArgType::Choice(&["model", "uv", "brush", "setup", "animation", "workbench", "game", "paint"]), N)])
        .example("operandi.toolbox.set uv")
        .ui(),
    // A layout's open command (LAY-02): which toolbox tab opens.
    script("operandi.toolbox.tab", Tools, "Opens a toolbox tab, switching to the first toolbox set that has it: create, select, deform, duplicate, polygon, edge, vertex, edit, topology, curve, sculpt, uv, setup, constraints, weighting, rig, baking, animation or workbench. The Model and UV layouts open with it.")
        .operandi()
        .args(&[arg("tab", ArgType::Choice(&["create", "select", "deform", "duplicate", "polygon", "edge", "vertex", "edit", "topology", "curve", "sculpt", "uv", "setup", "constraints", "weighting", "rig", "baking", "animation", "workbench"]), N)])
        .example("operandi.toolbox.tab uv")
        .ui(),
    // The Help menu (Jason 2026-11-16): Report Bug moved off the layout bar.
    // Report Bug's capture (BUG-19): bare opens the dialog; all four
    // options skip it (UIC-162).
    script("operandi.help.reportBug", Help, "Opens the bug report form on GitHub in the browser, filled in with this build and system and, as chosen, the tool and selection, recent commands and recent log; a screenshot of the window is saved in the reports folder and put on the clipboard to paste in. The form links to the idea form. Nothing is sent until you submit it there. Give all four options to skip the dialog.")
        .operandi()
        .args(&[
            opt("screenshot", BOOL, N, Some("true")),
            opt("toolState", BOOL, N, Some("true")),
            opt("commands", BOOL, N, Some("true")),
            opt("log", BOOL, N, Some("true")),
        ])
        .example("operandi.help.reportBug screenshot:false toolState:true commands:true log:true")
        .ui().side_effect(),
    script("operandi.help.documentation", Help, "Opens the User Guide for this version of Operandi in the browser (the test guide for a test build).")
        .operandi()
        .ui(),
    // Help > What's This? and F1 (UIC-06, help_mode.rs).
    script("operandi.help.whatsThis", Help, "Help mode: the next click on any control, panel or view opens its page in the User Guide instead of using it. Esc or a right-click leaves without opening anything; run it again to leave too.")
        .operandi()
        .aliases(&["What's This?"])
        .ui(),
    // Help > Check for Updates (Jason 2026-11-23; update_check.rs): an
    // Operandi-only feature, so an operandi.* name (UIC-165).
    script("operandi.update.check", Help, "Asks GitHub for the newest build on the Update Channel (Preferences > Defaults > Application) and offers to open its download page when it is newer than this one. Always reports back. Nothing is downloaded or installed.")
        .operandi()
        .ui().side_effect(),
    // Menus as forms (follows layouts stage 7, LAY-Q66): a command for
    // every menu entry that had none, so every menu is a form and every
    // menu click is a Command History line (UIC-160). Placeholders until
    // the name map names them (NAMES-NEEDED.md).
    script("operandi.scene.current", Files, "Makes the open scene at index: (from 0, the File > Current list's order) the current one.")
        .operandi()
        .args(&[arg("index", INT, N)])
        .example("operandi.scene.current index:0")
        .ui(),
    script("operandi.project.new", Files, "Makes a project folder at path: called name: and sets it; without path: opens the New Project dialog.")
        .operandi()
        .args(&[opt("path", ArgType::Path, N, None), opt("name", TEXT, N, None)])
        .ui().side_effect(),
    script("operandi.project.set", Files, "Sets the project to the folder path:; without path: opens Set Project.")
        .operandi()
        .args(&[opt("path", ArgType::Path, N, None)])
        .ui(),
    script("operandi.project.openRecent", Files, "Sets the project to Recent Projects' entry index: (from 0, newest first).")
        .operandi()
        .args(&[opt("index", INT, N, None)])
        .example("operandi.project.openRecent index:0")
        .ui(),
    script("operandi.project.loadPreset", Presets, "Loads the project's copy of preset: (layout, viewportDisplay or inputMap); it becomes your own setting.")
        .operandi()
        .args(&[arg("preset", ArgType::Choice(&["layout", "viewportDisplay", "inputMap"]), N)])
        .example("operandi.project.loadPreset preset:layout")
        .ui(),
    script("operandi.project.savePreset", Presets, "Saves your preset: (layout, viewportDisplay or inputMap) into the project for others.")
        .operandi()
        .args(&[arg("preset", ArgType::Choice(&["layout", "viewportDisplay", "inputMap"]), N)])
        .example("operandi.project.savePreset preset:inputMap")
        .ui().side_effect(),
    script("operandi.inputMap", Preferences, "Opens the Input Map: every action and its key.")
        .operandi()
        .ui(),
    script("operandi.select.polygonTagType", Selection, "Sets the polygon tag type Materials mode selects by: material, part, smoothingGroup or selectionSet.")
        .operandi()
        .args(&[arg("type", ArgType::Choice(&["material", "part", "smoothingGroup", "selectionSet"]), N)])
        .example("operandi.select.polygonTagType type:material")
        .ui(),
    script("operandi.lasso.style", Selection, "Sets the lasso's shape: freeform, rectangle, circle or ellipse.")
        .operandi()
        .args(&[arg("style", ArgType::Choice(&["freeform", "rectangle", "circle", "ellipse"]), N)])
        .example("operandi.lasso.style style:rectangle")
        .ui(),
    script("operandi.lasso.partial", Selection, "Partial lasso: also selects what the lasso only touches; bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.view.showLocked", ViewportOptions, "Shows locked mesh items in a colour of their own; bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.view.showFalloffs", ViewportOptions, "Tints points by the falloff's weight (FO-06); bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.viewport.close", View, "Closes the active view (a window keeps its last view).")
        .operandi()
        .ui(),
    script("operandi.help.navigation", Help, "Shows the mouse navigation of the current preset in the status line.")
        .operandi()
        .ui(),
    script("operandi.window.diagnostics", Debug, "Opens the Diagnostics window.")
        .operandi()
        .ui(),
    script("operandi.window.nodeView", Shading, "Opens the node view of the selected Material Group, or of the group the selected layer is in; with nothing suitable selected, the group last shown (Window > Node View). With the node view open and in front, closes it (maximised, first restores its group).")
        .operandi()
        .ui(),
    script("operandi.window.frameTime", Debug, "Shows the frame time over the view; bare toggles.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui(),
    script("operandi.mcp.bridge", Debug, "Turns the MCP bridge (scripting from outside the app) on or off; bare toggles. macOS and Linux only.")
        .operandi()
        .args(&[opt("on", BOOL, N, None)])
        .ui().side_effect(),
    script("operandi.window.size", View, "Sets the main window to a standard size: compact (1100 x 720) or standard (1440 x 900).")
        .operandi()
        .args(&[arg("size", ArgType::Choice(&["compact", "standard"]), N)])
        .example("operandi.window.size size:compact")
        .ui(),
    script("pbview.basePathAdd", Presets, "Adds a folder as a Preset Browser root; basePathKey: is its label.")
        .args(&[
            arg("path", ArgType::Path, N),
            opt("basePathKey", TEXT, N, None),
            opt("set", TEXT, N, None),
        ])
        .ui(),
    script("pbview.basePathRemove", Presets, "Takes a root out of the Preset Browser; its files stay on disk.")
        .args(&[opt("path", ArgType::Path, N, None), opt("basePathKey", TEXT, N, None)])
        .ui(),
    script("pbview.refresh", Presets, "Reads the Preset Browser's folders again.").ui(),
    script("pbview.mode", Presets, "The Preset Browser's view: vertical (0, was thumbnails), list (1) or horizontal (2); ? returns the name. The numbers never change.")
        .args(&[arg("mode", ArgType::Choice(&["vertical", "list", "horizontal"]), N)])
        .ui(),
    script("pbview.sortMode", Presets, "Sorts the Preset Browser's contents: name, type, modified or size.")
        .args(&[arg("mode", ArgType::Choice(&["name", "type", "modified", "size"]), N)])
        .ui(),
    script("pbview.filterString", Presets, "Searches the shown folder and everything below it.")
        .args(&[opt("filter", TEXT, N, None)])
        .ui(),
    script("pbview.filterStringEnable", Presets, "Shows or hides the Preset Browser's search (bare toggles).")
        .args(&[opt("state", BOOL, N, None)])
        .ui(),
    script("operandi.pbview.thumbnailSize", Presets, "The Preset Browser's thumbnail size in points (48 to 192).")
        .operandi()
        .args(&[arg("size", ArgType::Float, N)])
        .ui(),
];
