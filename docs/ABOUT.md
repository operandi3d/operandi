# What's this all about?

Operandi is a free, open-source 3D polygon modeler. It's an homage to Modo,
the late, great modeler whose fast, friendly way of working many of us
learned to love. When Modo's road came to an end, we wanted somewhere for
that way of working to carry on, in an app the community owns.

Operandi is its own program, written from scratch in Rust. It aims to feel
familiar to Modo users. It is not a copy of Modo, and it contains no Modo
code.

## Built clean

Operandi was built **without reverse engineering Modo**. No one working on
it decompiled, disassembled, or dug through Modo's program files to find out
how it works, and no tool built for that (REA or anything like it) was used.

What we did use:

- **Public documentation:** Modo's published user guide and help pages.
- **Observation:** using Modo the way any customer would, and writing down
  what it does, including through its public scripting interface.
- **Plain-text settings:** reading Modo's human-readable configuration and
  preference files for default values.

The work is split in two, the way clean-room projects usually are. One side
studied Modo and wrote **behaviour specs**: plain descriptions of what a
tool does, never how Modo's code does it. The other side wrote Operandi from
those specs alone and never looked at Modo at all.

**LXO files:** Operandi can open and save `.lxo` scene files so people can
bring their own work across. That support was written from the file
format's public documentation and from files we made ourselves, for
compatibility only.

Every contribution follows the same core rule: nothing from reverse
engineering Modo, and nothing copied from it. See
[CONTRIBUTING.md](../CONTRIBUTING.md).

## The fine print

Operandi is not affiliated with, endorsed by, or connected to Foundry or
Modo. Modo is a trademark of The Foundry Visionmongers Ltd. We use the name
only to say what inspired Operandi and which files it can open.

Operandi is free software under the GNU General Public License v3 or later
([LICENSE](../LICENSE)). You can use it for anything, including paid work,
and share it freely. If you build on its code, your version has to stay
free and open too.

Rendering is by [Cycles](https://www.cycles-renderer.org), from the Blender
project. Full credits are in [DEPENDENCIES.md](DEPENDENCIES.md).

## Ideas we were given

- Twist Bevel: idea inspired by the "Twist Bevel" tool for Modo by FUSION.
  Profile Bias, Segment Spacing and Center Bias: ideas from Modest, a
  Blender add-on by Christian.
