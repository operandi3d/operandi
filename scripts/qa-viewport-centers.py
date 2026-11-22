#!/usr/bin/env python3
"""Local VAC Metal captures; synthetic camera states, never native orbit claims.
Uses only generated VAC_F01/F04 fixtures in ignored development output.
"""
import json, os, pathlib, subprocess
root = pathlib.Path(__file__).resolve().parents[1]
out = root / 'development/viewport-centers/renders'
out.mkdir(parents=True, exist_ok=True)
# Reproducible original VAC-F01/F03/F04 data. Never replace an existing scene.
for name, offset in [('VAC_F01', 0), ('VAC_F04', 10000)]:
    path=out.parent / f'{name}.opscene'
    if path.exists():
        continue
    vertices=[[x+offset,y,z+offset] for x,y,z in [(2,0,-1),(4,0,-1),(4,2,-1),(2,2,-1),(2,0,1),(4,0,1),(4,2,1),(2,2,1)]]
    faces=[[0,3,2,1],[4,5,6,7],[0,4,7,3],[1,2,6,5],[0,1,5,4],[3,7,6,2]]
    item={'id':1,'name':name+'_Offset_Box','visible':True,'transform':{'translation':[0,0,0],'rotation':[0,0,0],'scale':[1,1,1]},'mesh':{'vertices':vertices,'faces':[{'vertices':v,'uv':[[i,0],[i+1,0],[i+1,1],[i,1]],'material':'cap' if i==5 else 'wall','smooth':i==5} for i,v in enumerate(faces)]}}
    scene={'items':[item],'next_id':2,'selection':{'item':1,'mode':'Polygon','vertices':[],'edges':[],'faces':[5]},'workplane':{'fixed':False,'plane':0,'center':[0,0,0]}}
    with path.open('x') as f:
        json.dump({'format':'Operandi','version':2,'scene':scene},f,indent=2)
cases = []
for view in ['Front','Back','Top','Bottom','Left','Right']:
    cases.append((view, 'VAC_F01', {'VIEW':view,'TOOL':'None'}))
for tool in ['Move','Rotate','Scale','Transform','Bevel']:
    cases.append((tool, 'VAC_F01', {'VIEW':'Perspective','TOOL':tool}))
for label,size,scale in [('1080p','1920x1080','1'),('1440p','2560x1440','1'),('4k','3840x2160','1.5'),('5k','5120x2880','2')]:
    cases.append((label,'VAC_F01',{'SIZE':size,'SCALE':scale,'TOOL':'Transform','VIEW':'Perspective'}))
for yaw in [0,1.57,3.14,4.71]:
    for pitch in [-0.7,0.02,0.7]:
        cases.append((f'orbit-{yaw}-{pitch}','VAC_F01',{'YAW':str(yaw),'PITCH':str(pitch),'VIEW':'Perspective','TOOL':'None'}))
cases.append(('distant','VAC_F04',{'VIEW':'Perspective','TOOL':'None'}))
for plane in range(3):
    cases.append((f'fixed-{plane}','VAC_F01',{'VIEW':'Perspective','FIXED_PLANE':str(plane),'TOOL':'None'}))
for name,fixture,options in cases:
    env=os.environ.copy()
    env.update({'OPERANDI_QA_OFFSCREEN':'1','OPERANDI_QA_KEEP_SELECTION':'1','OPERANDI_QA_CAPTURE':str(out/name),'OPERANDI_QA_SIZE':'1440x932','OPERANDI_QA_SCALE':'1','OPERANDI_DEV_DATA':str(root/'development/viewport-centers/qa-data')})
    env.update({'OPERANDI_QA_'+key:value for key,value in options.items()})
    subprocess.run([str(root/'target/debug/operandi-app'), str(root/'development/viewport-centers'/f'{fixture}.opscene')], env=env, check=True, timeout=30)
    meta=json.loads((out/f'{name}.json').read_text())
    assert meta['grid_step']>0
    subprocess.run(['sips','-s','format','png',str(out/f'{name}.ppm'),'--out',str(out/f'{name}.png')], check=True, stdout=subprocess.DEVNULL)
    print(name, 'plane',meta['actual_plane'],'step',meta['grid_step'],flush=True)
print(len(cases),'controlled Metal captures; see method in JSON. No native-display certification.')
