#!/usr/bin/env python3
"""FONT-T01–T07: Operandi-only simulated font, scale and fallback captures."""
import argparse
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', required=True)
parser.add_argument('--scene', required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
out = root / 'development/typography/renders'
out.mkdir(parents=True, exist_ok=True)
results = []
profiles = [(1100, 720, 1), (1440, 900, 1), (1440, 900, 1.25),
            (1440, 900, 1.5), (1440, 900, 2)]
cases = [(w,h,s,c,'Lucida') for w,h,s in profiles for c in ['Create','Polygon']]
cases += [(1440,900,1,c,f) for f in ['SF','missing'] for c in ['Create','Polygon']]
for logical_w, logical_h, scale, category, font in cases:
    w, h = int(logical_w * scale), int(logical_h * scale)
    name = f'{logical_w}x{logical_h}-{scale}x-{category}-{font}'
    prefix = out / name
    env = os.environ | {
        'OPERANDI_QA_OFFSCREEN': '1', 'OPERANDI_QA_SIZE': f'{w}x{h}',
        'OPERANDI_QA_SCALE': str(scale), 'OPERANDI_QA_CAPTURE': str(prefix),
        'OPERANDI_QA_CATEGORY': category, 'OPERANDI_QA_TOOL': 'Sphere',
        'OPERANDI_QA_KEEP_SELECTION': '1', 'OPERANDI_QA_VIEW': 'Perspective',
        'OPERANDI_QA_FONT': font, 'OPERANDI_DEV_DATA': str(out / (name + '-data')),
    }
    proc = subprocess.run([args.binary, args.scene], env=env,
                          capture_output=True, text=True, timeout=40)
    prefix.with_suffix('.log').write_text(proc.stdout + proc.stderr)
    assert proc.returncode == 0, proc.stderr
    record = json.loads(Path(str(prefix) + '.json').read_text())
    resolved = record['typography']['font']
    if font == 'Lucida':
        assert resolved['family'] == 'Lucida Grande' and resolved['face'] == 'Regular'
        assert resolved['face_index'] == 0 and resolved['fallback'] is None
        if scale == 1:
            for row, expected in zip(record['typography']['advances'], [30.5625,38.21875,47.09375]):
                assert abs(row['layout_width']-expected) <= 1/64
    elif font == 'SF':
        assert resolved['family'] == 'System Font' and resolved['fallback'] is None
    else:
        assert resolved['fallback'] and resolved['postscript_name'] == 'Ubuntu-Light'
    assert resolved['standard_size'] == 10
    rects = record['toolbox']
    create = rects['Category Create']
    polygon = rects['Category Polygon']
    assert abs(create[1] - rects['Transform'][1]) <= 1
    assert abs(create[1] + create[3] - polygon[1]) < .01
    for label in ['Create', 'Polygon']:
        surface = rects[f'Category {label} surface']
        ink = rects[f'Category {label} ink']
        assert surface[2] == 18 and 46 <= surface[3] < 60
        assert ink[0] > surface[0] and ink[1] > surface[1]
        assert ink[0] + ink[2] < surface[0] + surface[2]
        assert ink[1] + ink[3] < surface[1] + surface[3]
    # The existing primitive/transform grid stays 210 units at default width.
    assert abs(rects['Transform'][2] - 51) < .01
    if category == 'Create':
        assert rects['Cube'][2:] == rects['Transform'][2:]
    raw = Path(str(prefix) + '.ppm').read_bytes().split(b'\n', 3)
    assert raw[:3] == [b'P6', f'{w} {h}'.encode(), b'255']
    samples = {}
    for label in ['Create', 'Polygon']:
        x, y, width, height = rects[f'Category {label} surface']
        px, py = int((x+2)*scale), int((y+height/2)*scale)
        offset = (py*w+px)*3
        rgb = list(raw[3][offset:offset+3])
        expected = [255,160,0] if category == label else [107,107,107]
        assert rgb == expected, (name, label, rgb, expected)
        samples[label] = rgb
    record.update(profile=name, category=category, native=False,
                  font=record['typography']['font'],
                  quarter_turn='clockwise; native-scale font galley; pixel-snapped ink center',
                  fill_samples=samples)
    results.append(record)
    subprocess.run(['sips','-s','format','png',str(prefix)+'.ppm','--out',str(prefix)+'.png'],
                   check=True,capture_output=True)
    Path(str(prefix)+'.ppm').unlink()
    print(name+' PASS',flush=True)
(out/'matrix.json').write_text(json.dumps(results,indent=2))
