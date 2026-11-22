# Backdrop item: behaviour rules

Tags: **observed** (oracle or capture), **guide** (help text, not
measured), **predicted** (our reading, not yet seen).

## Creating and loading

- **BD-01 (guide, observed)** Item > Create Backdrop, or Add Item >
  Backdrop Item, adds one Backdrop at the world origin with no parent. It
  becomes the only selected item. The first is named "Backdrop Item", later
  ones "Backdrop Item (2)", "Backdrop Item (3)" and so on.
- **BD-02 (observed, corrected in v2)** A new Backdrop has no image. Until
  one is set it draws as a 1 m square filled with a fine white dot pattern,
  centred on the item, in the views where it would show (BD-O-58). (v1 said
  it draws nothing; that was wrong.) Operandi deliberately draws its own
  orange outline placeholder instead (see README). Defaults: Projection Type Front, Show In Perspective on, Keep Aspect on,
  Flip off, Invert off, Blend off, Contrast 0 %, Brightness 0 %,
  Transparency 0 %, Use Uniform Size off, Pixel Size 10 mm; position 0,
  rotation 0, scale 1. (Oracle BD-O-01.)
- **BD-03 (guide, observed)** The Image setting picks an image from the
  scene's loaded images, or loads a new file from disk. Loading a file adds
  one still-image entry to the scene, named after the file. The Backdrop
  only refers to that image; several Backdrops may share one image.
  Attaching an image does not change the Backdrop's transform.
  Loading a file that is already loaded reuses the existing entry. A
  different file with the same name gets the name with " (2)" added
  (BD-O-63).
- **BD-04 (observed)** Changing the Backdrop's image or any of its settings
  never changes the image entry itself.

## Where it is drawn

- **BD-05 (observed)** The picture is a flat rectangle centred on the
  item's position, lying in the plane that faces the chosen projection:
  - Front and Back: the XY plane through the item. Image left at -X, image
    top at +Y.
  - Top and Bottom: the XZ plane through the item. Image left at -X, image
    top at -Z.
  - Left and Right: the YZ plane through the item. Image left at -Z, image
    top at +Y.
  (Oracles BD-O-02..07.)
- **BD-06 (observed)** Paired projections share one placement, so the image
  reads normally from Front, Top and Left and appears mirrored left to right
  from Back, Bottom and Right. Flip (BD-21) is the fix for the mirrored
  view.
- **BD-07 (observed)** In orthographic views a Backdrop is drawn only in the
  view whose direction matches its projection: a Front Backdrop shows in the
  Front view but not in Back, Top or the others. (BD-O-08.)
- **BD-08 (observed)** Item position moves the picture within its plane.
  The position along the viewing axis (Z for Front) makes no visible
  difference in the matching orthographic view. (BD-O-09.)
- **BD-09 (observed)** Item rotation turns the picture: a 30° rotation
  about Z on a Front Backdrop turns it 30° counter-clockwise as seen from
  Front. (BD-O-10.) Rotation about other axes: BD-Q05.
- **BD-10 (observed)** Draw order in an orthographic view: the Backdrop
  covers the viewport background and the grid, and every mesh is drawn over
  it, even a mesh placed behind the Backdrop's plane. The picture is opaque
  unless Transparency is above 0. (BD-O-11.)
- **BD-11 (observed)** In the Perspective view, Backdrops with Show In
  Perspective on are drawn as flat rectangles in their 3D plane (BD-05).
  With it off they are hidden in perspective but still shown in their
  orthographic view. (BD-O-12, BD-O-13.)
- **BD-11a (observed, v2)** In Perspective the picture is drawn over meshes,
  even a mesh nearer the camera than the picture; the floor grid in front
  of the picture still shows over it (BD-O-60). This is the opposite of the
  orthographic views (BD-10). Operandi deliberately keeps meshes on top
  here (see README); BD-O-60 records Modo's behaviour for reference only.
- **BD-11b (observed, v2, one sample)** When two Backdrops overlap in the
  same view, the one created first was drawn on top (BD-O-61).
- **BD-12 (observed)** A Camera-projection Backdrop is not drawn in
  orthographic or Perspective views. In a view looking through a camera it
  fills the camera's frame region exactly, stretched to the frame's shape
  (a 2:1 image filled a 16:9 frame), and the grid draws over it. Its
  position, scale and pixel size have no observed effect there (BD-Q06).
  (BD-O-14.)
- **BD-13 (observed)** Each 3D view has a Show Backdrop switch, on by
  default. Off hides every Backdrop in that view only. (BD-O-15.)
- **BD-14 (guide)** Backdrops never appear in renders, and hiding the item
  (its visibility toggle in the Item List) hides it in all views.

## Size

Let the image be w × h pixels, p the Pixel Size, and (sa, sb) the item's
scale along the plane's horizontal and vertical axes: (X, Y) for
Front/Back, (X, Z) for Top/Bottom, (Z, Y) for Left/Right. The scale along
the viewing axis is ignored.

- **BD-15 (observed)** Base size. With Use Uniform Size off the base
  height is H0 = h × p; with it on, H0 = 1 m whatever the image size.
- **BD-16 (observed)** Keep Aspect on: the picture keeps the image's shape,
  W0 = H0 × w / h, and both sides are multiplied by the smaller of sa and
  sb. Scaling one axis alone therefore changes nothing until the other is
  scaled too. Example: 512 × 256 at 4 mm, scale (2, 3) gives 4.096 ×
  2.048 m (uniform off) or 4 × 2 m (uniform on). (BD-O-16..BD-O-28.)
