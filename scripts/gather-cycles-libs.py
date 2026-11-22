#!/usr/bin/env python3
"""Copy the shared libraries liboperandi_cycles loads (and what they load in
turn) from Blender's precompiled library pack into one folder next to it.

Linux and Windows only; build-cycles.sh does the same for macOS with otool.
Libraries that aren't in the pack are system libraries (glibc, KERNEL32,
the MSVC runtime...) and are left alone.

    gather-cycles-libs.py <bridge library> <library pack folder> <destination folder>

On Linux, each copied library gets RUNPATH $ORIGIN (with patchelf, if it's
installed) so it finds its neighbours in the same folder.
Standard library only, so it runs on a fresh build machine.
"""
import glob
import os
import shutil
import struct
import subprocess
import sys


def elf_needed(path):
    """DT_NEEDED names of an ELF file, read with binutils' readelf."""
    out = subprocess.run(
        ["readelf", "-d", path], check=True, capture_output=True, text=True
    ).stdout
    return [
        line.split("[", 1)[1].split("]", 1)[0]
        for line in out.splitlines()
        if "(NEEDED)" in line
    ]


def pe_imports(path):
    """DLL names a PE file imports, including delay-loaded ones."""
    with open(path, "rb") as f:
        d = f.read()
    pe = struct.unpack_from("<I", d, 0x3C)[0]
    if d[pe : pe + 4] != b"PE\0\0":
        raise ValueError(f"{path} is not a PE file")
    sections = struct.unpack_from("<H", d, pe + 6)[0]
    opt_size = struct.unpack_from("<H", d, pe + 20)[0]
    opt = pe + 24
    pe32_plus = struct.unpack_from("<H", d, opt)[0] == 0x20B
    image_base = (
        struct.unpack_from("<Q", d, opt + 24)[0]
        if pe32_plus
        else struct.unpack_from("<I", d, opt + 28)[0]
    )
    data_dirs = opt + (112 if pe32_plus else 96)
    table = []
    for i in range(sections):
        vsize, va, raw_size, raw_ptr = struct.unpack_from("<IIII", d, opt + opt_size + 40 * i + 8)
        table.append((va, vsize or raw_size, raw_ptr))

    def offset(rva):
        for va, size, raw in table:
            if va <= rva < va + size:
                return rva - va + raw
        raise ValueError(f"RVA {rva:#x} outside every section of {path}")

    def cstr(rva):
        o = offset(rva)
        return d[o : d.index(b"\0", o)].decode("ascii")

    names = []
    import_rva = struct.unpack_from("<I", d, data_dirs + 8 * 1)[0]
    if import_rva:
        o = offset(import_rva)
        while d[o : o + 20] != b"\0" * 20:
            names.append(cstr(struct.unpack_from("<I", d, o + 12)[0]))
            o += 20
    delay_rva = struct.unpack_from("<I", d, data_dirs + 8 * 13)[0]
    if delay_rva:
        o = offset(delay_rva)
        while True:
            attributes, name = struct.unpack_from("<II", d, o)
            if name == 0:
                break
            # Old-style descriptors store addresses, not RVAs.
            names.append(cstr(name if attributes & 1 else name - image_base))
            o += 32
    return names


def main():
    bridge, pack, dest = sys.argv[1:4]
    windows = bridge.lower().endswith(".dll")
    deps_of = pe_imports if windows else elf_needed
    key = (lambda n: n.lower()) if windows else (lambda n: n)

    # Every file in the pack by name (symlinks too: Linux packs name
    # libraries by their soname links).
    in_pack = {}
    for root, _dirs, files in os.walk(pack):
        for name in files:
            in_pack.setdefault(key(name), os.path.join(root, name))

    os.makedirs(dest, exist_ok=True)
    copied, system = [], set()
    pending = [bridge]
    # OpenImageDenoise loads its device back ends at run time, so nothing
    # imports them; take the CPU one along.
    for pattern in ("*OpenImageDenoise_device_cpu*.dll", "libOpenImageDenoise_device_cpu.so*"):
        for found in glob.glob(os.path.join(pack, "**", pattern), recursive=True):
            target = os.path.join(dest, os.path.basename(found))
            if not os.path.exists(target):
                shutil.copy2(found, target)
                copied.append(target)
                pending.append(target)
    while pending:
        lib = pending.pop()
        for dep in deps_of(lib):
            target = os.path.join(dest, dep)
            if os.path.exists(target):
                continue
            source = in_pack.get(key(dep))
            if source is None:
                system.add(dep)
                continue
            shutil.copy2(source, target)  # follows symlinks: a real file
            copied.append(target)
            pending.append(target)

    if not windows and copied:
        if shutil.which("patchelf"):
            for lib in copied:
                os.chmod(lib, 0o755)
                subprocess.run(["patchelf", "--set-rpath", "$ORIGIN", lib], check=True)
        else:
            print("warning: patchelf not found; copied libraries keep their own RUNPATH")
    print(f"Copied {len(copied)} libraries into {dest}")
    print("Left to the system: " + ", ".join(sorted(system, key=str.lower)))


if __name__ == "__main__":
    main()
