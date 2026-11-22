#!/usr/bin/env python3
"""Render a locally saved Extrude result and its armed panel at simulated scales.
Input must be an original local QA scene; no reference-product scenes are used.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', required=True)
parser.add_argument('--scene', required=True)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
records = []
for width, height, scale in [(1100, 720, 1), (1920, 1080, 1), (2560, 1440, 1.5),
                             (3840, 2160, 2), (5120, 2880, 2), (1440, 900, 1)]:
    prefix = args.output / f'{width}x{height}-{scale}x'
    env = os.environ | {'OPERANDI_QA_OFFSCREEN': '1', 'OPERANDI_QA_TOOL': 'Extrude',
        'OPERANDI_QA_KEEP_SELECTION': '1', 'OPERANDI_QA_VIEW': 'Perspective',
        'OPERANDI_QA_CATEGORY': 'Polygon', 'OPERANDI_QA_SIZE': f'{width}x{height}',
        'OPERANDI_QA_SCALE': str(scale), 'OPERANDI_QA_CAPTURE': str(prefix),
        'OPERANDI_DEV_DATA': str(args.output / (prefix.name + '-data'))}
    result = subprocess.run([args.binary, args.scene], env=env, capture_output=True,
                            text=True, timeout=60)
    if result.returncode:
        raise SystemExit(result.stderr + result.stdout)
    record = json.loads(Path(str(prefix) + '.json').read_text())
    assert record['mesh_counts'] == [12, 20, 9], record
    assert record['selection'] == [0], record
    assert record['live'] == 'Extrude', record
    assert len(record['handles']) == 3, record
    assert record['polygon_parameters']['extent'] == [0, 0, 0], record
    records.append(record)
    print(f'PASS {prefix.name}: saved two-stage 12V/20E/9F, armed Extrude panel')
(args.output / 'matrix.json').write_text(json.dumps(records, indent=2))
