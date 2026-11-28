//! Animation commands (one section of `COMMANDS`, in table order).
use super::*;

pub(super) const COMMANDS: &[Entry] = &[
    // Animation (BEHAVIOR-TIME-KEYS, BEHAVIOR-TIMELINE; anim_commands.rs).
    // Operandi names (no reference names in the map; UIC-165 provisional).
    script("operandi.anim.query", Items, "Reads the scene's time settings, playback, pending edits, markers and animated channels with their keys and values (UIC-163), as JSON.")
        .on_editor(
            "Read animation state as JSON: current frame, fps, scene and current range, time display, Auto Key, snap, real time, end behavior, key time, playing, pending edits, markers, and each animated channel's keys (frame, value, slopes), value now and values at frames.",
            &[
            jopt("channels", "string", "Only these channels, item:name,..."),
            jopt("item", "integer", "Only this item's channels."),
            jopt("frames", "string", "Frames to read each curve at, comma separated."),
        ],
            crate::anim_commands::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.anim.query frames:0,12,24"),
    script("operandi.rig.query", Items, "Reads the rig pieces (all, or one item's) with their channels, inputs, outputs and state, and the item's evaluated world, driven channels and pivot, as JSON (UIC-163).")
        .on_editor(
            "Read rig pieces as JSON: id, kind, label, item, targets, refs, channels (name: value), missing, broken, inputs, outputs; with item, also its evaluated world (position, rotation XYZ radians, scale, axes), driven channels, whether it drives a piece, and its pivot.",
            &[jopt("item", "integer", "Only this item's pieces, plus its evaluated world.")],
            crate::rig_commands::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.rig.query"),
    script("operandi.rig.compensation", Items, "Compensation (RG-11/RG-30): rig edits keep items where they are in world space.")
        .on_editor(
            "Turn Compensation on or off (on: toggle, true or false). Rig commands default to it.",
            &[jopt("on", "string", "toggle (default), true or false.")],
            crate::rig_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.rig.compensation on:true"),
    script("operandi.rig.constrain", Items, "Applies a constraint (RG-26..46): the first picked item follows the others, or with multiple each item follows the last.")
        .on_editor(
            "Apply a constraint, one undo step. kind: position, rotation, scale, transform, direction, distance, path, pathPosition or pathNormal. items and targets as id lists (default the selection: first picked = item, the rest = targets; multiple:true = every item but the last gets its own constraint on the last). compensate keeps the item where it is (default Compensation).",
            &[
            jreq("kind", "string", "Constraint kind."),
            jopt("items", "string", "Constrained item ids, comma separated."),
            jopt("targets", "string", "Target item ids (curve items for paths), comma separated."),
            jopt("multiple", "boolean", "Each item gets its own constraint on the last selected."),
            jopt("compensate", "boolean", "Keep the item where it is (default Compensation)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.constrain kind:position items:3 targets:4"),
    script("operandi.rig.unconstrain", Items, "Removes every constraint of one kind from the items (Shift+click, RG-27).")
        .on_editor(
            "Remove constraints of kind from items (default the selection), one undo step.",
            &[
            jreq("kind", "string", "Constraint kind."),
            jopt("items", "string", "Item ids, comma separated."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.unconstrain kind:position items:3"),
    script("operandi.rig.remove", Items, "Deletes one rig piece and its keys.")
        .on_editor(
            "Delete a rig piece (its keys go too), one undo step; a dynamic parent with compensate keeps the item where it is.",
            &[
            jreq("piece", "integer", "Rig piece id."),
            jopt("compensate", "boolean", "Keep the item where it is (default Compensation)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.remove piece:7"),
    script("operandi.rig.rename", Items, "Renames a rig piece as the Items list shows it (an empty name goes back to its kind's label).")
        .on_editor(
            "Give a rig piece (constraint, link, modifier, IK solver) its own name in the Items list and Schematic, one undo step; an empty name goes back to the kind's label.",
            &[
            jreq("piece", "integer", "The rig piece."),
            jopt("name", "string", "The new name (empty: the kind's label)."),
        ],
            crate::rig_list::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.rename piece:7 name:Follow"),
    script("operandi.rig.set", Items, "Sets a rig piece's channel (Influence, a weight, Axis, Percent...).")
        .on_editor(
            "Set a channel of a rig piece, one undo step; Auto Key keys it as for any channel. Names as operandi.rig.query lists them (influence 0-1, weight.0, axis 0/1/2, roll radians, percent 0-1...).",
            &[
            jreq("piece", "integer", "Rig piece id."),
            jreq("channel", "string", "Channel name."),
            jreq("value", "number", "New value."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.set piece:7 channel:influence value:0.5"),
    script("operandi.rig.upVector", Items, "Assign Up Vector, Assign Axis Up Vector or Remove Up Vector on a direction constraint (RG-44).")
        .on_editor(
            "Up vector of a direction constraint: item (its position defines up), with axis X/Y/Z (that item's axis is up); no item removes it (back to the Up Vector channels). One undo step.",
            &[
            jreq("piece", "integer", "Direction constraint id."),
            jopt("item", "integer", "Up item id; omit to remove."),
            jopt("axis", "string", "X, Y or Z: use that item's axis."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.upVector piece:7 item:5"),
    script("operandi.rig.dynamicParentAdd", Items, "Add Parent (RG-20): the child gets a dynamic parent with these parents appended; the current parent becomes the last added.")
        .on_editor(
            "Add dynamic parents to item (default the selection: first picked = child, the rest = parents), one undo step. Index 0 is World. compensate keeps the child where it is.",
            &[
            jopt("item", "integer", "The child item id."),
            jopt("parents", "string", "Parent item ids, comma separated."),
            jopt("compensate", "boolean", "Keep the child where it is (default Compensation)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.dynamicParentAdd item:3 parents:4"),
    script("operandi.rig.dynamicParentSet", Items, "Switches the dynamic parent (RG-21..24): 0 = World, n = the n-th parent.")
        .on_editor(
            "Set the dynamic parent index, one undo step. compensate rewrites the offsets so nothing jumps; key (default: when Parent is animated and Auto Key is on) keys the switch and its offsets, stepped.",
            &[
            jopt("item", "integer", "The child item id."),
            jopt("piece", "integer", "Or the dynamic parent piece id."),
            jreq("parent", "integer", "0 = World, 1.. = the parents in order."),
            jopt("compensate", "boolean", "Keep the child where it is (default Compensation)."),
            jopt("key", "boolean", "Key the switch and offsets now."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.dynamicParentSet item:3 parent:0"),
    script("operandi.rig.dynamicParentRemove", Items, "Remove Dynamic Parent (RG-25).")
        .on_editor(
            "Delete the item's dynamic parent, one undo step; compensate rewrites the item's own transform so it stays where it is now.",
            &[
            jopt("item", "integer", "The child item id (default the selection)."),
            jopt("compensate", "boolean", "Keep the item where it is (default Compensation)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.dynamicParentRemove item:3"),
    script("operandi.rig.parent", Items, "Parent (RG-11/12): the last picked item becomes the parent of the others.")
        .on_editor(
            "Parent items to parent (default the selection: the last picked is the parent), one undo step; compensate keeps them where they are.",
            &[
            jopt("items", "string", "Child item ids, comma separated."),
            jopt("parent", "integer", "Parent item id."),
            jopt("compensate", "boolean", "Keep them where they are (default Compensation)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.parent items:3 parent:4"),
    script("operandi.rig.unparent", Items, "Unparent (RG-11): the items move to the scene root.")
        .on_editor(
            "Unparent items (default the selection), one undo step; compensate keeps them where they are.",
            &[
            jopt("items", "string", "Item ids, comma separated."),
            jopt("compensate", "boolean", "Keep them where they are (default Compensation)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.unparent items:3"),
    script("operandi.rig.insert", Items, "Insert (RG-12): the first item becomes the second's parent, in the second's old place.")
        .on_editor(
            "Insert item above child in the hierarchy, one undo step; compensate keeps them where they are.",
            &[
            jopt("item", "integer", "The new parent."),
            jopt("child", "integer", "The item that goes under it."),
            jopt("compensate", "boolean", "Keep them where they are (default Compensation)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.insert item:4 child:3"),
    script("operandi.rig.extract", Items, "Extract (RG-12): the item leaves its chain; its children move up to its parent.")
        .on_editor(
            "Extract item from the hierarchy, one undo step; compensate keeps everything where it is.",
            &[
            jopt("item", "integer", "Item id (default the selection)."),
            jopt("compensate", "boolean", "Keep them where they are (default Compensation)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.extract item:3"),
    script("operandi.rig.pivot", Items, "Sets an item's pivot (RG-14); the item's position compensates so the geometry stays.")
        .on_editor(
            "Set the pivot position and/or rotation (x,y,z; metres and radians, in the item's space), one undo step; the geometry doesn't move.",
            &[
            jopt("item", "integer", "Item id (default the active item)."),
            jopt("position", "string", "Pivot position x,y,z."),
            jopt("rotation", "string", "Pivot rotation x,y,z (radians)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.pivot item:3 position:0,1,0"),
    script("operandi.rig.linkAdd", Items, "Add Link (RG-50..52): the driven channel takes the driver's value, directly or through a relationship curve.")
        .on_editor(
            "Link driven (item:channel) to driver (item:channel), one undo step. mode direct (default: the raw value, no unit conversion) or relationship (through a curve keyed with operandi.rig.relationshipKey). Groups (pos, a vector user channel) link component by component; text channels are refused.",
            &[
            jreq("driven", "string", "The driven channel, item:name."),
            jreq("driver", "string", "The driver channel, item:name."),
            jopt("mode", "string", "direct (default) or relationship."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.linkAdd driven:3:scl.Z driver:4:pos.X"),
    script("operandi.rig.linkRemove", Items, "Remove Link (RG-50).")
        .on_editor(
            "Remove a link by piece, or every link driving driven (item:channel, or a group), one undo step.",
            &[
            jopt("piece", "integer", "The link piece id."),
            jopt("driven", "string", "The driven channel, item:name."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.linkRemove driven:3:scl.Z"),
    script("operandi.rig.relationshipKey", Items, "Key (RG-52): stores a driver value → driven value pair on a relationship link.")
        .on_editor(
            "Store a key on a relationship link's curve: driver and driven values (default the driver's current value and the driven channel's own value), smooth-flat, holding outside the keys. One undo step.",
            &[
            jreq("piece", "integer", "The relationship link id."),
            jopt("driver", "number", "Driver value."),
            jopt("driven", "number", "Driven value."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.relationshipKey piece:7 driver:10 driven:1"),
    script("operandi.rig.relationshipKeyRemove", Items, "Delete Key (RG-52) on a relationship link.")
        .on_editor(
            "Delete the relationship key at driver (default the driver's current value), one undo step.",
            &[
            jreq("piece", "integer", "The relationship link id."),
            jopt("driver", "number", "Driver value of the key."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.relationshipKeyRemove piece:7 driver:10"),
    script("operandi.rig.userChannelAdd", Items, "Add User Channel (RG-53).")
        .on_editor(
            "Add a user channel to item (default the active item), one undo step. type float, distance, angle, percentage, time, integer, boolean, choice, string or divider; mode scalar, xy, xyz, rgb, rgba, uv or uvw (name.X... channels); default, min, max; choices for a choice.",
            &[
            jopt("item", "integer", "Item id (default the active item)."),
            jreq("name", "string", "Channel name (no spaces)."),
            jopt("display", "string", "Display name."),
            jopt("type", "string", "Value type (default float)."),
            jopt("mode", "string", "Components (default scalar)."),
            jopt("default", "number", "Default value."),
            jopt("min", "number", "Minimum."),
            jopt("max", "number", "Maximum."),
            jopt("choices", "string", "Choice names, comma separated."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.userChannelAdd item:3 name:crank type:angle"),
    script("operandi.rig.userChannelEdit", Items, "Edit User Channel (RG-53): rename, display name, range or default.")
        .on_editor(
            "Edit a user channel, one undo step; renaming keeps its keys and links.",
            &[
            jopt("item", "integer", "Item id (default the active item)."),
            jreq("name", "string", "Channel name."),
            jopt("rename", "string", "New name."),
            jopt("display", "string", "Display name."),
            jopt("min", "number", "Minimum."),
            jopt("max", "number", "Maximum."),
            jopt("default", "number", "Default value."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.userChannelEdit item:3 name:crank rename:spin"),
    script("operandi.rig.userChannelRemove", Items, "Delete User Channel (RG-53): removes it and the links using it.")
        .on_editor(
            "Delete a user channel, its keys and every link using it, one undo step (the status line says how many links went).",
            &[
            jopt("item", "integer", "Item id (default the active item)."),
            jreq("name", "string", "Channel name."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.userChannelRemove item:3 name:crank"),
    script("operandi.rig.userChannelSet", Items, "Set User Channel (RG-53): a user channel's value, keyed per Auto Key.")
        .on_editor(
            "Set a user channel's value (name.X for one component of a vector channel), one undo step; Auto Key keys it like any channel edit.",
            &[
            jopt("item", "integer", "Item id (default the active item)."),
            jreq("name", "string", "Channel name."),
            jreq("value", "number", "The value: percents 0–1, angles radians, true/false for a boolean."),
        ],
            crate::rig_user_set::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.userChannelSet item:3 name:crank value:0.5"),
    script("operandi.channelSet.create", Items, "Create Channel Set (CP-10): a channel panel from the selected channels; it becomes current.")
        .on_editor(
            "Create a channel set from the named or selected channels, one undo step; it becomes the current set and its panel shows over the 3D view.",
            &[
            jopt("name", "string", "Its name (default Channel Set)."),
            jopt("channels", "string", "item:name,... (default the channel selection, UIC-119)."),
        ],
            crate::channel_sets::run,
        )
        .undo()
        .operandi()
        .example("operandi.channelSet.create name:Ship channels:3:uPct,5:rot.Z"),
    script("operandi.channelSet.add", Items, "Add Selected Channels (CP-20) to the current channel set.")
        .on_editor(
            "Add channels to a channel set, one undo step; ones already in it stay where they are.",
            &[
            jopt("set", "integer", "Channel set id (default the current set)."),
            jopt("channels", "string", "item:name,... (default the channel selection, UIC-119)."),
        ],
            crate::channel_sets::run,
        )
        .undo()
        .operandi()
        .example("operandi.channelSet.add channels:5:rot.Y"),
    script("operandi.channelSet.remove", Items, "Remove Selected Channels (CP-22) from the current channel set.")
        .on_editor(
            "Remove channels from a channel set (all:true empties it), one undo step.",
            &[
            jopt("set", "integer", "Channel set id (default the current set)."),
            jopt("channels", "string", "item:name,... (default the channel selection, UIC-119)."),
            jopt("all", "boolean", "Remove every channel."),
        ],
            crate::channel_sets::run,
        )
        .undo()
        .operandi()
        .example("operandi.channelSet.remove channels:5:rot.Y"),
    script("operandi.channelSet.move", Items, "Move a channel's row in its channel set (CP-23).")
        .on_editor(
            "Move one row of a channel set to to: (0 is the top) or by: rows, one undo step.",
            &[
            jopt("set", "integer", "Channel set id (default the current set)."),
            jreq("channel", "string", "The row, item:name."),
            jopt("to", "integer", "New place, 0 = top."),
            jopt("by", "integer", "Rows to move, negative = up."),
        ],
            crate::channel_sets::run,
        )
        .undo()
        .operandi()
        .example("operandi.channelSet.move channel:5:rot.Y by:-1"),
    script("operandi.channelSet.rename", Items, "Rename a channel set.")
        .on_editor(
            "Rename a channel set, one undo step.",
            &[
            jopt("set", "integer", "Channel set id (default the current set)."),
            jreq("name", "string", "New name."),
        ],
            crate::channel_sets::run,
        )
        .undo()
        .operandi()
        .example("operandi.channelSet.rename name:Controls"),
    script("operandi.channelSet.delete", Items, "Delete a channel set (its channels stay).")
        .on_editor(
            "Delete a channel set, one undo step; the channels themselves are untouched.",
            &[
            jopt("set", "integer", "Channel set id (default the current set)."),
        ],
            crate::channel_sets::run,
        )
        .undo()
        .operandi()
        .example("operandi.channelSet.delete set:12"),
    script("operandi.channelSet.current", Items, "Picks the current channel set (CP-30); none closes the panel.")
        .on_editor(
            "Pick the channel set whose panel shows over the 3D view, or none; view state, not an undo step.",
            &[
            jopt("set", "string", "Channel set id, or none."),
        ],
            crate::channel_sets::run,
        )
        .ui()
        .operandi()
        .example("operandi.channelSet.current set:none"),
    script("operandi.rig.bindUserChannel", Items, "Bind into User Channel (RG-54): a user channel on target drives the channel.")
        .on_editor(
            "Add a user channel of the same kind to target (named after the channel unless name is given) and link the channel to it, one undo step; the channel becomes driven.",
            &[
            jreq("channel", "string", "The channel to drive, item:name."),
            jreq("target", "integer", "Item that gets the user channel."),
            jopt("name", "string", "User channel name."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.bindUserChannel channel:3:pos.Y target:4"),
    script("operandi.rig.modifierAdd", Items, "Adds a modifier (RG-60..62): Expression, Linear Blend, Math, Clamp or Switch; wired only as far as its arguments say (RG-71).")
        .on_editor(
            "Add a modifier, one undo step. kind expression, blend, math, clamp or switch; output item:channel it drives (optional: a modifier added without one only exists until wired); inputs as name=item:channel,... (A, B, Blend, Min, Max, Index, or an expression's variable names; up to 16); expr the expression (RG-60 syntax); op for math: add, subtract, multiply, divide, min, max or power.",
            &[
            jreq("kind", "string", "Modifier kind."),
            jopt("output", "string", "The driven channel, item:name."),
            jopt("inputs", "string", "Inputs, name=item:channel, comma separated."),
            jopt("expr", "string", "Expression text."),
            jopt("op", "string", "Math operation."),
            jopt("item", "integer", "Item it belongs to when it has no output yet."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.modifierAdd kind:expression output:3:rot.Z inputs:dist=4:pos.X expr:{dist / (2*pi*0.5) * 2*pi}"),
    script("operandi.rig.modifierSet", Items, "Wires or edits a modifier: its output, inputs (an empty channel unwires), expression or operation.")
        .on_editor(
            "Edit a modifier, one undo step: output item:channel, inputs name=item:channel,... (name= unwires), expr, op. Loops are refused.",
            &[
            jreq("piece", "integer", "The modifier id."),
            jopt("output", "string", "The driven channel, item:name."),
            jopt("inputs", "string", "Inputs, name=item:channel, comma separated."),
            jopt("expr", "string", "Expression text."),
            jopt("op", "string", "Math operation."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.modifierSet piece:7 inputs:B=4:pos.Y"),
    script("operandi.rig.bake", Items, "Bake Selected (§8): the evaluated world transform as plain keys per frame, rig pieces disabled.")
        .on_editor(
            "Bake items (default the selection) over the current range (or start..end frames): their evaluated transforms become keys on every frame and their rig pieces are disabled, not deleted. One undo step.",
            &[
            jopt("items", "string", "Item ids, comma separated."),
            jopt("start", "number", "First frame (default the current range's)."),
            jopt("end", "number", "Last frame (default the current range's)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.bake items:3 start:0 end:24"),
    script("operandi.rig.schematic", Items, "Reads the Schematic's nodes (items and rig pieces), their ports and wires as JSON (RG-70, UIC-163).")
        .on_editor(
            "Read the rig as Schematic nodes and wires: nodes (key item:<id> or piece:<id>, label, kind, inputs, outputs, broken, column) and wires (from, from_port, to, to_port, piece).",
            &[],
            crate::rig_commands::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.rig.schematic"),
    script("operandi.rig.schematicArrange", Items, "Arranges a Schematic workspace's nodes by data flow (SC-60): nodes moved by hand stay put unless named.")
        .on_editor(
            "Lay out a workspace's nodes left to right by data flow, ordered to cut wire crossings, one undo step. With nodes, just those (even hand-placed ones); without, every node not moved by hand since the last Arrange, placed below the hand-placed ones.",
            &[
            jreq("workspace", "integer", "The workspace id (operandi.rig.workspaces)."),
            jopt("nodes", "string", "Node keys to arrange, comma-separated (item:<id>, piece:<id>)."),
        ],
            crate::schematic_pages::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.schematicArrange workspace:40"),
    script("operandi.rig.schematicChannels", Items, "Shows channels on a Schematic workspace as ports of their items, putting items not on it yet at the drop (#54: channels dragged from Properties).")
        .on_editor(
            "Show channels as ports on a workspace, one undo step: each channel's item comes onto the page (at at:, else arranged below) and keeps the channel as an input and output port, wired or not.",
            &[
            jreq("workspace", "integer", "The workspace id."),
            jreq("channels", "string", "item:name pairs, comma-separated: 3:pos.X,3:rot.Y."),
            jopt("at", "string", "x/y on the page for items not on it yet."),
        ],
            crate::schematic_pages::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.schematicChannels workspace:40 channels:3:pos.X at:40/60"),
    script("operandi.rig.schematicPlace", Items, "Puts nodes on a Schematic workspace or moves them there (SC-10, SC-91); nodes given without a place arrive arranged.")
        .on_editor(
            "Place nodes on a workspace, one undo step: place puts each key at x/y (moving it if it is there already, marked as moved by hand unless pinned:false); nodes adds keys not on the page yet, arranged together below what is there (SC-60). Never changes the items or pieces.",
            &[
            jreq("workspace", "integer", "The workspace id."),
            jopt("place", "string", "key@x/y pairs, comma-separated: item:3@10/20,piece:7@-40/5."),
            jopt("nodes", "string", "Node keys to add without a place, comma-separated."),
            jopt("pinned", "boolean", "Mark placed nodes as moved by hand (default true)."),
        ],
            crate::schematic_pages::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.schematicPlace workspace:40 nodes:item:3,piece:7"),
    script("operandi.rig.schematicRemove", Items, "Takes nodes off a Schematic workspace (SC-40, SC-41); the items and pieces stay in the scene.")
        .on_editor(
            "Remove nodes from a workspace page, one undo step. The items and rig pieces are untouched.",
            &[
            jreq("workspace", "integer", "The workspace id."),
            jreq("nodes", "string", "Node keys, comma-separated (item:<id>, piece:<id>)."),
        ],
            crate::schematic_pages::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.schematicRemove workspace:40 nodes:item:3"),
    script("operandi.rig.workspaceAdd", Items, "New Schematic workspace (SC-01, SC-02): a named page, empty or with nodes at given places.")
        .on_editor(
            "Add a Schematic workspace, one undo step. place puts nodes on it at once (key@x/y pairs, as schematicPlace). The new workspace is the last in operandi.rig.workspaces.",
            &[
            jopt("name", "string", "The name (default Workspace)."),
            jopt("place", "string", "key@x/y pairs, comma-separated."),
        ],
            crate::schematic_pages::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.workspaceAdd name:Arms"),
    script("operandi.rig.workspaceClear", Items, "Empties a Schematic workspace (SC-06): its nodes leave the page, the scene keeps its items and pieces.")
        .on_editor(
            "Remove every node from a workspace, one undo step; nothing in the scene changes.",
            &[jreq("workspace", "integer", "The workspace id.")],
            crate::schematic_pages::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.workspaceClear workspace:40"),
    script("operandi.rig.workspaceRemove", Items, "Deletes a Schematic workspace (the page only; the scene keeps its items and pieces).")
        .on_editor(
            "Delete a workspace page, one undo step; nothing in the scene changes.",
            &[jreq("workspace", "integer", "The workspace id.")],
            crate::schematic_pages::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.workspaceRemove workspace:40"),
    script("operandi.rig.workspaceRename", Items, "Renames a Schematic workspace (SC-04).")
        .on_editor(
            "Rename a workspace, one undo step.",
            &[
            jreq("workspace", "integer", "The workspace id."),
            jreq("name", "string", "The new name."),
        ],
            crate::schematic_pages::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.workspaceRename workspace:40 name:Legs"),
    script("operandi.rig.workspaces", Items, "Reads the Schematic workspaces and their node places as JSON (SC-91, UIC-163).")
        .on_editor(
            "Read the saved Schematic workspaces: id, name and nodes (key, x, y, pinned).",
            &[],
            crate::schematic_pages::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.rig.workspaces"),
    script("operandi.rig.unwire", Items, "Deletes a Schematic wire (RG-72): a link goes, a modifier input or output is unwired, a constraint loses that target.")
        .on_editor(
            "Delete the wire at piece's port (a link: the link; a modifier: that input, or out; a constraint or dynamic parent: that target, the piece with its last), one undo step.",
            &[
            jreq("piece", "integer", "The rig piece id."),
            jreq("port", "string", "The piece's port (A, driver, target 2, out...)."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.rig.unwire piece:7 port:A"),
    script("operandi.anim.setup", Items, "Setup mode (SK-01): items show their setup (rest) pose, keys are ignored and transform edits change setup values.")
        .on_editor(
            "Turn Setup mode on or off (on: toggle, true or false). A view state: not an undo step.",
            &[jopt("on", "string", "toggle (default), true or false.")],
            crate::rig_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.anim.setup on:true"),
    script("operandi.anim.itemToSetup", Items, "Item to Setup (SK-03): the items' current values become their setup values.")
        .on_editor(
            "Copy the current (edit) values of the items' animated channels into their setup values, one undo step. channels limits it to names such as pos.Y.",
            &[
            jopt("items", "string", "Item ids (default the selection)."),
            jopt("channels", "string", "Only these channels, comma separated."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.anim.itemToSetup items:3"),
    script("operandi.anim.restoreSetup", Items, "Restore Setup (SK-03): the items' edit values go and their setup values come back.")
        .on_editor(
            "Throw away the edit values (the keys) of the items' animated channels and put back their setup values, one undo step. channels limits it to names such as pos.Y.",
            &[
            jopt("items", "string", "Item ids (default the selection)."),
            jopt("channels", "string", "Only these channels, comma separated."),
        ],
            crate::rig_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.anim.restoreSetup items:3"),
    script("operandi.ik.addGoal", Items, "Add Goal (IK-22): a box goal at the joint with a position and an orientation task wired to it.")
        .on_editor(
            "Add a goal at the joint's world place and position + orientation tasks on it (mode position, rotation or both), one undo step.",
            &[
            jopt("joint", "integer", "The joint (default the selected item)."),
            jopt("mode", "string", "position, rotation or both (default)."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.addGoal joint:7"),
    script("operandi.ik.addTask", Items, "Add Position Task / Add Orientation Task (IK-22): pins the joint where it is now.")
        .on_editor(
            "Add a position or orientation task with no goal (mode), holding the joint at its world position or orientation now; one undo step.",
            &[
            jopt("joint", "integer", "The joint (default the selected item)."),
            jreq("mode", "string", "position or orientation."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.addTask joint:9 mode:position"),
    script("operandi.ik.addToChain", Items, "Add to Chain (IK-26): joints (or branches) join their full-body IK chain.")
        .on_editor(
            "Add joints (default the selection) to the full-body IK chain above them, one undo step.",
            &[
            jopt("joints", "string", "Joint ids, comma separated."),
            jopt("piece", "integer", "The solver (default the one above the joints)."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.addToChain joints:12"),
    script("operandi.ik.assign", Items, "Assign IK (IK-20..21): the hierarchy under the root joint (or only its branches to the end joints) becomes one full-body IK chain; needs Setup mode.")
        .on_editor(
            "Make one full-body IK solver on the root joint covering its joint hierarchy (or the branches to the end joints listed after it), one undo step. Needs Setup mode; joints with scale are refused.",
            &[
            jopt("joints", "string", "The root, then any end joints (default the selection)."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.assign joints:1"),
    script("operandi.ik.assignGoal", Items, "Assign Goal (IK-22): an existing item becomes the joint's position, rotation or both goal.")
        .on_editor(
            "Wire tasks on the joint to goal (mode position, rotation or both), one undo step.",
            &[
            jopt("joint", "integer", "The joint (default the selected item)."),
            jreq("goal", "integer", "The goal item."),
            jopt("mode", "string", "position, rotation or both (default)."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.assignGoal joint:7 goal:20"),
    script("operandi.ik.remove", Items, "Remove IK (IK-26): removes an IK solver, its tasks and the IK settings of its joints.")
        .on_editor(
            "Remove the selected joint's (or piece's) IK solver with its tasks, joint IK settings and keys, one undo step.",
            &[
            jopt("piece", "integer", "The solver (default the selected chain's)."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.remove"),
    script("operandi.ik.removeFromChain", Items, "Remove from Chain (IK-26): a branch leaves its full-body IK chain from its base.")
        .on_editor(
            "Remove joints' branches (default the selection) from their full-body IK chain, one undo step.",
            &[
            jopt("joints", "string", "Joint ids, comma separated."),
            jopt("piece", "integer", "The solver (default the one above the joints)."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.removeFromChain joints:12"),
    script("operandi.ik.removeTask", Items, "Removes a joint's IK tasks (one kind, or all).")
        .on_editor(
            "Remove the joint's position or orientation task (kind) or both, one undo step.",
            &[
            jopt("joint", "integer", "The joint (default the selected item)."),
            jopt("kind", "string", "position or orientation (default both)."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.removeTask joint:9"),
    script("operandi.ik.taskSet", Items, "Task settings (IK-25): Enable, Weight, Precision, Number of Joints.")
        .on_editor(
            "Change the joint's tasks (kind, default both), one undo step: enable, weight, precision, joints (how far up the chain; -1 all).",
            &[
            jopt("joint", "integer", "The joint (default the selected item)."),
            jopt("kind", "string", "position or orientation (default both)."),
            jopt("enable", "boolean", "On or off."),
            jopt("weight", "number", "Pull (default 1)."),
            jopt("precision", "number", "Default 1."),
            jopt("joints", "number", "Joints up the chain it moves (-1 all)."),
        ],
            crate::ik_body::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.taskSet joint:9 weight:2"),
    script("operandi.ik.applyChain", Items, "Apply IK (Chain) (IK-15..17): a FABRIK solver from the root to the end joint (2 to 100 joints) with a goal at the end; needs Setup mode.")
        .on_editor(
            "Make the chain from root to end (default the selection: root, then end) one FABRIK IK chain, one undo step. Needs Setup mode. Adds <root>_IKGoal under <root>_IKRoot.",
            &[jopt("joints", "string", "Root and end joint ids, comma separated.")],
            crate::ik_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.applyChain joints:3,9"),
    script("operandi.ik.disable", Items, "Disable IK (IK-33): no IK solver runs in this scene (saved); joints keep their pose. Setup mode only.")
        .on_editor(
            "Turn every IK solver off or on (on: toggle, true or false), one undo step. Setup mode only.",
            &[
            jopt("on", "string", "toggle (default), true or false."),
        ],
            crate::ik_switch::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.disable on:true"),
    script("operandi.ik.match", Items, "Match Position / Rotation (IK-33): moves the goal onto its end joint.")
        .on_editor(
            "Move each solver's goal onto its end joint's world position, rotation or both (mode), one undo step.",
            &[
            jopt("piece", "integer", "One solver (default every solver of the selected joints and goals)."),
            jopt("mode", "string", "position, rotation or both (default)."),
        ],
            crate::ik_switch::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.match mode:position"),
    script("operandi.ik.selectSolvers", Items, "Select IK Solvers (IK-33): selects the joints the selection's IK solvers sit on.")
        .on_editor(
            "Select the root joints of the IK solvers of the selected joints and goals.",
            &[
            jopt("piece", "integer", "One solver (default every solver of the selected joints and goals)."),
        ],
            crate::ik_switch::run,
        )
        .ui()
        .operandi()
        .example("operandi.ik.selectSolvers"),
    script("operandi.ik.selectSwitch", Items, "Select Switch Channels (IK-32): selects the items whose channels the IK/FK switch keys.")
        .on_editor(
            "Select the chain's joints, goal and pole of the selected solvers (the switch channels' items).",
            &[
            jopt("piece", "integer", "One solver (default every solver of the selected joints and goals)."),
        ],
            crate::ik_switch::run,
        )
        .ui()
        .operandi()
        .example("operandi.ik.selectSwitch"),
    script("operandi.ik.setKeys", Items, "IK/FK: Set Keys (IK-32), or Delete Keys with delete:true, on every switch channel at the current time.")
        .on_editor(
            "Key (or with delete, remove the keys at the current time of) Blend, the chain's joints' position and rotation, the goal's position and rotation and the pole's position, one undo step.",
            &[
            jopt("piece", "integer", "One solver (default every solver of the selected joints and goals)."),
            jopt("delete", "boolean", "Delete the keys instead (Shift+click)."),
        ],
            crate::ik_switch::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.setKeys"),
    script("operandi.ik.switch", Items, "Switch to FK / Switch to IK / Toggle IK/FK (IK-31): copies one pose into the other and keys it, no pop.")
        .on_editor(
            "Switch the selected chains (mode toggle by their Blend, fk or ik), one undo step. To FK: the solved pose into the joints, keyed, Blend keyed 0. To IK: the goal and pole onto the FK pose, keyed, Blend keyed 1.",
            &[
            jopt("piece", "integer", "One solver (default every solver of the selected joints and goals)."),
            jopt("mode", "string", "toggle (default), fk or ik."),
        ],
            crate::ik_switch::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.switch mode:fk"),
    script("operandi.ik.updateRest", Items, "Update Rest Pose (IK-33): solvers take their bend side and twist reference from the joints as they are now.")
        .on_editor(
            "Re-read the rest pose of the selected solvers (or all), one undo step: the two-bone bend side and the chain's goal twist reference.",
            &[
            jopt("piece", "integer", "One solver (default every solver of the selected joints and goals)."),
            jopt("all", "boolean", "Every solver (default the selected)."),
        ],
            crate::ik_switch::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.updateRest all:true"),
    script("operandi.ik.applyTwoBone", Items, "Apply IK (Two-Bone) (IK-01..02): a goal at the end joint, a group holding it and the chain, and a two-bone solver on the root; needs Setup mode.")
        .on_editor(
            "Make three joints (root, middle, end; default the selection in pick order) a two-bone IK chain, one undo step. Needs Setup mode; the chain's bend there is the side it keeps. Adds <root>_IKGoal under <root>_IKRoot.",
            &[jopt("joints", "string", "Root, middle and end joint ids, comma separated.")],
            crate::ik_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.applyTwoBone joints:3,4,5"),
    script("operandi.ik.jointQuery", Items, "Reads a joint's IK settings and limits as JSON (IK-24, IK-40; angles in degrees).")
        .on_editor(
            "Read a joint's IK settings as JSON: active, enableLimits, limitFk, limitsAxis, maxVelocity and per axis X/Y/Z translate, translateWeight, rotate, rotateWeight, useLimits, min, max (degrees).",
            &[jopt("joint", "integer", "The joint (default the selected item).")],
            crate::ik_joint::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.ik.jointQuery joint:4"),
    script("operandi.ik.jointSet", Items, "Sets a joint's IK settings and limits (IK-24, IK-40..42): Active, Enable Limits, Limit FK, per-axis Use Limits and Min/Max in degrees.")
        .on_editor(
            "Change a joint's IK settings, one undo step: active, enableLimits, limitFk, limitsAxis X/Y/Z, maxVelocity (degrees), and per axis (X, Y, Z suffix) useLimits, min, max (degrees), translate, translateWeight, rotate, rotateWeight.",
            &[
            jopt("joint", "integer", "The joint (default the selected item)."),
            jopt("active", "boolean", "The joint takes part in IK (default true)."),
            jopt("enableLimits", "boolean", "Use the axes' limits (default false)."),
            jopt("limitFk", "boolean", "Limits clamp FK edits too (default false)."),
            jopt("limitsAxis", "string", "The axis down the bone: X, Y or Z (default Z)."),
            jopt("maxVelocity", "number", "Degrees a solver step may turn it."),
            jopt("useLimitsX", "boolean", "Limit X."),
            jopt("minX", "number", "Degrees."),
            jopt("maxX", "number", "Degrees."),
            jopt("translateX", "boolean", "Full-body IK may move it along X."),
            jopt("translateWeightX", "number", "Translation weight on X."),
            jopt("rotateX", "boolean", "Full-body IK may turn it about X (default true)."),
            jopt("rotateWeightX", "number", "0 locks, above 1 favours X."),
            jopt("useLimitsY", "boolean", "Limit Y."),
            jopt("minY", "number", "Degrees."),
            jopt("maxY", "number", "Degrees."),
            jopt("translateY", "boolean", "Full-body IK may move it along Y."),
            jopt("translateWeightY", "number", "Translation weight on Y."),
            jopt("rotateY", "boolean", "Full-body IK may turn it about Y (default true)."),
            jopt("rotateWeightY", "number", "0 locks, above 1 favours Y."),
            jopt("useLimitsZ", "boolean", "Limit Z."),
            jopt("minZ", "number", "Degrees."),
            jopt("maxZ", "number", "Degrees."),
            jopt("translateZ", "boolean", "Full-body IK may move it along Z."),
            jopt("translateWeightZ", "number", "Translation weight on Z."),
            jopt("rotateZ", "boolean", "Full-body IK may turn it about Z (default true)."),
            jopt("rotateWeightZ", "number", "0 locks, above 1 favours Z."),
        ],
            crate::ik_joint::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.jointSet joint:4 enableLimits:true useLimitsX:true minX:0 maxX:150"),
    script("operandi.ik.pole", Items, "Create, Assign or Remove IK Up Vector (IK-06): the elbow or knee points at the pole.")
        .on_editor(
            "mode create (default) adds a pole locator one bone out on the bend side; assign uses pole (any item); remove goes back to Orient. One undo step.",
            &[
            jopt("piece", "integer", "The solver (default the selected chain's)."),
            jopt("mode", "string", "create (default), assign or remove."),
            jopt("pole", "integer", "The item, for assign."),
        ],
            crate::ik_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.ik.pole mode:create"),
    script("operandi.ik.query", Items, "Reads IK solvers as JSON (UIC-163): chain joints' solved positions and rotations at a time, goal, pole, and whether the goal is met.")
        .on_editor(
            "Read IK solvers as JSON: piece, kind, root, goal, pole, blend, enable, joints (joint, name, position, rotation w,x,y,z), distance (end to goal) and met (within tolerance).",
            &[
            jopt("piece", "integer", "Only this solver."),
            jopt("time", "number", "Seconds (default the current time)."),
            jopt("tolerance", "number", "Met within this distance (default 0.001)."),
        ],
            crate::ik_commands::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.ik.query"),
    script("operandi.pose.drag", Items, "Pose tool drag (IK-50..54), scripted: a joint to a position with the chain up to the nearest pin following; or move the hierarchy, twist a joint, or turn one joint (FK).")
        .on_editor(
            "Pose a hierarchy, one undo step (Auto Key keys what moved). mode ik (default): joint to world position to, the chain up to the nearest pin (or the root) follows, pins hold; rigid: the whole hierarchy moves (by, or so joint reaches to); twist: the joint turns angle degrees about its bone; fk: only the joint turns, aiming its bone at to.",
            &[
            jopt("joint", "integer", "The joint (default the selected item)."),
            jopt("to", "string", "World position x,y,z."),
            jopt("by", "string", "Offset x,y,z (rigid)."),
            jopt("mode", "string", "ik (default), rigid, twist or fk."),
            jopt("angle", "number", "Degrees (twist)."),
            jopt("pins", "string", "Pinned joints, comma separated."),
            jopt("root", "integer", "The hierarchy's root (default the joint's top item)."),
            jopt("rootWeight", "number", "0 (default) keeps the root still; above 0 it moves too."),
            jopt("pinWeight", "number", "Position Pins weight (default 1)."),
            jopt("pinPrecision", "number", "Position Pins precision (default 1)."),
        ],
            crate::pose_tool::run,
        )
        .undo()
        .operandi()
        .example("operandi.pose.drag joint:9 to:0,1.6,0.3 pins:5,7"),
    script("operandi.skel.add", Items, "Adds a skeleton joint (SK-10): a child of parent, or a new root, with symmetry and naming (SK-14/15).")
        .on_editor(
            "Add a joint at a world position, as a child of parent (or a new chain's root), one undo step. symmetry X/Y/Z also adds the mirrored joint under the mirrored parent, named with the side suffixes; the parent is re-aimed at its new first child unless preserveAlignment.",
            &[
            jreq("position", "string", "World position x,y,z."),
            jopt("parent", "integer", "Parent joint id (none: a new root)."),
            jopt("symmetry", "string", "off (default), X, Y or Z."),
            jopt("prefix", "string", "Name prefix (SK-15)."),
            jopt("name", "string", "Base name (default Joint); a number is added."),
            jopt("suffixPlus", "string", "Suffix on the + side of the mirror plane (default _L)."),
            jopt("suffixMinus", "string", "Suffix on the - side (default _R)."),
            jopt("direction", "string", "Axis down the bone: X, Y or Z (default Z, SK-16)."),
            jopt("axis", "string", "Axis across the bone (roll): X, Y or Z (default X)."),
            jopt("preserveAlignment", "boolean", "Keep orientations instead of re-aiming (default false)."),
        ],
            crate::skeleton::run,
        )
        .undo()
        .operandi()
        .example("operandi.skel.add position:0,1,0"),
    script("operandi.skel.align", Items, "Aims joints down their bones (SK-16): Direction along the bone to the first child, Axis across it.")
        .on_editor(
            "Re-aim a joint (or the selected joints), and with chain its descendants, at their first children, one undo step. Children keep their world places.",
            &[
            jopt("joint", "integer", "The joint (default the selected joints)."),
            jopt("chain", "boolean", "Also its descendants (default true)."),
            jopt("direction", "string", "Axis down the bone: X, Y or Z (default Z, SK-16)."),
            jopt("axis", "string", "Axis across the bone (roll): X, Y or Z (default X)."),
            jopt("preserveAlignment", "boolean", "Keep orientations instead of re-aiming (default false)."),
        ],
            crate::skeleton::run,
        )
        .undo()
        .operandi()
        .example("operandi.skel.align joint:4 direction:Z axis:X"),
    script("operandi.skel.delete", Items, "Deletes a joint (SK-12); its children re-parent to its parent in place.")
        .on_editor(
            "Delete a joint, one undo step; its children move to its parent keeping their world places.",
            &[jreq("joint", "integer", "The joint.")],
            crate::skeleton::run,
        )
        .undo()
        .operandi()
        .example("operandi.skel.delete joint:4"),
    script("operandi.skel.insert", Items, "Inserts a joint halfway along a bone (SK-12), splitting it in two.")
        .on_editor(
            "Split the bone from joint to its first child with a new joint at its middle, one undo step.",
            &[
            jreq("joint", "integer", "The joint whose bone is split."),
            jopt("prefix", "string", "Name prefix (SK-15)."),
            jopt("name", "string", "Base name (default Joint); a number is added."),
            jopt("suffixPlus", "string", "Suffix on the + side of the mirror plane (default _L)."),
            jopt("suffixMinus", "string", "Suffix on the - side (default _R)."),
            jopt("direction", "string", "Axis down the bone: X, Y or Z (default Z, SK-16)."),
            jopt("axis", "string", "Axis across the bone (roll): X, Y or Z (default X)."),
            jopt("preserveAlignment", "boolean", "Keep orientations instead of re-aiming (default false)."),
        ],
            crate::skeleton::run,
        )
        .undo()
        .operandi()
        .example("operandi.skel.insert joint:4"),
    script("operandi.skel.move", Items, "Moves a joint (SK-12 Edit, SK-13 Compensation): with compensate the rest of the chain stays where it is.")
        .on_editor(
            "Move a joint to a world position, one undo step. compensate (default true) keeps its children in place; off, they move with it. The parent and the joint are re-aimed unless preserveAlignment.",
            &[
            jreq("joint", "integer", "The joint."),
            jreq("position", "string", "World position x,y,z."),
            jopt("compensate", "boolean", "Children stay put (default true)."),
            jopt("direction", "string", "Axis down the bone: X, Y or Z (default Z, SK-16)."),
            jopt("axis", "string", "Axis across the bone (roll): X, Y or Z (default X)."),
            jopt("preserveAlignment", "boolean", "Keep orientations instead of re-aiming (default false)."),
        ],
            crate::skeleton::run,
        )
        .undo()
        .operandi()
        .example("operandi.skel.move joint:4 position:0,2,0"),
    script("operandi.skel.query", Items, "Reads the skeleton's joints as JSON (UIC-163): parent, children, first child, world position, bone length and axes.")
        .on_editor(
            "Read joints as JSON: id, name, parent, firstChild, children, position, boneLength, axes X/Y/Z. root limits it to that joint's chain, depth first.",
            &[jopt("root", "integer", "Only this joint's chain.")],
            crate::skeleton::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.skel.query"),
    script("operandi.skin.bind", Items, "Bind (SK-20..23): weights for each joint under the root, one weight map per joint and a skin piece on each mesh; needs Setup mode.")
        .on_editor(
            "Bind the meshes (items, default the selected meshes) to the skeleton under root (default the last selected joint), one undo step. Needs Setup mode. A failed Heat bind changes nothing and selects the points to fix.",
            &[
            jopt("items", "string", "Mesh item ids, comma separated (default the selected meshes)."),
            jopt("root", "integer", "The root joint (default the last selected joint)."),
            jopt("type", "string", "rigid, distance, visible or heat (default heat)."),
            jopt("falloff", "string", "low, medium (default) or high."),
            jopt("segments", "boolean", "Bind as Segments: weight along each joint's line to its children (default true)."),
            jopt("limitWeights", "boolean", "Keep at most weightsPerVertex joints a point (default false)."),
            jopt("weightsPerVertex", "integer", "1 to 8 (default 4)."),
            jopt("minimumWeight", "number", "Drop weights below this, 0 to 1 (default 0)."),
            jopt("dualQuaternion", "boolean", "The skin blends by dual quaternions (default false)."),
        ],
            crate::skin_bind::run,
        )
        .undo()
        .operandi()
        .example("operandi.skin.bind type:heat"),
    script("operandi.skin.bakeWeights", Items, "Bake Weights (SK-34): the skin's effective weights go into its weight maps and its options back to neutral.")
        .on_editor(
            "Write the effective weights (after normalise, limit, threshold, sharpness, smooth influences) into the joints' weight maps and reset those options, one undo step.",
            &[jopt("item", "integer", "The bound mesh (default the selected one).")],
            crate::skin_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.skin.bakeWeights"),
    script("operandi.skin.query", Items, "Reads a bound mesh's skin as JSON (UIC-163): channels and joints; with vertex, its weights and deformed position at a time.")
        .on_editor(
            "Read the skin as JSON: item, op, interpolation, limitSources, maxSources, threshold, sharpness, world, joints (joint, name, map, enable, influence, missing). With vertex: weights (map: effective weight), rest and position (deformed, at time seconds, default now).",
            &[
            jopt("item", "integer", "The bound mesh (default the selected one)."),
            jopt("vertex", "integer", "A point index."),
            jopt("time", "number", "Seconds (default the current time)."),
        ],
            crate::skin_commands::run,
        )
        .query()
        .quiet()
        .operandi()
        .example("operandi.skin.query vertex:0"),
    script("operandi.skin.rebind", Items, "Rebind / Add Joints to Bind (SK-25): adds joints to a bound mesh's skin with empty weight maps; needs Setup mode.")
        .on_editor(
            "Add joints (default the selected joints) not yet in the skin of the meshes (items, default the selected meshes), with empty weight maps, leaving the other weights alone; one undo step. Needs Setup mode.",
            &[
            jopt("items", "string", "Mesh item ids (default the selected meshes)."),
            jopt("joints", "string", "Joint ids, comma separated (default the selected joints)."),
        ],
            crate::skin_bind::run,
        )
        .undo()
        .operandi()
        .example("operandi.skin.rebind joints:12"),
    script("operandi.skin.set", Items, "Sets the skin's channels (SK-31) or one joint influence's Enable and Interpolation (SK-35).")
        .on_editor(
            "Change the skin of a bound mesh, one undo step: interpolation local/linear/dualQuaternion, limitSources, maxSources 1-8, threshold 0-1, sharpness, world; with joint, that influence's enable and influence linear/smooth.",
            &[
            jopt("item", "integer", "The bound mesh (default the selected one)."),
            jopt("interpolation", "string", "local (default), linear or dualQuaternion."),
            jopt("limitSources", "boolean", "Keep only the strongest maxSources joints a point."),
            jopt("maxSources", "integer", "1 to 8 (default 3)."),
            jopt("threshold", "number", "Ignore weights below this (0-1)."),
            jopt("sharpness", "number", "Positive binds harder to the strongest joint, negative spreads."),
            jopt("world", "boolean", "Joints act in world space (default true)."),
            jopt("joint", "integer", "One influence's joint, for enable and influence."),
            jopt("enable", "boolean", "That influence on or off."),
            jopt("influence", "string", "That influence's weights: linear or smooth."),
        ],
            crate::skin_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.skin.set interpolation:dualQuaternion"),
    script("operandi.skin.unbind", Items, "Unbind (SK-24): removes the skin piece; keeps the weight maps unless keepMaps is false; needs Setup mode.")
        .on_editor(
            "Remove the skin from the meshes (items, default the selected meshes), one undo step; keepMaps false deletes the joints' weight maps too. Needs Setup mode.",
            &[
            jopt("items", "string", "Mesh item ids (default the selected meshes)."),
            jopt("keepMaps", "boolean", "Keep the weight maps (default true)."),
        ],
            crate::skin_bind::run,
        )
        .undo()
        .operandi()
        .example("operandi.skin.unbind"),
];
