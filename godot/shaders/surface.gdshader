shader_type spatial;
uniform vec4 tint : source_color = vec4(0.5);
uniform int surface_kind = 0;
varying vec3 world_pos;
varying vec3 world_normal;
void vertex(){
 world_pos=(MODEL_MATRIX*vec4(VERTEX,1.0)).xyz;
 world_normal=normalize(MODEL_NORMAL_MATRIX*NORMAL);
}
float hash(vec2 p){return fract(sin(dot(p,vec2(12.9898,78.233)))*43758.5453);}
void fragment(){
 vec2 p=abs(world_normal.y)>0.7?world_pos.xz:vec2(abs(world_normal.x)>0.7?world_pos.z:world_pos.x,world_pos.y);
 float detail=0.94+hash(floor(p*42.0))*0.12;
 if(surface_kind==1){
  float row=floor(p.y/.25);
  vec2 brick=vec2(fract((p.x+mod(row,2.0)*.3)/.6),fract(p.y/.25));
  float mortar=step(brick.x,.025)+step(brick.y,.055);
  detail*=mix(.92+hash(vec2(floor((p.x+mod(row,2.0)*.3)/.6),row))*.16,.5,clamp(mortar,0.,1.));
 }
 if(surface_kind==3){vec2 tile=fract(p*.8);detail*=mix(.95,.6,step(tile.x,.015)+step(tile.y,.015));}
 if(surface_kind==5){detail*=.9+.1*sin(p.x*95.+sin(p.y*3.)*2.);}
 ALBEDO=tint.rgb*detail;
 ROUGHNESS=.92;
}
