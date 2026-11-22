//! Construction roles returned with an extrusion candidate. They refer to the
//! input's dense layout, never positions or guessed nearest points. Consumers
//! validate this map before rebinding stable identities or authored data.
use crate::{Mesh, Result};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WallOrigin {
    pub face: usize,
    pub selected: usize,
    pub owner: usize,
    pub a: u32,
    pub b: u32,
    pub row: usize,
    /// Exact output indices in oriented wall order, recorded during construction.
    pub vertices: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EditCorrespondence {
    pub source_layout: Vec<Vec<u32>>,
    pub source_vertex_count: usize,
    /// Each new row point's original source point; original prefix is unchanged.
    pub vertex_sources: Vec<u32>,
    /// Source face for every surviving cap, additional cap, or owned wall.
    pub face_sources: Vec<usize>,
    pub walls: Vec<WallOrigin>,
}
impl EditCorrespondence {
    pub(crate) fn identity(source: &Mesh) -> Self {
        Self {
            source_layout: source.faces.iter().map(|f| f.vertices.clone()).collect(),
            source_vertex_count: source.vertices.len(),
            vertex_sources: (0..source.vertices.len() as u32).collect(),
            face_sources: (0..source.faces.len()).collect(),
            walls: Vec::new(),
        }
    }
    pub fn validate(&self, source: &Mesh, output: &Mesh) -> Result<()> {
        source.validate()?;
        output.validate()?;
        if self.source_vertex_count != source.vertices.len()
            || self.source_layout
                != source
                    .faces
                    .iter()
                    .map(|f| f.vertices.clone())
                    .collect::<Vec<_>>()
            || self.vertex_sources.len() != output.vertices.len()
            || self.face_sources.len() != output.faces.len()
            || output.vertices.len() < source.vertices.len()
            || output.faces.len() < source.faces.len()
            || self
                .vertex_sources
                .iter()
                .any(|&v| v as usize >= source.vertices.len())
            || self.vertex_sources[..source.vertices.len()]
                .iter()
                .enumerate()
                .any(|(i, &v)| i != v as usize)
            || self.face_sources[..source.faces.len()]
                .iter()
                .enumerate()
                .any(|(i, &f)| i != f)
            || self.face_sources.iter().any(|&f| f >= source.faces.len())
        {
            return Err("Extrude correspondence scope/layout/coverage mismatch".into());
        }
        let mut seen = BTreeSet::new();
        for w in &self.walls {
            if w.face < source.faces.len()
                || !seen.insert(w.face)
                || w.face >= output.faces.len()
                || w.selected >= source.faces.len()
                || w.owner >= source.faces.len()
                || self.face_sources[w.face] != w.owner
                || w.row == 0
                || w.vertices.len() != 4
                || output.faces[w.face].vertices != w.vertices
                || !source.faces[w.owner].vertices.contains(&w.a)
                || !source.faces[w.owner].vertices.contains(&w.b)
            {
                return Err("Invalid extrusion wall origin".into());
            }
            let mapped: Vec<_> = w
                .vertices
                .iter()
                .map(|&v| self.vertex_sources[v as usize])
                .collect();
            if mapped != [w.a, w.a, w.b, w.b] && mapped != [w.a, w.b, w.b, w.a] {
                return Err("Wall endpoints disagree with source correspondence".into());
            }
        }
        for (f, &s) in self.face_sources.iter().enumerate() {
            if !seen.contains(&f) {
                let source_cycle = &source.faces[s].vertices;
                let mapped = output.faces[f]
                    .vertices
                    .iter()
                    .map(|&v| self.vertex_sources[v as usize])
                    .collect::<Vec<_>>();
                let mut reversed = source_cycle.clone();
                reversed[1..].reverse();
                if mapped != *source_cycle && mapped != reversed {
                    return Err("Cap vertex/corner ordinal coverage mismatch".into());
                }
            }
        }
        Ok(())
    }
    /// Authored source corner and whether it is on a new tip, for each wall
    /// corner. Cap records use source-face ordinal correspondence instead.
    pub fn wall_corner(&self, output_face: usize, corner: usize) -> Option<(usize, usize, bool)> {
        let w = self.walls.iter().find(|w| w.face == output_face)?;
        let v = *w.vertices.get(corner)?;
        let source = self.vertex_sources[v as usize];
        let c = self.source_layout[w.owner]
            .iter()
            .position(|&x| x == source)?;
        Some((w.owner, c, v as usize >= self.source_vertex_count))
    }
}
