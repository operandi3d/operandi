#!/usr/bin/env python3
"""Audit the original local native demo, including exact attribute comparison."""
import collections, hashlib, json, pathlib
root=pathlib.Path(__file__).resolve().parents[1]
out=root/'development/inset-center'
actual=json.loads((out/'IC_Native_Demonstration.opscene').read_text())['scene']
expected=json.loads((out/'IC_Cylinder_Demonstration.opscene').read_text())['scene']
base=json.loads((out/'IC_Cylinder_Input.opscene').read_text())['scene']
assert actual==expected, 'Native and portable two-stage results differ'
mesh=actual['items'][0]['mesh'];src=base['items'][0]['mesh']
assert actual['items'][1]==base['items'][1]
assert actual['selection']==base['selection']
edges=collections.defaultdict(list)
for face in mesh['faces']:
    vs=face['vertices']
    for a,b in zip(vs,vs[1:]+vs[:1]):edges[min(a,b),max(a,b)].append((a,b))
assert all(len(uses)==2 and uses[0]==uses[1][::-1] for uses in edges.values())
assert [len(mesh['vertices']),len(edges),len(mesh['faces'])]==[88,152,66]
assert [len(mesh['faces'][f]['vertices']) for f in [0,1]]==[24,24]
for fi in actual['selection']['faces']:
    a,b=src['faces'][fi],mesh['faces'][fi]
    assert {k:v for k,v in a.items() if k!='vertices'}=={k:v for k,v in b.items() if k!='vertices'}
for name,values in src['uv_maps'].items():assert mesh['uv_maps'][name][:len(values)]==values
manifest=json.loads((root/'assets/branding/branding-manifest.json').read_text())
for asset in manifest['assets']:
    assert hashlib.sha256((root/'assets/branding'/asset['file']).read_bytes()).hexdigest()==asset['sha256']
record={'exact_scene_equal':True,'native':'IC_Native_Demonstration.opscene','expected':'IC_Cylinder_Demonstration.opscene','vef':[88,152,66],'closed_consistently_oriented':True,'selected_caps':actual['selection']['faces'],'unchanged_hidden_item':True,'exact_cap_and_map_attributes':True,'branding_master_hashes_unchanged':True,'native_sha256':hashlib.sha256((out/'IC_Native_Demonstration.opscene').read_bytes()).hexdigest()}
(out/'native-audit.json').write_text(json.dumps(record,indent=2))
print(json.dumps(record,indent=2))
