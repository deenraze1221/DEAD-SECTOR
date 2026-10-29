extends Node3D
var main
var lamps: Array=[]
var lights: Array=[]
var pools: MultiMeshInstance3D
var ash: CPUParticles3D
var time=0.0
var applied_cut=-2
var mist: Array=[]
var mist_material: ShaderMaterial
var mist_scroll=0.0

func haze_texture() -> ImageTexture:
 var image=Image.create(64,64,false,Image.FORMAT_RGBA8)
 for y in range(64):
  for x in range(64):
   var distance=Vector2(x-31.5,y-31.5).length()/31.5
   image.set_pixel(x,y,Color(1,1,1,pow(maxf(0,1-distance),2)))
 return ImageTexture.create_from_image(image)

func _ready() -> void:
 var texture=haze_texture()
 for asset in main.bridge.data.source_assets:
  if asset.type=="lamp": lamps.append(Vector3(asset.x,0,asset.z))
 pools=MultiMeshInstance3D.new()
 var mesh=PlaneMesh.new()
 mesh.size=Vector2(5.4,5.4)
 var material=StandardMaterial3D.new()
 material.albedo_texture=texture
 material.albedo_color=Color(1,.85,.6,.2)
 material.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA
 material.blend_mode=BaseMaterial3D.BLEND_MODE_ADD
 material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 material.depth_draw_mode=BaseMaterial3D.DEPTH_DRAW_DISABLED
 mesh.material=material
 var multi=MultiMesh.new()
 multi.transform_format=MultiMesh.TRANSFORM_3D
 multi.mesh=mesh
 multi.instance_count=lamps.size()
 for i in range(lamps.size()): multi.set_instance_transform(i,Transform3D(Basis(),lamps[i]+Vector3(0,.035,0)))
 pools.multimesh=multi
 pools.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
 add_child(pools)
 for i in range(8):
  var light=OmniLight3D.new()
  light.light_cull_mask=1
  light.light_color=Color("#d4bd86")
  light.omni_range=11
  light.light_energy=1.0
  light.visible=false
  add_child(light)
  lights.append(light)
 var fog_material=StandardMaterial3D.new()
 fog_material.albedo_texture=texture
 fog_material.albedo_color=Color(.55,.65,.57,.16)
 fog_material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 fog_material.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA
 fog_material.billboard_mode=BaseMaterial3D.BILLBOARD_ENABLED
 var quad=QuadMesh.new()
 quad.size=Vector2(4,2)
 quad.material=fog_material
 for hive in main.bridge.data.hives:
  var fog=CPUParticles3D.new()
  fog.amount=18
  fog.lifetime=6
  fog.local_coords=false
  fog.emission_shape=CPUParticles3D.EMISSION_SHAPE_BOX
  fog.emission_box_extents=Vector3(4,.4,4)
  fog.mesh=quad
  fog.initial_velocity_min=.1
  fog.initial_velocity_max=.3
  fog.direction=Vector3.UP
  fog.gravity=Vector3(0,.015,0)
  add_child(fog)
  fog.position=Vector3(hive.x,1,hive.z)
 # Low ground mist: two scrolling noise sheets that follow the player and take the weather fog colour.
 var noise=FastNoiseLite.new()
 noise.noise_type=FastNoiseLite.TYPE_SIMPLEX_SMOOTH
 noise.frequency=.012
 noise.fractal_octaves=3
 var noise_texture=NoiseTexture2D.new()
 noise_texture.noise=noise
 noise_texture.width=256
 noise_texture.height=256
 noise_texture.seamless=true
 mist_material=ShaderMaterial.new()
 mist_material.shader=load("res://shaders/mist.gdshader")
 mist_material.set_shader_parameter("noise_texture",noise_texture)
 for height in [.3,1.15]:
  var sheet=MeshInstance3D.new()
  var plane=PlaneMesh.new()
  plane.size=Vector2(90,90)
  sheet.mesh=plane
  sheet.material_override=mist_material
  sheet.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
  sheet.position.y=height
  add_child(sheet)
  mist.append(sheet)
 ash=CPUParticles3D.new()
 ash.amount=100
 ash.local_coords=false
 ash.lifetime=10
 ash.emission_shape=CPUParticles3D.EMISSION_SHAPE_BOX
 ash.emission_box_extents=Vector3(22,5,22)
 ash.gravity=Vector3(.01,-.025,0)
 ash.initial_velocity_min=0
 ash.initial_velocity_max=.08
 var dust=BoxMesh.new()
 dust.size=Vector3(.02,.02,.02)
 var dust_material=StandardMaterial3D.new()
 dust_material.albedo_color=Color(.65,.7,.61,.3)
 dust_material.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA
 dust_material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 dust.material=dust_material
 ash.mesh=dust
 add_child(ash)
 ash.position=main.player.position+Vector3.UP*4

func _process(delta: float) -> void:
 if not main.player.active: return
 time-=delta
 pools.visible=main.graphics_quality>0
 var night=main.climate!=null and main.climate.night
 mist_scroll+=delta
 var fog_amount={"light":.10,"medium":.17,"heavy":.30}[main.climate.fog] if main.climate!=null else .1
 if main.climate!=null and main.climate.kind=="clear": fog_amount*=.6
 var env=main.get_node("WorldEnvironment").environment
 mist_material.set_shader_parameter("scroll",mist_scroll)
 mist_material.set_shader_parameter("density",fog_amount*(.6 if night else 1.0))
 mist_material.set_shader_parameter("mist_color",env.fog_light_color.lightened(.12))
 for sheet in mist:
  sheet.visible=main.graphics_quality>0
  sheet.position.x=main.player.position.x
  sheet.position.z=main.player.position.z
 pools.multimesh.mesh.material.albedo_color=Color(1,.85,.6,.36 if night else .1)
 ash.emitting=main.graphics_quality>0
 if ash.position.distance_to(main.player.position+Vector3.UP*4)>12: ash.position=main.player.position+Vector3.UP*4
 var cut=main.battle.blackout_district()
 if cut!=applied_cut:
  applied_cut=cut
  for i in range(lamps.size()):
   var live=main.battle.district_of(lamps[i].x,lamps[i].z)!=cut
   pools.multimesh.set_instance_transform(i,Transform3D(Basis().scaled(Vector3.ONE if live else Vector3.ZERO),lamps[i]+Vector3(0,.035,0)))
 if time>0: return
 time=.5
 var nearest=lamps.filter(func(l): return main.battle.district_of(l.x,l.z)!=cut)
 nearest.sort_custom(func(a,b): return a.distance_squared_to(main.player.position)<b.distance_squared_to(main.player.position))
 var count=[0,4,8][main.graphics_quality]
 for i in range(lights.size()):
  lights[i].visible=i<count and i<nearest.size() and nearest[i].distance_to(main.player.position)<35
  if lights[i].visible:
   lights[i].position=nearest[i]+Vector3.UP*4.7
   lights[i].light_energy=2.6 if night else .9
