extends Node3D
var main
var lamps: Array=[]
var lights: Array=[]
var pools: MultiMeshInstance3D
var ash: CPUParticles3D
var time=0.0

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
  light.light_energy=.85
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
 ash.emitting=main.graphics_quality>0
 if ash.position.distance_to(main.player.position+Vector3.UP*4)>12: ash.position=main.player.position+Vector3.UP*4
 if time>0: return
 time=.5
 var nearest=lamps.duplicate()
 nearest.sort_custom(func(a,b): return a.distance_squared_to(main.player.position)<b.distance_squared_to(main.player.position))
 var count=[0,4,8][main.graphics_quality]
 for i in range(lights.size()):
  lights[i].visible=i<count and i<nearest.size() and nearest[i].distance_to(main.player.position)<35
  if lights[i].visible: lights[i].position=nearest[i]+Vector3.UP*4.7
