//! Selection inference (spec selection-inference-2026-11-15 v1.1): a point
//! or edge tool started from another selection mode works out its elements
//! from the current selection. It is the first layer of the tool chain,
//! ahead of the action centre, falloff and symmetry: the converted set is
//! written into the tool's own element list (selected points or edges) of
//! every active mesh, and every later layer reads it as a native one.
//!
//! - Edges and polygons give their points; polygons give their outline
//!   edges (the Boundary set), or every edge when they have no outline;
//!   points give the edges between them. Nothing becomes polygons (SI-01).
//! - Only the current mode's selection is read (SI-04), and the mode and
//!   that selection stay as they were (SI-05, SI-06).
//! - The set is worked out once, when the tool starts (SI-11).
use crate::multi_active::ComponentSelection;
use crate::tool_card::Takes;
use crate::{Command, EdgeCommand, Editor, Mesh, Mode, Result, Tool, VertexCommand};
use std::collections::BTreeSet;

/// What the last tool start inferred, for the status line (SI-08) and to
/// keep the selection made in the current mode (SI-06).
#[derive(Clone, Debug, PartialEq)]
pub struct Inference {
    pub tool: Tool,
    /// The mode the selection was made in (and stays in).
    pub from: Mode,
    /// "Vertex Bevel on 12 points from 3 polygons".
    pub note: String,
}

/// The polygons' outline: edges with exactly one selected polygon on them,
/// or an open border of a selected one (SI-02). Inside edges are left out.
/// The same set as Select ▸ Boundary.
pub fn outline_edges(mesh: &Mesh, faces: &[u32]) -> Vec<(u32, u32)> {
    let mut seen = BTreeSet::new();
    let mut odd: BTreeSet<(u32, u32)> = BTreeSet::new();
    for &f in faces {
        if (f as usize) >= mesh.faces.len() || !seen.insert(f) {
            continue;
        }
        for (a, b) in mesh.element_spans(f as usize) {
            let e = (a.min(b), a.max(b));
            if !odd.remove(&e) {
                odd.insert(e);
            }
        }
    }
    odd.into_iter().collect()
}

/// Every edge of the selected polygons, sorted.
fn all_edges(mesh: &Mesh, faces: &[u32]) -> Vec<(u32, u32)> {
    let mut set = BTreeSet::new();
    for &f in faces {
        if (f as usize) < mesh.faces.len() {
            set.extend(
                mesh.element_spans(f as usize)
                    .map(|(a, b)| (a.min(b), a.max(b))),
            );
        }
    }
    set.into_iter().collect()
}

/// The points `mode`'s selection `c` gives, in selection order (SI-07): an
/// edge's first then second point, a polygon's corners in winding order;
/// a point already listed isn't added again. `shown` leaves hidden points
/// out (SI-10).
pub fn points_from(
    mesh: &Mesh,
    mode: Mode,
    c: &ComponentSelection,
    shown: impl Fn(u32) -> bool,
) -> Vec<u32> {
    let listed: Box<dyn Iterator<Item = u32>> = match mode {
        Mode::Vertex => Box::new(c.vertices.iter().copied()),
        Mode::Edge => Box::new(c.edges.iter().flat_map(|&(a, b)| [a, b])),
        Mode::Polygon => Box::new(
            c.faces
                .iter()
                .filter_map(|&f| mesh.faces.get(f as usize))
                .flat_map(|f| f.vertices.iter().copied()),
        ),
        Mode::Item => Box::new(std::iter::empty()),
    };
    let mut seen = vec![false; mesh.vertices.len()];
    listed
        .filter(|&v| {
            let fresh = seen.get(v as usize).is_some_and(|s| !s) && shown(v);
            if fresh {
                seen[v as usize] = true;
            }
            fresh
        })
        .collect()
}

