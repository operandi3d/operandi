"""Operandi scripting.

Every call goes through the same commands as the command line, menus and
the MCP bridge, so whatever a script does a person could type.

    import operandi
    operandi.run("operandi.time.set frame:12")
    with operandi.undo_group("Bounce"):
        operandi.run("transform.channel name:pos.Y value:2")
        operandi.run("operandi.key.set channels:1:pos.Y")
    print(operandi.query("operandi.anim.query"))
"""

import contextlib as _contextlib
import json as _json

import _operandi


class CommandError(Exception):
    """A command refused or failed; the message is the command's own."""


def _line(command, args):
    if not args:
        return command
    parts = [command]
    for key, value in args.items():
        if isinstance(value, bool):
            value = "true" if value else "false"
        elif isinstance(value, (list, dict)):
            value = _json.dumps(value, separators=(",", ":"))
        value = str(value)
        if any(c.isspace() for c in value) or value == "":
            value = "{" + value + "}"
        parts.append(f"{key}:{value}")
    return " ".join(parts)


def run(command, check=True, **args):
    """Runs a command line, e.g. run("operandi.time.set frame:12") or
    run("operandi.time.set", frame=12).

    Returns True, or the command's answer text for a line that answers.
    A command that refuses or fails raises CommandError with its own
    message; with check=False it returns False instead.
    """
    ok, text = _operandi.call("run", _line(command, args))
    if not ok:
        if check:
            raise CommandError(text)
        return False
    return True if text is None else text


def query(command, **args):
    """Runs a query (it changes nothing) and returns its answer: a Python
    value for a JSON answer, else the text. Raises CommandError when the
    line isn't a query or the query fails."""
    ok, text = _operandi.call("query", _line(command, args))
    if not ok:
        raise CommandError(text)
    try:
        return _json.loads(text)
    except (TypeError, ValueError):
        return text


@_contextlib.contextmanager
def undo_group(name="Script"):
    """Every undo step the block makes becomes one, named name. Groups
    nest: only the outermost one makes the step."""
    ok, text = _operandi.call("group_begin", name)
    if not ok:
        raise CommandError(text)
    try:
        yield
    finally:
        _operandi.call("group_end", "")


def mesh_get(item=None, flat=False):
    """A mesh item's points and polygons (item space, metres) as
    {"item", "points", "polygons"}. points is a list of (x, y, z), or with
    flat=True one memoryview of doubles x0, y0, z0, x1, ..."""
    ok, found, points, polygons = _operandi.mesh_get(item, flat)
    if not ok:
        raise CommandError(found)
    if flat:
        points = memoryview(points).cast("d")
    return {"item": found, "points": points, "polygons": polygons}


def mesh_set(points, polygons=None, item=None):
    """Sets a mesh item's points, and its polygons when given, as one undo
    step. points: a list of (x, y, z), or any buffer of doubles (array("d"),
    a memoryview) holding x0, y0, z0, x1, ... polygons: lists of point
    indices. Points alone must keep the point count."""
    ok, text = _operandi.mesh_set(points, polygons, item)
    if not ok:
        raise CommandError(text)


def set_time(frame):
    """Goes to a frame (not an undo step)."""
    run("operandi.time.set", frame=frame)


def set_channel(item, name, value):
    """Sets an item channel such as pos.Y, rot.X or scl.Z (one undo step;
    Auto Key keys it when on)."""
    run("transform.channel", name=name, value=value, item=item)


def set_key(item, *names):
    """Keys the named channels of item at the current time."""
    run("operandi.key.set", channels=",".join(f"{item}:{n}" for n in names))
