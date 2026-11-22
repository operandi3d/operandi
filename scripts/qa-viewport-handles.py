#!/usr/bin/env python3
"""VP2 local original-fixture Metal captures, not native input/display claims."""
import json, os, pathlib, subprocess
root=pathlib.Path(__file__).resolve().parents[1]
out=root/'development/viewport-handles/renders'
out.mkdir(parents=True,exist_ok=True)
fixture=out.parent/'Empty_VP2.opscene'
if not fixture.exists():
    scene={'items':[{'id':1,'name':'VP2_QA_Mesh','visible':True,'transform':{'translation':[0,0,0],'rotation':[0,0,0],'scale':[1,1,1]},'mesh':{'vertices':[],'faces':[]}}],'next_id':2,'selection':{'item':1,'mode':'Polygon','vertices':[],'edges':[],'faces':[]},'workplane':{'fixed':False,'plane':0,'center':[0,0,0]}}
    with fixture.open('x') as f: json.dump({'format':'Operandi','version':2,'scene':scene},f,indent=2)
cases=[]
for plane in range(3):
    for stage in ['flat','solid']:
        cases.append((f'cube-{plane}-{stage}',{'VIEW':'Perspective','FIXED_PLANE':str(plane),'TOOL':'Cube','CUBE_STAGE':stage}))
for stage in ['flat','solid']:
    cases.append((f'cube-XZ-{stage}-2x',{'VIEW':'Perspective','FIXED_PLANE':'0','TOOL':'Cube','CUBE_STAGE':stage,'SIZE':'2880x1864','SCALE':'2'}))
for view in ['Front','Right','Top']:
    cases.append((view,{'VIEW':view,'TOOL':'None'}))
for name,options in cases:
    env=os.environ|{'OPERANDI_QA_OFFSCREEN':'1','OPERANDI_QA_KEEP_SELECTION':'1','OPERANDI_QA_SIZE':'1440x932','OPERANDI_QA_SCALE':'1','OPERANDI_QA_CAPTURE':str(out/name),'OPERANDI_DEV_DATA':str(out.parent/'qa-data')}
    env.update({'OPERANDI_QA_'+k:v for k,v in options.items()})
    subprocess.run([str(root/'target/debug/operandi-app'),str(fixture)],env=env,check=True,timeout=30)
    record=json.loads((out/(name+'.json')).read_text())
    if options['TOOL']=='Cube':
        expected=[4,4,1] if options['CUBE_STAGE']=='flat' else [8,12,6]
        assert record['mesh_counts']==expected,(name,record['mesh_counts'])
        normal={'0':1,'1':0,'2':2}[options['FIXED_PLANE']]
        actual=record['primitive_parameters']['size'][normal]
        assert abs(actual-(0 if options['CUBE_STAGE']=='flat' else 1))<1e-5,(name,actual)
    else:
        assert record['actual_plane']=={'Front':2,'Right':1,'Top':0}[name]
    subprocess.run(['sips','-s','format','png',str(out/(name+'.ppm')),'--out',str(out/(name+'.png'))],check=True,stdout=subprocess.DEVNULL)
    print(name,'PASS',flush=True)
print(len(cases),'controlled Metal captures, synthetic camera and pointer states.')
