# Scene file association (.opscene)

## macOS (done)

- `assets/macos/opscene-document-type.plist` is merged into every app's
  Info.plist by `scripts/macos-app-bundle.sh` (QA builds and releases). It
  exports the type `org.operandi3d.opscene` (extension `opscene`, conforms to
  `public.data`/`public.content`) and claims it as **Owner**, role Editor.
- Finder shows `OperandiScene.icns`, built from
  `assets/branding/operandi-document-icon.png` (OP-DOC-1). Regenerate the
  PNG with `tools/document-icon/make-document-icon.swift` and update its
  hash in `branding-manifest.json`.
- `crates/operandi-app/src/open_documents.rs` takes the "open documents"
  Apple Event (winit doesn't pass it on). Its handler is installed when
  NSApplication posts will-finish-launching, so a file that launches the
  app opens too. Files are queued and opened one per frame through
  `request_open_path`, the same path as File > Open and a window drop; the
  queue waits while a dialog is open.
- Every QA build declares the same type, so with several QA apps installed
  Finder picks one of them. Finder > Get Info > Open with > Change All…
  chooses which; `lsregister -f "<app>"` re-registers a moved app.

## Windows (to do, with the installer)

The app already opens a file named as its first argument
(`open_documents::from_args`), which is how Windows passes it. What's
missing is registration, which a zip can't do; an installer (MSI via WiX,
or Inno Setup) writes it:

1. `HKCU\Software\Classes\.opscene` (default) = `Operandi.Scene`
   (HKLM for an all-users install).
2. `HKCU\Software\Classes\Operandi.Scene` (default) = `Operandi Scene`,
   `DefaultIcon` = path to a `.ico`, `shell\open\command` =
   `"C:\…\operandi-app.exe" "%1"`.
3. Optionally `Applications\operandi-app.exe\SupportedTypes` and
   `OpenWithProgids` so it lists under Open With.
4. Call `SHChangeNotify(SHCNE_ASSOCCHANGED, …)` after installing (the
   installer tools do this) so Explorer refreshes icons.
5. A `.ico` document icon (16–256 px) made from the same PNG.

A second launch while Operandi runs starts a second copy. To open the file
in the running app instead, the app needs a single-instance check (a named
mutex or pipe) that forwards the path to the first copy. That is the
Windows/Linux counterpart of the Apple Event and needs its own small piece
of work.

## Linux (to do)

Also argument-based. The AppImage's `operandi.desktop` needs
`Exec=operandi %f` and `MimeType=application/x-operandi-scene;`, plus a
shared-mime-info XML (`<mime-type type="application/x-operandi-scene">`
with `<glob pattern="*.opscene"/>` and an icon). An AppImage doesn't
install these itself: either ship them for AppImageLauncher/appimaged to
pick up, or add a "Register file type" step that writes them under
`~/.local/share/{applications,mime/packages,icons}` and runs
`update-mime-database` and `update-desktop-database`. Flatpak or distro
packages would install them normally. The single-instance note above
applies here too.
