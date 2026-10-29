shader_type canvas_item;
uniform sampler2D screen_texture : hint_screen_texture, filter_nearest;
uniform bool low_health=false;
uniform float hurt=0.;
uniform float detail=.65;
uniform float rain=0.;
float h21(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
void fragment(){
 vec2 resolution=1.0/SCREEN_PIXEL_SIZE;
 vec2 grid=resolution*min(1.0,540.0/resolution.y);
 vec2 uv=(floor(SCREEN_UV*grid)+.5)/grid;
 float wet=0.;
 if(rain>.01){
  vec2 cells=vec2(16.,9.);
  for(int layer=0;layer<2;layer++){
   vec2 cuv=SCREEN_UV*cells*(layer==0?1.:1.9)+vec2(float(layer)*7.3,0.);
   vec2 id=floor(cuv);
   float seed=h21(id+float(layer)*31.);
   float age=fract(TIME*(.12+seed*.1)+seed*5.);
   vec2 centre=id+vec2(.25+.5*h21(id+1.7),.2+.5*h21(id+4.1));
   centre.y-=age*age*.55;
   vec2 delta=(cuv-centre)*vec2(1.,.8);
   float drop=smoothstep(.2+.08*seed,.0,length(delta))*smoothstep(.0,.15,age)*smoothstep(1.,.7,age)*step(seed,rain*.55);
   wet=max(wet,drop);
   uv-=normalize(delta+vec2(.0001))*drop*.012;
  }
 }
 vec3 color=texture(screen_texture,uv).rgb;
 vec2 texel=1./grid;
 vec3 neighbors=(texture(screen_texture,uv+vec2(texel.x,0.)).rgb+texture(screen_texture,uv-vec2(texel.x,0.)).rgb+texture(screen_texture,uv+vec2(0.,texel.y)).rgb+texture(screen_texture,uv-vec2(0.,texel.y)).rgb)*.25;
 color=max(vec3(0.),color+(color-neighbors)*.07*detail);
 color=mix(color,color*vec3(.92,1.02,1.08)+vec3(.012,.018,.022),wet*.8);
 float luminance=dot(color,vec3(.2126,.7152,.0722));
 color=mix(vec3(luminance),color,.9)*vec3(.97,1.0,1.02);
 // Grim grade: crush blacks, cold green-teal shadows, slightly warmer highlights, firmer contrast.
 float shade=smoothstep(.0,.55,luminance);
 color*=mix(vec3(.86,1.0,.99),vec3(1.03,1.0,.96),shade);
 color=pow(max(color,vec3(0.)),vec3(1.04));
 color=mix(color,color*color*(3.0-2.0*color),.2);
 float vignette=1.0-.36*smoothstep(.2,.8,length(SCREEN_UV-.5));
 color*=1.0-.025*step(.5,fract(SCREEN_UV.y*resolution.y*.5));
 color=mix(color,vec3(.6,.025,.015),smoothstep(.28,.7,length(SCREEN_UV-.5))*hurt*.5);
 if(low_health)color=mix(color,vec3(.5,.025,.015),smoothstep(.26,.68,length(SCREEN_UV-.5))*.55);
 float grain=fract(sin(dot(FRAGCOORD.xy+floor(TIME*24.),vec2(12.9898,78.233)))*43758.5453)-.5;
 COLOR=vec4(color*vignette+grain*.011*detail,1.);
}
