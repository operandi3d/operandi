#!/usr/bin/env python3
"""Pick and download CC0 materials for the Operandi Library pack.

    python3 -I scripts/fetch-library-materials.py [--per-category N]
        [--downloads DIR] [--starter DIR] [--dry-run]

Reads the public catalogues of Poly Haven (api.polyhaven.com) and ambientCG
(ambientcg.com/api/v2), sorts each site's materials into the pack's
categories from the site's own category labels, picks the most downloaded
ones per category (no near-duplicates: one per name stem or variant
series), and downloads their 1K JPG maps, one request at a time with a
short pause. Textures only: no site renders or previews are fetched.

DIR defaults to ~/Developer/Operandi-Content/Downloads (never the repo).
Each material lands in DIR/<site>/<id>/; DIR/sources.tsv lists every
download with its category, page URL, licence page and the date it was
fetched, and scripts/build-starter-pack.sh --picks reads that file.
Materials without a colour map plus at least one other map are skipped
(and listed in DIR/skipped.tsv). Materials already in the starter folder
(and their stems) are left out.
"""

import argparse
import datetime
import json
import re
import sys
import time
import urllib.request
import zipfile
from pathlib import Path

AGENT = "Operandi-library-builder (+https://github.com/manfriday/operandi)"
PAUSE = 1.5  # seconds between requests
PH_LICENCE = "https://polyhaven.com/license"
ACG_LICENCE = "https://docs.ambientcg.com/license/"

CATEGORIES = ["Metal", "Wood", "Stone", "Brick", "Tiles", "Concrete", "Fabric",
              "Leather", "Ground", "Plastic", "Paint", "Marble", "Roofing",
              "Paving", "Organic", "Misc"]

# ambientCG "displayCategory" -> pack category. Anything else (signs,
# decals, ornaments, smudges, cards) is left out.
ACG_CATEGORY = {
    "Metal": "Metal", "Metal Plates": "Metal", "Diamond Plate": "Metal",
    "Corrugated Steel": "Metal", "Metal Walkway": "Metal", "Rust": "Metal",
    "Sheet Metal": "Metal", "Chainmail": "Metal", "Foil": "Metal", "Grate": "Metal",
    "Wood": "Wood", "Wood Floor": "Wood", "Planks": "Wood", "Wood Siding": "Wood",
    "Chipboard": "Wood", "Bamboo": "Wood",
    "Rock": "Stone", "Rocks": "Stone", "Granite": "Stone", "Travertine": "Stone",
    "Onyx": "Stone",
    "Bricks": "Brick", "Painted Bricks": "Brick",
    "Tiles": "Tiles", "Glazed Terracotta": "Tiles", "Porcelain": "Tiles",
    "Concrete": "Concrete", "Plaster": "Concrete", "Facade": "Concrete",
    "Fabric": "Fabric", "Carpet": "Fabric", "Rope": "Fabric", "Net": "Fabric",
    "Leather": "Leather",
    "Ground": "Ground", "Gravel": "Ground", "Snow": "Ground", "Grass": "Ground",
    "Clay": "Ground",
    "Plastic": "Plastic", "Rubber": "Plastic", "Styrofoam": "Plastic",
    "Foam": "Plastic", "Acoustic Foam": "Plastic", "Sponge": "Plastic",
    "Painted Plaster": "Paint", "Painted Metal": "Paint", "Painted Wood": "Paint",
    "Paint": "Paint",
    "Marble": "Marble", "Terrazzo": "Marble",
    "Roofing Tiles": "Roofing", "Thatched Roof": "Roofing",
    "Paving Stones": "Paving", "Asphalt": "Paving", "Road": "Paving",
    "Tactile Paving": "Paving", "Pathway": "Paving",
    "Bark": "Organic", "Scattered Leaves": "Organic", "Tree End": "Organic",
    "Wood Chips": "Organic", "Moss": "Organic", "Wicker": "Organic",
    "Cork": "Organic", "Leaf": "Organic",
    "Lava": "Misc", "Ice": "Misc", "Paper": "Misc", "Cardboard": "Misc",
    "Wallpaper": "Misc", "Office Ceiling": "Misc", "Solar Panel": "Misc",
    "Ivory": "Misc", "Tatami": "Misc", "Fence": "Misc",
}


