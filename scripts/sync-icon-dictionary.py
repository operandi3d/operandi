#!/usr/bin/env python3
"""Copy only integrated, user-admitted SVGs; never redraw or normalize artwork."""
import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / 'specifications/icon-dictionary/handoff/icon-dictionary.html'
DEST = ROOT / 'crates/operandi-app/assets/icons'
# UI role -> dictionary lookup name. Drawing reuse does not equate commands.
ROLES = {
    'Cube / scene mesh': ('Box', 'cube.svg'),
    'Sphere': ('prim.sphere', 'sphere.svg'),
    'Cylinder': ('prim.cylinder', 'cylinder.svg'),
    'Cone': ('prim.cone', 'cone.svg'),
    'Ellipsoid': ('prim.ellipsoid', 'ellipsoid.svg'),
    'Bridge': ('edge.bridge', 'bridge.svg'),
    'Spike': ('poly.spikey', 'spike.svg'),
    'Transform': ('Transform', 'transform.svg'),
    'Vertices': ('selType.vertex', 'vertex.svg'),
    'Edges': ('selType.edge', 'edge.svg'),
    'Polygons': ('selType.polygon', 'polygon.svg'),
    'Move': ('TransformMove', 'move.svg'),
    'Rotate': ('TransformRotate', 'rotate.svg'),
    'Scale': ('TransformScale', 'scale.svg'),
    'Bevel': ('poly.bevel', 'bevel.svg'),
    'Chamfer Edit': ('chamfer.edit', 'chamfer-edit.svg'),
    'Poly Haul': ('poly.haul', 'poly-haul.svg'),
    'Sketch Extrude': ('poly.sweep', 'sketch-extrude.svg'),
    'Smooth Shift': ('poly.smshift', 'smooth-shift.svg'),
    'Extrude': ('poly.extrude', 'extrude.svg'),
    'Visibility': ('{uiicon} eyeGrey', 'eye.svg'),
    'Disclosure': ('tab.collapse', 'disclosure.svg'),
    'Action Center': ('actionCenter.state', 'action-center.svg'),
    'Falloff': ('falloff.state', 'falloff.svg'),
    'Symmetry': ('symmetry.state', 'symmetry.svg'),
    'Snapping': ('tool.snapState', 'snapping.svg'),
    'Work Plane': ('workPlane.state', 'workplane.svg'),
    'Add Loop': ('edge.addLoop', 'edge-add-loop.svg'),
    'Add Point': ('edge.addPoint', 'edge-add-point.svg'),
    'Edge Extend': ('edge.extend', 'edge-extend.svg'),
    'Edge Extrude': ('edge.extrude', 'edge-extrude.svg'),
    'Edge Bevel': ('edge.bevel', 'edge-bevel.svg'),
    'Edge Slide': ('edge.slide', 'edge-slide.svg'),
    'Edge Relax': ('edge.relax', 'edge-relax.svg'),
}


# Exact admitted extension records and original artwork; changes require review.
EXTENSIONS = {
    'Polygon Reduction': ('b0eb60f4177424569b33476b4dd756af163f297c4c8fd73a29f12d0c116d18ec', 'specifications/m1-polygon-reduction-basic-2026-10-29/poly.reduct.svg', '44864dc8b9b0556d6f088c10493af26bb489aefec28795acf370d99a0e4cf6d6'),
    'Scale Offset': ('56bf6ad46daf504b0ffd91b30fd16d740af8d3e761de1551ed3a74fc8ce84cae', 'specifications/m1-command-row-script-icon-2026-10-29/script-action.svg', '92697a8b6afabd287109f7c2f9a1793c76bb1804d3af3b950bc379438f352b4b'),
    'Thicken': ('de9fb89bc670e4c108718564456c89dd17e2d44a77a6b325ce75c865dc9b7d1e', 'specifications/m1-thicken-icon-2026-10-28/poly.thicken.svg', 'bb979f749834c4de0aee72ec5773f68454ab07285c748a9067afcb30a25fc3ca'),
    'Triple': ('65c58a0868b0e0983867711bad8e468e2eca29c1751f7da38a93583933c3e21e', 'specifications/m1-polygon-triple-basic-2026-10-28/poly.triple.svg', '180074822c1cb891462fa2a92ceaec98e67eab2fcb85463c3e49c62270b22733'),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = json.loads(re.search(
        r'<script[^>]*id="icon-dictionary-data"[^>]*>(.*?)</script>',
        SOURCE.read_text(), re.S)[1])
    assets = {a['id']: a for a in data['assets']}
    expected = {}
    manifest = {'source': str(SOURCE.relative_to(ROOT)),
                'schema': data['schema'], 'batch': data['batch'],
                'limitations': data['limitations'], 'roles': {},
                'reused_roles': {'Mesh Constraint': {
                    'lookup': 'Box', 'file': 'cube.svg',
                    'note': 'Neutral mesh cube reused for this labeled control; no dedicated Mesh Constraint state drawing exists in this admitted batch. No command equivalence is implied.'}}}
    for role, (name, filename) in ROLES.items():
        asset = assets[data['lookup'][name]]
        expected[filename] = asset['svg']
        manifest['roles'][role] = {'lookup': name, 'file': filename,
                                  **{k: v for k, v in asset.items() if k != 'svg'}}
    # Validate the complete existing manifest before either checking or writing.
    # Never silently drop unknown roles, even in write mode.
    current = json.loads((DEST / 'manifest.json').read_text())
    if set(current['roles']) != set(ROLES) | set(EXTENSIONS):
        raise SystemExit('Unexpected or missing manifest roles')
    for role, (record_hash, source, svg_hash) in EXTENSIONS.items():
        record = current['roles'][role]
        encoded = json.dumps(record, sort_keys=True, separators=(',', ':')).encode()
        if hashlib.sha256(encoded).hexdigest() != record_hash:
            raise SystemExit(f'Extension metadata differs: {role}')
        installed = (DEST / record['file']).read_bytes()
        if not (ROOT / source).exists():
            # The m1-* source folders were removed on 2026-11-05.
            # Skip the source comparison and check
            # the installed copy against the recorded hash only.
            print(f'note: {role} source {source} removed; checked installed copy only')
            if hashlib.sha256(installed).hexdigest() != svg_hash:
                raise SystemExit(f'Extension artwork differs: {role}')
            manifest['roles'][role] = record
            continue
        artwork = (ROOT / source).read_bytes()
        if hashlib.sha256(artwork).hexdigest() != svg_hash or installed != artwork:
            raise SystemExit(f'Extension artwork differs: {role}')
        manifest['roles'][role] = record
    if current != manifest:
        raise SystemExit('Dictionary manifest metadata differs')
    expected['manifest.json'] = json.dumps(current, indent=2) + '\n' 
    if args.check:
        for name, content in expected.items():
            if (DEST / name).read_text() != content:
                raise SystemExit(f'Dictionary resource differs: {name}')
        print(f'{len(ROLES)} shared SVGs match admitted dictionary exactly')
    else:
        DEST.mkdir(parents=True, exist_ok=True)
        for name, content in expected.items():
            (DEST / name).write_text(content)


if __name__ == '__main__':
    main()
