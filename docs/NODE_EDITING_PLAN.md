# Node editing for materials (A3 slices 2+): plan

Spec: the maintainer's NE-01..NE-19 (behaviour only, 2026-11-20), tests
NE-T1..T8. Slice 1 (read-only Show as Nodes) is on main.

## What the node view shows

A Material Group's node view is built from the Shader Tree's own layers
plus the group's free nodes, one box per thing the spec names:

- **Layer nodes** (made by the tree, never stored as nodes): one box per
  material, texture, shader, Node Material or inner group layer, and one
  **mixing node** per layer that stacks (its opacity and blend mode), the
  mixing nodes chained bottom to top into a **Material Output** box. A
  material layer's inputs are its channels (Diffuse Color, Roughness...),
  so an inline edit there is literally the Properties edit (NE-01).
- **Free nodes** (added in the node view): standard MaterialX nodes, stored.

Why not edit slice 1's expanded `operandi_*` form directly: that form is the
*folded* surface (constants inlined, one layer's blend can be ten mix nodes,
Base Color may be Diffuse Color × Amount), so a value on it can't be traced
back to one layer field, and "the texture layer node" or "the mixing node"
doesn't exist as one box. The layer view maps 1:1 onto layers, which is
what NE-01/05/06/11/19 need. The expanded form stays available read-only
(the window's "As rendered" switch), since that is still exactly what
Cycles reads.

UIC-244 holds: the tree plus the group's free nodes are the one source of
truth. Both views are made from them each time; renderers and the viewport
read only the combined MaterialX form (tree form + free nodes) the writer
makes from them.

## Data model (native version 55)

`ShaderTree::node_views: BTreeMap<NodeId, GroupNodes>`, keyed by Material
Group:

```
GroupNodes {
    free: Vec<FreeNode>,          // name, MaterialX category, output type,
                                  // input values (kept even when wired)
    wires: Vec<Wire>,             // free output -> free input | layer channel
    places: BTreeMap<String, [f32; 2]>,  // any box, by view name (NE-09)
    hidden: BTreeSet<String>,     // NE-07, view only
}
```

Layer channel targets are `(layer id, Ch)`. In the free list free nodes
never get rows (NE-12). Deleting a layer drops the wires into it in the same
step (NE-13, NE-19: undo restores layer and wires together, since both sit
in the one tree snapshot). Positions and hidden flags never change a render
(NE-09, NE-16).

Native 55: older builds would drop the free nodes silently, so a scene with
any refuses to open there; a scene without keeps its older number.

## NE-03: how a wire drives an input (chosen approach)

A new expression node, `Expr::Nodes(Arc<NodeOut>)`: "output `out` of free
node `n`", carrying the free sub-graph as MaterialX elements. When the tree
folds a material layer whose channel is wired, that channel's value is this
expression instead of the layer's own constant (which stays stored: remove
the wire and the constant is back, pixel-identical).

- **Form / export:** `materialx::tree::write` writes the free nodes as the
  standard MaterialX nodes they are, wired by `nodename` into the
  `operandi_*` form; `tree::read` rebuilds `Expr::Nodes` from them. So the
  combined graph is one MaterialX document, and Cycles keeps rendering tree
  materials from that form read back (UIC-244). `xpos`/`ypos` on free nodes
  (NE-09). MaterialX/USD export write the same free nodes; re-import keeps
  them as nodes of the graph (NE-10).
- **Cycles:** the free sub-graph goes through the existing
  `materialx::cycles` translator into Cycles nodes (the same node list Node
  Materials use) and a new op reads one of their outputs. That is the
  missing reverse of NM-13's op→graph drivers. **C ABI bump 27 → 28**:
  `OC_EXPR_NODE` and an output-socket name list at the end of `OcSurface`;
  the C++ creates graph nodes before the ops so either can feed the other.
- **Viewport:** the same Cycles nodes lowered to tree expressions by
  `graph_view` (Math, Noise... in Rust and WGSL already); a node it can't
  lower falls back to the input's own value, as graph layers do (MXG-Q07).

Rejected: a whole-surface graph render whenever a wire exists (loses
see-through, fog, holdout and any `operandi_*` node without a standard
implementation); and growing the op list with every Cycles node (more C++,
no reuse of the translator).

## Slices (each committed on the branch)

1. *(on main)* read-only view.
2. **Free nodes saved:** `GroupNodes`, ShaderTreeEdit `Nodes { group, edit }`
   (add, delete, connect, disconnect, set value, move, hide, show all),
   undo names (NE-18), native 55, layer delete drops wires. NE-T5 save part,
   NE-T6/T7 core.
3. **Wires drive renders:** `Expr::Nodes`, translator/lowering entry points,
   form write/read, Cycles ABI 28, viewport. NE-T1, NE-T5 export/re-import.
4. **Editing in the node view:** layer view, drag to move, wire drag (red
   when refused, NE-08), Add menu (NE-02), Delete/Hide/Show All, stubs
   (NE-07), locks (NE-05), inline values (NE-01), dimmed unwired (NE-14),
   named commands (UIC-160). NE-T3, T4, T8.
5. **Shader Tree and Properties:** "Nodes (N)" row (NE-11), driven channels
   greyed in place with the UIC-112 driven light and a "driven by nodes"
   marker that opens the view on the driving node (NE-04). NE-T2.
6. Full tier, QA app, `docs/NODE_EDITING_GUI_QA.md`.

## Commands (UIC-160, placeholders in docs/NAMES-NEEDED.md)

`operandi.nodes.add`, `.delete`, `.connect`, `.disconnect`, `.setValue`,
`.move`, `.hide`, `.showAll`; all undoable except none. One gesture, one
command, one undo step (UIC-224..227); value and move drags record their
final value on release.

## Risks

- Cycles rebuild for ABI 28 (`scripts/build-cycles.sh`); the op ordering
  change in the C++ is the only behavioural change there.
- Equality: `Expr::Nodes` compares by its MaterialX text, so the
  form-read-back check (`graph == p`) holds.
- Nodes the viewport can't lower draw with the input's own value
  (provisional, as MXG-Q07).
- Which layer inputs are wire targets: material layer channels, free node
  inputs and texture layers' plain settings (node_view_settings.rs, NE-Q01);
  what stays unwired is listed in NE-Q07. Mixing node opacity takes none.
