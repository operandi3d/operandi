struct Grid {
 eye: vec4<f32>, right: vec4<f32>, up: vec4<f32>, forward: vec4<f32>,
 u: vec4<f32>, v: vec4<f32>, color: vec4<f32>, axis_u: vec4<f32>, axis_v: vec4<f32>, options: vec4<f32>,
 view: vec4<f32>
};
@group(0) @binding(0) var<uniform> grid: Grid;
@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> {
 let p=array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
 return vec4(p[i],1.,1.);
}
fn lattice(p:vec2<f32>,derivative:vec2<f32>,step:f32)->f32 {
 let cell=p/step;
 let footprint=max(derivative/step,vec2(0.00001));
 let distance=abs(fract(cell-0.5)-0.5)/footprint;
 // Suppress unresolved lines without losing the next major lattice.
 let visible=1.-smoothstep(0.15,0.65,max(footprint.x,footprint.y));
 return (1.-min(min(distance.x,distance.y),1.))*visible;
}
fn shade(pixel:vec4<f32>)->Shaded {
 // view.x is the view's lens, the same one the mesh projection uses.
 let xy=vec2((pixel.x/grid.u.w-0.5)*grid.right.w/grid.view.x,(0.5-pixel.y/grid.v.w)/grid.view.x);
 let normal=cross(grid.u.xyz,grid.v.xyz);
 var ray=grid.forward.xyz;
 var origin=grid.eye.xyz;
 if grid.forward.w>0.5 {ray=normalize(ray+grid.right.xyz*xy.x+grid.up.xyz*xy.y);}
 else {origin+=grid.up.w*(grid.right.xyz*xy.x+grid.up.xyz*xy.y);}
 let denom=dot(ray,normal);
 let t=-dot(origin,normal)/select(0.0000001,denom,abs(denom)>0.0000001);
 let point=origin+ray*t;
 let p=vec2(dot(point,grid.u.xyz),dot(point,grid.v.xyz));
 let footprint=fwidth(p)*grid.options.y;
 let minor=lattice(p,footprint,grid.eye.w);
 let major=lattice(p,footprint,grid.eye.w*5.);
 let coarse=lattice(p,footprint,grid.eye.w*25.);
 // view.y: Draw Faded Grid (1 fades with distance, 0 doesn't). up.w is
 // the focus distance: the fade runs from one to four of them (#58: from
 // six to 35 it began past where the grid shows, so it did nothing).
 let fade=select(1.,1.-smoothstep(grid.up.w*1.,grid.up.w*4.,length(point-grid.eye.xyz)),grid.view.y>0.5);
 var alpha=max(minor*0.16,max(major*0.32,coarse*0.44))*fade*grid.options.x*grid.options.z;
 var premultiplied=grid.color.rgb*alpha;
 if grid.color.w>0.5 {
   // Use viewport-relative coordinates so the ground-axis fade remains visible
   // in orthographic views as well as perspective, at every display scale.
   let uv=pixel.xy/vec2(grid.u.w,grid.v.w);
   let border=min(min(uv.x,1.-uv.x),min(uv.y,1.-uv.y));
   var axis_fade=fade*smoothstep(0.,0.16,border)*0.7;
   var d=abs(p+vec2(grid.axis_v.w,grid.axis_u.w))/max(footprint,vec2(0.00001));
   if grid.options.w>0.5 {
     // Zero lines in Top/Front/Side: solid, edge to edge, a touch wider.
     axis_fade=1.;
     d=d/1.25;
   }
   let along_u=(1.-min(d.y,1.))*axis_fade;
   premultiplied=grid.axis_u.rgb*along_u+premultiplied*(1.-along_u);
   alpha=along_u+alpha*(1.-along_u);
   let along_v=(1.-min(d.x,1.))*axis_fade;
   premultiplied=grid.axis_v.rgb*along_v+premultiplied*(1.-along_v);
   alpha=along_v+alpha*(1.-along_v);
 }
 // The hit's depth as the mesh and backdrop projections write it, for the
 // depth-tested draw over meshes and backdrops (original-compatible A5).
 // Perspective: (z - near) / z, near in view.z. Orthographic:
 // (z + offset) / range, offset and range in view.z and view.w. Pushed a
 // hair farther so a surface lying on the plane keeps covering it
 // instead of speckling through.
 let z_eye=dot(point-grid.eye.xyz,grid.forward.xyz);
 var out:Shaded;
 if grid.forward.w>0.5 {
   let z=max(z_eye,grid.view.z*1.1)*1.0005;
   out.depth=clamp((z-grid.view.z)/z,0.,1.);
 } else {
   out.depth=clamp((z_eye+grid.view.z)/max(grid.view.w,0.000001)+0.000002,0.,1.);
 }
 // Orthographic: the plane draws wherever it lies along the view, behind
 // the zoomed-in eye too (bug report #2).
 if (t<=0. && grid.forward.w>0.5) || abs(denom)<0.000001 {out.color=vec4(0.);out.depth=1.;return out;}
 out.color=vec4(premultiplied,alpha);
 return out;
}
struct Shaded { color: vec4<f32>, depth: f32 };
@fragment fn fs(@builtin(position) pixel:vec4<f32>)->@location(0) vec4<f32> {
 return shade(pixel).color;
}
struct DepthOut { @location(0) color: vec4<f32>, @builtin(frag_depth) depth: f32 };
// Depth-tested (A5): grid in front of a Perspective backdrop draws over it.
@fragment fn fs_depth(@builtin(position) pixel:vec4<f32>)->DepthOut {
 let s=shade(pixel);
 var out:DepthOut; out.color=s.color; out.depth=s.depth; return out;
}
