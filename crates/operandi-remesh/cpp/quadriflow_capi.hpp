/* SPDX-FileCopyrightText: 2019 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * Adapted from Blender intern/quadriflow/quadriflow_capi.hpp at e1bec63572c7092feb1af6719f8b4f76acce15fc.
 * Operandi changes: sharp_angle and QFLOW_free_result. */

#ifndef QUADRIFLOW_CAPI_HPP
#define QUADRIFLOW_CAPI_HPP

#ifdef __cplusplus
extern "C" {
#endif

struct QuadriflowRemeshData {
  const float *verts;
  const int *faces;
  int totfaces;
  int totverts;

  float *out_verts;
  int *out_faces;
  int out_totverts;
  int out_totfaces;

  int target_faces;
  bool preserve_sharp;
  /* Operandi: crease angle in degrees for preserve_sharp. */
  float sharp_angle;
  bool preserve_boundary;
  bool adaptive_scale;
  bool minimum_cost_flow;
  bool aggresive_sat;
  int rng_seed;
};

void QFLOW_quadriflow_remesh(QuadriflowRemeshData *qrd,
                             void (*update_cb)(void *, float progress, int *cancel),
                             void *update_cb_data);

/* Frees out_verts and out_faces and zeroes the counts. */
void QFLOW_free_result(QuadriflowRemeshData *qrd);

#ifdef __cplusplus
}
#endif

#endif  // QUADRIFLOW_CAPI_HPP
