#!/usr/bin/env python3
"""Render admitted dictionary integration with the Operandi-only QA harness."""
import argparse
import json
import os
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', required=True)
p.add_argument('--output', type=Path, help='Separate local QA output directory')
args = p.parse_args()
root = Path(__file__).resolve().parent.parent
out = args.output or root / 'development/icon-dictionary/qa'
out.mkdir(parents=True, exist_ok=True)
profiles = [(w, h, s, 'Create')
            for w, h in [(1920, 1080), (2560, 1440), (3840, 2160), (5120, 2880)]
            for s in [1, 1.5, 2]]
profiles += [(1440, 900, s, 'Polygon') for s in [1, 1.5, 2]]
profiles += [(1100, 720, 1, 'Create')]
results = []
for w, h, scale, category in profiles:
    name = f'{w}x{h}-{scale}x-{category}'
    prefix = out / name
    env = os.environ | {'OPERANDI_QA_OFFSCREEN': '1',
                        'OPERANDI_QA_SIZE': f'{w}x{h}',
                        'OPERANDI_QA_SCALE': str(scale),
                        'OPERANDI_QA_CATEGORY': category,
                        'OPERANDI_QA_TOOL': 'Move',
                        'OPERANDI_QA_CAPTURE': str(prefix),
                        'OPERANDI_DEV_DATA': str(out / (name + '-data'))}
    subprocess.run([args.binary, str(root / 'fixtures/VS_Box_Run01.opscene')],
                   env=env, check=True, capture_output=True)
    record = json.loads(Path(str(prefix) + '.json').read_text())
    assert record['selection'] and record['live'] == 'Move'
    assert record['scale'] == scale
    expected = {'Move', 'Rotate', 'Scale'} | ({'Cube'} if category == 'Create' else {'Bevel', 'Extrude'})
    assert expected <= record['toolbox'].keys()
    record['profile'] = name
    record['density'] = ('usable' if w / scale >= 1100 and h / scale >= 720
                         else 'below-minimum stress only')
    results.append(record)
    # Prefix contains fractional scales; append suffix rather than replacing it.
    subprocess.run(['sips', '-s', 'format', 'png', str(prefix) + '.ppm',
                    '--out', str(prefix) + '.png'], check=True, capture_output=True)
    Path(str(prefix) + '.ppm').unlink()
    print(name, 'Pass', flush=True)
(out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
