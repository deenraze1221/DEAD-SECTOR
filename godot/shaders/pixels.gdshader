shader_type canvas_item;
uniform sampler2D screen_texture : hint_screen_texture, filter_nearest;
uniform bool low_health=false;
uniform float hurt=0.;
uniform float detail=.65;
void fragment(){
 vec2 resolution=1.0/SCREEN_PIXEL_SIZE;
 vec2 grid=resolution*min(1.0,540.0/resolution.y);
 vec2 uv=(floor(SCREEN_UV*grid)+.5)/grid;
 vec3 color=texture(screen_texture,uv).rgb;
 vec2 texel=1./grid;
 vec3 neighbors=(texture(screen_texture,uv+vec2(texel.x,0.)).rgb+texture(screen_texture,uv-vec2(texel.x,0.)).rgb+texture(screen_texture,uv+vec2(0.,texel.y)).rgb+texture(screen_texture,uv-vec2(0.,texel.y)).rgb)*.25;
 color=max(vec3(0.),color+(color-neighbors)*.07*detail);
 float luminance=dot(color,vec3(.2126,.7152,.0722));
 color=mix(vec3(luminance),color,.94)*vec3(.98,1.0,1.025);
 float vignette=1.0-.23*smoothstep(.2,.72,length(SCREEN_UV-.5));
 color*=1.0-.025*step(.5,fract(SCREEN_UV.y*resolution.y*.5));
 color=mix(color,vec3(.6,.025,.015),smoothstep(.28,.7,length(SCREEN_UV-.5))*hurt*.5);
 if(low_health)color=mix(color,vec3(.5,.025,.015),smoothstep(.26,.68,length(SCREEN_UV-.5))*.55);
 float grain=fract(sin(dot(FRAGCOORD.xy+floor(TIME*24.),vec2(12.9898,78.233)))*43758.5453)-.5;
 COLOR=vec4(color*vignette+grain*.004*detail,1.);
}
