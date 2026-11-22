#!/usr/bin/env python3
"""Original Operandi-only controlled captures; standard library + macOS sips.
Read back raw framebuffer RGB before any screenshot/display ICC transform.
"""
import argparse
import collections
import json
import os
from pathlib import Path
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--binary', required=True)
p.add_argument('--scene', required=True)
args = p.parse_args()
root = Path(__file__).resolve().parent.parent
out = root / 'development/milestone-1.1/toolbox-amendment/renders'
out.mkdir(parents=True, exist_ok=True)
profiles = [(w, h, s, 'Create', 'Move', False)
            for w, h in [(1920, 1080), (2560, 1440), (3840, 2160), (5120, 2880)]
            for s in [1, 1.5, 2]]
profiles += [(w, h, s, cat, 'Cube' if cat == 'Create' else 'Move', False)
             for w, h, s in [(1440, 900, 1), (1100, 720, 1), (2200, 1440, 2)]
             for cat in ['Create', 'Polygon']]
profiles += [(1440, 900, 2, 'Create', 'Cube', True)]
results = []
for w, h, scale, category, tool, scroll in profiles:
    name = f'{w}x{h}-{scale}x-{category}-{tool}' + ('-scroll' if scroll else '')
    prefix = out / name
    data = out / (name + '-data')
    data.mkdir(exist_ok=True)
    env = os.environ | {'OPERANDI_QA_OFFSCREEN': '1', 'OPERANDI_QA_SIZE': f'{w}x{h}',
        'OPERANDI_QA_SCALE': str(scale), 'OPERANDI_QA_CAPTURE': str(prefix),
        'OPERANDI_QA_CATEGORY': category, 'OPERANDI_QA_TOOL': tool,
        'OPERANDI_DEV_DATA': str(data)}
    if scroll:
        env['OPERANDI_QA_SCROLL'] = '1'
    proc = subprocess.run([args.binary, args.scene], env=env, capture_output=True, text=True, timeout=40)
    (out / (name + '.log')).write_text(proc.stdout + proc.stderr)
    assert proc.returncode == 0, proc.stderr
    record = json.loads(Path(str(prefix) + '.json').read_text())
    assert record['capture'] == [w, h]
    assert len(record['selection']) == 1
    raw = Path(str(prefix) + '.ppm').read_bytes().split(b'\n', 3)
    assert raw[:3] == [b'P6', f'{w} {h}'.encode(), b'255']
    pixels = raw[3]
    rects = record['toolbox']
    active = rects['Category ' + category]
    inactive = rects['Category ' + ('Polygon' if category == 'Create' else 'Create')]
    header = rects['Primitives' if category == 'Create' else 'Polygon Tools']
    samples = {
        'background': ([2, 300], [82, 82, 82]),
        'header': ([header[0] + header[2] - 8, header[1] + 5], [71, 71, 71]),
        'neutral': ([inactive[0] + 4, inactive[1] + 4], [107, 107, 107]),
        'active': ([active[0] + 4, active[1] + 4], [255, 160, 0]),
        'border': ([inactive[0], inactive[1] + 15], [55, 55, 55]),
    }
    measured = {}
    for role, (point, expected) in samples.items():
        x, y = (int(v * scale) for v in point)
        offset = (y * w + x) * 3
        actual = list(pixels[offset:offset + 3])
        assert actual == expected, (name, role, actual, expected)
        measured[role] = {'logical_sample': point, 'rgb': actual, 'target': expected}
    hist = collections.Counter(pixels[i:i + 3] for y in range(int(52 * scale), int(min(h/scale, 500)*scale))
                               for i in range((y*w)*3, (y*w+int(242*scale))*3, 3))
    assert hist[bytes([210, 210, 210])] > 0
    assert hist[bytes([0, 0, 0])] > 0
    record.update(profile=name, category=category, tool=tool, scroll=scroll, flat_samples=measured,
                  light_text_pixels=hist[bytes([210, 210, 210])], active_text_pixels=hist[bytes([0, 0, 0])],
                  color_method='raw egui/wgpu display framebuffer; sRGB code values, no ICC/display conversion')
    results.append(record)
    subprocess.run(['sips', '-s', 'format', 'png', str(prefix) + '.ppm', '--out', str(prefix) + '.png'],
                   check=True, capture_output=True)
    Path(str(prefix) + '.ppm').unlink()
    print(name, 'Pass', flush=True)
(out / 'results.json').write_text(json.dumps(results, indent=2))
