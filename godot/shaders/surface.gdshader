shader_type spatial;
uniform vec4 tint : source_color = vec4(0.5);
uniform float wetness=0.;
uniform int surface_kind = 0;
varying vec3 world_pos;
varying vec3 world_normal;
void vertex(){
 world_pos=(MODEL_MATRIX*vec4(VERTEX,1.0)).xyz;
 world_normal=normalize(MODEL_NORMAL_MATRIX*NORMAL);
}
float hash(vec2 p){return fract(sin(dot(p,vec2(12.9898,78.233)))*43758.5453);}
float vnoise(vec2 p){
 vec2 i=floor(p),f=fract(p);
 f=f*f*(3.0-2.0*f);
 return mix(mix(hash(i),hash(i+vec2(1.,0.)),f.x),mix(hash(i+vec2(0.,1.)),hash(i+vec2(1.,1.)),f.x),f.y);
}
void fragment(){
 vec2 p=abs(world_normal.y)>0.7?world_pos.xz:vec2(abs(world_normal.x)>0.7?world_pos.z:world_pos.x,world_pos.y);
 float detail=.97+hash(floor(p*12.0))*.06;
 if(surface_kind==1){
  float row=floor(p.y/.25);
  vec2 cell=vec2((p.x+mod(row,2.0)*.3)/.6,p.y/.25);
  vec2 brick=fract(cell);
  vec2 edge=min(brick,1.0-brick);
  vec2 width=max(fwidth(cell),vec2(.001));
  vec2 line=1.0-smoothstep(vec2(.014,.025),vec2(.014,.025)+width,edge);
  float mortar=max(line.x,line.y);
  detail*=mix(.88+hash(floor(cell))*.15,.65,mortar);
 }
 if(surface_kind==3){
  vec2 cell=p*.8,edge=min(fract(cell),1.0-fract(cell));
  vec2 line=1.0-smoothstep(vec2(.008),vec2(.008)+max(fwidth(cell),vec2(.001)),edge);
  detail*=mix(.96,.72,max(line.x,line.y));
 }
 if(surface_kind==5){detail*=.94+.06*sin(p.x*70.+sin(p.y*3.)*2.);}
 vec3 base=tint.rgb*detail*COLOR.rgb;
 // Grime: dirt gathers near the ground, rain streaks run down walls, blotches break up flat colour.
 float vertical=1.0-smoothstep(.45,.9,abs(world_normal.y));
 float ground_dirt=(1.0-smoothstep(.0,3.2,world_pos.y))*vertical;
 float streak=vnoise(vec2((abs(world_normal.x)>.7?world_pos.z:world_pos.x)*2.3,world_pos.y*.22));
 float blotch=vnoise(p*.17)*.6+vnoise(p*.9)*.4;
 float stain=vnoise(p*.45+7.3);
 float dirt=ground_dirt*(.18+.24*blotch)+vertical*smoothstep(.55,.9,streak)*.16+smoothstep(.62,.88,stain)*.14;
 base*=1.0-clamp(dirt,0.,.5);
 base*=.9+.1*blotch;
 float luma=dot(base,vec3(.299,.587,.114));
 base=mix(vec3(luma),base,.82);
 ALBEDO=base;
 if(surface_kind==4){ALBEDO*=mix(1.,.55,wetness);ROUGHNESS=mix(.92,.2,wetness);}else{ROUGHNESS=.92;}
}
