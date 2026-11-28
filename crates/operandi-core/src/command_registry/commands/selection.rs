//! Selection phrases (one section of `COMMANDS`, in table order).
use super::*;

pub(super) const COMMANDS: &[Entry] = &[
    // ---------------------------------------------------------- selection
    action("Vertices", Selection, "Vertex selection mode.")
        .operandi().undo(),
    action("Edges", Selection, "Edge selection mode.")
        .operandi().undo(),
    action("Polygons", Selection, "Polygon selection mode.")
        .operandi().undo(),
    action("Items", Selection, "Item selection mode.")
        .operandi().undo(),
    action("Materials", Selection, "Toggles the polygon tag (Materials) selection mode.")
        .operandi().undo(),
    // CV-03 / UIC-66: Alt+Shift+Space (Space's old job before Mini
    // Properties). The mode argument isn't supported yet.
    script("select.nextMode", Selection, "Switches to the next selection mode (Vertices, Edges, Polygons).")
        .args(&[opt("mode", INT, N, None)])
        .spelled("Select Next Mode").undo(),
    script("select.more", Selection, "Selects more.").aliases(&["More"]).spelled("Select More").undo(),
    script("select.less", Selection, "Selects less.").aliases(&["Less"]).spelled("Select Less").undo(),
    script("select.between", Selection, "Selects between two selected elements.").aliases(&["Between"]).spelled("Select Between").undo(),
    action("Select Symmetrical", Selection, "Selects the symmetrical elements.")
        .modo("select.symmetrical").undo(),
    action("Selection Fill", Selection, "Explains Selection Fill (a Shift+Cmd click).")
        .operandi().undo(),
    action("Select Through", Selection, "Explains that Select Through is unavailable.")
        .operandi().undo(),
    action("Next Loop", Selection, "Selects the next loop.")
        .modo("select.loop next").undo(),
    action("Previous Loop", Selection, "Selects the previous loop.")
        .modo("select.loop prev").undo(),
    script("select.shortestPath", Selection, "Adds the shortest path between the first and last selected elements.")
        .spelled("Shortest Path").undo(),
    action("Shortest Path Mode", Selection, "Toggles Shortest Path Mode: Shift+click adds the element and the path to it.")
        .operandi().ui(),
    script("select.edgeSharp", Selection, "Selects edges whose polygons meet at a normal angle between min and max; bare, opens the dialog.")
        .aliases(&["Sharp..."])
        .args(&[opt("min", FLOAT, N, Some("89.55")), opt("max", FLOAT, N, Some("180"))])
        .undo(),
    script("select.edgeCoplanar", Selection, "Selects edges whose two polygons are flat within flatness; bare, opens the dialog.")
        .aliases(&["Coplanar Edges..."])
        .args(&[opt("flatness", FLOAT, N, Some("0.005"))])
        .undo(),
    script("select.vertexColinear", Selection, "Selects points on the line through their outline neighbours; bare, opens the dialog.")
        .aliases(&["Colinear Vertices..."])
        .args(&[opt("distance", FLOAT, N, Some("0.000001"))])
        .undo(),
    script("select.edgeDiagonal", Selection, "Selects the edges that would merge triangle pairs into quads; bare, opens the dialog.")
        .aliases(&["Diagonal..."])
        .args(&[opt("flatness", FLOAT, N, Some("20")), opt("max_angle", FLOAT, N, Some("170"))])
        .undo(),
    action("Coplanar Polygons Tool", Selection, "Opens the Coplanar Polygons Tool: grows the selected polygons to neighbours facing the same way.")
        .modo("tool.set select.polygonCoplanar on").ui(),
    script("select.conditionalLoop", Selection, "Grows the last selected edge into a loop until a stop rule ends it.")
        .spelled("Conditional Loop").undo(),
    action("Conditional Loop Tool", Selection, "Opens Conditional Loop with its stop rules.")
        .modo("tool.set seam.marker on").ui(),
    script("select.lazyState", Selection, "Toggles Lazy Selection: a click picks the nearest point or edge from further away.")
        .spelled("Lazy Selection").ui(),
    script("select.closeLoop", Selection, "Closes the selected loop.")
        .spelled("Close Loop").undo(),
    script("select.expand", Selection, "Grows the selection.")
        .spelled("Grow").undo(),
    script("select.contract", Selection, "Shrinks the selection.")
        .spelled("Shrink").undo(),
    script("select.loop", Selection, "Selects loops.")
        .spelled("Loop").undo(),
    script("select.ring", Selection, "Selects rings.")
        .spelled("Ring").undo(),
    script("select.all", Selection, "Selects everything.").spelled("Select All").undo(),
    script("select.drop", Selection, "Drops the selection of the selection mode (Esc with no tool on).")
        .spelled("Drop Selection").undo(),
    script("select.invert", Selection, "Inverts the selection.")
        .spelled("Invert").undo(),
    script("select.connect", Selection, "Selects connected elements.")
        .spelled("Connected").undo(),
    script("@ppi_invert_selection.py", Selection, "Inverts within connected pieces.")
        .spelled("Invert Connected").undo(),
    script("select.convert", Selection, "Converts the selection to vertices, edges or polygons.")
        .aliases(&["Convert to Edges", "Convert to Polygons"])
        .spelled("Convert to Vertices")
        .args(&[arg("type", ArgType::Choice(&["vertex", "edge", "polygon"]), N)])
        .example("select.convert edge").undo(),
    script("select.boundary", Selection, "Selects the selection's boundary.")
        .spelled("Boundary").undo(),
    action("Select Set", Selection, "Selects a polygon set by name.")
        .operandi()
        .args(&[arg("name", TEXT, N)])
        .example("select set Front").undo(),
    action("Falloff", Tools, "Sets the spatial falloff.")
        .operandi()
        .args(&[arg("type", ArgType::Choice(&["none", "linear", "cylinder", "radial", "airbrush", "screen", "element", "noise", "curvature", "vertex map", "selection", "soft selection", "lasso", "path", "image", "coplanar"]), N)])
        .example("falloff none").ui(),
    script("falloff.vertexMap", Tools, "Adds a Vertex Map falloff (tool.set falloff.vertexMap on).")
        .args(&[
            opt("mode", ArgType::Choice(&["magnitude", "component"]), N, Some("magnitude")),
            opt("index", INT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.invert", Tools, "Adds an Invert falloff (tool.set falloff.invert on).").tool(),
    script("falloff.selection", Tools, "Adds a Selection falloff: 0 on the selection's border, full Steps rings in (tool.set falloff.selection on).")
        .args(&[
            opt("steps", INT, N, Some("2")),
            opt("shape", ArgType::Choice(&["linear", "easeIn", "easeOut", "smooth", "custom"]), N, Some("smooth")),
            opt("p0", FLOAT, N, Some("0")),
            opt("p1", FLOAT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.selection steps:5 shape:easeOut")
        .tool(),
    script("falloff.softSelection", Tools, "Adds a Soft Selection falloff: the selection at full weight, fading out over Radius beyond it (tool.set falloff.softSelection on).")
        .operandi()
        .args(&[
            opt("radius", FLOAT, N, Some("0.1")),
            opt("connectivity", ArgType::Choice(&["true", "false"]), N, Some("true")),
            opt("shape", ArgType::Choice(&["linear", "easeIn", "easeOut", "smooth", "custom"]), N, Some("linear")),
            opt("p0", FLOAT, N, Some("0")),
            opt("p1", FLOAT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.softSelection radius:0.5")
        .tool(),
    script("falloff.element", Tools, "Adds an Element falloff: a press picks the element under the pointer and the drag moves it, with Range around it (tool.set falloff.element on).")
        .operandi()
        .args(&[
            opt("mode", ArgType::Choice(&["auto", "autoCenter", "vertex", "edge", "edgeCenter", "polygon", "polygonCenter"]), N, Some("auto")),
            opt("range", FLOAT, N, Some("0")),
            opt("connect", ArgType::Choice(&["ignore", "connectivity", "rigid", "edgeLoops"]), N, Some("ignore")),
            opt("shape", ArgType::Choice(&["linear", "easeIn", "easeOut", "smooth", "custom"]), N, Some("linear")),
            opt("p0", FLOAT, N, Some("0")),
            opt("p1", FLOAT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.element mode:polygon range:0.3")
        .tool(),
    script("falloff.coplanar", Tools, "Adds a Coplanar falloff: the selection plus neighbouring polygons facing within Angle of it (tool.set falloff.coplanar on).")
        .operandi()
        .args(&[
            opt("angle", FLOAT, N, Some("0")),
            opt("connectivity", ArgType::Choice(&["true", "false"]), N, Some("true")),
            opt("shape", ArgType::Choice(&["linear", "easeIn", "easeOut", "smooth", "custom"]), N, Some("linear")),
            opt("p0", FLOAT, N, Some("0")),
            opt("p1", FLOAT, N, Some("0")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.coplanar angle:10")
        .tool(),
    script("falloff.screen", Tools, "Adds a Screen falloff: a circle on screen, drawn in the view by press and drag (tool.set falloff.screen on).")
        .operandi()
        .args(&[
            opt("size", FLOAT, N, Some("20")),
            opt("transparent", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.lasso", Tools, "Adds a Lasso falloff: a shape drawn in the view (tool.set falloff.lasso on).")
        .operandi()
        .args(&[
            opt("style", ArgType::Choice(&["lasso", "rectangle", "circle", "ellipse"]), N, Some("rectangle")),
            opt("softBorder", FLOAT, N, Some("0")),
            opt("world", ArgType::Choice(&["true", "false"]), N, Some("true")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.noise", Tools, "Adds a Noise falloff: smooth 3D noise of position over Scale (tool.set falloff.noise on).")
        .operandi()
        .args(&[
            opt("scale", FLOAT, N, Some("1")),
            opt("world", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .example("falloff.noise scale:0.25")
        .tool(),
    script("falloff.image", Tools, "Adds an Image falloff: a picture's brightness, laid over the view or through the selected UV map; choose the picture in its panel (tool.set falloff.image on).")
        .operandi()
        .args(&[
            opt("repeat", ArgType::Choice(&["true", "false"]), N, Some("true")),
            opt("softBorder", FLOAT, N, Some("5")),
            opt("useUV", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("scale", FLOAT, N, Some("100")),
            opt("posX", FLOAT, N, Some("0")),
            opt("posY", FLOAT, N, Some("0")),
            opt("rotation", FLOAT, N, Some("0")),
            opt("world", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.path", Tools, "Adds a Path falloff: a tube of radius Size around a curve clicked in the view (tool.set falloff.path on).")
        .operandi()
        .args(&[
            opt("size", FLOAT, N, Some("1")),
            opt("world", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.curvature", Tools, "Adds a Curvature falloff: weights from how sharply the surface bends (tool.set falloff.curvature on).")
        .operandi()
        .args(&[
            opt("scale", FLOAT, N, Some("1")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    script("falloff.airbrush", Tools, "Adds an Airbrush falloff: weights painted by dragging in the view (tool.set falloff.airbrush on).")
        .operandi()
        .args(&[
            opt("size", FLOAT, N, Some("20")),
            opt("strength", FLOAT, N, Some("0.1")),
            opt("transparent", ArgType::Choice(&["true", "false"]), N, Some("false")),
            opt("mix", ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]), N, Some("multiply")),
        ])
        .tool(),
    action("Symmetry", Tools, "Sets symmetry.")
        .operandi()
        .args(&[arg("axis", ArgType::Choice(&["off", "x", "y", "z"]), N)])
        .example("symmetry off").ui(),
    action("Action Center", Tools, "Sets the action centre (element then picks, and screen places, on each press with a transform tool on).")
        .operandi()
        .args(&[arg(
            "center",
            ArgType::Choice(&[
                "automatic",
                "selection",
                "selection auto axis",
                "origin",
                "local",
                "element",
                "screen",
            ]),
            N,
        )])
        .example("action center origin").ui(),
];