def ph_category(asset):
    """Poly Haven's category path (and roofing label) -> pack category."""
    cats = asset.get("categories", [])
    if "aerial" in cats or any(c.startswith("collection: moon") for c in cats):
        return None
    if "roofing" in cats:
        return "Roofing"
    path = asset.get("category") or ""
    rules = [
        ("Ground & Terrain/Regolith", None),
        ("Wood/Bark", "Organic"),
        ("Wood/Boards & Planks/Painted", "Paint"),
        ("Wood/", "Wood"),
        ("Metal/", "Metal"),
        ("Stone/Rock & Cliffs/Marble", "Marble"),
        ("Stone/Slabs & Tiles/Marble", "Marble"),
        ("Stone/Cobblestone & Paving", "Paving"),
        ("Stone/Slabs & Tiles/Flagstones", "Paving"),
        ("Stone/", "Stone"),
        ("Brick & Block/Concrete Block & Paver/Interlocking Pavers", "Paving"),
        ("Brick & Block/Concrete Block & Paver/Herringbone Pavers", "Paving"),
        ("Brick & Block/", "Brick"),
        ("Ceramic/", "Tiles"),
        ("Concrete/Plaster & Stucco/Painted", "Paint"),
        ("Concrete/Cast Walls & Floors/Painted", "Paint"),
        ("Concrete/", "Concrete"),
        ("Ground & Terrain/Forest Floor/Leaf Litter", "Organic"),
        ("Ground & Terrain/Forest Floor/Mulch", "Organic"),
        ("Ground & Terrain/", "Ground"),
        ("Asphalt & Bitumen/", "Paving"),
        ("Textiles & Leather/Leather", "Leather"),
        ("Textiles & Leather/", "Fabric"),
        ("Plastic", "Plastic"),
        ("Rubber", "Plastic"),
    ]
    for prefix, cat in rules:
        if path.startswith(prefix):
            return cat
    if "leather" in cats:
        return "Leather"
    if "fabric" in cats:
        return "Fabric"
    return None


def get(url, timeout=120):
    req = urllib.request.Request(url, headers={"User-Agent": AGENT})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        data = r.read()
    time.sleep(PAUSE)
    return data


def catalogues(downloads):
    acg_path = downloads / "acg_catalog.json"
    ph_path = downloads / "ph_catalog.json"
    if not acg_path.exists():
        found, offset = [], 0
        while True:
            page = json.loads(get(
                "https://ambientcg.com/api/v2/full_json?type=Material&limit=250"
                f"&offset={offset}&sort=Popular&include=tagData,technicalData"))
            assets = page["foundAssets"]
            found += assets
            offset += len(assets)
            if not assets or offset >= page["numberOfResults"]:
                break
        acg_path.write_text(json.dumps(found))
    if not ph_path.exists():
        ph_path.write_bytes(get("https://api.polyhaven.com/assets?t=textures"))
    return json.loads(acg_path.read_text()), json.loads(ph_path.read_text())


def acg_series(asset_id):
    """("Metal", 49) for Metal049A: variants and neighbours share a series."""
    m = re.match(r"([A-Za-z]+)(\d+)[A-Z]?$", asset_id)
    return (m.group(1), int(m.group(2))) if m else (asset_id, -10)


def ph_stem(name):
    return re.sub(r"(_\d+)+$", "", name)


def existing(starter):
    names = set()
    if starter.is_dir():
        for preset in starter.rglob("*.oppreset"):
            names.add(preset.stem)
    return names


def candidates(acg, ph, have, today):
    """Per category: [(site, id, url, downloads)] most downloaded first."""
    out = {c: [] for c in CATEGORIES}
    for a in acg:
        cat = ACG_CATEGORY.get(a.get("displayCategory") or "")
        if not cat or a["assetId"] in have:
            continue
        if (a.get("releaseDate") or "9999")[:10] > today:
            continue  # still early access
        maps = set(a.get("maps") or [])
        if "color" not in maps or len(maps) < 2:
            continue
        out[cat].append(("ambientCG", a["assetId"], a["shortLink"],
                         a.get("downloadCount") or 0))
    for name, a in ph.items():
        cat = ph_category(a)
        if not cat or name in have:
            continue
        out[cat].append(("PolyHaven", name, f"https://polyhaven.com/a/{name}",
                         a.get("download_count") or 0))
    for cat in out:
        out[cat].sort(key=lambda c: -c[3])
    return out


class Picker:
    """Keeps one material per Poly Haven stem and per ambientCG series."""

    def __init__(self, have):
        self.ph_stems = {ph_stem(n) for n in have}
        self.acg = [acg_series(n) for n in have]

    def near_duplicate(self, site, asset_id, cat):
        if site == "PolyHaven":
            return ph_stem(asset_id) in self.ph_stems
        word, num = acg_series(asset_id)
        if cat == "Misc" and sum(1 for w, _ in self.acg if w == word) >= 2:
            return True  # Misc is for variety: two papers, two fences at most
        return any(w == word and abs(n - num) <= 1 for w, n in self.acg)

    def take(self, site, asset_id):
        if site == "PolyHaven":
            self.ph_stems.add(ph_stem(asset_id))
        else:
            self.acg.append(acg_series(asset_id))


def safe_extract(zip_path, dest):
    with zipfile.ZipFile(zip_path) as z:
        for info in z.infolist():
            name = Path(info.filename)
            if name.is_absolute() or ".." in name.parts or info.is_dir():
                continue
            if len(name.parts) != 1:
                continue  # ambientCG zips are flat; ignore anything nested
            (dest / name.name).write_bytes(z.read(info))


