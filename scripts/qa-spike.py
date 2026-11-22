#!/usr/bin/env python3
"""Original Spike numeric egui/Metal checks; simulated sizes/scales only."""
import argparse
import json
import os
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--binary', required=True)
p.add_argument('--output', type=Path)
a = p.parse_args()
out = a.output or Path(__file__).resolve().parent.parent / 'development/spike-numeric/renders'
out.mkdir(parents=True, exist_ok=True)
fixture = out / 'original-square.opscene'
fixture.write_text(json.dumps({'format': 'Operandi', 'version': 4, 'scene': {
    'items': [{'id': 1, 'name': 'Spike original square', 'visible': True,
               'transform': {'translation': [0,0,0], 'rotation': [0,0,0], 'scale': [1,1,1]},
               'mesh': {'vertices': [[-1,-1,0],[1,-1,0],[1,1,0],[-1,1,0]],
                        'faces': [{'vertices': [0,1,2,3], 'uv': None, 'material': 'Original', 'smooth': False}]}}],
    'next_id': 2, 'selection': {'item': 1, 'mode': 'Polygon', 'vertices': [], 'edges': [], 'faces': []},
    'workplane': {'fixed': False, 'plane': 0, 'center': [0,0,0]}}}))
profiles = [(1100,720,1,0),(1440,900,1,.5),(1920,1080,1,-.25),
            (2560,1440,1.5,.5),(3840,2160,2,.5),(5120,2880,2,.5)]
records=[]
for w,h,scale,strength in profiles:
    prefix = out / f'{w}x{h}-{scale}x-{strength}'
    env = os.environ | {'OPERANDI_QA_OFFSCREEN': '1', 'OPERANDI_QA_TOOL': 'None',
        'OPERANDI_QA_SPIKE': str(strength), 'OPERANDI_QA_SIZE': f'{w}x{h}',
        'OPERANDI_QA_SCALE': str(scale), 'OPERANDI_QA_CAPTURE': str(prefix),
        'OPERANDI_QA_KEEP_SELECTION': '1', 'OPERANDI_QA_VIEW': 'Perspective',
        'OPERANDI_DEV_DATA': str(out / (prefix.name + '-data'))}
    proc=subprocess.run([a.binary,str(fixture)],env=env,capture_output=True,text=True,timeout=60)
    if proc.returncode: raise SystemExit(proc.stderr + proc.stdout)
    record=json.loads(Path(str(prefix)+'.json').read_text())
    assert record['mesh_counts']==[5,8,4],record
    assert record['spike_strength']==strength,record
    assert record['selection']==[],record
    assert 'Spike' in record['toolbox'],record
    assert not record['gesture_captured'],record
    records.append(record)
    print(f'PASS {prefix.name}: 5V/8E/4F, numeric Spike active, Metal capture')
(out/'matrix.json').write_text(json.dumps(records,indent=2))
