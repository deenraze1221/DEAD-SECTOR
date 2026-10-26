extends Node3D
var player
var rig: Node3D
var legs: Array=[]
var materials: Array=[]
var alpha=0.0

func setup(owner_player, bridge) -> void:
 player=owner_player
 rig=bridge.creature_node(bridge.data.local_soldier)
 add_child(rig)
 rig.position.z=.18
 collect(rig)

func collect(node: Node) -> void:
 if node.name in ["left-hip","right-hip"]: legs.append(node)
 if node is MeshInstance3D:
  # Separate body alpha from shared city/weapon materials.
  node.mesh=node.mesh.duplicate()
  for i in range(node.mesh.get_surface_count()):
   var material=node.mesh.surface_get_material(i).duplicate()
   if material is StandardMaterial3D:
    material.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA_HASH
    node.mesh.surface_set_material(i,material)
    materials.append(material)
 for child in node.get_children(): collect(child)

func _process(delta: float) -> void:
 if player==null: return
 var target=clampf((-player.camera.rotation.x-.85)/.2,0,.85) if player.active else 0.0
 alpha=lerpf(alpha,target,1-exp(-delta*12))
 visible=alpha>.005
 for material in materials: material.albedo_color.a=alpha
 var speed=Vector2(player.get_real_velocity().x,player.get_real_velocity().z).length()
 var weight=clampf(speed/4.5,0,1) if player.is_on_floor() else 0.0
 var forward=-player.velocity.dot(player.global_basis.z)
 var direction=-1 if forward<-.1 else 1
 for i in range(legs.size()):
  var angle=.2+sin(player.distance_walked*TAU/1.65+i*PI)*.24*weight*direction
  legs[i].rotation.x=angle
  legs[i].position.y=.1175+.74*cos(angle)
  var knee=legs[i].get_node_or_null("knee")
  if knee: knee.rotation.x=0
  var foot=legs[i].find_child("foot",true,false)
  if foot: foot.rotation.x=-angle
