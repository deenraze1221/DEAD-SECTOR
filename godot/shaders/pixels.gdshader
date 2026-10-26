shader_type canvas_item;
uniform sampler2D screen_texture : hint_screen_texture, filter_nearest;
void fragment(){
 vec2 resolution=1.0/SCREEN_PIXEL_SIZE;
 float size=max(1.0,floor(resolution.y/540.0));
 vec2 uv=(floor(SCREEN_UV*resolution/size)+.5)*size/resolution;
 vec3 color=texture(screen_texture,uv).rgb;
 float vignette=1.0-.16*smoothstep(.25,.75,length(SCREEN_UV-.5));
 COLOR=vec4(color*vignette,1.);
}
