# OpenPBR slot check (overrides BEHAVIOR-OPENPBR-MATERIALX.md)

Target OpenPBR 1.1.2 (MaterialX libraries/bxdf/open_pbr_surface.mtlx).

Inputs (default): base_weight 1, base_color 0.8, base_diffuse_roughness 0, base_metalness 0; specular_weight 1, specular_color 1, specular_roughness 0.3, specular_ior 1.5, specular_roughness_anisotropy 0; transmission_weight 0, transmission_color 1, transmission_depth 0, transmission_scatter 0, transmission_scatter_anisotropy 0, transmission_dispersion_scale 0, transmission_dispersion_abbe_number 20; subsurface_weight 0, subsurface_color 0.8, subsurface_radius 1, subsurface_radius_scale (1,0.5,0.25), subsurface_scatter_anisotropy 0; fuzz_weight 0, fuzz_color 1, fuzz_roughness 0.5; coat_weight 0, coat_color 1, coat_roughness 0, coat_roughness_anisotropy 0, coat_ior 1.6, coat_darkening 1; thin_film_weight 0, thin_film_thickness 0.5 (micrometres; x1000 for nm), thin_film_ior 1.4; emission_luminance 0 nits, emission_color 1; geometry_opacity 1, geometry_thin_walled false, geometry_normal, geometry_coat_normal, geometry_tangent, geometry_coat_tangent.

Corrections:

- OPM-Q01 subsurface has its own subsurface_color (+radius_scale, scatter_anisotropy).
- OPM-Q02 dispersion = transmission_dispersion_scale + abbe_number, convert to Cycles.
- OPM-Q03 spec missed transmission_scatter_anisotropy, coat_roughness_anisotropy, geometry_coat_tangent; coat_color not "tint".
- OPM-Q04 thin film micrometres vs Cycles nm.
- OPM-Q05 no gltf_pbr->open_pbr graph in MaterialX; write own.
- OPM-Q06 target 1.1.2.

Licences: OpenPBR and MaterialX both Apache-2.0 (fine for GPLv3); OpenPBR examples/ (83 .mtlx) usable for OPM-T2.
