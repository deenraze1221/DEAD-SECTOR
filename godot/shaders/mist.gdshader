shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, shadows_disabled;
uniform sampler2D noise_texture : repeat_enable, filter_linear;
uniform vec4 mist_color : source_color = vec4(.2,.26,.3,1.);
uniform float density = .16;
uniform float scroll = 0.;
varying vec3 world_pos;
void vertex(){ world_pos=(MODEL_MATRIX*vec4(VERTEX,1.)).xyz; }
void fragment(){
 vec2 uv=world_pos.xz*.045;
 float n=texture(noise_texture,uv+vec2(scroll*.012,scroll*.007)).r;
 float m=texture(noise_texture,uv*2.3-vec2(scroll*.009,0.)).r;
 float cloud=smoothstep(.38,.82,n*.65+m*.35);
 float reach=1.0-smoothstep(9.,38.,length(world_pos.xz-INV_VIEW_MATRIX[3].xz));
 ALBEDO=mist_color.rgb;
 ALPHA=cloud*reach*density;
}
