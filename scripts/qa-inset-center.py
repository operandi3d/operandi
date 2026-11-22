#!/usr/bin/env python3
"""IC original input, controlled Metal captures; no native/physical input claim."""
import json, os, pathlib, subprocess
root=pathlib.Path(__file__).resolve().parents[1]
out=root/'development/inset-center/renders';out.mkdir(parents=True,exist_ok=True)
records=[]
profiles=[('1100x720','1'),('1440x900','1')]+[(size,scale) for size in ['1920x1080','2560x1440','3840x2160','5120x2880'] for scale in ['1','1.5','2']]
for size,scale in profiles:
    for stage in ['A','B']:
        label=f'Curved-{stage}-{size}-{scale}'.replace('.','p');prefix=out/label
        env=os.environ|{'OPERANDI_QA_OFFSCREEN':'1','OPERANDI_QA_KEEP_SELECTION':'1','OPERANDI_QA_SELECTION_FRAME':'1','OPERANDI_QA_CAPTURE':str(prefix),'OPERANDI_QA_SIZE':size,'OPERANDI_QA_SCALE':scale,'OPERANDI_QA_TOOL':'Bevel','OPERANDI_QA_POLYGON_STAGE':stage,'OPERANDI_QA_CURVED_INSET':'0.01' if stage=='A' else '0.025','OPERANDI_QA_VIEW':'Perspective','OPERANDI_QA_CATEGORY':'Polygon','OPERANDI_DEV_DATA':str(out.parent/'qa-data')}
        subprocess.run([str(root/'target/debug/operandi-app'),str(out.parent/'IC_Patch_Input.opscene')],env=env,check=True,timeout=60,stdout=subprocess.DEVNULL)
        meta=json.loads(prefix.with_suffix('.json').read_text());assert meta['live']=='Bevel';assert meta['status']=='Ready',meta['status'];assert meta['mesh_counts']==([16,26,11] if stage=='A' else [24,42,19]);assert meta['history']==(stage=='B')
        subprocess.run(['sips','-s','format','png',str(prefix.with_suffix('.ppm')),'--out',str(prefix.with_suffix('.png'))],check=True,stdout=subprocess.DEVNULL)
        records.append({'capture':label,'gpu':meta['gpu'],'counts':meta['mesh_counts'],'method':meta['method']});print(label,flush=True)
(out/'matrix.json').write_text(json.dumps(records,indent=2))
print(f'{len(records)} controlled Metal captures pass; native/physical results separate.')
