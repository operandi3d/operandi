//! Graph editor commands (one section of `COMMANDS`, in table order).
use super::*;

pub(super) const COMMANDS: &[Entry] = &[
    // Graph editor (BEHAVIOR-GRAPH-EDITOR; anim_graph.rs, anim_commands.rs).
    script("operandi.key.offset", Items, "Moves the selected keys by frames and by a value at once, one undo step (GE-40/45: a graph editor drag's result).")
        .on_editor(
            "Move the selected keys by frames in time and by value, one undo step (GE-40/45).",
            &[
            jopt("frames", "number", "Frames to move by (default 0)."),
            jopt("value", "number", "Value to add (default 0)."),
            jopt("link", "boolean", "Related keys (X/Y/Z, R/G/B) at the same times move in time too (GE-42)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.offset frames:2 value:0.5"),
    script("operandi.key.transform", Items, "Transforms the selected keys at once, one undo step: times scaled around a pivot then moved, values the same (GE-44 Box Scale, GE-40/43).")
        .on_editor(
            "Transforms the selected keys at once, one undo step: times scaled around a pivot then moved, values the same (GE-44 Box Scale, GE-40/43).",
            &[
            jopt("frames", "number", "Frames to move by (default 0)."),
            jopt("value", "number", "Value to add (default 0)."),
            jopt("timeFactor", "number", "Time scale (default 1)."),
            jopt("timePivot", "number", "Frame to scale time around (default 0)."),
            jopt("valueFactor", "number", "Value scale (default 1)."),
            jopt("valuePivot", "number", "Value to scale around (default 0)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.transform timeFactor:2 timePivot:0"),
    script("operandi.key.dropCopies", Items, "Drops copies of the selected keys on every frame along a drag, values moving evenly (GE-31 Ctrl+Shift+drag).")
        .on_editor(
            "Drops copies of the selected keys on every frame along a drag, values moving evenly (GE-31 Ctrl+Shift+drag).",
            &[
            jreq("frames", "number", "Frames dragged (negative: earlier)."),
            jopt("value", "number", "Value change at the end (default 0)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.dropCopies frames:4"),
    script("operandi.curve.retime", Items, "Stretches timing between two frames so one frame lands on another; keys between follow (GE-X5 retime).")
        .on_editor(
            "Stretches timing between two frames so one frame lands on another; keys between follow (GE-X5 retime).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jreq("low", "number", "Fixed frame before."),
            jreq("high", "number", "Fixed frame after."),
            jreq("from", "number", "The frame that moves."),
            jreq("to", "number", "Where it lands."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.retime low:0 high:24 from:12 to:8"),
    script("operandi.curve.saveEnvelope", Items, "Saves channels' curves as an envelope preset file (GE-83).")
        .on_editor(
            "Saves channels' curves as an envelope preset file (GE-83).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jreq("path", "string", "The .openvelope file to write."),
        ],
            crate::anim_commands::run,
        )
        .side_effect()
        .operandi()
        .example("operandi.curve.saveEnvelope path:{bounce.openvelope}"),
    script("operandi.curve.applyEnvelope", Items, "Puts an envelope preset's curves on channels from a frame (GE-83).")
        .on_editor(
            "Puts an envelope preset's curves on channels from a frame (GE-83).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jreq("path", "string", "The .openvelope file."),
            jopt("frame", "number", "Start frame (default the current frame)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.applyEnvelope path:{bounce.openvelope}"),
    script("operandi.key.setValue", Items, "Sets, adds to or multiplies the selected keys' values (GE-71 Set Value).")
        .on_editor(
            "Sets, adds to or multiplies the selected keys' values (GE-71 Set Value).",
            &[
            jreq("value", "number", "The value."),
            jopt("mode", "string", "set (default), add or multiply."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.setValue value:2"),
    script("operandi.key.scaleValue", Items, "Scales the selected keys' values around a value (GE-43).")
        .on_editor(
            "Scales the selected keys' values around a value (GE-43).",
            &[
            jreq("factor", "number", "Factor."),
            jopt("pivot", "number", "The value to scale around (default 0)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.scaleValue factor:1.5"),
    script("operandi.key.setFrame", Items, "Puts the selected keys at a frame (the earliest there, spacing kept), or moves them by frames (GE-71 Set Frame).")
        .on_editor(
            "Puts the selected keys at a frame (the earliest there, spacing kept), or moves them by frames (GE-71 Set Frame).",
            &[
            jreq("frame", "number", "Frame (or frames to move by with relative)."),
            jopt("relative", "boolean", "Move by frame instead of to it."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.setFrame frame:10"),
    script("operandi.key.handle", Items, "Sets the selected keys' in or out handle: slope (value per frame) and/or weight; that side becomes Manual, a unified key's other side follows the slope (GE-50/53).")
        .on_editor(
            "Sets the selected keys' in or out handle: slope (value per frame) and/or weight; that side becomes Manual, a unified key's other side follows the slope (GE-50/53).",
            &[
            jopt("side", "string", "in or out (default out)."),
            jopt("slope", "number", "Slope, value per frame."),
            jopt("weight", "number", "Weight 0.01-3 (1 is the plain length)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.handle side:out slope:0.1"),
    script("operandi.key.break", Items, "Breaks the selected keys' handles so each side moves alone (GE-51).")
        .on_editor(
            "Breaks the selected keys' handles so each side moves alone (GE-51).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.break"),
    script("operandi.key.unify", Items, "Unifies the selected keys' handles into one line (GE-51).")
        .on_editor(
            "Unifies the selected keys' handles into one line (GE-51).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.unify"),
    script("operandi.key.autoHandles", Items, "Puts the selected keys' handle weights back to Auto; with slopes:true their slopes too (GE-51/52).")
        .on_editor(
            "Puts the selected keys' handle weights back to Auto; with slopes:true their slopes too (GE-51/52).",
            &[
            jopt("slopes", "boolean", "Slopes back to Auto as well."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.autoHandles slopes:true"),
    script("operandi.key.add", Items, "Adds a key on a channel at a time and value (GE-31); the curve's own value when value is left out.")
        .on_editor(
            "Adds a key on a channel at a time and value (GE-31); the curve's own value when value is left out.",
            &[
            jreq("channel", "string", "item:name."),
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
            jopt("value", "number", "The key's value."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.add channel:1:pos.Y frame:6 value:1"),
    script("operandi.key.keepShape", Items, "Keys channels at a time without changing their curves (GE-32 Key Keeping Shape).")
        .on_editor(
            "Keys channels at a time without changing their curves (GE-32 Key Keeping Shape).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.keepShape channels:1:pos.Y frame:6"),
    script("operandi.curve.behavior", Items, "Sets curves' behaviour before the first key and/or after the last (GE-61).")
        .on_editor(
            "Sets curves' behaviour before the first key and/or after the last (GE-61).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("pre", "string", "constant, constantSmooth, linear, repeat, oscillate, oscillateSmooth, offsetRepeat, offsetRepeatSmooth or reset."),
            jopt("post", "string", "As pre."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.behavior post:repeat"),
    script("operandi.curve.interpolation", Items, "Sets curves' interpolation: every key's slopes (GE-60).")
        .on_editor(
            "Sets curves' interpolation: every key's slopes (GE-60).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jreq("type", "string", "curve, linear or stepped."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.interpolation type:linear"),
    script("operandi.curve.cycle", Items, "Turns the cycle option on or off: the last key matches the first (GE-62).")
        .on_editor(
            "Turns the cycle option on or off: the last key matches the first (GE-62).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("on", "boolean", "On (default true)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.cycle on:true"),
    script("operandi.curve.shadow", Items, "Copy to Shadow, Replace with Shadow, Swap or Clear Shadow (GE-80).")
        .on_editor(
            "Copy to Shadow, Replace with Shadow, Swap or Clear Shadow (GE-80).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("action", "string", "copy (default), replace, swap or clear."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.shadow action:copy"),
    script("operandi.curve.eulerFilter", Items, "Unwraps rotation curves that jump by whole turns (GE-X1).")
        .on_editor(
            "Unwraps rotation curves that jump by whole turns (GE-X1).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.eulerFilter"),
    script("operandi.curve.simplify", Items, "Removes keys that change the curve by no more than a tolerance (GE-X2).")
        .on_editor(
            "Removes keys that change the curve by no more than a tolerance (GE-X2).",
            &[
            jopt("channels", "string", "Channels as item:name,...; default the selected channels, else the selected keys' channels."),
            jopt("tolerance", "number", "Largest change allowed (default 0.001)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.curve.simplify tolerance:0.01"),
    script("operandi.time.set", Items, "Sets the current time (TK-01); not an undo step (TK-05). Snaps to a frame with Time Snap on.")
        .on_editor(
            "Sets the current time (TK-01); not an undo step (TK-05). Snaps to a frame with Time Snap on.",
            &[
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.set frame:24"),
    script("operandi.time.step", Items, "Steps the current time by frames (TK-63; Shift+Left/Right, comma/period).")
        .on_editor(
            "Steps the current time by frames (TK-63; Shift+Left/Right, comma/period).",
            &[
            jopt("frames", "number", "Frames to step (default 1; negative steps back)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.step frames:-1"),
    script("operandi.time.go", Items, "Goes to the current range's first or last frame, or the previous, next, first or last key of the selected items or channels (TK-63).")
        .on_editor(
            "Goes to the current range's first or last frame, or the previous, next, first or last key of the selected items or channels (TK-63).",
            &[
            jopt("to", "string", "first, last, prevKey, nextKey, firstKey or lastKey (default first)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.go to:last"),
    script("operandi.time.play", Items, "Plays or stops playback through the current range (TK-60; /).")
        .on_editor(
            "Plays or stops playback through the current range (TK-60; /).",
            &[
            jopt("backwards", "boolean", "Play backwards (default false)."),
            jopt("stop", "boolean", "true stops, false starts; default toggles."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.play stop:true"),
    script("operandi.time.keyTime", Items, "Sets the key time alone, where new keys land (TL-20), or resets it to follow the current time.")
        .on_editor(
            "Sets the key time alone, where new keys land (TL-20), or resets it to follow the current time.",
            &[
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
            jopt("reset", "boolean", "Follow the current time again."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.time.keyTime reset:true"),
    script("operandi.channel.copy", Items, "Copies the values of channels: (default the selected channels) for Paste (#54).")
        .on_editor(
            "Copies channel values for Paste.",
            &[jopt("channels", "string", "item:name,... (default the selected channels).")],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.channel.copy channels:1:pos.X,1:pos.Y"),
    script("operandi.channel.paste", Items, "Pastes the copied channel values onto channels: (default the selected channels; with none, the selected items' channels of the copied names): by name, else in order. One undo step (#54).")
        .on_editor(
            "Pastes copied channel values: by name, else in order; one undo step.",
            &[jopt("channels", "string", "item:name,... (default the selected channels, else the selected items).")],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.channel.paste channels:2:pos.X,2:pos.Y"),
    script("operandi.channel.select", Items, "Selects a channel for keying (UIC-119), or clears the channel selection.")
        .on_editor(
            "Selects a channel for keying (UIC-119), or clears the channel selection.",
            &[
            jopt("channel", "string", "item:name, or none."),
            jopt("add", "boolean", "Toggle it in the selection (default false)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.channel.select channel:none"),
    script("operandi.key.select", Items, "Selects keys in the track row (TL-41).")
        .on_editor(
            "Selects keys in the track row (TL-41).",
            &[
            jopt("keys", "string", "Keys as item:name@frame,...."),
            jopt("mode", "string", "replace, add, remove, all or clear (default replace)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.key.select mode:clear"),
    script("operandi.marker.select", Items, "Selects markers (TL-52).")
        .on_editor(
            "Selects markers (TL-52).",
            &[
            jopt("ids", "string", "Marker ids, comma separated."),
            jopt("add", "boolean", "Add to the selection (default false)."),
        ],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.marker.select ids:1"),
    script("operandi.key.copy", Items, "Copies the selected keys (or the selected items' keys at the key time) to the key clipboard (TL-45/47); not an undo step.")
        .on_editor(
            "Copies the selected keys (or the selected items' keys at the key time) to the key clipboard (TL-45/47); not an undo step.",
            &[],
            crate::anim_commands::run,
        )
        .ui()
        .operandi()
        .example("operandi.key.copy"),
    script("operandi.key.set", Items, "Sets keys at the key time with the channels' current values (TK-20/21; S keys the selected channels).")
        .on_editor(
            "Sets keys at the key time with the channels' current values (TK-20/21; S keys the selected channels).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.set channels:1:pos.Y"),
    script("operandi.key.toggle", Items, "A status-light click: keys the channels, or removes their keys at the key time when every one has one (TK-20).")
        .on_editor(
            "A status-light click: keys the channels, or removes their keys at the key time when every one has one (TK-20).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.toggle channels:1:pos.X"),
    script("operandi.key.remove", Items, "Removes the channels' keys at the key time or the given time (TK-24 Remove Key).")
        .on_editor(
            "Removes the channels' keys at the key time or the given time (TK-24 Remove Key).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.remove channels:1:pos.Y"),
    script("operandi.key.transforms", Items, "Keys position, rotation, scale or all three of the selected items (TK-22; Shift+W/E/R/Y).")
        .on_editor(
            "Keys position, rotation, scale or all three of the selected items (TK-22; Shift+W/E/R/Y).",
            &[
            jopt("type", "string", "position, rotation, scale or all (default all)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.transforms type:position"),
    script("operandi.key.items", Items, "Keys every animated channel of the selected items (TK-23; Shift+S).")
        .on_editor(
            "Keys every animated channel of the selected items (TK-23; Shift+S).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.items"),
    script("operandi.key.removeAnimation", Items, "Removes every key of the channels, keeping their current values (TK-24).")
        .on_editor(
            "Removes every key of the channels, keeping their current values (TK-24).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.removeAnimation channels:1:pos.Y"),
    script("operandi.key.reset", Items, "Removes every key of the channels and puts back their default values (TK-24).")
        .on_editor(
            "Removes every key of the channels and puts back their default values (TK-24).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.reset channels:1:pos.Y"),
    script("operandi.key.bake", Items, "Keys the channels on every frame of the current range (TK-24).")
        .on_editor(
            "Keys the channels on every frame of the current range (TK-24).",
            &[
            jopt("channels", "string", "Channels as item:name,... (pos.X, rot.Y, scl.Z or an item.channel name); default the selected channels."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.bake channels:1:pos.Y"),
    script("operandi.key.apply", Items, "Keys every edited-but-not-keyed channel at the key time (TK-41 Apply).")
        .on_editor(
            "Keys every edited-but-not-keyed channel at the key time (TK-41 Apply).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.apply"),
    script("operandi.key.discard", Items, "Drops every edited-but-not-keyed value back to its curve (TK-41 Discard).")
        .on_editor(
            "Drops every edited-but-not-keyed value back to its curve (TK-41 Discard).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.discard"),
    script("operandi.key.autoKey", Items, "Sets Auto Key: off, animated or all (TK-30).")
        .on_editor(
            "Sets Auto Key: off, animated or all (TK-30).",
            &[
            jreq("mode", "string", "off, animated or all."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.autoKey mode:animated"),
    script("operandi.key.move", Items, "Moves the selected keys by frames (TL-42, Shift...).")
        .on_editor(
            "Moves the selected keys by frames (TL-42, Shift...).",
            &[
            jreq("frames", "number", "Frames to move (negative moves earlier)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.move frames:2"),
    script("operandi.key.scale", Items, "Scales the selected keys' timing around a pivot (TL-43, Scale...).")
        .on_editor(
            "Scales the selected keys' timing around a pivot (TL-43, Scale...).",
            &[
            jreq("factor", "number", "Timing factor, above 0."),
            jopt("frame", "number", "Pivot frame (default the key time)."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.scale factor:2"),
    script("operandi.key.invert", Items, "Reverses the selected keys' timing within their span (TL-45 Invert...).")
        .on_editor(
            "Reverses the selected keys' timing within their span (TL-45 Invert...).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.invert"),
    script("operandi.key.delete", Items, "Deletes the selected keys (TL-45 Delete / Delete Related).")
        .on_editor(
            "Deletes the selected keys (TL-45 Delete / Delete Related).",
            &[
            jopt("related", "boolean", "Also the sibling X/Y/Z keys at the same times."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.delete related:true"),
    script("operandi.key.slope", Items, "Sets the selected keys' slope type (TL-45/46).")
        .on_editor(
            "Sets the selected keys' slope type (TL-45/46).",
            &[
            jreq("type", "string", "auto, smoothFlat, autoFlat, flat, linear, linearIn, linearOut, stepped or manual."),
            jopt("side", "string", "in, out or both (default both; linearIn/Out pick theirs)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.slope type:linear"),
    script("operandi.key.cut", Items, "Copies the selected keys to the key clipboard, then deletes them (TL-45).")
        .on_editor(
            "Copies the selected keys to the key clipboard, then deletes them (TL-45).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.cut"),
    script("operandi.key.paste", Items, "Pastes the key clipboard at the key time (TL-45): merge, insert (later keys move along) or replace (keys in the pasted span go).")
        .on_editor(
            "Pastes the key clipboard at the key time (TL-45): merge, insert (later keys move along) or replace (keys in the pasted span go).",
            &[
            jopt("mode", "string", "merge, insert or replace (default merge)."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.key.paste mode:replace"),
    script("operandi.time.insertFrame", Items, "Moves the selected items' keys after the key time one frame later (TL-21).")
        .on_editor(
            "Moves the selected items' keys after the key time one frame later (TL-21).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.insertFrame"),
    script("operandi.time.removeFrame", Items, "Moves the selected items' keys after the key time one frame earlier, never deleting keys (TL-21).")
        .on_editor(
            "Moves the selected items' keys after the key time one frame earlier, never deleting keys (TL-21).",
            &[],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.removeFrame"),
    script("operandi.time.settings", Items, "Sets the scene's time settings (TK-03/04): frame rate, scene and current range (frames), time display, Auto Key, Time Snap, Play Real Time, End Behavior, Auto Update Existing Keys.")
        .on_editor(
            "Sets the scene's time settings (TK-03/04): frame rate, scene and current range (frames), time display, Auto Key, Time Snap, Play Real Time, End Behavior, Auto Update Existing Keys.",
            &[
            jopt("fps", "number", "Frames per second."),
            jopt("start", "number", "Scene range start, frames."),
            jopt("end", "number", "Scene range end, frames."),
            jopt("rangeStart", "number", "Current range start, frames."),
            jopt("rangeEnd", "number", "Current range end, frames."),
            jopt("display", "string", "frames, seconds, smpte or film."),
            jopt("autoKey", "string", "off, animated or all."),
            jopt("snap", "boolean", "Time Snap."),
            jopt("realTime", "boolean", "Play Real Time."),
            jopt("endBehavior", "string", "hold, repeat or wrap."),
            jopt("autoUpdate", "boolean", "Auto Update Existing Keys."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.settings fps:24 start:0 end:120"),
    script("operandi.time.range", Items, "Sets the current range in frames (TL-13), or resets it to the scene range.")
        .on_editor(
            "Sets the current range in frames (TL-13), or resets it to the scene range.",
            &[
            jopt("start", "number", "Start frame."),
            jopt("end", "number", "End frame."),
            jopt("reset", "boolean", "Back to the scene range."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.range start:0 end:48"),
    script("operandi.time.fit", Items, "Fits the current range to the selected items' or channels' keys, or every key (TL-14; A / Shift+A).")
        .on_editor(
            "Fits the current range to the selected items' or channels' keys, or every key (TL-14; A / Shift+A).",
            &[
            jopt("all", "boolean", "Every key in the scene."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.time.fit all:true"),
    script("operandi.marker.add", Items, "Adds a time marker at the current or given time, owned by the selected item or the scene (TL-50/51; M).")
        .on_editor(
            "Adds a time marker at the current or given time, owned by the selected item or the scene (TL-50/51; M).",
            &[
            jopt("frame", "number", "Time in frames."),
            jopt("time", "number", "Time in seconds (instead of frame)."),
            jopt("scene", "boolean", "Owned by the scene even with items selected."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.marker.add frame:12"),
    script("operandi.marker.edit", Items, "Changes markers (TL-52/53): time, shift, length, end, label, shape, colour, Show Label, Frame Limits, Snap to Frames.")
        .on_editor(
            "Changes markers (TL-52/53): time, shift, length, end, label, shape, colour, Show Label, Frame Limits, Snap to Frames.",
            &[
            jopt("ids", "string", "Marker ids; default the selected markers."),
            jopt("frame", "number", "New start frame."),
            jopt("shift", "number", "Frames to move by."),
            jopt("length", "number", "Range length, frames (0: a plain marker)."),
            jopt("endFrame", "number", "Range end frame."),
            jopt("label", "string", "Label."),
            jopt("shape", "string", "triangle, square, diamond or bar."),
            jopt("color", "string", "r,g,b 0-1."),
            jopt("showLabel", "boolean", "Show the label above the marker."),
            jopt("frameLimits", "boolean", "Limit playback and stepping to the range."),
            jopt("snapToFrames", "boolean", "Move start and end onto whole frames."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.marker.edit ids:1 label:Contact"),
    script("operandi.marker.delete", Items, "Deletes markers (TL-52).")
        .on_editor(
            "Deletes markers (TL-52).",
            &[
            jopt("ids", "string", "Marker ids; default the selected markers."),
        ],
            crate::anim_commands::run,
        )
        .undo()
        .operandi()
        .example("operandi.marker.delete ids:1"),
];
