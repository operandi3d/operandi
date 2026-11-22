// Operandi's C interface to the Alembic library. GPL-3.0-or-later, Operandi.
// Alembic stores polygons wound clockwise seen from the front; Operandi
// winds them counter-clockwise, so both directions reverse each polygon.
#include "operandi_alembic.h"

#include <Alembic/AbcCoreOgawa/All.h>
#include <Alembic/AbcGeom/All.h>

#include <cstring>
#include <exception>
#include <string>
#include <vector>

namespace {

using namespace Alembic::AbcGeom;

void set_error(char *error, size_t len, const std::string &text) {
    if (error == nullptr || len == 0) return;
    std::strncpy(error, text.c_str(), len - 1);
    error[len - 1] = '\0';
}

struct Reader {
    void *context;
    OaMeshFn on_mesh;
};

// Per-corner UVs from a UV parameter, or empty when it has none usable.
std::vector<float> corner_uvs(IV2fGeomParam param, const Int32ArraySample &indices,
                              size_t points) {
    std::vector<float> out;
    if (!param.valid()) return out;
    IV2fGeomParam::Sample sample = param.getExpandedValue(ISampleSelector());
    V2fArraySamplePtr values = sample.getVals();
    if (!values) return out;
    const size_t corners = indices.size();
    GeometryScope scope = sample.getScope();
    if (scope == kFacevaryingScope && values->size() == corners) {
        out.reserve(corners * 2);
        for (size_t i = 0; i < corners; ++i) {
            out.push_back((*values)[i].x);
            out.push_back((*values)[i].y);
        }
    } else if ((scope == kVertexScope || scope == kVaryingScope) && values->size() == points) {
        out.reserve(corners * 2);
        for (size_t i = 0; i < corners; ++i) {
            const int32_t v = indices[i];
            if (v < 0 || static_cast<size_t>(v) >= points) return {};
            out.push_back((*values)[v].x);
            out.push_back((*values)[v].y);
        }
    }
    return out;
}

void emit(Reader &reader, const std::string &name, const M44d &world, P3fArraySamplePtr positions,
          Int32ArraySamplePtr face_counts, Int32ArraySamplePtr face_indices,
          std::vector<float> uvs) {
    if (!positions || !face_counts || !face_indices) return;
    std::vector<double> points;
    points.reserve(positions->size() * 3);
    for (size_t i = 0; i < positions->size(); ++i) {
        const V3d p = V3d((*positions)[i]) * world;
        points.push_back(p.x);
        points.push_back(p.y);
        points.push_back(p.z);
    }
    const bool mirrored = world.determinant() < 0.0;
    std::vector<int32_t> counts(face_counts->get(), face_counts->get() + face_counts->size());
    std::vector<int32_t> indices(face_indices->get(), face_indices->get() + face_indices->size());
    // Reverse each polygon unless a mirroring transform already did.
    if (!mirrored) {
        size_t at = 0;
        for (int32_t n : counts) {
            if (n < 0 || at + static_cast<size_t>(n) > indices.size()) break;
            for (int32_t a = 1, b = n - 1; a < b; ++a, --b) {
                std::swap(indices[at + a], indices[at + b]);
                if (!uvs.empty()) {
                    std::swap(uvs[2 * (at + a)], uvs[2 * (at + b)]);
                    std::swap(uvs[2 * (at + a) + 1], uvs[2 * (at + b) + 1]);
                }
            }
            at += static_cast<size_t>(n);
        }
    }
    OaMesh mesh;
    mesh.name = name.c_str();
    mesh.points = points.data();
    mesh.point_count = positions->size();
    mesh.face_counts = counts.data();
    mesh.face_count = counts.size();
    mesh.indices = indices.data();
    mesh.index_count = indices.size();
    mesh.uvs = uvs.empty() ? nullptr : uvs.data();
    reader.on_mesh(reader.context, &mesh);
}

void walk(Reader &reader, const IObject &object, const M44d &parent) {
    M44d world = parent;
    const MetaData &meta = object.getMetaData();
    if (IXform::matches(meta)) {
        IXform xform(object, kWrapExisting);
        XformSample sample;
        xform.getSchema().get(sample, ISampleSelector());
        world = sample.getInheritsXforms() ? sample.getMatrix() * parent : sample.getMatrix();
    } else if (IPolyMesh::matches(meta)) {
        IPolyMesh poly(object, kWrapExisting);
        IPolyMeshSchema &schema = poly.getSchema();
        IPolyMeshSchema::Sample sample;
        schema.get(sample, ISampleSelector());
        std::vector<float> uvs;
        if (sample.getFaceIndices() && sample.getPositions())
            uvs = corner_uvs(schema.getUVsParam(), *sample.getFaceIndices(),
                             sample.getPositions()->size());
        emit(reader, object.getName(), world, sample.getPositions(), sample.getFaceCounts(),
             sample.getFaceIndices(), std::move(uvs));
    } else if (ISubD::matches(meta)) {
        ISubD subd(object, kWrapExisting);
        ISubDSchema &schema = subd.getSchema();
        ISubDSchema::Sample sample;
        schema.get(sample, ISampleSelector());
        std::vector<float> uvs;
        if (sample.getFaceIndices() && sample.getPositions())
            uvs = corner_uvs(schema.getUVsParam(), *sample.getFaceIndices(),
                             sample.getPositions()->size());
        emit(reader, object.getName(), world, sample.getPositions(), sample.getFaceCounts(),
             sample.getFaceIndices(), std::move(uvs));
    }
    for (size_t i = 0; i < object.getNumChildren(); ++i) walk(reader, object.getChild(i), world);
}

} // namespace

