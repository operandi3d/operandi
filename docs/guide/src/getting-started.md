<!-- help-ids: scene.new, scene.open, scene.save, scene.saveAs, scene.close, app.undo, app.redo, Frame All, Frame Selection, operandi.help.navigation ; template: tutorial ; since: 0.1.0 -->
# Getting started

Operandi is a free, open-source polygon modeler. You build 3D models from
points, edges and polygons, shape them with modeling tools, give them
materials and lights, and render pictures of them. Operandi is licensed
under the GPL v3.

This page gets you from download to a saved scene you can turn around in.

> **Cmd, Ctrl or Control?** This guide writes keyboard shortcuts with
> **Ctrl**, such as **Ctrl+S**: on a Mac, press **Cmd** wherever a shortcut
> says **Ctrl**. In clicks and drags the guide writes **Control**, such as
> **Control**-click: that is the real Control key on every system, a Mac's
> too. **Alt** is the **Option** key on a Mac keyboard.

## Install Operandi

Operandi's builds are on the project's
[releases page](https://github.com/operandi3d/operandi/releases). Each
release has one file per system.

### macOS

The Mac build is for Apple silicon Macs. It is signed and notarized, so
macOS opens it without extra warnings.

1. Download `Operandi-<version>-macOS.dmg`.
2. Open the disk image.
3. Drag **Operandi** to **Applications**.
4. Open Operandi from your Applications folder.

### Windows

The Windows build is for 64-bit Windows. It isn't signed, so Windows
SmartScreen warns you the first time you run it.

1. Download `Operandi-<version>-Windows-x64.zip`.
2. Unzip it to any folder.
3. Run `Operandi.exe`.
4. If Windows says it protected your PC, click **More info**, then
   **Run anyway**.

### Linux

The Linux build is an AppImage for x86_64.

1. Download `Operandi-<version>-Linux-x86_64.AppImage`.
2. Make it runnable: `chmod +x Operandi-<version>-Linux-x86_64.AppImage`.
3. Open it.

Each release also lists a `SHA256SUMS.txt` file, so you can check that a
download arrived intact.

## First launch

Operandi opens with a new, empty scene and the **Model** tool set. The big
area in the middle is the **3D view**, where you see and shape your model.
[Interface](interface/window.md) names every part of the window.

## Create, open and save a scene

A scene holds everything you make: meshes, materials, lights and cameras.
Operandi saves scenes as `.opscene` files.

### Start a new scene

1. Choose **File > New**.

The new scene opens in its own tab, so the scene you were working on stays
open.

### Open a scene

1. Choose **File > Open…**.
2. Pick an `.opscene` file and click **Open**.

To bring models from other file formats (OBJ, STL, glTF, FBX, USD and
more) into the current scene, use **File > Import…** instead.

### Save a scene

1. Choose **File > Save**, or press **Ctrl+S**.
2. The first time, pick a folder and a file name, then click **Save**.

To save a copy under a new name, choose **File > Save As…**. To close the
scene, choose **File > Close Scene** (**Ctrl+W**). If the scene has unsaved
changes, Operandi asks before closing it.

## Move around the 3D view

You move around with the **Alt** key held down and a mouse drag in the
**3D view**. These are the standard navigation keys:

| To | Do this |
|---|---|
| Orbit (turn around the model) | **Alt** + left drag |
| Pan (slide the view) | **Alt+Shift** + left drag |
| Zoom | **Alt+Control** + left drag, or turn the scroll wheel |
| Roll (tilt the view) | **Alt** + middle drag |
| Look around (turn the view on the spot) | **Alt** + right drag |
| Pan up and down only | **Alt+Shift** + right drag |
| Zoom to a box | **Alt+Control** + right drag a rectangle |

You can also drag the **Navigation buttons** at the right of the
**View header** to pan, orbit and zoom without a key held.

These keys help you find your model again:

| To | Press |
|---|---|
| Fit the whole scene in the view | **A** (**View > Frame All**) |
| Fit the selection in the view | **Shift+A** (**View > Frame Selection**) |
| Zoom in or out a step | **.** (period) or **,** (comma) |
| Top, Front or Right view | **Numpad 1**, **Numpad 2** or **Numpad 3** |
| Back to the perspective view; again to look through the scene's camera, and again to come back | **Numpad .** |
| Fill the window with this view, and back | **Numpad 0** or **Alt+0** |

**View > Navigation Help** shows these gestures in the message line at the
bottom of the window.

> **Tip:** If your hand already knows another set of navigation gestures,
> **Edit > Preferences…** (**Ctrl+,**) has other presets under **Input**,
> such as middle-button navigation without **Alt**.

## Undo and redo

Operandi keeps a history of your changes, so you can step back and forward
through them.

- **Undo**: press **Ctrl+Z** or choose **Edit > Undo**.
- **Redo**: press **Ctrl+Shift+Z** or choose **Edit > Redo**.

On Windows and Linux, the **Undo** and **Redo** buttons at the top of the
window do the same.

The **Edit** menu names the change that **Undo** will take back, so you
know what you are undoing before you do it. While a tool is active, each
drag is its own undo step.

## What you learned

- Where to download Operandi and how to install it.
- How to start, open, save and close a scene.
- How to orbit, pan and zoom the [3D view](interface/window.md), and how to
  frame your model.
- How to undo and redo.

Next, meet the rest of the window in [Interface](interface/window.md).
