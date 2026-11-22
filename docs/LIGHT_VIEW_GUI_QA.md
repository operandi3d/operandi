# Light view GUI check (branch claude/project-thread-bywy7w)

Use the QA build with its own data folder.

1. New scene. Open the view type menu at the viewport's top left: after
   Camera there is **Light ▸**. With the scene's lights deleted it is
   greyed and its tooltip says "The scene has no lights".
2. Add a Spot light and a Point light. Light ▸ lists (Current Selection)
   first, then both lights by name. With a mesh selected, (Current
   Selection) is greyed ("Select a light").
3. Pick the Spot light by name. The view looks from the spot along its
   aim, the header shows the spot's name, the spot's own glyph isn't
   drawn, and the whole cone just fits the view's shorter side.
4. Orbit (Alt+drag), pan and dolly. The light moves with the view (watch
   it in a second viewport); its cone follows. Each press-to-release is
   one undo step: Cmd+Z puts the light back and the view follows it.
5. Pick the Point light: the view looks down its -Z from its position.
   Navigating moves it the same way.
6. Pick (Current Selection), then select the other light in the Items
   list: the view jumps to it. Select a mesh: the view stays on the last
   light.
7. Lock the light (Assembly > Lock) and orbit: the light stays, the view
   snaps back on release, and the status bar says it's locked.
8. Pick Top: the view leaves Light view with the ordinary lens. Pick
   Camera, then a light: each replaces the other.
9. Delete the light you're looking through: the view drops back to
   Perspective and the status bar says "Light view needs a light in the
   scene".
