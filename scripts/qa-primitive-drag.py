#!/usr/bin/env python3
"""Original fixtures and full egui/Metal Ctrl-drag captures (synthetic input).

Run after cargo build --locked -p operandi-app --features qa.
"""
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'development/primitive-drag/renders'
OUT.mkdir(parents=True, exist_ok=True)
fixture = OUT.parent / 'Empty_PrimitiveDrag.opscene'
if not fixture.exists():
    scene = {
        'items': [{'id': 1, 'name': 'PrimitiveDrag_QA', 'visible': True,
                   'transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0], 'scale': [1, 1, 1]},
                   'mesh': {'vertices': [], 'faces': []}}],
        'next_id': 2,
        'selection': {'item': 1, 'mode': 'Polygon', 'vertices': [], 'edges': [], 'faces': []},
        'workplane': {'fixed': False, 'plane': 0, 'center': [0, 0, 0]},
    }
    with fixture.open('x') as stream:
        json.dump({'format': 'Operandi', 'version': 2, 'scene': scene}, stream)

for tool in ('Cube', 'Sphere', 'Cylinder'):
    for view, plane, scale, gesture in (
        ('Top', '0', 1, 'Ctrl'),
        ('Front', '2', 1, 'Ctrl'),
        ('Perspective', '1', 1, 'Ctrl'),
        ('Perspective', '0', 2, 'Ctrl'),
        ('Top', '0', 1, 'Ordinary'),
    ):
        name = f'{tool}-{view}-{plane}-{scale}x-{gesture}'
        env = os.environ | {
            'OPERANDI_QA_OFFSCREEN': '1', 'OPERANDI_QA_KEEP_SELECTION': '1',
            'OPERANDI_QA_CAPTURE': str(OUT / name),
            'OPERANDI_QA_SIZE': '1440x932' if scale == 1 else '2880x1864',
            'OPERANDI_QA_SCALE': str(scale), 'OPERANDI_QA_VIEW': view,
            'OPERANDI_QA_FIXED_PLANE': plane, 'OPERANDI_QA_TOOL': tool,
            'OPERANDI_QA_PRIMITIVE_DRAG': gesture,
            'OPERANDI_DEV_DATA': str(OUT.parent / 'qa-data'),
        }
        subprocess.run([str(ROOT / 'target/debug/operandi-app'), str(fixture)],
                       env=env, check=True, timeout=30)
        record = json.loads((OUT / (name + '.json')).read_text())
        p = record['primitive_parameters']
        size = p['size'] if tool == 'Cube' else [v * 2 for v in p['radius']]
        expected = [4., 4., 4.] if gesture == 'Ctrl' else {
            '0': [2., 0., .75], '1': [0., 2., .75], '2': [2., .75, 0.],
        }[plane]
        assert all(abs(a - b) < 1e-5 for a, b in zip(size, expected)), (name, size)
        center = {'0': [0., 0., -.125], '1': [0., 0., -.125], '2': [0., -.125, 0.]}[plane]
        if gesture == 'Ctrl':
            center = {'0': [-1., 0., -.5], '1': [0., -1., -.5], '2': [-1., -.5, 0.]}[plane]
        assert all(abs(a - b) < 1e-5 for a, b in zip(p['position'], center)), (name, p)
        counts = {'Cube': [8, 12, 6], 'Sphere': [266, 552, 288], 'Cylinder': [48, 72, 26]}
        # Ordinary radial drawing is a display-only flat draft, unlike Cube's
        # admitted flat mesh. This follow-up must preserve that distinction.
        flat = [4, 4, 1] if tool == 'Cube' else [0, 0, 0]
        assert record['mesh_counts'] == (counts[tool] if gesture == 'Ctrl' else flat), (name, record['mesh_counts'])
        assert record['live'] == tool and record['history'] == 0
        assert not record['gesture_captured'] and 'PM-Q02' not in record['status']
        subprocess.run(['sips', '-s', 'format', 'png', str(OUT / (name + '.ppm')),
                        '--out', str(OUT / (name + '.png'))], check=True, stdout=subprocess.DEVNULL)
        print(name, 'PASS', flush=True)
print('15 controlled Metal captures pass. Held-Ctrl events and 2x scaling are synthetic.')
