#!/usr/bin/env python3
"""GA-001..003: original-fixture Metal raster checks; synthetic display states.

Run after cargo build --locked -p operandi-app --features qa.
Does not open a native window or read user scenes.
"""
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'development/ground-axes/renders'
OUT.mkdir(parents=True, exist_ok=True)
FIXTURE = OUT.parent / 'Empty_GroundAxes.opscene'
if not FIXTURE.exists():
    scene = {
        'items': [{'id': 1, 'name': 'GroundAxes_QA', 'visible': True,
                   'transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0], 'scale': [1, 1, 1]},
                   'mesh': {'vertices': [], 'faces': []}}],
        'next_id': 2,
        'selection': {'item': 1, 'mode': 'Polygon', 'vertices': [], 'edges': [], 'faces': []},
        'workplane': {'fixed': False, 'plane': 0, 'center': [0, 0, 0]},
    }
    with FIXTURE.open('x') as stream:
        json.dump({'format': 'Operandi', 'version': 2, 'scene': scene}, stream)


def capture(name, **options):
    env = os.environ | {
        'OPERANDI_QA_OFFSCREEN': '1', 'OPERANDI_QA_KEEP_SELECTION': '1',
        'OPERANDI_QA_SIZE': '1440x932', 'OPERANDI_QA_SCALE': '1',
        'OPERANDI_QA_TOOL': 'None', 'OPERANDI_QA_VIEW': 'Top',
        'OPERANDI_QA_PLANE_DISPLAY': 'Off',
        'OPERANDI_QA_CAPTURE': str(OUT / name),
        'OPERANDI_DEV_DATA': str(OUT.parent / 'qa-data'),
    }
    env.update({'OPERANDI_QA_' + key: value for key, value in options.items()})
    subprocess.run([str(ROOT / 'target/debug/operandi-app'), str(FIXTURE)],
                   env=env, check=True, timeout=30)
    record = json.loads((OUT / (name + '.json')).read_text())
    assert record['mesh_counts'] == [0, 0, 0] and record['history'] == 0
    assert record['handles'] == [] and record['live'] is None
    pixels = (OUT / (name + '.ppm')).read_bytes().split(b'\n', 3)[3]
    subprocess.run(['sips', '-s', 'format', 'png', str(OUT / (name + '.ppm')),
                    '--out', str(OUT / (name + '.png'))], check=True, stdout=subprocess.DEVNULL)
    return record, pixels


def fade_check(on, off):
    record, image = on
    _, baseline = off
    scale = record['scale']
    left, top, width, height = [v * scale for v in record['viewport']]
    stride = record['capture'][0]

    def strength(axis, fraction):
        x = left + width * (fraction if axis == 0 else .5)
        y = top + height * (.5 if axis == 0 else fraction)
        values = []
        for delta in range(-3 * int(scale), 3 * int(scale) + 1):
            px, py = round(x), round(y)
            if axis == 0:
                py += delta
            else:
                px += delta
            i = (py * stride + px) * 3
            values.append(sum(abs(image[i + c] - baseline[i + c]) for c in range(3)))
        return max(values)

    # Both ends of each real world-origin axis fade, without a hard cutoff.
    for axis in (0, 1):
        for sign in (1, -1):
            fractions = [.02, .08, .3] if sign == 1 else [.98, .92, .7]
            edge, middle, core = [strength(axis, f) for f in fractions]
            assert core > 20 and edge < core * .2 and edge < middle < core, (axis, sign, edge, middle, core)
            print('axis', axis, 'side', sign, 'edge/mid/core', edge, middle, core)

    # No added green idle Y axis. The unchanged corner badge keeps its Y cue.
    for y in range(round(top + 60 * scale), round(top + height - 35 * scale)):
        for x in range(round(left + 5 * scale), round(left + width - 5 * scale)):
            i = (y * stride + x) * 3
            r, g, b = image[i:i+3]
            assert image[i:i+3] == baseline[i:i+3] or g <= max(r, b), (x, y, r, g, b)


top = capture('top-axes')
off = capture('top-hidden', GROUND_AXES='Off')
fade_check(top, off)
bottom = capture('bottom-axes', VIEW='Bottom')
bottom_off = capture('bottom-hidden', VIEW='Bottom', GROUND_AXES='Off')
fade_check(bottom, bottom_off)
large = capture('top-axes-2x', SIZE='2880x1864', SCALE='2')
large_off = capture('top-hidden-2x', SIZE='2880x1864', SCALE='2', GROUND_AXES='Off')
fade_check(large, large_off)
# Workplane transparency must not alter the restored ground-axis layer.
transparent = capture('top-transparent-workplane', PLANE_DISPLAY='On', WORKPLANE_OPACITY='0')
left, upper, width, height = [round(v) for v in top[0]['viewport']]
stride = top[0]['capture'][0]
# The status caption intentionally reports the different visibility settings.
for y in range(upper + 35, upper + height - 35):
    a, b = (y * stride + left) * 3, (y * stride + left + width) * 3
    assert top[1][a:b] == transparent[1][a:b], 'Transparent workplane changed axes-only rendering'
# Original grid and workplane settings remain independent, including views with
# an edge-on ground plane, where no spurious upright Y axis may appear.
for view in ('Perspective', 'Front', 'Right'):
    result = capture(view.lower(), VIEW=view, PLANE_DISPLAY='On', GRID='On')
    assert result[0]['actual_plane'] == {'Perspective': 2, 'Front': 2, 'Right': 1}[view]
    assert result[0]['ground_axes'] and result[0]['ground_grid']
print('10 controlled Metal captures passed; no physical DPI/input claim.')
