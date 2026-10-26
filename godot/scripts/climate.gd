extends Node3D
var main
var kind="overcast"
var fog="light"
var night=false
var hour=6.0
var wave=0
var rain: MultiMeshInstance3D
var drops: Array[Vector3]=[]
var rng=RandomNumberGenerator.new()
var tick=0.0
var wetness=0.0
var storm_timer=8.0
var flash_timer=0.0
var thunder_timer=-1.0
var flash: DirectionalLight3D
var weather_materials: Array=[]
var roof_cells: Dictionary={}

static func selection(seed_value: int, number: int) -> Dictionary:
 var state=(seed_value^((number*0x9e3779b1)&0xffffffff))&0xffffffff
 state=(state*1664525+1013904223)&0xffffffff
 var a=float(state)/4294967296
 state=(state*1664525+1013904223)&0xffffffff
 var b=float(state)/4294967296
 return {"weather":"overcast" if a<.55 else "rain" if a<.75 else "storm" if a<.9 else "clear","fog":"light" if b<.6 else "medium" if b<.9 else "heavy"}

func _ready() -> void:
 rng.seed=int(main.generation.seed)
 for building in main.bridge.data.buildings:
  roof_cells[Vector2i(roundi((building.cx+108)/24),roundi((building.cz+108)/24))]=building
 rain=MultiMeshInstance3D.new()
 var multi=MultiMesh.new()
 multi.transform_format=MultiMesh.TRANSFORM_3D
 var mesh=BoxMesh.new()
 mesh.size=Vector3(.012,.45,.012)
 var material=StandardMaterial3D.new()
 material.albedo_color=Color(.55,.7,.72,.45)
 material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 material.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA
 mesh.material=material
 multi.mesh=mesh
 multi.instance_count=650
 rain.multimesh=multi
 rain.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
 add_child(rain)
 for i in range(650): drops.append(main.player.position+Vector3(rng.randf_range(-24,24),rng.randf_range(2,20),rng.randf_range(-24,24)))
 flash=DirectionalLight3D.new()
 flash.light_color=Color("#d5e7f1")
 flash.light_cull_mask=1
 flash.light_energy=0
 flash.rotation_degrees=Vector3(-70,20,0)
 add_child(flash)
 for material_record in main.bridge.materials:
  if material_record is ShaderMaterial: weather_materials.append(material_record)
 roll(1)

func roll(number: int) -> void:
 wave=number
 var state=selection(int(main.generation.seed),number)
 kind=state.weather
 fog=state.fog
 storm_timer=rng.randf_range(8,16)

func _process(delta: float) -> void:
 if not main.player.active:
  flash.light_energy=0
  return
 if wave!=main.battle.number: roll(main.battle.number)
 var phase=(wave-1)%4
 night=phase>=2
 var progress=0.0 if main.battle.phase=="PREPARATION" else 1.0 if main.battle.phase in ["CLEARANCE","INTERMISSION"] else minf(1,main.battle.time/120)
 hour=fposmod(6+phase*6+progress*6,24)
 # Dawn/dusk blend according to the same clock; two day waves, two night waves.
 var daylight=clampf(sin((hour-6)/24*TAU)*1.4,0,1)
 var env=main.get_node("WorldEnvironment").environment
 var sky=env.sky.sky_material
 var day_color=Color("#58665b") if kind=="clear" else Color("#334b43") if kind=="overcast" else Color("#233933")
 var color=Color("#09130f").lerp(day_color,daylight)
 env.fog_light_color=color
 env.fog_density={"light":.009,"medium":.016,"heavy":.058}[fog]
 env.ambient_light_energy=lerpf(.16,.42,daylight)
 sky.sky_top_color=color*.75
 sky.sky_horizon_color=color
 sky.ground_horizon_color=color
 var sun=main.get_node("Sun")
 sun.rotation_degrees.x=-maxf(8,sin((hour-6)/24*TAU)*55)
 sun.light_energy=lerpf(.08,.85 if kind=="clear" else .5,daylight)
 wetness=move_toward(wetness,1.0 if kind in ["rain","storm"] else 0.0,delta*(.08 if kind in ["rain","storm"] else .012))
 for material in weather_materials: material.set_shader_parameter("wetness",wetness)
 flash_timer=maxf(0,flash_timer-delta)
 flash.light_energy=4 if flash_timer>0 else 0
 if thunder_timer>=0:
  thunder_timer-=delta
  if thunder_timer<=0: main.sound.play("thunder",null,main.weather_volume);thunder_timer=-1
 if kind=="storm":
  storm_timer-=delta
  if storm_timer<=0:
   flash_timer=.15
   thunder_timer=rng.randf_range(.6,2.4)
   storm_timer=rng.randf_range(7,17)
 rain.visible=kind in ["rain","storm"]
 tick+=delta
 if tick<1.0/30 or not rain.visible: return
 var elapsed=tick
 tick=0
 var count=[160,400,650][main.graphics_quality]
 rain.multimesh.visible_instance_count=count
 for i in range(count):
  var p=drops[i]
  p+=Vector3(-1,-15,.3)*elapsed
  if p.y<main.player.position.y-2 or Vector2(p.x-main.player.position.x,p.z-main.player.position.z).length()>27:
   p=main.player.position+Vector3(rng.randf_range(-24,24),rng.randf_range(10,20),rng.randf_range(-24,24))
  drops[i]=p
  var under_roof=false
  var key=Vector2i(roundi((p.x+108)/24),roundi((p.z+108)/24))
  if roof_cells.has(key):
   var building=roof_cells[key]
   under_roof=absf(p.x-building.x)<building.w/2 and absf(p.z-building.z)<building.d/2 and p.y<building.h
  rain.multimesh.set_instance_transform(i,Transform3D(Basis().scaled(Vector3.ZERO if under_roof else Vector3.ONE),p))
