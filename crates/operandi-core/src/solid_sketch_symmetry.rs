//! Solid Sketch with Symmetry (symmetry-2026-11-13 section 6, all [OP]:
//! Operandi's own design, Jason 2026-11-14).
//!
//! Twins are real nodes of the tree that point at each other
//! (`SketchNode::twin`); every edit goes through the movement path idea of
//! the shared service: the edited node changes, then its twin is rewritten
//! as its mirror image (SS-SYM-01). Seam nodes sit on the plane, have no
//! twin, and keep a mirror-symmetric box (SS-SYM-02). Because a twin's
//! frame is its leader's frame mirrored (`SolidSketch::frames`) and seam
//! boxes are symmetric, the cage built from the tree is one welded,
//! mirror-symmetric mesh (SS-SYM-05): links from a seam node to a node and
//! its twin come off the same seam box.
//!
//! Recommended answers used (README Q6–Q9): seam nodes only turn about the
//! plane's axis; a link crossing the plane is allowed with a warning; a
//! symmetric sketch picked up with Symmetry off or on another axis keeps
//! its twins linked and says so; Symmetry turned on mid-sketch gives twins
//! only to new nodes.
use crate::Result;
use crate::solid_sketch::{NodeFrame, SketchNode, SolidSketch};
use glam::{DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// The mirror plane `normal · p = offset`, in the mesh item's local space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SketchMirror {
    pub normal: DVec3,
    pub offset: f64,
}

/// SS-SYM-04: the status message for a link whose nodes are on opposite
/// sides of the plane.
pub const CROSSING_MESSAGE: &str = "This link crosses the symmetry plane";

impl SketchMirror {
    /// The plane of a Symmetry setting (already in the item's local space);
    /// `None` when Off.
    pub fn of(symmetry: crate::Symmetry) -> Option<Self> {
        Some(Self {
            normal: symmetry.normal()?,
            offset: symmetry.offset,
        })
    }
    /// The same plane, within a hair (the axis labels a status message).
    pub fn same_as(self, other: Self) -> bool {
        self.normal.distance(other.normal) < 1e-9 && (self.offset - other.offset).abs() < 1e-9
    }
    /// "X", "Y", "Z" or "the sketch's plane", for messages.
    pub fn label(self) -> &'static str {
        let n = self.normal;
        if n.distance(DVec3::X) < 1e-9 {
            "X"
        } else if n.distance(DVec3::Y) < 1e-9 {
            "Y"
        } else if n.distance(DVec3::Z) < 1e-9 {
            "Z"
        } else {
            "its own plane"
        }
    }
    fn distance(self, p: DVec3) -> f64 {
        self.normal.dot(p) - self.offset
    }
    pub fn point(self, p: DVec3) -> DVec3 {
        p - self.normal * (2. * self.distance(p))
    }
    pub fn vector(self, v: DVec3) -> DVec3 {
        v - self.normal * (2. * self.normal.dot(v))
    }
    pub fn onto(self, p: DVec3) -> DVec3 {
        p - self.normal * self.distance(p)
    }
    /// A rotation seen in the mirror: same angle, opposite sense about the
    /// mirrored axis (SYM3-11). The axis is a pseudo-vector, so `-M v`.
    pub fn rotation(self, q: DQuat) -> DQuat {
        let v = -self.vector(DVec3::new(q.x, q.y, q.z));
        DQuat::from_xyzw(v.x, v.y, v.z, q.w).normalize()
    }
    /// The part of `q` that turns about the plane's axis only (SS-SYM-02):
    /// the twist of a swing-twist split about the normal.
    pub fn about_axis(self, q: DQuat) -> DQuat {
        let n = self.normal;
        let along = n * DVec3::new(q.x, q.y, q.z).dot(n);
        let t = DQuat::from_xyzw(along.x, along.y, along.z, q.w);
        if t.length_squared() < 1e-24 {
            DQuat::IDENTITY
        } else {
            t.normalize()
        }
    }
    /// A twin's frame: its leader's axes mirrored, then made right-handed
    /// again (Z = X × Y), so its box is the mirror image and its faces
    /// wind outward like every other box.
    pub fn mirror_frame(self, f: NodeFrame) -> NodeFrame {
        let (x, y) = (self.vector(f.x), self.vector(f.y));
        NodeFrame {
            x,
            y,
            z: x.cross(y),
        }
    }
    /// A seam node's frame (SS-SYM-02): the axis nearest the plane's normal
    /// becomes exactly the normal (keeping its sign), the other two lie in
    /// the plane, so the box is mirror-symmetric.
    pub fn seam_frame(self, f: NodeFrame) -> NodeFrame {
        let n = self.normal;
        let flat = |v: DVec3, along: DVec3| {
            let p = v - along * v.dot(along);
            p.try_normalize()
                .unwrap_or_else(|| along.any_orthonormal_vector())
        };
        let d = [f.x.dot(n), f.y.dot(n), f.z.dot(n)];
        let k = (0..3)
            .max_by(|&a, &b| d[a].abs().total_cmp(&d[b].abs()))
            .unwrap_or(0);
        let axis = n * if d[k] < 0. { -1. } else { 1. };
        match k {
            0 => {
                let y = flat(f.y, axis);
                NodeFrame {
                    x: axis,
                    y,
                    z: axis.cross(y),
                }
            }
            1 => {
                let x = flat(f.x, axis);
                NodeFrame {
                    x,
                    y: axis,
                    z: x.cross(axis),
                }
            }
            _ => {
                let x = flat(f.x, axis);
                NodeFrame {
                    x,
                    y: axis.cross(x),
                    z: axis,
                }
            }
        }
    }
    /// `node` mirrored, keeping its links.
    fn mirrored(self, node: &SketchNode, keep: &SketchNode) -> SketchNode {
        SketchNode {
            center: self.point(node.center),
            size: node.size,
            twist: -node.twist,
            rotation: self.rotation(node.rotation),
            parent: keep.parent,
            twin: keep.twin,
            seam: false,
        }
    }
}

