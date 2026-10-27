extends Node3D
var main
var traces: Array=[]
var splashes: CPUParticles3D
var timer=0.0

func _ready() -> void:
 # World-space rain impacts, restricted to exposed ground around the player.
 splashes=CPUParticles3D.new()
 splashes.amount=40
 splashes.lifetime=.35
 splashes.local_coords=false
 splashes.direction=Vector3.UP
 splashes.spread=45
 splashes.initial_velocity_min=.3
 splashes.initial_velocity_max=1.2
 splashes.gravity=Vector3(0,-8,0)
 splashes.emission_shape=CPUParticles3D.EMISSION_SHAPE_BOX
 splashes.emission_box_extents=Vector3(4,.01,4)
 var mesh=BoxMesh.new();mesh.size=Vector3(.018,.035,.018)
 var material=StandardMaterial3D.new()
 material.albedo_color=Color("#8ea7a4")
 mesh.material=material
 splashes.mesh=mesh
 add_child(splashes)
 splashes.emitting=false

func tracer(from: Vector3,to: Vector3) -> void:
 if main.graphics_quality==0 or from.distance_to(to)<.05: return
 var mesh=MeshInstance3D.new()
 var beam=CylinderMesh.new()
 beam.top_radius=.004;beam.bottom_radius=.008
 beam.height=from.distance_to(to)
 beam.radial_segments=4
 var material=StandardMaterial3D.new()
 material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 material.albedo_color=Color(1,.76,.36,.55)
 material.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA
 beam.material=material
 mesh.mesh=beam
 mesh.position=(from+to)*.5
 mesh.quaternion=Quaternion(Vector3.UP,from.direction_to(to))
 mesh.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
 add_child(mesh)
 traces.append({"node":mesh,"life":.065})
 if traces.size()>24: traces.pop_front().node.queue_free()

func trail(parent: Node3D,green: bool) -> void:
 var particles=CPUParticles3D.new()
 particles.amount=14
 particles.lifetime=.28
 particles.local_coords=false
 particles.direction=Vector3.UP
 particles.spread=180
 particles.initial_velocity_min=.05
 particles.initial_velocity_max=.3
 particles.gravity=Vector3(0,-.3,0)
 particles.scale_amount_min=.03
 particles.scale_amount_max=.10
 var mesh=SphereMesh.new();mesh.radius=.5;mesh.height=1;mesh.radial_segments=6;mesh.rings=3
 var mat=StandardMaterial3D.new()
 mat.albedo_color=Color("#7b9b31") if green else Color("#b75c38")
 mat.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 mesh.material=mat
 particles.mesh=mesh
 parent.add_child(particles)

func _process(delta: float) -> void:
 for trace in traces.duplicate():
  trace.life-=delta
  if trace.life<=0: trace.node.queue_free();traces.erase(trace)
 timer-=delta
 if timer>0: return
 timer=.2
 splashes.emitting=false
 if not main.world_running() or main.graphics_quality==0 or main.climate.kind not in ["rain","storm"]: return
 var p=main.player.position
 var key=Vector2i(roundi((p.x+108)/24),roundi((p.z+108)/24))
 if main.climate.roof_cells.has(key):
  var building=main.climate.roof_cells[key]
  if absf(p.x-building.x)<building.w/2+4 and absf(p.z-building.z)<building.d/2+4: return
 splashes.position=Vector3(p.x,.04,p.z)
 splashes.emitting=true
