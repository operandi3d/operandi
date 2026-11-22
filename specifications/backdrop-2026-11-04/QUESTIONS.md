# Backdrop item: open questions (after v1)

"Hands-on" means it needs a person at Modo; research is following it up.

| ID | Question | Route | State |
|---|---|---|---|
| BD-Q01 | Can a Backdrop be selected by clicking its picture in a viewport (ortho, perspective)? Does a click on the picture pass through to a mesh behind it? Rectangle/lasso selection? | Hands-on | Open |
| BD-Q02 | Does the transform tool's handle sit at the item position, and do Move/Rotate/Scale drags in the Front view act in the picture's plane? | Hands-on | Open |
| BD-Q03 | Auto Fit for Back, Bottom and Left: which box face does the Backdrop land on? | Answered v2 | Smallest Z / Y / X (BD-31) |
| BD-Q04 | Auto Fit: is the out-of-plane scale kept or reset? Several active meshes: one combined box? No active mesh? | Scale: answered v2, reset to 1 (BD-33). Several/no meshes: script | Partly |
| BD-Q05 | Rotation about an in-plane axis (for example X on a Front Backdrop): does the ortho view show a foreshortened picture, and does Perspective show the tilted plane? | Script + capture | Open |
| BD-Q06 | Camera projection: do position, scale, Pixel Size, Flip and the colour options apply? Which camera when several exist? | Script + capture | Partly (fills frame) |
| BD-Q07 | Do new Backdrops take Use Uniform Size and Pixel Size from the Display preferences? (Needs a preference change and back, Jason's OK.) | Script, with approval | Open |
| BD-Q08 | Backdrop Size / Backdrop Size Keep Aspect: what dialog, which fields, which axes per projection, and the result. | Hands-on | Open |
| BD-Q09 | The Image control: list of loaded images, "(load image)" file dialog, choosing none, deleting the image entry while in use. | Hands-on | Open |
| BD-Q10 | Gamma and Blur: does any viewport mode (Advanced viewport, other drawing styles) show an effect? | Hands-on | Open |
| BD-Q11 | The older per-view Background Imagery options: still working in 17.1? Worth building? | Jason's call | Open |
| BD-Q12 | Undo: is creating, attaching an image, Auto Fit, and each option change one undo step? | Hands-on | Open |
| BD-Q13 | Reopening a scene whose image file has moved: message, placeholder, or silent? (Risky in Modo: run by hand, not by script.) | Hands-on | Open |
| BD-Q14 | Draw order between two Backdrops that overlap in the same view (item order? depth?). | One sample: first-created on top (BD-11b); needs a reorder test | Partly |
| BD-Q15 | Is the picture depth-tested against meshes in Perspective? | Answered v2: no, it covers meshes (BD-11a) | Answered |
| BD-Q16 | Image alpha channel: are transparent pixels in a PNG drawn transparent? | Answered v2: yes, linear mix (BD-27) | Answered |
| BD-Q17 | Item visibility toggle and Item List behaviour (eye, lock, render toggle hidden?). | Hands-on | Open |
| BD-Q18 | (From the build side.) A free orthographic view, one not lined up with Front/Top/Left etc.: are Backdrops drawn, and how? | Modo has no free-rotating orthographic view type; dragging an ortho view round is hands-on. Operandi's choice stands. | Partly |
| BD-Q19 | (From the build side.) Reference command names (Operandi used operandi.backdrop.create / .image / .attr / .autoFit / .size). | Research can't supply Modo's internal command names under the clean-room rule; specs give UI names only (Create Backdrop, Image, Auto Fit, Backdrop Size). Operandi's own names are its call. | Answered (no change) |
| BD-Q20 | (From the build side.) How image entries are named. Seen: loading `quad_512x256.png` made an entry named `quad_512x256` (file name without extension). Same file twice reuses the entry; same name from another folder gets " (2)" (BD-03). Renaming still open. | Mostly answered v2 |

Answered in v1: defaults, projection placement and mirroring, which views
show what, size rules for all Uniform/Keep Aspect/Scale combinations seen,
colour maths and order, Transparency, Blend, Auto Fit for Front/Top/Right,
Show In Perspective, per-view Show Backdrop, selection outline, save
round trip.