impl SolidSketch {
    /// True when any node has a twin.
    pub fn has_twins(&self) -> bool {
        self.nodes.iter().any(|n| n.twin.is_some())
    }
    /// How far from the plane still counts as on it: a hair, relative to
    /// the sketch's reach (the viewport snaps clicks onto the plane first,
    /// SYM3-23).
    fn seam_tolerance(&self, p: DVec3) -> f64 {
        let reach = self
            .nodes
            .iter()
            .map(|n| n.center.abs().max_element())
            .fold(p.abs().max_element(), f64::max);
        (reach * 3e-6).max(1e-9)
    }
    /// SS-SYM-04: a link whose two nodes are on opposite sides.
    pub fn crossing_links(&self) -> bool {
        let Some(m) = self.mirror else {
            return false;
        };
        self.nodes.iter().any(|n| {
            n.parent.is_some_and(|p| {
                let (a, b) = (m.distance(n.center), m.distance(self.nodes[p].center));
                let tol = self.seam_tolerance(n.center);
                (a > tol && b < -tol) || (a < -tol && b > tol)
            })
        })
    }

    /// Add a node with Symmetry's creation rules (SS-SYM-01..03): on the
    /// plane it becomes a seam node; elsewhere it gets a twin under the
    /// parent's twin (under a seam parent, the parent itself; under the
    /// root with no twin, the root: a crossing link, SS-SYM-04). With
    /// `twin_new` false, or no mirror plane, it is an ordinary node
    /// (SS-SYM-09). Returns the new node's index (the one the user placed).
    pub fn add_symmetric(
        &mut self,
        center: DVec3,
        size: f64,
        parent: Option<usize>,
        twin_new: bool,
    ) -> Result<usize> {
        let first = self.nodes.is_empty();
        let index = self.add(center, size, parent)?;
        let Some(m) = self.mirror.filter(|_| twin_new) else {
            return Ok(index);
        };
        // Spec v3.1 section 11 (SYM-B3-10, seen): the first node always
        // lands on the plane, wherever it was clicked.
        if first || m.distance(center).abs() <= self.seam_tolerance(center) {
            let n = &mut self.nodes[index];
            n.center = m.onto(center);
            n.seam = true;
            return Ok(index);
        }
        let parent = self.nodes[index].parent;
        let twin_parent = match parent {
            Some(p) => Some(self.nodes[p].twin.unwrap_or(p)),
            // Only reached for a root made before Symmetry was on: its
            // twin hangs off it (a crossing link).
            None => Some(index),
        };
        let twin = self.add(m.point(center), size, twin_parent)?;
        self.nodes[index].twin = Some(twin);
        self.nodes[twin].twin = Some(index);
        Ok(index)
    }

