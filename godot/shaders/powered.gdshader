shader_type spatial;
render_mode unshaded;
// Unlit lights and signs that go dark while their district has no power (HTML: "light" and "label" materials).
uniform vec4 tint : source_color = vec4(1.0);
uniform sampler2D label_texture : source_color, filter_nearest;
uniform bool use_texture = false;
uniform float power_cut = -1.0;
varying vec3 world_pos;
void vertex(){
 world_pos=(MODEL_MATRIX*vec4(VERTEX,1.0)).xyz;
}
void fragment(){
 vec4 sample_color=use_texture?texture(label_texture,UV):vec4(1.0);
 float quadrant=world_pos.x<0.?(world_pos.z<0.?0.:1.):(world_pos.z<0.?2.:3.);
 float power=abs(quadrant-power_cut)<.1?.06:1.0;
 ALBEDO=sample_color.rgb*tint.rgb*COLOR.rgb*power;
 if(use_texture){ALPHA=sample_color.a;ALPHA_SCISSOR_THRESHOLD=.5;}
}