/// The edges `mode`'s selection `c` gives, and whether polygons with no
/// outline gave every edge instead (SI-02, SI-15). Points give every edge
/// with both ends selected (M06). Hidden edges are left out.
pub fn edges_from(
    mesh: &Mesh,
    mode: Mode,
    c: &ComponentSelection,
    shown: impl Fn((u32, u32)) -> bool,
) -> (Vec<(u32, u32)>, bool) {
    let (edges, no_outline) = match mode {
        Mode::Edge => (c.edges.clone(), false),
        Mode::Vertex => {
            let mut picked = vec![false; mesh.vertices.len()];
            for &v in &c.vertices {
                if let Some(p) = picked.get_mut(v as usize) {
                    *p = true;
                }
            }
            let between = mesh
                .edges()
                .iter()
                .copied()
                .filter(|&(a, b)| picked[a as usize] && picked[b as usize])
                .collect();
            (between, false)
        }
        Mode::Polygon => {
            let outline = outline_edges(mesh, &c.faces);
            if outline.is_empty() && !c.faces.is_empty() {
                (all_edges(mesh, &c.faces), true)
            } else {
                (outline, false)
            }
        }
        Mode::Item => (Vec::new(), false),
    };
    (
        edges.into_iter().filter(|&e| shown(e)).collect(),
        no_outline,
    )
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
fn mode_noun(mode: Mode, n: usize) -> String {
    match mode {
        Mode::Vertex => plural(n, "point", "points"),
        Mode::Edge => plural(n, "edge", "edges"),
        _ => plural(n, "polygon", "polygons"),
    }
}

impl Editor {
    /// The shared inference step, run first when tool `t` starts. A point
    /// or edge tool started from another component mode gets its elements
    /// worked out from the current selection on every active mesh; a
    /// polygon tool refuses Vertex and Edge mode with a reason (SI-03).
    /// Native starts change nothing.
    pub(crate) fn infer_tool_selection(&mut self, t: Tool) -> Result<()> {
        self.inference = None;
        let card = t.card();
        let from = self.scene.selection.mode;
        if from == Mode::Item {
            return Ok(());
        }
        let target = match card.takes {
            Takes::Native => return Ok(()),
            Takes::Polygons => {
                if !card.modes.contains(from) {
                    return Err(format!(
                        "{} needs polygons: switch to Polygon mode, or Convert the selection",
                        t.label()
                    ));
                }
                return Ok(());
            }
            Takes::Points => Mode::Vertex,
            Takes::Edges => Mode::Edge,
        };
        if from == target {
            return Ok(());
        }
        let (sources, made, no_outline) = self.convert_selection(from, target);
        let what = match target {
            Mode::Vertex => plural(made, "point", "points"),
            _ if no_outline => format!(
                "all {} of {} (no outline)",
                plural(made, "edge", "edges"),
                mode_noun(from, sources)
            ),
            _ => plural(made, "edge", "edges"),
        };
        let note = if sources == 0 {
            format!("{} on nothing selected: as in its own mode", t.label())
        } else if no_outline {
            format!("{} on {what}", t.label())
        } else {
            format!("{} on {what} from {}", t.label(), mode_noun(from, sources))
        };
        self.inference = Some(Inference {
            tool: t,
            from,
            note,
        });
        Ok(())
    }

    /// Writes the `target` elements `from`'s selection gives into every
    /// active mesh's `target` list (SI-10); the mode is left alone.
    /// Returns how many source elements were read, how many came out, and
    /// whether polygons with no outline gave every edge.
    fn convert_selection(&mut self, from: Mode, target: Mode) -> (usize, usize, bool) {
        let mut ids: Vec<u64> = self.scene.selection.item.into_iter().collect();
        ids.extend(self.scene.selection.others.keys().copied());
        let (mut sources, mut made, mut no_outline) = (0, 0, false);
        for id in ids {
            let Some(item) = self.scene.item(id) else {
                continue;
            };
            let mesh = &item.mesh;
            let protection = crate::Protection::of(mesh, &item.elements);
            let vertex_hidden = protection.as_ref().map(|p| p.vertex_hidden.clone());
            let shown_point = |v: u32| {
                vertex_hidden
                    .as_ref()
                    .is_none_or(|h| !h.get(v as usize).copied().unwrap_or(false))
            };
            let c = self.scene.selection.clone().take_components(id);
            sources += c.count(from);
            let mut converted = c.clone();
            if target == Mode::Vertex {
                converted.vertices = points_from(mesh, from, &c, shown_point);
                made += converted.vertices.len();
            } else {
                let (edges, whole) =
                    edges_from(mesh, from, &c, |(a, b)| shown_point(a) && shown_point(b));
                no_outline |= whole;
                made += edges.len();
                converted.edges = edges;
            }
            self.scene.selection.put_components(id, converted);
        }
        (sources, made, no_outline)
    }

    /// Back to mode `from` with the selection `was` made in it, where its
    /// elements still exist (SI-05, SI-06); the other lists (the tool's
    /// own result) stay as the edit left them.
    fn restore_source(&mut self, from: Mode, was: &crate::scene::Selection) {
        let mut ids: Vec<u64> = was.item.into_iter().collect();
        ids.extend(was.others.keys().copied());
        for id in ids {
            let Some(item) = self.scene.item(id) else {
                continue;
            };
            let mesh = &item.mesh;
            let was = was.clone().take_components(id);
            let mut now = self.scene.selection.clone().take_components(id);
            match from {
                Mode::Polygon => {
                    now.faces = was
                        .faces
                        .into_iter()
                        .filter(|&f| (f as usize) < mesh.faces.len())
                        .collect();
                }
                Mode::Edge => {
                    let existing: BTreeSet<(u32, u32)> = mesh.edges().iter().copied().collect();
                    now.edges = was
                        .edges
                        .into_iter()
                        .filter(|e| existing.contains(e))
                        .collect();
                }
                Mode::Vertex => {
                    now.vertices = was
                        .vertices
                        .into_iter()
                        .filter(|&v| (v as usize) < mesh.vertices.len())
                        .collect();
                }
                Mode::Item => {}
            }
            self.scene.selection.put_components(id, now);
        }
        self.scene.selection.mode = from;
    }

    /// After a tool's preview: the selection made in the starting mode is
    /// kept where its elements still exist (SI-06); the tool's own result
    /// stays in its element list.
    pub(crate) fn keep_inferred_source(&mut self) {
        if self.inferred_mode().is_none() {
            return;
        }
        let Some(from) = self.inference.as_ref().map(|i| i.from) else {
            return;
        };
        let Some(was) = self.session.live.as_ref().map(|l| l.base.selection.clone()) else {
            return;
        };
        self.restore_source(from, &was);
    }

    /// One-shot point and edge commands that infer (spec §4: Join, Join
    /// Averaged, Align and the distance Merge take points; Split and Split Normals take edges).
    /// Mode-aware commands (Remove, Collapse, Split Points...) don't.
    pub(crate) fn command_takes(cmd: &Command) -> Option<Mode> {
        match cmd {
            Command::VertexCommand(
                VertexCommand::Join { .. }
                | VertexCommand::Align { .. }
                | VertexCommand::Merge { .. },
            ) => Some(Mode::Vertex),
            Command::EdgeCommand(EdgeCommand::Split | EdgeCommand::SplitNormals) => {
                Some(Mode::Edge)
            }
            _ => None,
        }
    }

    /// Runs `cmd` on the `target` elements the current selection gives,
    /// then puts the mode and the selection made in it back. The undo step
    /// it records goes from and back to that selection (SI-09).
    pub(crate) fn dispatch_inferred(
        &mut self,
        target: Mode,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        let from = self.scene.selection.mode;
        let saved = self.scene.selection.clone();
        let mark = self.history_mark();
        self.convert_selection(from, target);
        self.scene.selection.mode = target;
        if let Err(e) = self.dispatch_inner(cmd, report) {
            self.scene.selection = saved;
            return Err(e);
        }
        self.restore_source(from, &saved);
        // By mark (Astra P9-006): at Max Undo Level the length doesn't grow.
        if self.steps_since(mark) > 0
            && let Some(step) = self.history.undo.last_mut().map(std::sync::Arc::make_mut)
        {
            step.before.selection = saved;
            step.after.selection = self.scene.selection.clone();
        }
        Ok(())
    }

    /// The mode tool `t` works in when its start was inferred.
    pub(crate) fn inferred_mode_for(&self, t: Tool) -> Option<Mode> {
        let i = self.inference.as_ref().filter(|i| i.tool == t)?;
        let target = match t.card().takes {
            Takes::Points => Mode::Vertex,
            Takes::Edges => Mode::Edge,
            _ => return None,
        };
        (target != i.from).then_some(target)
    }
    /// The open tool's own mode, when its start was inferred. The viewport
    /// draws the inferred elements as that mode's selection (SI-Q05).
    pub fn inferred_mode(&self) -> Option<Mode> {
        self.inferred_mode_for(self.session.live.as_ref()?.tool)
    }

    /// The status line while a tool works on an inferred selection
    /// (SI-08), or `None`.
    pub fn inference_note(&self) -> Option<&str> {
        let i = self.inference.as_ref()?;
        (self.session.live.as_ref().map(|l| l.tool) == Some(i.tool)).then_some(i.note.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two quads side by side: 0-1-2 along the bottom, 3-4-5 the top.
    fn strip() -> Mesh {
        let mut m = Mesh {
            vertices: (0..6)
                .map(|i| glam::DVec3::new((i % 3) as f64, (i / 3) as f64, 0.))
                .collect(),
            ..Default::default()
        };
        for f in [vec![0, 1, 4, 3], vec![1, 2, 5, 4]] {
            m.faces.push(crate::Face {
                vertices: f,
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
        m
    }
    fn sel(vertices: &[u32], edges: &[(u32, u32)], faces: &[u32]) -> ComponentSelection {
        ComponentSelection {
            vertices: vertices.to_vec(),
            edges: edges.to_vec(),
            faces: faces.to_vec(),
        }
    }

    #[test]
    fn outline_leaves_the_inside_edge_out() {
        let m = strip();
        let outline = outline_edges(&m, &[0, 1]);
        assert_eq!(outline.len(), 6);
        assert!(!outline.contains(&(1, 4)));
        // One polygon: all four of its edges.
        assert_eq!(outline_edges(&m, &[0]).len(), 4);
    }

    #[test]
    fn points_follow_selection_order_without_repeats() {
        let m = strip();
        let from_edges = points_from(&m, Mode::Edge, &sel(&[], &[(1, 4), (0, 1)], &[]), |_| true);
        assert_eq!(from_edges, [1, 4, 0]);
        let from_faces = points_from(&m, Mode::Polygon, &sel(&[], &[], &[1, 0]), |_| true);
        assert_eq!(from_faces, [1, 2, 5, 4, 0, 3]);
        let shown = points_from(&m, Mode::Polygon, &sel(&[], &[], &[0]), |v| v != 4);
        assert_eq!(shown, [0, 1, 3]);
    }

    #[test]
    fn edges_from_points_and_from_a_closed_selection() {
        let m = strip();
        let (between, _) = edges_from(&m, Mode::Vertex, &sel(&[0, 1, 4], &[], &[]), |_| true);
        assert_eq!(between, [(0, 1), (1, 4)]);
        let (none, _) = edges_from(&m, Mode::Vertex, &sel(&[0, 2], &[], &[]), |_| true);
        assert!(none.is_empty());
        // Nothing selected gives nothing: the tool's own empty rule applies.
        let (empty, whole) = edges_from(&m, Mode::Polygon, &sel(&[], &[], &[]), |_| true);
        assert!(empty.is_empty() && !whole);
    }
}
