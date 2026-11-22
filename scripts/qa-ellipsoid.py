#!/usr/bin/env python3
"""Original Ellipsoid numeric egui/Metal captures; simulated sizes/scales only."""
import argparse
import json
import os
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', required=True)
p.add_argument('--output', type=Path)
args = p.parse_args()
root = Path(__file__).resolve().parent.parent
out = args.output or root / 'development/ellipsoid-numeric/renders'
out.mkdir(parents=True, exist_ok=True)
vertices = [[-4 + x * .5, -4 + y * .5, -4 + z * .5] for x, y, z in [
    (-1,-1,-1),(1,-1,-1),(1,1,-1),(-1,1,-1),
    (-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1)]]
faces = [{'vertices': v, 'uv': None, 'material': 'Original', 'smooth': False}
         for v in [[0,3,2,1],[4,5,6,7],[0,1,5,4],[1,2,6,5],[2,3,7,6],[3,0,4,7]]]
scene = {'format': 'Operandi', 'version': 4, 'scene': {
    'items': [{'id': 1, 'name': 'Original Ellipsoid QA source', 'visible': True,
               'transform': {'translation': [0,0,0], 'rotation': [0,0,0], 'scale': [1,1,1]},
               'mesh': {'vertices': vertices, 'faces': faces}}],
    'next_id': 2, 'selection': {'item': 1, 'mode': 'Item', 'vertices': [], 'edges': [], 'faces': []},
    'workplane': {'fixed': True, 'plane': 0, 'center': [4,5,6]}}}
fixture = out / 'original-source.opscene'
fixture.write_text(json.dumps(scene))
profiles = [('bulged',1440,900,1,False), ('bulged',1100,720,1,False),
            ('bulged',1100,720,1,True), ('bulged',1920,1080,1,False),
            ('bulged',2560,1440,1.5,False), ('bulged',3840,2160,2,False),
            ('bulged',5120,2880,2,False)]
profiles += [(name,1440,900,1,False) for name in ['zero','extreme','subdiv','catmull','invalid']]
records = []
for profile,w,h,scale,scroll in profiles:
    name = f'{profile}-{w}x{h}-{scale}x' + ('-scroll' if scroll else '')
    prefix = out / name
    env = os.environ | {'OPERANDI_QA_OFFSCREEN': '1', 'OPERANDI_QA_TOOL': 'None',
        'OPERANDI_QA_ELLIPSOID': profile, 'OPERANDI_QA_SIZE': f'{w}x{h}',
        'OPERANDI_QA_SCALE': str(scale), 'OPERANDI_QA_CAPTURE': str(prefix),
        'OPERANDI_QA_KEEP_SELECTION': '1', 'OPERANDI_QA_VIEW': 'Perspective',
        'OPERANDI_DEV_DATA': str(out / (name + '-data'))}
    if scroll:
        env['OPERANDI_QA_SCROLL'] = '1'
    proc = subprocess.run([args.binary, str(fixture)], env=env, capture_output=True,
                          text=True, timeout=60)
    prefix.with_suffix('.log').write_text(proc.stdout + proc.stderr)
    if proc.returncode:
        raise SystemExit(f'{name} failed: {proc.stderr}')
    record = json.loads(Path(str(prefix) + '.json').read_text())
    assert record['capture'] == [w,h] and abs(record['scale']-scale) < .001
    ell = record['ellipsoid']
    assert ell and ell['preview_counts'] == ([26,56,32] if profile == 'extreme' else [38,81,45])
    assert bool(ell['error']) == (profile == 'invalid')
    assert record['mesh_counts'] == [8,12,6] and not record['gesture_captured']
    assert 'Ellipsoid' in record['toolbox']
    record['profile'] = name
    records.append(record)
    subprocess.run(['sips','-s','format','png',str(prefix)+'.ppm','--out',str(prefix)+'.png'],
                   check=True, capture_output=True)
    Path(str(prefix)+'.ppm').unlink()
    print(name + ' PASS', flush=True)
(out / 'matrix.json').write_text(json.dumps(records, indent=2))
