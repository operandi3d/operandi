# Render item: open questions

Each has a pick. Build on the pick; Jason can overrule later.

- **RI-Q01 Which layout?** Path-tracer layout (Frame + Settings), or also
  the legacy Global Illumination tab? Pick: path-tracer only. Cycles has
  no irradiance cache, ambient term or photon map, so those rows would be
  dead switches. Note: the reference's new scenes start on the legacy
  renderer `[S]`; we still copy the path-tracer layout.
- **RI-Q02 Where does Render Camera live?** ANSWERED: top of the Frame
  tab (Jason's panel).
- **RI-Q03 Default Samples.** ANSWERED: new scenes 64 (path tracer panel).
  Old scenes keep their stored value (16 today) so they render the same.
- **RI-Q04 Noise Threshold scale.** New scenes 10 % (path tracer panel).
  Old scenes keep their Adaptive Threshold. Pick: Cycles threshold =
  percent / 100 (10 % = 0.1); 0 % = Cycles picks. If QA finds 10 % much
  noisier than expected, change the scale factor, not the default.
- **RI-Q05 Denoise.** The reference app sets denoising per Render Output.
  Pick: move Operandi's scene Denoise onto the Final Color output's
  Denoising row (None / Open Image Denoise), default Open Image Denoise
  so today's look is kept. No Denoise row on the Render item.
- **RI-Q06 Depth defaults.** ANSWERED: Diffuse 8, Reflection 4,
  Refraction 8 (path tracer panel). Builder: say what Operandi passes
  Cycles today if it differs.
- **RI-Q07 Maximum Radiance.** Reference clamps relative to the brightest
  output; Cycles clamps an absolute value. Pick: pass 10.0 straight to
  Cycles' indirect clamp, 0 = off. Direct light is never clamped.
- **RI-Q08 Indirect Caustics default.** Guide silent. Pick: Both (matches
  Cycles' default, so nothing changes).
- **RI-Q09 Max Render Time.** On its own Preview tab, 5 min [S]. Pick:
  it limits the Preview only, never F9 final renders.
- **RI-Q10 Which defaults to confirm later.** If Jason ever wants a bridge
  readback, the rows marked `[pick]` for defaults are the list: Maximum
  Quality, Diffuse Depth, Path Threshold, Indirect Caustics, Displacement
  Rate (1.0 px known), Minimum Edge Length, DPI. Not needed to start.
- **RI-Q11 QuickCam Resolution** (100 %, Frame tab). Not in the guide.
  Pick: don't build until someone knows what it does.
