shader_type canvas_item;
uniform sampler2D screen_texture : hint_screen_texture, filter_nearest;
uniform bool low_health=false;
void fragment(){
 vec2 resolution=1.0/SCREEN_PIXEL_SIZE;
 vec2 grid=resolution*min(1.0,540.0/resolution.y);
 vec2 uv=(floor(SCREEN_UV*grid)+.5)/grid;
 vec3 color=texture(screen_texture,uv).rgb;
 float luminance=dot(color,vec3(.2126,.7152,.0722));
 color=mix(vec3(luminance),color,.88)*vec3(.89,1.0,.84);
 float vignette=1.0-.23*smoothstep(.2,.72,length(SCREEN_UV-.5));
 color*=1.0-.025*step(.5,fract(SCREEN_UV.y*resolution.y*.5));
 if(low_health)color=mix(color,vec3(.5,.025,.015),smoothstep(.26,.68,length(SCREEN_UV-.5))*.55);
 COLOR=vec4(color*vignette,1.);
}
