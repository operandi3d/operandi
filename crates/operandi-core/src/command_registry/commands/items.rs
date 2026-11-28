//! Item commands and the Mesh Stack (one section of `COMMANDS`, in table order).
use super::*;

pub(super) const COMMANDS: &[Entry] = &[
    // ---------------------------------------------------------- items
    script("item.create", Items, "Adds one item after the selected item.")
        .headless(
            "Add an item from the Items list's Add menu (camera, a light, a mesh...), alone selected.",
            &[jreq(
            "type",
            "string",
            "Item type key; see the `types` list in command_help item.create.",
        )],
        )
        .args(&[arg("type", ITEM_TYPES, N)])
        .undo()
        .example("item.create mesh"),
    script("item.pattern", Items, "Selects, unselects or hides items whose names match a pattern.")
        .operandi()
        .headless(
            "Select, unselect or hide the items whose names match a pattern (Items list header).",
            &[
            jreq("action", "string", "\"select\", \"unselect\" or \"hide\"."),
            jopt(
                "pattern",
                "string",
                "Name pattern with * and ? wildcards; default \"*\".",
            ),
            jopt(
                "limit_to_children",
                "boolean",
                "Only descendants of the selected items; default false.",
            ),
        ],
        )
        .args(&[
            arg("action", ArgType::Choice(&["select", "unselect", "hide"]), N),
            opt("pattern", TEXT, N, Some("*")),
        ])
        .undo()
        .example("item.pattern select"),
    script("item.editorColor", Items, "Sets the selected items' Editor Color; bare reports it.")
        .operandi()
        .args(&[opt("color", TEXT, N, None)])
        .undo()
        .query(),
    // The MCP bridge's commands that had no typed form (extensibility
    // stage 2): their arguments are the bridge's JSON ones.
    script("mesh.new", Items, "Adds an empty mesh item and makes it active.")
        .operandi()
        .undo()
        .headless(
            "Add an empty mesh item and make it active.",
            &[jopt(
            "name",
            "string",
            "Name for the new mesh; default \"Mesh\" (numbered when taken).",
        )],
        ),
    // The map's Select Item (PD-Q12); item.select is its deprecated alias
    // (command_names::COMMAND_RENAMES).
    script("select.item", Selection, "Makes an item the active item (none clears the item selection); mode and mask are accepted, not used yet.")
        .during_transform_tool()
        .undo()
        .headless(
            "Make an item the active (selected) item (the map's Select Item; was item.select, still accepted). Doesn't change the component mode; the select tool's item argument does the same in any mode.",
            &[
            jopt(
                "item",
                "integer",
                "Item id; omit to clear the item selection.",
            ),
            jopt(
                "mode",
                "integer",
                "Accepted, not used yet (the map's mode).",
            ),
            jopt("mask", "string", "Accepted, not used yet (the map's mask)."),
        ],
        )
        .example("select.item item:2"),
    // N11 (Jason, 2026-11-16): Ctrl+G groups the selected items under a
    // new Group Locator (GL-04).
    action("Parent to Group Locator", Items, "Parents the selected items to a new Group Locator, in place.")
        .undo()
        .operandi(),
    script("item.delete", Items, "Deletes the active item; children:delete|root deletes every selected item.")
        .undo()
        .headless(
            "Delete the active item; with `children`, every selected item.",
            &[
            jopt(
                "children",
                "string",
                "\"delete\" deletes their child items too; \"root\" moves them to the scene root in place (default).",
            ),
            jopt(
                "item",
                "string",
                "A mesh operation to delete instead, written op:<item>:<op> (Mesh Stack).",
            ),
        ],
        ),
    script("item.name", Items, "Renames the active item.")
        .undo()
        .headless(
            "Rename the active item, or a shader-tree layer with layer.",
            &[
            jreq("name", "string", "New name, 1–1024 bytes."),
            jopt(
                "layer",
                "integer",
                "A shader-tree layer id to rename instead (Operandi's own argument).",
            ),
        ],
        )
        .example("item.name Base"),
    script("item.visibility", Items, "Toggles an item's visibility.")
        .operandi()
        .undo()
        .headless(
            "Toggle an item's visibility.",
            &[JSON_ITEM],
        ),
    script("item.parent", Items, "Parents an item to another, or to the scene root.")
        .undo()
        .headless(
            "Parent an item to another (or to the scene root).",
            &[
            JSON_ITEM,
            jopt(
                "parent",
                "integer",
                "New parent's id; omit for the scene root.",
            ),
            jopt(
                "keep_position",
                "boolean",
                "Keep the item where it is in the world; default true.",
            ),
            jopt(
                "position",
                "integer",
                "With item:op:<item>:<op>, the op's new place in evaluation order (0 runs first).",
            ),
        ],
        ),
    // Mesh Stack (mesh_stack_editor.rs), reference names from the registry
    // thread (MESHOPS-NAMES 2026-11-11). item.delete, item.parent,
    // item.channel and select.item take an op as item:op:<item>:<op>.
    script("meshop.create", Modeling, "Adds a mesh operation to the selected mesh, above the selected row of its stack.")
        .undo()
        .headless(
            "Add a mesh operation to the selected mesh, directly above the selected row of its Mesh Operations list (on top when none is selected). Polygons selected on the Base Mesh become its Select by Index.",
            &[
            jreq(
                "type",
                "string",
                "Op type, such as prim.cube.item, poly.bevel.item, pmodel.meshmerge (Merge Meshes), boolean.meshop.item, solidDrill.meshop.item or axisDrill.meshop.item; command_help meshop.create lists every type (types).",
            ),
            jopt("mode", "integer", "Accepted, not used yet."),
            jopt("category", "string", "Accepted, not used yet."),
            jopt("applyDefaultPreset", "boolean", "Accepted, not used yet."),
        ],
        )
        .example("meshop.create type:poly.bevel.item"),
    script("deformer.enable", Modeling, "Turns a mesh operation on or off.")
        .undo()
        .headless(
            "Turn a mesh operation on or off (its eye); off passes its input through.",
            &[
            jopt(
                "deformer",
                "string",
                "The op, op:<item>:<op>; default the selected one.",
            ),
            jopt("enable", "boolean", "Default true."),
        ],
        ),
    script("deformer.freeze", Modeling, "Freezes a mesh operation and those below it into the Base Mesh; duplicate:true makes a new mesh item.")
        .undo()
        .headless(
            "Freeze a mesh operation and every op below it into the Base Mesh; the ops above stay live. With duplicate, the frozen copy is a new mesh item.",
            &[
            jopt(
                "duplicate",
                "boolean",
                "Freeze into a new mesh item; default false.",
            ),
            jopt(
                "deformer",
                "string",
                "The op, op:<item>:<op>; default the selected one.",
            ),
            jopt("mesh", "integer", "Accepted, not used yet."),
        ],
        ),
    // Wrap's Rebind button as a command (MOQ-68 batch 14), so scripts can
    // bind; Operandi's own name.
    script("deformer.rebind", Modeling, "Rebinds a Wrap mesh operation to its Target, as things are now.")
        .operandi()
        .undo()
        .headless(
            "Rebind a Wrap mesh operation, as its Rebind button does: stick each point of the step's input to its nearest spot on the Target as they are now, and save that binding with the step. Refuses when the op isn't a Wrap or has no Target.",
            &[jopt(
                "deformer",
                "string",
                "The Wrap op, op:<item>:<op>; default the selected one.",
            )],
        )
        .example("deformer.rebind deformer:op:2:5"),
    script("mesh.freeze", Modeling, "Freezes the selected mesh's whole Mesh Stack.")
        .undo()
        .headless(
            "Freeze the selected mesh's whole Mesh Stack into a plain mesh.",
            &[],
        ),
    script("mirror.clone.create", Modeling, "Adds a Mirror mesh operation to the selected mesh.")
        .undo()
        .headless(
            "Add a Mirror mesh operation (a Clone Effector with a pmodel.mirror generator) to the selected mesh, as meshop.create does.",
            &[],
        ),
    script("array.clone.create", Modeling, "Adds an Array mesh operation to the selected mesh.")
        .undo()
        .headless(
            "Add an Array mesh operation (a Clone Effector with a pmodel.array generator) to the selected mesh, as meshop.create does.",
            &[],
        ),
    script("deformer.selectBaseMesh", Modeling, "Edit Base Mesh on or off for a mesh with a Mesh Stack.")
        .headless(
            "Edit Base Mesh on or off: direct tools then edit the mesh's Base Mesh, with the stack's result drawn as a ghost.",
            &[JSON_ITEM, jopt("select", "boolean", "Default true.")],
        ).undo(),
    // MeshBlend (meshblend/commands.rs): live smooth booleans. item is the
    // MeshBlend Item (default the selected one); source and of are mesh ids.
    script("meshblend.new", Modeling, "Makes a MeshBlend Item from the selected meshes, all as Primaries.")
        .operandi()
        .undo()
        .headless(
            "Make a MeshBlend Item: a new mesh whose Mesh Stack blends the selected meshes (all Primaries, unioned) as smooth surfaces. The new item is selected.",
            &[],
        )
        .example("meshblend.new"),
    script("meshblend.addSource", Modeling, "Adds a mesh to a MeshBlend Item as a Primary or a Trim.")
        .operandi()
        .undo()
        .headless(
            "Add a mesh to a MeshBlend Item with a role. A Trim acts on the Primaries in of, or on every Primary when of is left out.",
            &[
                JSON_ITEM,
                jreq("source", "integer", "The mesh item to add."),
                jopt("role", "string", "primary (default), subtract or intersect."),
                jopt("of", "array", "For a Trim: the Primary mesh ids it acts on."),
            ],
        )
        .example("meshblend.addSource source:7 role:subtract"),
    script("meshblend.role", Modeling, "Changes the role of a MeshBlend source.")
        .operandi()
        .undo()
        .headless(
            "Change a MeshBlend source's role (primary, subtract or intersect). A new Trim acts on every Primary until Trim or Untrim changes that.",
            &[
                JSON_ITEM,
                jreq("source", "integer", "The source mesh."),
                jreq("role", "string", "primary, subtract or intersect."),
            ],
        ),
    script("meshblend.trim", Modeling, "Makes MeshBlend Trims act on more Primaries.")
        .operandi()
        .undo()
        .headless(
            "Relate Trims to Primaries: each Trim in source then acts on each Primary in of.",
            &[
                JSON_ITEM,
                jreq("source", "array", "The Trim mesh id, or a list of them."),
                jreq("of", "array", "The Primary mesh id, or a list of them."),
            ],
        ),
    script("meshblend.untrim", Modeling, "Stops MeshBlend Trims acting on some Primaries.")
        .operandi()
        .undo()
        .headless(
            "Remove relationships: each Trim in source stops acting on each Primary in of.",
            &[
                JSON_ITEM,
                jreq("source", "array", "The Trim mesh id, or a list of them."),
                jreq("of", "array", "The Primary mesh id, or a list of them."),
            ],
        ),
    script("meshblend.clearRole", Modeling, "Takes a mesh out of a MeshBlend Item.")
        .operandi()
        .undo()
        .headless(
            "Take a source out of a MeshBlend Item; it is an ordinary mesh again.",
            &[JSON_ITEM, jreq("source", "integer", "The source mesh.")],
        ),
    script("meshblend.source", Modeling, "Sets a MeshBlend source's settings.")
        .operandi()
        .undo()
        .headless(
            "Set a MeshBlend source's settings: enabled (in the result or not), subdivision (0-3, how finely its smooth surface becomes polygons) and keepCage (use the cage's points).",
            &[
                JSON_ITEM,
                jreq("source", "integer", "The source mesh."),
                jopt("enabled", "boolean", "In the result or not."),
                jopt("subdivision", "integer", "0 to 3; 0 uses the cage as it is."),
                jopt("keepCage", "boolean", "Use the cage's points instead of the smooth surface."),
            ],
        ),
    script("meshblend.strip", Modeling, "Sets one MeshBlend strip's own width and profile.")
        .operandi()
        .undo()
        .headless(
            "Give one strip of a MeshBlend Item its own Width and Profile (Override Defaults), by the strip's stable id as the MeshBlend panel lists it. clear removes the override.",
            &[
                JSON_ITEM,
                jreq("id", "string", "The strip, like 3-7-0 (its two sources' ids and its place)."),
                jopt("override", "boolean", "Override Defaults on or off; default on for a new override."),
                jopt("width", "number", "Its Strip Width, a distance."),
                jopt("profile", "number", "Its Strip Profile in percent, 0 to 200."),
                jopt("clear", "boolean", "Remove the override."),
            ],
        ),
    script("meshblend.convert", Modeling, "Turns a MeshBlend Item into an ordinary mesh.")
        .operandi()
        .undo()
        .headless(
            "Freeze a MeshBlend Item's result into an ordinary mesh: in place, or as a new mesh item with duplicate.",
            &[JSON_ITEM, jopt("duplicate", "boolean", "Make a new mesh item; default false.")],
        ),
    script("selop.indexcreate", Selection, "Gives the selected mesh operation a Select by Index of the selected polygons.")
        .undo()
        .headless(
            "Give the selected mesh operation a Select by Index of the polygons selected on its Base Mesh.",
            &[],
        ),
    script("selop.selection", Selection, "Sets, clears, adds to or removes from a mesh operation's Select by Index.")
        .undo()
        .headless(
            "Edit a mesh operation's Select by Index from the polygons selected on its Base Mesh.",
            &[
            jopt(
                "mode",
                "string",
                "set (default), clear, add or remove; or 0 to 3.",
            ),
            jopt(
                "item",
                "string",
                "The op, op:<item>:<op>; default the selected one.",
            ),
        ],
        ),
    script("selop.create", Selection, "Adds a selection operation to a mesh operation's Selection input.")
        .operandi()
        .undo()
        .headless(
            "Add a selection operation (index.selop.item, selbyrange.selop.item, pattern.selop.item, previous.selop, operandi.selset.selop) on top of a mesh operation's Selection input.",
            &[
                jreq("type", "string", "The selection operation's type."),
                jopt("item", "string", "The op, op:<item>:<op>; default the selected one."),
            ],
        ),
    script("selop.edit", Selection, "Changes or removes one of a mesh operation's selection operations.")
        .operandi()
        .undo()
        .headless(
            "Change values of, or remove, a mesh operation's selection operation (0 = bottom).",
            &[
                jreq("index", "integer", "Which selection operation, from the bottom (0)."),
                jopt("values", "object", "Values to set: enable, invert, blendmode (override, add, subtract, multiply), and the kind's own (index, start, end, step, pattern, source, name)."),
                jopt("remove", "boolean", "Remove it instead."),
                jopt("item", "string", "The op, op:<item>:<op>; default the selected one."),
            ],
        ),
    script("item.drop", Items, "Runs a Drop Action from the sources onto a destination item.")
        .operandi()
        .undo()
        .headless(
            "Run a Drop Action, as when dragging items onto another in the Items list.",
            &[
            jreq(
                "action",
                "string",
                "Drop action key; see the `actions` list in command_help item.drop.",
            ),
            jreq("destination", "integer", "The item dropped onto."),
            jopt(
                "sources",
                "array of integers",
                "Items dropped; default the item selection in order.",
            ),
            jopt(
                "ray_origin",
                "[x, y, z]",
                "Place / Place and Align only: world-space ray start, metres.",
            ),
            jopt(
                "ray_direction",
                "[x, y, z]",
                "Place / Place and Align only: world-space ray direction.",
            ),
        ],
        )
        .example("item.drop action:match_position destination:1"),
    script("item.transform", Items, "Sets an item's translation, rotation (radians) or scale.")
        .during_transform_tool()
        .operandi()
        .undo()
        .headless(
            "Set an item's transform (one undo step). Values are absolute; channels not given keep their value.",
            &[
            JSON_ITEM,
            jopt(
                "translation",
                "[x, y, z]",
                "Position relative to the parent, metres, absolute.",
            ),
            jopt(
                "rotation",
                "[x, y, z]",
                "Euler angles, radians, absolute (X then Y then Z, as scene_info reports).",
            ),
            jopt(
                "scale",
                "[x, y, z]",
                "Scale factors, absolute (1 = unscaled).",
            ),
        ],
        ),
    script("symmetry.set", Tools, "Sets symmetry: axis off|x|y|z and offset.")
        .operandi()
        .undo()
        .headless(
            "Set modelling symmetry.",
            &[
            jopt(
                "axis",
                "string",
                "\"off\" (default), \"x\", \"y\" or \"z\".",
            ),
            jopt(
                "offset",
                "number",
                "Symmetry plane offset along the axis, metres; default 0.",
            ),
        ],
        ),
    script("select.symmetrical", Selection, "Selects the symmetrical counterparts of the selection.")
        .undo()
        .headless(
            "Add the symmetrical counterparts of the selection.",
            &[],
        ),
    script("pref.value", Preferences, "Sets (or with ? reads) one preference by name; saves it like the Preferences window. Names are Operandi's own for now.")
        .args(&[arg("name", TEXT, N), arg("value", TEXT, N)])
        .query()
        .example("pref.value input.trackball ?"),
];