    /// Ctrl-insert with Symmetry: the link into `child` gets the node, and
    /// the mirrored link into `child`'s twin gets its twin.
    pub fn insert_symmetric(
        &mut self,
        child: usize,
        center: DVec3,
        size: f64,
        twin_new: bool,
    ) -> Result<usize> {
        let partner = self.nodes.get(child).and_then(|n| n.twin);
        let index = self.insert(child, center, size)?;
        let Some(m) = self.mirror.filter(|_| twin_new) else {
            return Ok(index);
        };
        let on_plane = m.distance(center).abs() <= self.seam_tolerance(center);
        let Some(partner) = partner.filter(|_| !on_plane) else {
            if on_plane {
                self.nodes[index].center = m.onto(center);
                self.nodes[index].seam = true;
            }
            return Ok(index);
        };
        // `insert` moved every index at or past `child` up by one.
        let partner = if partner >= child {
            partner + 1
        } else {
            partner
        };
        let twin = self.insert(partner, m.point(center), size)?;
        let index = if index >= partner { index + 1 } else { index };
        self.nodes[index].twin = Some(twin);
        self.nodes[twin].twin = Some(index);
        Ok(index)
    }

    /// Delete a node and its twin (SS-SYM-06); children rejoin parents.
    /// Returns the node to make current.
    pub fn delete_symmetric(&mut self, index: usize) -> Result<Option<usize>> {
        let twin = self.nodes.get(index).ok_or("That node doesn't exist")?.twin;
        let current = self.delete(index)?;
        let Some(t) = twin else {
            return Ok(current);
        };
        let t = if t > index { t - 1 } else { t };
        let after = self.delete(t)?;
        Ok(match current {
            Some(c) if c == t => after,
            Some(c) if c > t => Some(c - 1),
            other => other,
        }
        .filter(|_| !self.nodes.is_empty()))
    }

    /// Delete a node's branch and its twin's branch.
    pub fn delete_branch_symmetric(&mut self, index: usize) -> Result<Option<usize>> {
        let node = self.nodes.get(index).ok_or("That node doesn't exist")?;
        let (twin, parent) = (node.twin, node.parent);
        let mut gone = self.branch(index);
        if let Some(t) = twin {
            gone.extend(self.branch(t));
        }
        gone.sort_unstable();
        gone.dedup();
        let survives = parent.filter(|p| gone.binary_search(p).is_err());
        let current = survives.map(|p| p - gone.iter().filter(|&&g| g < p).count());
        self.remove_indices(&gone);
        Ok(current.filter(|_| !self.nodes.is_empty()))
    }

    /// Movement path for a node edit (SS-SYM-01/02): after an edit that
    /// changed nodes (positions, sizes, turns, twist) but not the tree,
    /// rewrite each twin as its partner's mirror image, taking the side the
    /// user worked on (`primary`, and the nodes changed with it) as the
    /// lead, and keep seam nodes on the plane, turning only about its axis.
    pub fn sync_twins(&mut self, before: &SolidSketch, primary: usize) {
        let Some(m) = self.mirror else {
            return;
        };
        if self.nodes.len() != before.nodes.len() {
            return;
        }
        let side = |s: &SolidSketch, i: usize| m.distance(s.nodes[i].center).signum();
        let primary_side = self.nodes.get(primary).map(|_| side(before, primary));
        for a in 0..self.nodes.len() {
            let Some(b) = self.nodes[a].twin.filter(|&b| b > a) else {
                continue;
            };
            let changed = |i: usize| self.nodes[i] != before.nodes[i];
            let lead = match (changed(a), changed(b)) {
                (false, false) => continue,
                (true, false) => a,
                (false, true) => b,
                // Both moved (a seam node's branch): the user's side leads.
                (true, true) => {
                    if b == primary || primary_side == Some(side(before, b)) && a != primary {
                        b
                    } else {
                        a
                    }
                }
            };
            let follow = if lead == a { b } else { a };
            let mirrored = m.mirrored(&self.nodes[lead], &self.nodes[follow]);
            self.nodes[follow] = mirrored;
        }
        for n in self.nodes.iter_mut().filter(|n| n.seam) {
            n.center = m.onto(n.center);
            n.rotation = m.about_axis(n.rotation);
        }
    }

    /// A translation that keeps a seam node on the plane (SYM3-13): the
    /// part along the normal is dropped.
    pub fn seam_delta(&self, index: usize, delta: DVec3) -> DVec3 {
        match (self.mirror, self.nodes.get(index)) {
            (Some(m), Some(n)) if n.seam => delta - m.normal * m.normal.dot(delta),
            _ => delta,
        }
    }
    /// A turn a seam node may take (SS-SYM-02): about the plane's axis only.
    pub fn seam_turn(&self, index: usize, turn: DQuat) -> DQuat {
        match (self.mirror, self.nodes.get(index)) {
            (Some(m), Some(n)) if n.seam => m.about_axis(turn),
            _ => turn,
        }
    }
}