- **BD-17 (observed)** Keep Aspect off: the picture starts as a square of
  side H0 and is stretched independently, width = H0 × sa, height = H0 × sb.
  So turning Keep Aspect off on an unscaled 2:1 image squeezes it to a
  square. (BD-O-29..BD-O-36.)
- **BD-18 (observed)** Pixel Size can't be changed while Use Uniform Size is
  on; Modo refuses the edit with a note that it only applies when Use
  Uniform Size is off. Show the field disabled in that state. (BD-O-37.)
- **BD-19 (guide)** Backdrop Size and Backdrop Size Keep Aspect (buttons on
  the panel) ask for a width and height (or one width) and resize the
  Backdrop to match. Exact behaviour: BD-Q08.

## Colour and display options

Pixel values below are 0..1. The adjustments are applied per pixel in this
order, then clamped to 0..1:

1. Invert: v → 1 − v.
2. Contrast c (−100..+100 % as −1..+1): v → v × (1 + c). It scales toward
   black, not around mid-grey.
3. Brightness b (same range): v → v + b.
4. Clamp to 0..1.

- **BD-20 (observed)** The order and maths above match every sampled value
  to within one 8-bit step (BD-O-38..BD-O-45). Applies to R, G and B
  alike.
- **BD-21 (observed)** Flip mirrors the picture left to right within its
  rectangle; the rectangle itself does not move. (BD-O-46.)
- **BD-22 (observed)** Transparency t: the picture is mixed with whatever is
  behind it at (1 − t). 0 % is opaque, 100 % invisible. Measured over the
  viewport background: 255 → 152 and 0 → 24 at 50 % over grey 48.
  (BD-O-47, BD-O-48.)
- **BD-23 (observed)** Blend off: each image pixel is a hard-edged square
  (nearest pixel). Blend on: smooth interpolation between neighbouring
  pixels; pixel centres keep their values. (BD-O-49.)
- **BD-24 (observed)** Colour options belong to each Backdrop. Note: in
  Modo, Backdrops that share one image sometimes show each other's colour
  settings; treat that as a Modo fault and keep them independent.
- **BD-25 (observed)** Modo keeps three more per-item settings with no panel
  controls (named Gamma, Blur and Render Item in Modo's help). Changing
  Gamma or Blur had no visible effect. Don't expose them; keep their values
  if a scene file carries them (see BD-Q10).
- **BD-26 (guide)** Contrast, Brightness and Transparency are percentage
  fields on the panel.
- **BD-27 (observed, v2)** The image's own alpha is respected: fully
  transparent pixels show what is behind, partly transparent ones mix
  linearly (50 % white over grey 48 drew 152) (BD-O-59).

## Auto Fit

Box below means the bounding box of the active mesh items (the mesh layers
being edited, which can stay active when only the Backdrop is selected).

- **BD-30 (observed)** Auto Fit works only with Use Uniform Size on. With it
  off nothing changes (Modo's message says it only applies in uniform size).
  (BD-O-56.)
- **BD-31 (observed)** Auto Fit moves the Backdrop to the box centre within
  its plane, and along the viewing axis to the box face nearest the matching
  view: largest Z for Front, largest Y for Top, largest X for Right;
  smallest Z for Back, smallest Y for Bottom, smallest X for Left
  (BD-O-50..BD-O-55, BD-O-64..BD-O-66).
- **BD-32 (observed)** Auto Fit sets the in-plane scale so the picture's
  height equals the box height in the plane (Y for Front/Right, Z for Top).
  With Keep Aspect on both in-plane scales get that value, so width follows
  the image shape. With Keep Aspect off the horizontal scale is the box
  width instead. Earlier position and in-plane scale are overwritten;
  rotation is left alone. (BD-O-50..BD-O-55.)
- **BD-33 (observed, v2)** Auto Fit resets the out-of-plane scale to 1 (a
  preset of 3 came back as 1 for Front and Top). (BD-O-67.)

## Selection, interaction, files

- **BD-40 (observed)** A selected Backdrop shows a one-pixel outline in the
  item-selection colour round its picture rectangle; unselected Backdrops
  have no outline. (Capture 26.)
- **BD-41 (guide)** Backdrops are moved, rotated and scaled with the normal
  item transform tools and appear in the Item List like other items.
  Clicking a Backdrop in a viewport to select it: BD-Q01.
- **BD-42 (guide)** View fitting includes visible Backdrops when the
  Display preference "Include Backdrops in Viewport Fitting" is on (default
  on).
- **BD-43 (guide)** Display preferences "Display Backdrops in Uniform Size"
  (default off) and "Pixel Size" (default 10 mm) set the starting values for
  new Backdrops (predicted; only the matching defaults were seen, BD-Q07).
- **BD-50 (observed)** Saving and reopening a scene restores every Backdrop
  setting, name, transform and image link; the image is referred to by its
  file path, not copied into the scene. (BD-O-57.)
- **BD-51 (observed)** Number settings round-trip as single-precision values
  (0.3 comes back as 0.30000001).
- **BD-52 (guide)** The Properties panel order is: name and transform
  controls, then Image, Show In Perspective, Projection Type, Keep Aspect,
  Flip, Invert, Blend, Contrast, Brightness, Transparency, Use Uniform Size,
  Pixel Size, then the Auto Fit, Backdrop Size and Backdrop Size Keep
  Aspect buttons.

## Not this item

The Schematic's "node backdrops" (frames that group nodes in the node
graph) share the word but are a different feature. Modo also has an older
per-view "Background Imagery" option set (image, invert, pixel blending,
overlay, contrast, brightness, transparency) in the 3D viewport
properties; it is not part of this spec (BD-Q11).
