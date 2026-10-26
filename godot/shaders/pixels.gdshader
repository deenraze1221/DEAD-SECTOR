shader_type canvas_item;
uniform sampler2D screen_texture : hint_screen_texture, filter_nearest;
void fragment(){
 vec2 resolution=1.0/SCREEN_PIXEL_SIZE;
 vec2 grid=resolution*min(1.0,540.0/resolution.y);
 vec2 uv=(floor(SCREEN_UV*grid)+.5)/grid;
 vec3 color=texture(screen_texture,uv).rgb;
 float luminance=dot(color,vec3(.2126,.7152,.0722));
 color=mix(vec3(luminance),color,.88)*vec3(.95,.98,1.0);
 float vignette=1.0-.23*smoothstep(.2,.72,length(SCREEN_UV-.5));
 COLOR=vec4(color*vignette,1.);
}