def download_acg(asset_id, dest):
    dest.mkdir(parents=True, exist_ok=True)
    zip_path = dest.parent / f"{asset_id}_1K-JPG.zip"
    zip_path.write_bytes(get(f"https://ambientcg.com/get?file={asset_id}_1K-JPG.zip"))
    try:
        safe_extract(zip_path, dest)
    finally:
        zip_path.unlink(missing_ok=True)
    # Site previews (<id>.png) aren't part of the material.
    for f in dest.glob("*.png"):
        if "_1K" not in f.name:
            f.unlink()
    files = [f.name for f in dest.iterdir()]
    if not any("_Color." in f for f in files):
        return "no colour map"
    maps = [f for f in files if f.endswith(".jpg") and "_Color." not in f]
    return None if maps else "colour map only"


# Poly Haven file keys that aren't texture maps, or duplicate one.
PH_SKIP = {"blend", "gltf", "mtlx", "nor_dx", "usd", "fbx"}


def download_ph(name, dest):
    files = json.loads(get(f"https://api.polyhaven.com/files/{name}"))
    if "Diffuse" not in files:
        return "no colour map"
    others = [k for k in files if k not in PH_SKIP and k != "Diffuse"]
    if not others:
        return "colour map only"
    dest.mkdir(parents=True, exist_ok=True)
    got_colour = False
    for key in ["Diffuse"] + others:
        res = files[key].get("1k") if isinstance(files[key], dict) else None
        if not res:
            continue
        entry = res.get("jpg") or res.get("png")
        if not entry or "url" not in entry:
            continue
        url = entry["url"]
        if not url.startswith("https://dl.polyhaven.org/"):
            continue
        (dest / url.rsplit("/", 1)[1]).write_bytes(get(url))
        got_colour = got_colour or key == "Diffuse"
    if not got_colour:  # named _diff, _diffuse, _albedo... by asset
        return "no 1K colour map"
    return None


def main():
    home = Path.home() / "Developer" / "Operandi-Content"
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--per-category", type=int, default=14)
    ap.add_argument("--downloads", type=Path, default=home / "Downloads")
    ap.add_argument("--starter", type=Path, default=home / "Starter")
    ap.add_argument("--dry-run", action="store_true", help="list picks only")
    args = ap.parse_args()

    downloads = args.downloads.expanduser()
    downloads.mkdir(parents=True, exist_ok=True)
    today = datetime.date.today().isoformat()
    acg, ph = catalogues(downloads)
    have = existing(args.starter.expanduser())
    pool = candidates(acg, ph, have, today)

    sources = downloads / "sources.tsv"
    done = {}
    if sources.exists():
        for line in sources.read_text().splitlines()[1:]:
            cols = line.split("\t")
            done[(cols[1], cols[2])] = cols
    skipped_path = downloads / "skipped.tsv"
    skipped = {}
    if skipped_path.exists():
        for line in skipped_path.read_text().splitlines():
            site, asset_id, reason = line.split("\t")
            skipped[(site, asset_id)] = reason

    picker = Picker(have)
    rows = list(done.values())
    for cat in CATEGORIES:
        count = sum(1 for r in done.values() if r[0] == cat)
        for r in done.values():
            if r[0] == cat:
                picker.take(r[1], r[2])
        # Alternate the two sites so each category mixes both.
        by_site = {s: [c for c in pool[cat] if c[0] == s] for s in ("PolyHaven", "ambientCG")}
        order = []
        while by_site["PolyHaven"] or by_site["ambientCG"]:
            for s in ("PolyHaven", "ambientCG"):
                if by_site[s]:
                    order.append(by_site[s].pop(0))
        for site, asset_id, url, _ in order:
            if count >= args.per_category:
                break
            key = (site, asset_id)
            if key in done or key in skipped or picker.near_duplicate(site, asset_id, cat):
                continue
            if args.dry_run:
                print(f"{cat}\t{site}\t{asset_id}")
                picker.take(site, asset_id)
                count += 1
                continue
            dest = downloads / site / asset_id
            try:
                reason = (download_acg if site == "ambientCG" else download_ph)(asset_id, dest)
            except Exception as e:  # network or archive trouble: skip it
                reason = f"download failed: {e}"
            if reason:
                skipped[key] = reason
                with skipped_path.open("a") as f:
                    f.write(f"{site}\t{asset_id}\t{reason}\n")
                print(f"skip {site}/{asset_id}: {reason}", file=sys.stderr)
                continue
            licence = PH_LICENCE if site == "PolyHaven" else ACG_LICENCE
            row = [cat, site, asset_id, url, licence, today, str(dest)]
            rows.append(row)
            done[key] = row
            picker.take(site, asset_id)
            count += 1
            with sources.open("w") as f:
                f.write("category\tsite\tid\turl\tlicence\tfetched\tfolder\n")
                for r in rows:
                    f.write("\t".join(r) + "\n")
            print(f"{cat}: {site}/{asset_id}", flush=True)
        print(f"== {cat}: {count}", flush=True)


if __name__ == "__main__":
    main()
