#!/usr/bin/env python3
"""Original M1.6 scene and offscreen Metal checks; no physical-display claim."""
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
out = args.output or root / 'development/milestone-1.6/renders'
out.mkdir(parents=True, exist_ok=True)
vertices = [[-4+x*.5, -4+y*.5, -4+z*.5] for x,y,z in [
    (-1,-1,-1),(1,-1,-1),(1,1,-1),(-1,1,-1),
    (-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1)]]
faces = [{'vertices':v,'uv':None,'material':'original','smooth':False}
         for v in [[0,3,2,1],[4,5,6,7],[0,4,7,3],[1,2,6,5],[0,1,5,4],[3,7,6,2]]]
scene = {'format':'Operandi','version':1,'scene':{
    'items':[{'id':1,'name':'Original QA mesh','visible':True,
              'transform':{'translation':[0,0,0],'rotation':[0,0,0],'scale':[1,1,1]},
              'mesh':{'vertices':vertices,'faces':faces}}],
    'next_id':2,'selection':{'item':1,'mode':'Polygon','vertices':[],'edges':[],'faces':[0]},
    'workplane':{'fixed':True,'plane':0,'center':[0,0,0]}}}
fixture = out / 'original-box.opscene'
fixture.write_text(json.dumps(scene))
profiles = [(1100,720,1),(1920,1080,1),(2560,1440,1.5),(3840,2160,2),(5120,2880,2)]
results = []
for tool in ['Sphere','Cylinder']:
    for w,h,scale in profiles:
        for state in (['solid','draft'] if w==1920 else ['solid']):
            name = f'{tool}-{state}-{w}x{h}-{scale}x'
            prefix = out / name
            env = os.environ | {'OPERANDI_QA_OFFSCREEN':'1','OPERANDI_QA_SIZE':f'{w}x{h}',
                'OPERANDI_QA_SCALE':str(scale),'OPERANDI_QA_CAPTURE':str(prefix),
                'OPERANDI_QA_TOOL':tool,'OPERANDI_QA_RADIAL_STATE':state,
                'OPERANDI_QA_KEEP_SELECTION':'1','OPERANDI_QA_VIEW':'Top' if state=='draft' else 'Perspective',
                'OPERANDI_DEV_DATA':str(out / (name+'-data'))}
            proc = subprocess.run([args.binary,str(fixture)],env=env,capture_output=True,text=True,timeout=40)
            prefix.with_suffix('.log').write_text(proc.stdout+proc.stderr)
            if proc.returncode:
                raise SystemExit(f'{name} failed: {proc.stderr}')
            record = json.loads(Path(str(prefix)+'.json').read_text())
            assert record['live']==tool and record['capture']==[w,h]
            assert abs(record['scale']-scale)<.001
            record['profile']=name
            results.append(record)
            subprocess.run(['sips','-s','format','png',str(prefix)+'.ppm','--out',str(prefix)+'.png'],check=True,capture_output=True)
            Path(str(prefix)+'.ppm').unlink()
            print(name+' PASS',flush=True)
(out/'matrix.json').write_text(json.dumps(results,indent=2))
