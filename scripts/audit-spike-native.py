#!/usr/bin/env python3
"""Verify the original SA-A01-derived native QA scene by semantic correspondence."""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--directory',type=Path,default=Path.home()/'Library/Caches/Operandi-Rust/SpikeNumericQA')
a=p.parse_args()
def normalize(item):
    item=copy.deepcopy(item)
    for k in ['parent','source_id','source_display_name']:item.setdefault(k,None)
    item['transform'].setdefault('zxy',False)
    m=item['mesh'];m.setdefault('uv_maps',{});attr=m.setdefault('attributes',{})
    for k in ['representations','creases','degenerate_faces','smoothing_labels']:attr.setdefault(k,[])
    for k in ['uv_records','corner_normals','point_weights','point_colors','relative_morphs']:attr.setdefault(k,{})
    for f in m['faces']:
        f.setdefault('part',None);f.setdefault('material_label',None)
    return item
src=json.loads((a.directory/'Original_Attributed.opscene').read_text())
result=a.directory/'Native_Attributed.opscene'
out=json.loads(result.read_text());assert out['version']==5
source=normalize(src['scene']['items'][0])['mesh'];m=out['scene']['items'][0]['mesh']
assert normalize(out['scene']['items'][1])==normalize(src['scene']['items'][1])
assert m['vertices'][:6]==source['vertices'] and m['vertices'][6]==[-.5,0.,-.4]
assert len(m['faces'])==5 and out['scene']['selection']['faces']==[0,1,2,3]
assert m['faces'][4]==source['faces'][1]
for name,records in source['uv_maps'].items():
    assert m['uv_maps'][name][4]==records[1]
    for i in range(4):
        assert m['uv_maps'][name][i][:2]==[records[0][i],records[0][(i+1)%4]]
        assert m['uv_maps'][name][i][2]==[.5,.5]
attr=m['attributes'];old=source['attributes']
assert attr['creases']==old['creases']
assert attr['smoothing_labels']==['7']*4+['8']
assert attr['representations']==['Subdivs']*4+['CatmullClark']
assert attr['relative_morphs']['OriginalMorph']['6']==[0.,0.,0.]
for key in ['relative_morphs','point_weights','point_colors']:
    for name,records in old[key].items():
        assert all(attr[key][name][i]==value for i,value in records.items())
assert math.isclose(attr['point_weights']['OriginalWeight']['6'],.3)
assert all(math.isclose(x,y,abs_tol=1e-14) for x,y in zip(attr['point_colors']['OriginalColor']['6'],[.2,.25,.75,1]))
assert all(f[2]==[0.,.25,.25] for f in attr['corner_normals']['OriginalNormal'][:4])
assert attr['corner_normals']['OriginalNormal'][4]==old['corner_normals']['OriginalNormal'][1]
for f in m['faces'][:4]:
    for key in ['material','part','material_label','smooth']:assert f[key]==source['faces'][0][key]
print('Native v5 PASS: 7V/5F; apex(-.5,0,-.4); four selected descendants; UVs/presence, normals, weight/color, zero morph, labels/types/crease, unselected face and unrelated item preserved (defaults normalized).')
print('SHA256',hashlib.sha256(result.read_bytes()).hexdigest())
