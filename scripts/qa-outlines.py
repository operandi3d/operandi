#!/usr/bin/env python3
"""Original GPU regression: continuous single face outlines and surface occlusion."""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--binary', required=True)
p.add_argument('--scene', required=True)
args = p.parse_args()
root = Path(__file__).resolve().parent.parent
out = root / 'development/milestone-1.1/outline-fix'
out.mkdir(parents=True, exist_ok=True)
fixture = {'format': 'Operandi', 'version': 1, 'scene': {'items': [], 'next_id': 3,
    'selection': {'item': 1, 'mode': 'Polygon', 'vertices': [], 'edges': [], 'faces': [0]},
    'workplane': {'fixed': True, 'plane': 0, 'center': [0, 0, 0]}}}
for ident, (size, z) in enumerate([(1, 0), (2, .5)], 1):
    fixture['scene']['items'].append({'id': ident, 'name': f'Outline QA {ident}', 'visible': True,
        'transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0], 'scale': [1, 1, 1]},
        'mesh': {'vertices': [[-size, -size, z], [size, -size, z], [size, size, z], [-size, size, z]],
            'faces': [{'vertices': [0, 1, 2, 3], 'uv': None, 'material': '', 'smooth': False}]}})
(out / 'occluded.opscene').write_text(json.dumps(fixture))
fixture['scene']['items'][1]['mesh']['vertices'][0][0] = 0
fixture['scene']['items'][1]['mesh']['vertices'][3][0] = 0
(out / 'partial.opscene').write_text(json.dumps(fixture))
profiles = [(f'{view.lower()}-{scale}x', 1440*scale, 900*scale, scale, view, args.scene, False)
            for view in ['Perspective', 'Top'] for scale in [1, 2]]
profiles += [(name, 1440, 900, 1, 'Front', str(out / (name + '.opscene')), True)
             for name in ['occluded', 'partial']]
results = []
for name, w, h, scale, view, scene, keep in profiles:
    env = os.environ | {'OPERANDI_QA_OFFSCREEN': '1', 'OPERANDI_QA_VIEW': view,
        'OPERANDI_QA_SIZE': f'{w}x{h}', 'OPERANDI_QA_SCALE': str(scale),
        'OPERANDI_QA_CAPTURE': str(out / name), 'OPERANDI_DEV_DATA': str(out / 'qa-data'),
        'OPERANDI_QA_TOOL': 'None'}
    if keep:
        env['OPERANDI_QA_KEEP_SELECTION'] = '1'
    proc = subprocess.run([args.binary, scene], env=env, capture_output=True, text=True, timeout=40)
    (out / (name + '.log')).write_text(proc.stdout + proc.stderr)
    assert proc.returncode == 0, proc.stderr
    record = json.loads((out / (name + '.json')).read_text())
    header = (out / (name + '.ppm')).read_bytes().split(b'\n', 3)
    assert header[:3] == [b'P6', f'{w} {h}'.encode(), b'255']
    pixels = header[3]
    checks = []
    for index, (a, b) in enumerate(record['selected_edges']):
        count = max(20, int(math.dist(a, b)*scale))
        matches = []
        for i in range(count):
            t = .05 + .9*(i+.5)/count
            if name == 'partial' and index in [0, 2] and abs(t-.5) < .05:
                continue  # Exclude the occluder's own edge/hit corridor.
            x = round((a[0]+(b[0]-a[0])*t)*scale-.5)
            y = round((a[1]+(b[1]-a[1])*t)*scale-.5)
            red, green, blue = pixels[(y*w+x)*3:(y*w+x)*3+3]
            gold = red > 110 and green > 80 and red > green and green > blue*1.25
            visible = name != 'occluded'
            if name == 'partial':
                visible = index == 3 or (index == 0 and t < .5) or (index == 2 and t > .5)
            matches.append(gold == visible)
        fraction = sum(matches)/len(matches)
        assert fraction > .98, (name, index, fraction)
        checks.append(fraction)
    assert len(checks) == 4
    record['outline_checks'] = checks
    record['oracle'] = 'gold continuity at projected perimeter; covered fragments absent, no through-selection'
    results.append(record)
    subprocess.run(['sips', '-s', 'format', 'png', str(out / (name + '.ppm')),
                    '--out', str(out / (name + '.png'))], check=True, capture_output=True)
    print(name, 'Pass', checks, 'samples', record['msaa_samples'], flush=True)
(out / 'pixel-checks.json').write_text(json.dumps(results, indent=2))
