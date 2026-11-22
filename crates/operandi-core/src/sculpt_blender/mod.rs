// SPDX-FileCopyrightText: 2024 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender's sculpt brushes
// (`source/blender/editors/sculpt_paint/mesh/`) at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22. Blender is
// GPL-2.0-or-later; this module stays under that licence (see
// docs/DEPENDENCIES.md, "Blender sculpt brush maths").

//! Sculpt brush maths ported from Blender: falloff curves (`curve`), the
//! brush plane and local frame (`plane`), Kelvinlet elastic deformation
//! (`kelvinlet`) and the per-point brush rules (`brushes`).
//!
//! Only the maths comes from Blender, not its UI, tool settings or stroke
//! machinery (PBVH, undo, dyntopo). Operandi's own stroke
//! (`crate::sculpt`) finds the points under a dab, weighs them with its
//! tip, and calls these rules; which rule each Operandi brush uses is
//! listed on `crate::SculptBrush`.

pub mod brushes;
pub mod curve;
pub mod kelvinlet;
pub mod plane;
