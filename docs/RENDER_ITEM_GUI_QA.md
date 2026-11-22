# Render item — GUI check

Fresh scene (File > New) unless a step says otherwise.

1. Shader Tree: click **Render** (top row). It highlights and Properties
   shows four tabs, **Frame**, **Settings**, **Global Illumination** and **Preview**. Name reads "Render", greyed.
2. Click Settings, then a material, then Render again: Settings is still
   the tab shown.
3. Frame: Frame Width 1000. The Camera view's frame bands change shape; F9
   renders 1000 wide. Turn on the camera's Override Render Resolution: the
   camera's size wins.
4. Settings: Maximum Quality 4, F9: visibly noisy. Undo: back to 64.
   Drag a field's value: the whole drag is one undo step.
5. Open a scene saved before this build: it renders exactly as before
   (Maximum Quality 16, Noise Threshold 0 %, depths 7 / 7 / 7).
6. Refraction Depth 1 on a glass of water: it goes dark; 8 restores it.
7. Global Illumination > Indirect Caustics None vs Both, glass on a floor: the caustic
   patch goes away.
8. Final Color Output > Denoising: None renders noisier; Intel Open Image
   Denoise cleans it up.
9. Render Camera: with two cameras, pick the other; F9 and the item list's
   render camera follow.
10. Save, close, reopen: every value kept.
11. Preview tab: Convergence Target 50 %: the Preview says Finished at
    about half its Samples. Max Render Time 0.1: the Preview stops after
    about 6 s and keeps its picture; F9 renders to the end.
12. Direct Caustics (tutorial-gaps 2026-11-13). Glass of water on a
    plane, smooth shading on. Add a Spot Light above and behind it, cone
    12°, aimed through the glass at the plane; on its Light Material set
    Affect Diffuse 0 %, Affect Specular 0 % (Affect Caustics stays 100 %).
    F9: the plane is unlit by the spot. Global Illumination > Direct
    Caustics on: F9 shows a bright, focused caustic on the plane past the
    glass. Total Photons, Local Photons and Walkthrough Mode grey out while
    Direct Caustics is off. Affect Caustics 0 % on the spot: the caustic
    goes. Save, reopen: Direct Caustics still on.
13. Absorption Distance: a whisky-coloured liquid (Transparent Color
    amber, index 1.33) with Absorption Distance 50 mm: F9 shows it pale at
    the thin edges and deep amber in the middle; 0 mm makes it one even
    tint.
14. Render fixes F1 (whiskey caustic). Item 12's setup with the liquid of
    item 13 (amber, Absorption Distance 50 mm) inside the glass, Direct
    Caustics on: F9 shows an amber caustic on the plane, not a white one.
    A clear liquid gives a white caustic.
15. Render fixes F7 (glass shadow, Direct Caustics off). A glass ball
    (Transparent Color orange, index 1.5) on a plane, lit by a Directional
    Light at 45°, Direct Caustics off: F9 and the Preview show an even,
    orange see-through shadow, no bright spot and not black. Turn Direct
    Caustics on: the focused caustic comes back. Looking through the ball
    still refracts the plane.
16. Render fixes F3/F6. A new scene with no lights and only an
    environment: F9 and the Preview add no sun (no hard shadow). Delete
    every mesh: F9 and the Preview render the environment instead of
    "Nothing to render".
