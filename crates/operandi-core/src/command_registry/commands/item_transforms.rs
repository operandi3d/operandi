//! Item transform, Reset/Freeze and image commands (one section of `COMMANDS`, in table order).
use super::*;

pub(super) const COMMANDS: &[Entry] = &[
    // The map's Transform Channel (PD-Q12): on a live item tool's Texture
    // Locator it is one step inside the tool.
    script("transform.channel", Items, "Sets (or with ? reads) one transform channel: pos.X/Y/Z, rot.X/Y/Z, scl.X/Y/Z; one undo step.")
        .during_transform_tool()
        .undo()
        .query()
        .headless(
            "Set one transform channel of an item (default the active item); value \"?\" answers the current value. One undo step; on the Texture Locator a live Move/Rotate/Scale/Transform holds, one step inside that tool, as a drag of its handle (PD-Q12).",
            &[
            jreq(
                "name",
                "string",
                "pos.X/Y/Z (metres), rot.X/Y/Z (radians, or with deg) or scl.X/Y/Z (a Texture Locator's Size, at least 0.001).",
            ),
            jreq("value", "any", "The new value; \"?\" reads it."),
            jopt(
                "mode",
                "integer",
                "Not supported yet (the map's mode); leave it out.",
            ),
            JSON_ITEM,
            jopt(
                "filter",
                "string",
                "Not supported yet (the map's filter); leave it out.",
            ),
            jopt(
                "replace",
                "string",
                "Not supported yet (the map's replace); leave it out.",
            ),
        ],
        )
        .example("transform.channel name:scl.X value:2"),
    // Spec freeze-transforms-2026-11-14 section 8 (item_transform_ops.rs).
    script("transform.reset", Items, "Puts the selected items' translation, rotation, scale or all of them back to identity; the items move, their geometry doesn't.")
        .undo()
        .headless(
            "Reset part of every selected item's transform to identity (position 0, rotation 0, scale 100%). Children follow; geometry is untouched; locked items are skipped. One undo step, none when nothing changes.",
            &[jopt(
                "type",
                "string",
                "translation, rotation, scale or all; default all.",
            )],
        )
        .example("transform.reset rotation"),
    script("transform.freeze", Items, "Bakes the selected mesh items' translation, rotation, scale or whole transform into their geometry; nothing moves.")
        .undo()
        .headless(
            "Bake part of every selected mesh item's transform into its points, then set that part to identity, so nothing moves on screen. Children are compensated; mirrored scale reverses polygons; locked items and non-mesh items are skipped. One undo step, none when nothing changes.",
            &[jopt(
                "type",
                "string",
                "translation, rotation, scale or all; default all.",
            )],
        )
        .example("transform.freeze scale"),
    script("item.setType", Items, "Changes a light's type, keeping what both types share.")
        .undo()
        .headless(
            "Change a light's type, keeping what both types share (Light Properties > Type).",
            &[
            jreq(
                "type",
                "string",
                "Light type: Directional, Point, Spot, Area, Cylinder, Photometric, Mesh, Dome or Portal.",
            ),
            jopt(
                "mask",
                "string",
                "Not supported yet (the map's mask); leave it out.",
            ),
            JSON_ITEM,
        ],
        )
        .example("item.setType Spot"),
    script("light.temperature", Items, "Sets a light's colour from a colour temperature (kelvins).")
        .undo()
        .headless(
            "Set a light's colour from a colour temperature (Light Material > Set Temperature).",
            &[
            jreq("kelvins", "number", "Colour temperature, kelvins."),
            jopt(
                "gamma",
                "number",
                "Not supported yet (the map's gamma); leave it out.",
            ),
            JSON_ITEM,
        ],
        )
        .example("light.temperature 5600"),
    script("light.mesh", Items, "Sets a Mesh Light's Prototype mesh, or none.")
        .undo()
        .headless(
            "Set a Mesh Light's Prototype mesh (or none).",
            &[
            jopt(
                "light",
                "integer",
                "The Mesh Light's item id; default the active item.",
            ),
            jreq("mesh", "string", "The mesh item id, or \"none\"."),
        ],
        )
        .example("light.mesh mesh:none"),
    script("texture.setIMap", Shading, "Sets an Image Map layer's Image (scene image name or path, or none).")
        .undo()
        .headless(
            "Set an Image Map layer's Image: a scene image by name or file path, or none.",
            &[
            jreq(
                "image",
                "string",
                "Scene image name or file path, or \"none\".",
            ),
            jopt(
                "name",
                "string",
                "Not supported yet (the map's name); leave it out.",
            ),
            jreq("layer", "integer", "The Image Map layer's id."),
        ],
        ),
    script("texture.delete", Shading, "Deletes texture layer item: (a layer id) from the Shader Tree or an environment, with the Texture Locator no other layer uses (Clear Image on an environment). One undo step.")
        .args(&[arg("item", INT, N)])
        .undo()
        .headless(
            "Delete a texture layer (Shader Tree or environment) and its unshared Texture Locator.",
            &[jreq("item", "integer", "The texture layer's id.")],
        )
        .example("texture.delete item:4294967298"),
    script("operandi.texture.loadImage", Shading, "Browse… on an Image Map layer: the picture filename: becomes layer:'s image. One undo step. Without a filename opens the file panel.")
        .operandi()
        .args(&[arg("layer", INT, N), opt("filename", ArgType::Path, N, None)])
        .undo()
        .example("operandi.texture.loadImage layer:4294967298 filename:/textures/wood.png"),
    script("operandi.environment.loadImage", Shading, "Load Image…: the picture filename: becomes environment item:'s image (default the top environment, made when there is none); it replaces the picture of the environment's image layer, keeping its rotation and opacity, or adds an Image Map layer above the Environment Material. One undo step. Without a filename opens the file panel.")
        .operandi()
        .args(&[opt("filename", ArgType::Path, N, None), opt("item", INT, N, None)])
        .undo()
        .example("operandi.environment.loadImage filename:/hdri/studio.hdr"),
    script("texture.setLocator", Shading, "Sets a texture layer's Texture Locator item (or none).")
        .undo()
        .headless(
            "Set a texture layer's Texture Locator item (or none).",
            &[
            jreq("locator", "string", "Locator item id, or \"none\"."),
            jreq("layer", "integer", "The texture layer's id."),
        ],
        ),
    script("texture.setVMap", Shading, "Sets a Vertex Map texture layer's map (or none).")
        .undo()
        .headless(
            "Set a Vertex Map texture layer's map by name (or none).",
            &[
            jreq("name", "string", "Vertex map name, or \"none\"."),
            jreq("layer", "integer", "The Vertex Map layer's id."),
        ],
        ),
    script("mask.setPTag", Shading, "Sets a Group's Polygon Tag ((all), (none) or a tag name).")
        .undo()
        .headless(
            "Set a Group's Polygon Tag: a tag name, \"(all)\" or \"(none)\".",
            &[
            jreq("name", "string", "Tag name, (all) or (none)."),
            jreq("layer", "integer", "The Group layer's id."),
        ],
        ),
    script("mask.setPTagType", Shading, "Sets a Group's Polygon Tag Type; the tag goes back to (none).")
        .undo()
        .headless(
            "Set a Group's Polygon Tag Type (Material, Part or Selection Set); the tag goes back to (none).",
            &[
            jreq("name", "string", "Tag type."),
            jreq("layer", "integer", "The Group layer's id."),
        ],
        ),
    script("item.channel", Items, "Sets (or with ? reads) one channel of an item or shader layer; one undo step. Channel names are Operandi's own for now.")
        .undo()
        .query()
        .headless(
            "Set one channel of an item (default the active item) or of a shader layer; value \"?\" answers the current value. Channel names are Operandi's own for now (docs/COMMAND_REGISTRY.md lists them).",
            &[
            jreq(
                "name",
                "string",
                "Channel name, e.g. intensity, color (color.r for one part), opacity.",
            ),
            jreq(
                "value",
                "any",
                "The value: number, true/false, text, r,g,b, an item id or none; \"?\" reads it. Percents 0–1, distances metres, angles radians.",
            ),
            jopt(
                "mode",
                "integer",
                "Not supported yet (the map's mode); leave it out.",
            ),
            JSON_ITEM,
            jopt(
                "layer",
                "integer",
                "A shader-tree layer id instead of an item (Operandi's own argument).",
            ),
            jopt(
                "scene",
                "boolean",
                "true: the scene's own time channels fps, start, end, time (seconds) instead of an item (Operandi's own argument).",
            ),
        ],
        )
        .example("item.channel intensity ?"),
    script("environment.light", Shading, "Set Physical Sun: picks the directional light that places a Physically-based Daylight environment's sun (sunLight:<id>|none; envMaterial:<layer id>, default the active environment).")
        .undo()
        .headless(
            "Set Physical Sun: the directional light whose direction places the sun of a Physically-based Daylight environment (environments-2026-11-09 §3). One undo step.",
            &[
            jopt(
                "envMaterial",
                "integer",
                "The Environment Material's layer id; default the active environment's.",
            ),
            jreq("sunLight", "any", "A directional light's item id, or none."),
        ],
        )
        .example("environment.light sunLight:none"),
    script("smoothing.set", Modeling, "Sets the scene's smoothing amount, angle, crease by angle or groups.")
        .operandi()
        .undo()
        .headless(
            "Set the smooth-shading rule (one for every mesh: the default material's smoothing). Fields not given keep their value. Viewport, exports and renders all use it.",
            &[
            jopt(
                "amount",
                "number",
                "How far corners bend toward the averaged normal: 0 flat, 1 fully smooth (the default); -1..2 allowed.",
            ),
            jopt(
                "angle",
                "number",
                "Smoothing angle, radians: neighbouring polygons within this angle blend (default 40° = 0.698).",
            ),
            jopt(
                "crease_by_angle",
                "boolean",
                "Off blends every connected polygon (creases only from groups); default true.",
            ),
            jopt(
                "groups",
                "string",
                "Smoothing groups: \"off\", \"crease\" (default) or \"force\".",
            ),
        ],
        ),
    script("backdrop.add", Items, "Adds a Backdrop item at the origin; with an image it shows that picture, facing project (what dropping a picture on a view does, BD-60).")
        .args(&[
            opt("name", TEXT, N, None),
            opt("image", ArgType::Path, N, None),
            opt(
                "project",
                ArgType::Choice(&["front", "back", "top", "bottom", "left", "right", "camera"]),
                N,
                Some("front"),
            ),
        ])
        .undo()
        .example("backdrop.add name:Plan project:top"),
    script("backdrop.image", Items, "Shows a picture on the selected Backdrop (none clears); bare opens the file panel.")
        .operandi()
        .args(&[opt("path", ArgType::Path, N, None)])
        .undo(),
    script("backdrop.edit", Items, "Sets options of the selected Backdrop (map names, or the old backdrop.attr <name> <value>).")
        .args(&[
            opt("image", ArgType::Path, N, None),
            opt("project", TEXT, N, None),
            opt("aspect", BOOL, N, None),
            opt("flip", BOOL, N, None),
            opt("invert", BOOL, N, None),
            opt("blend", BOOL, N, None),
            opt("contrast", FLOAT, Unit::Percent, None),
            opt("brightness", FLOAT, Unit::Percent, None),
            opt("transparency", FLOAT, Unit::Percent, None),
        ])
        .undo()
        .example("backdrop.edit flip:true"),
    script("backdrop.fit", Items, "Fits the selected Backdrop to the active meshes' combined box (BD-61).").undo(),
    script("backdrop.size", Items, "Sizes the selected Backdrop's picture.")
        .operandi()
        .args(&[
            arg("width", FLOAT, Unit::Distance),
            opt("height", FLOAT, Unit::Distance, None),
        ])
        .undo()
        .example("backdrop.size 1"),
    script("clip.reload", Shading, "Reads a scene image's file again (Reload Image; all when bare) and updates the viewport; changes no scene data.")
        .args(&[opt("image", TEXT, N, Some("all"))]).side_effect(),
    script("clip.addStill", Shading, "Loads a picture file as a scene image entry only (what dropping a picture outside a 3D view does, BD-60).")
        .args(&[arg("filename", ArgType::Path, N)])
        .undo(),
    script("image.list", Shading, "Reports the scene's images: name, file path and pixel size.")
        .operandi().query(),
];