extern "C" int oa_read(const char *path, void *context, OaMeshFn on_mesh, char *error,
                       size_t error_len) {
    try {
        IArchive archive(Alembic::AbcCoreOgawa::ReadArchive(), path, ErrorHandler::kThrowPolicy);
        if (!archive.valid()) {
            set_error(error, error_len, "not an Alembic (Ogawa) archive");
            return 1;
        }
        Reader reader{context, on_mesh};
        M44d identity;
        identity.makeIdentity();
        walk(reader, archive.getTop(), identity);
        return 0;
    } catch (const std::exception &e) {
        set_error(error, error_len, e.what());
    } catch (...) {
        set_error(error, error_len, "unknown Alembic error");
    }
    return 1;
}

extern "C" int oa_write(const char *path, const OaMesh *meshes, size_t count, char *error,
                        size_t error_len) {
    try {
        OArchive archive(Alembic::AbcCoreOgawa::WriteArchive(), path,
                         Alembic::Abc::MetaData(), ErrorHandler::kThrowPolicy);
        OObject top = archive.getTop();
        for (size_t m = 0; m < count; ++m) {
            const OaMesh &mesh = meshes[m];
            std::vector<V3f> points(mesh.point_count);
            for (size_t i = 0; i < mesh.point_count; ++i)
                points[i] = V3f(static_cast<float>(mesh.points[3 * i]),
                                static_cast<float>(mesh.points[3 * i + 1]),
                                static_cast<float>(mesh.points[3 * i + 2]));
            std::vector<int32_t> counts(mesh.face_counts, mesh.face_counts + mesh.face_count);
            std::vector<int32_t> indices(mesh.indices, mesh.indices + mesh.index_count);
            std::vector<V2f> uvs;
            if (mesh.uvs != nullptr) {
                uvs.resize(mesh.index_count);
                for (size_t i = 0; i < mesh.index_count; ++i)
                    uvs[i] = V2f(mesh.uvs[2 * i], mesh.uvs[2 * i + 1]);
            }
            size_t at = 0;
            for (int32_t n : counts) {
                for (int32_t a = 1, b = n - 1; a < b; ++a, --b) {
                    std::swap(indices[at + a], indices[at + b]);
                    if (!uvs.empty()) std::swap(uvs[at + a], uvs[at + b]);
                }
                at += static_cast<size_t>(n);
            }
            OPolyMesh poly(top, mesh.name);
            OV2fGeomParam::Sample uv_sample;
            if (!uvs.empty())
                uv_sample = OV2fGeomParam::Sample(V2fArraySample(uvs), kFacevaryingScope);
            OPolyMeshSchema::Sample sample(V3fArraySample(points), Int32ArraySample(indices),
                                           Int32ArraySample(counts), uv_sample);
            poly.getSchema().set(sample);
        }
        return 0;
    } catch (const std::exception &e) {
        set_error(error, error_len, e.what());
    } catch (...) {
        set_error(error, error_len, "unknown Alembic error");
    }
    return 1;
}
