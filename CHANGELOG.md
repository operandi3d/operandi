# Changelog

Notable changes, newest first. The day-to-day build log is
[docs/PROGRESS.md](docs/PROGRESS.md).

## Unreleased

- OpenPBR Surface 1.1.2 is Operandi's material model: every Material
  channel has an OpenPBR identity, and new channels cover coat colour,
  IOR, darkening and anisotropy, sheen colour and roughness, thin film,
  Abbe number, transmission scatter, thin walled and subsurface albedo.
  New materials start at OpenPBR's defaults (scenes with them save as
  native version 41).
- MaterialX import and export: File > Import MaterialX… and File > Export
  Material as MaterialX…; the Preset Browser lists `.mtlx` files.
- **Look change:** the Material's Dispersion, stored only before, now
  renders on see-through glass. Older scenes that set it look different.
