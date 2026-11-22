/* Operandi's C interface to the Alembic library (Ogawa archives only).
 * GPL-3.0-or-later, Operandi. Every call catches C++ exceptions and turns
 * them into an error message; nothing throws across this boundary. */
#ifndef OPERANDI_ALEMBIC_H
#define OPERANDI_ALEMBIC_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

/* One polygon mesh, world space, Operandi winding (counter-clockwise seen
 * from the front). uvs is NULL or holds two floats per corner. */
typedef struct OaMesh {
    const char *name;
    const double *points; /* xyz per point */
    size_t point_count;
    const int32_t *face_counts;
    size_t face_count;
    const int32_t *indices;
    size_t index_count;
    const float *uvs;
} OaMesh;

/* Called once per mesh read; the data is valid only during the call. */
typedef void (*OaMeshFn)(void *context, const OaMesh *mesh);

/* Reads every polygon and subdivision mesh at its first sample.
 * Returns 0, or 1 with a message in error. */
int oa_read(const char *path, void *context, OaMeshFn on_mesh, char *error, size_t error_len);

/* Writes meshes as polygon meshes under the archive top, one each.
 * Names must be unique and non-empty. Returns 0, or 1 with a message. */
int oa_write(const char *path, const OaMesh *meshes, size_t count, char *error, size_t error_len);

#ifdef __cplusplus
}
#endif
#endif
