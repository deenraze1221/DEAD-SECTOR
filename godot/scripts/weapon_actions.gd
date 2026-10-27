extends Node3D
var player
var gun_id=""
var flash_mesh: MeshInstance3D
var casing_count=0
var base_hands: Dictionary={}
var profiles={
 "Pistol":{"action":"slide","kick":.075,"load":1},
 "Helsing":{"action":"rack","kick":.035,"load":4},
 "Tikhar":{"action":"bolt","kick":.065,"load":1},
 "Shambler":{"action":"drum","kick":.085,"load":5},
 "Shotgun":{"action":"break","kick":.12,"load":2},
 "Kalash":{"action":"rack","kick":.065,"load":2},
 "VSV":{"action":"bolt","kick":.075,"load":2},
 "Scorpion":{"action":"rack","kick":.042,"load":2},
 "Vector":{"action":"rack","kick":.032,"load":2},
 "G36":{"action":"rack","kick":.055,"load":2}
}
func setup(owner_player) -> void: player=owner_player

func bind() -> void:
 gun_id=player.config.id
 base_hands={}
 for name in ["SupportHand","TriggerHand"]:
  var hand=player.gun.get_node_or_null(name)
  if hand: base_hands[name]=hand.position
 var mesh=MeshInstance3D.new()
 mesh.name="MuzzleFlash"
 var cone=CylinderMesh.new()
 cone.top_radius=0
 cone.bottom_radius=.055
 cone.height=.16
 cone.radial_segments=5
 mesh.mesh=cone
 var material=StandardMaterial3D.new()
 material.albedo_color=Color("#ffe790")
 material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 mesh.material_override=material
 mesh.layers=2
 mesh.rotation.x=PI/2
 mesh.position=Vector3(0,-.014,-length()*.5-.06)
 player.gun.add_child(mesh)
 flash_mesh=mesh
 flash_mesh.hide()

func length() -> float:
 var part=player.get_parent().bridge.data.weapons[player.config.id].parts[0]
 var positions=player.get_parent().bridge.data.geometries[int(part.geometry)].position
 var min_z=INF
 var max_z=-INF
 for i in range(2,positions.size(),3): min_z=minf(min_z,positions[i]);max_z=maxf(max_z,positions[i])
 return max_z-min_z

func _process(_delta: float) -> void:
 if player==null: return
 if player.config.id!=gun_id or not is_instance_valid(flash_mesh): bind()
 flash_mesh.visible=player.flash.light_energy>1 and player.active
 flash_mesh.rotation.z=randf()*TAU
 var progress=1-player.reloading/float(player.config.reload) if player.reloading>0 else 0.0
 var drop=sin(progress*PI) if player.reloading>0 else 0.0
 var action=profiles[player.config.id].action
 var magazine=player.gun.get_node_or_null("WeaponMagazine")
 if magazine:
  magazine.position.y=-drop*.22
  magazine.rotation.x=drop*.2
 var body=player.gun.get_node_or_null("WeaponBody")
 if body:
  body.rotation.x=drop*.45 if action=="break" else 0.0
  body.rotation.z=-drop*.25 if action=="drum" else 0.0
 for name in base_hands:
  var hand=player.gun.get_node_or_null(name)
  if not hand: continue
  hand.position=base_hands[name]
  if name=="SupportHand":
   hand.position+=Vector3(-drop*.04,-drop*.09,drop*.10)
   if action in ["bolt","rack"]: hand.position.z+=sin(progress*TAU*2)*drop*.04
  var arm=player.gun.get_node_or_null("SupportForearm" if name=="SupportHand" else "TriggerForearm")
  if arm:
   var elbow=Vector3(-.29 if name=="SupportHand" else .23,-.28,.27)
   var delta=hand.position-elbow
   arm.position=(hand.position+elbow)*.5
   arm.quaternion=Quaternion(Vector3.UP,delta.normalized())
   arm.scale.y=delta.length()/arm.mesh.height

func fired() -> void:
 if casing_count>=24: return
 casing_count+=1
 var casing=RigidBody3D.new()
 casing.collision_layer=16
 casing.collision_mask=1
 casing.mass=.01
 var mesh=MeshInstance3D.new()
 var cylinder=CylinderMesh.new()
 cylinder.top_radius=.006
 cylinder.bottom_radius=.006
 cylinder.height=.025
 cylinder.radial_segments=6
 mesh.mesh=cylinder
 var material=StandardMaterial3D.new()
 material.albedo_color=Color("#b99b55")
 material.metallic=.5
 mesh.material_override=material
 casing.add_child(mesh)
 var shape=CollisionShape3D.new()
 var box=BoxShape3D.new()
 box.size=Vector3(.012,.025,.012)
 shape.shape=box
 casing.add_child(shape)
 player.get_parent().add_child(casing)
 casing.position=player.camera.global_position-player.camera.global_basis.z*.35+player.camera.global_basis.x*.15
 casing.linear_velocity=player.camera.global_basis.x*1.2+Vector3.UP*.8+player.velocity*.2
 casing.angular_velocity=Vector3(8,6,3)
 get_tree().create_timer(.9).timeout.connect(func(): if is_instance_valid(casing): casing.queue_free();casing_count-=1)
