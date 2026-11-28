<!-- help-ids: operandi.kit.list, operandi.kit.install, operandi.kit.enable, operandi.kit.disable, operandi.kit.reload, operandi.kit.openFolder ; template: concept ; since: 0.1.0 -->
# Kits and extensions

A kit is an add-on that extends Operandi. In this version a kit can add
file types to **File > Import**, read by a small program that comes with
the kit. This page shows how to install and turn on a kit, what else you
can extend today, and what isn't there yet.

## Install a kit

Kits come as a `.zip` file, one per kind of computer (Mac, Windows,
Linux). There is no Kits window yet, so you install and turn kits on by
typing commands in the command line.

1. Type `operandi.kit.install path:{/Users/me/Downloads/step_import-1.0.0-macos-aarch64.zip}`.
   Put your own file's path between the braces. The kit is copied into the
   **Kits** folder and left off.
2. Type `operandi.kit.list` to check it's there. The status line lists each
   kit with its version, whether it's on, and the file types it adds.
3. Type `operandi.kit.enable id:step_import`, using the kit's id from the
   list. The status line says which file types **File > Import** now reads.

A kit runs a program of its own on your computer when you import with it.
Only turn on kits from people you trust. Installing a kit never turns it
on; Operandi remembers which kits you turned on for next time.

- `operandi.kit.disable id:<id>` turns a kit off. Its file types leave
  **File > Import**; nothing else changes.
- `operandi.kit.openFolder` shows the **Kits** folder, inside Operandi's
  data folder. To remove a kit, quit Operandi and delete its folder there.
- `operandi.kit.reload` reads the folder again after you change a kit by
  hand.
- To update a kit, install the new zip with `replace:true` added.

Importing with a kit works like any other import, from **File > Import**
or by dropping the file on the window. Operandi waits while the kit reads
the file. If the kit fails, the import stops with the kit's own error
message and your scene is left as it was. A kit that takes longer than its
time limit is stopped.

## Put a script on a menu or button

Every menu and toolbox tab is a form, and a form's buttons run command
lines. Add a button whose line runs your script, and it works like a
built-in command.

This line adds a button to the **System** menu that runs a script file:

```python
import operandi

operandi.run("operandi.form.add",
             line="operandi.python.run file:{/Users/me/Scripts/wave.py}",
             form="menu.system")
```

You can type the same thing as a command line:
`operandi.form.add {operandi.python.run file:{/Users/me/Scripts/wave.py}} form:menu.system`.

- Open **System > Form Editor** to give the button a label, an icon, or a
  different place. You can also make a form of your own there, for a
  toolbox tab full of your scripts.
- Your changes are saved as your own copy of the form in the **Forms**
  folder, inside **Layouts** in Operandi's data folder. The shipped form
  stays as it was.
- `operandi.form.revert menu.system` puts the menu back the way it
  shipped and drops your copy.
- A script can't have a key of its own yet: the **Input Map** binds
  Operandi's own actions, not command lines.

## Scenes made with kits

A scene can hold items of a kind that comes from a kit, such as a point
cloud. This version has no kits installed, so it treats such items with
care instead of refusing the file:

- the scene opens, and the item shows in the **Items** list marked as
  needing its kit;
- the viewport draws it as a dashed box where it sits;
- it is saved back exactly as it came, so a later build with the kit gets
  it back whole;
- a script can read its values with
  `item.channel name:<value> value:? item:<id>`, but not change them.

Mesh operations of an unknown kind are kept the same way.

## Make a kit

[docs/KITS.md](https://github.com/operandi3d/operandi/blob/main/docs/KITS.md)
explains how to make a kit, and `examples/kits/off_import` in the source
is a small one to copy.

## For developers building Operandi

If you build Operandi from source, two entry points already take additions
from outside Operandi's own tables. Both are Rust, compiled into your own
build; nothing is loaded while Operandi runs.

| Entry point | Adds | Rules it checks |
|---|---|---|
| `Registry::register` (`crates/operandi-core/src/commands/command_runtime.rs`) | Commands under a namespace of your own, such as `mykit.cable.make` | Every name starts with the namespace; no namespace or name of Operandi's own; something to run it |
| `meshop_types::register` (`crates/operandi-core/src/meshops/meshop_types.rs`) | A mesh operation type, with its form and default values | A dotted id such as `mykit.cable.item`; every value has a form field; the defaults read back |

Read [CONTRIBUTING.md](https://github.com/operandi3d/operandi/blob/main/CONTRIBUTING.md)
before you start; it covers licences and how changes get in.

## What isn't there yet

This version has none of the following:

- a **Kits** window or menu (kits are managed with the commands above);
- a progress bar or **Cancel** while a kit imports;
- kits that export, add commands, panels or mesh operations, or draw and
  edit kit items;
- commands, panels or mesh operations written in Python;
- compiled modules loaded while Operandi runs.

Until then, Python scripts and forms are the other way to extend Operandi
without rebuilding it.

## Related

- [Python scripting](python.md)
- [Python reference](python-reference.md)
