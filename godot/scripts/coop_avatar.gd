extends Node3D
var main
var id=""
var health=100
var held=false
var pose: Dictionary={}
var rig: Node3D
var upper: Node3D
var mount: Node3D
var head: Node3D
var legs: Array=[]
var phase=0.0
var last_position=Vector3.ZERO
var age=0.0
var weapon=""
var beam: SpotLight3D
var label: Label3D
var hand_parts: Array=[]
var kick=0.0

func setup(game,identity: String) -> void:
 main=game
 id=identity
 rig=main.bridge.creature_node(main.bridge.data.soldier)
 add_child(rig)
 upper=rig.find_child("upper",true,false)
 head=rig.find_child("head",true,false)
 mount=rig.find_child("weapon-mount",true,false)
 if mount==null:
  mount=Node3D.new();mount.position=Vector3(.08,-.08,-.38);upper.add_child(mount)
 for name_value in ["left-hip","right-hip"]:
  var hip=rig.find_child(name_value,true,false)
  if hip!=null: legs.append(hip)
 for name_value in ["left-upper-arm","right-upper-arm","left-forearm","right-forearm"]:
  var arm=rig.find_child(name_value,true,false)
  if arm: arm.get_parent().remove_child(arm);arm.queue_free()
 beam=SpotLight3D.new()
 beam.light_energy=2.5
 beam.spot_range=30
 beam.spot_angle=24
 beam.position=Vector3(.1,0,-.2)
 head.add_child(beam)
 label=Label3D.new()
 label.text="SQUAD "+id.to_upper()
 label.position.y=2.15
 label.font=load("res://assets/terminal.ttf")
 label.font_size=24
 label.pixel_size=.003
 label.billboard=BaseMaterial3D.BILLBOARD_ENABLED
 label.no_depth_test=false
 add_child(label)
 equip("Pistol")

func equip(value: String) -> void:
 if value!="trap" and not main.bridge.data.weapons.has(value): value="Pistol"
 if weapon==value: return
 weapon=value
 for part in mount.get_children(): mount.remove_child(part);part.queue_free()
 for part in hand_parts: part.queue_free()
 hand_parts=[]
 if value=="trap":
  var model=main.bridge.creature_node(main.bridge.data.mission_models.trap)
  model.scale=Vector3.ONE*.55;model.rotation.x=.7;mount.add_child(model)
  main.bridge.build_hands(mount,{"support":[-.18,.03,.02],"trigger":[.18,.03,.02]})
 else: main.bridge.build_weapon(mount,value)
 for name_value in ["SupportForearm","TriggerForearm"]:
  var arm=mount.get_node_or_null(name_value)
  if arm: mount.remove_child(arm);arm.queue_free()
 var grips={"support":[-.18,.03,.02],"trigger":[.18,.03,.02]} if value=="trap" else main.bridge.data.weapons[value].grips
 for side in [-1,1]:
  var g=grips.support if side<0 else grips.trigger
  var wrist=Vector3(g[0],g[1],g[2])+mount.position+Vector3(0,0,.04)
  var shoulder=Vector3(side*.24,.08,0)
  var elbow=Vector3(side*.34,-.20,-.08)
  for points in [[shoulder,elbow],[elbow,wrist]]:
   var arm=MeshInstance3D.new()
   var mesh=CylinderMesh.new()
   mesh.top_radius=.045
   mesh.bottom_radius=.062
   mesh.height=points[0].distance_to(points[1])
   var mat=StandardMaterial3D.new()
   mat.albedo_color=Color("#59674f")
   mesh.material=mat
   arm.mesh=mesh
   arm.position=(points[0]+points[1])*.5
   arm.quaternion=Quaternion(Vector3.UP,points[0].direction_to(points[1]))
   upper.add_child(arm)
   hand_parts.append(arm)
 set_layers(mount)

func set_layers(node: Node) -> void:
 if node is GeometryInstance3D: node.layers=1;node.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_ON
 for child in node.get_children(): set_layers(child)

func update_pose(data: Dictionary) -> void:
 var fresh=pose.is_empty()
 pose=data
 health=int(data.get("hp",100))
 held=bool(data.get("held",false))
 age=0
 equip(str(data.get("weapon","Pistol")))
 if fresh: position=Vector3(data.p[0],data.p[1],data.p[2]);last_position=position

func hurt(amount: int) -> void:
 if health<=0: return
 health=maxi(0,health-amount)
 main.net.send({"type":"damage","target":id,"amount":amount})

func _process(delta: float) -> void:
 if pose.is_empty(): return
 kick=move_toward(kick,0,delta*1.8)
 mount.rotation.x=kick
 age+=delta
 var target=Vector3(pose.p[0],pose.p[1],pose.p[2])
 position=position.lerp(target,1-exp(-delta*14))
 rotation.y=lerp_angle(rotation.y,float(pose.yaw),1-exp(-delta*14))
 var speed=position.distance_to(last_position)/maxf(delta,.001) if age<.5 else 0.0
 last_position=position
 phase+=minf(9,speed)*delta*2.8
 var weight=clampf(speed/3,0,1) if health>0 else 0.0
 for i in range(legs.size()):
  legs[i].rotation.x=sin(phase+i*PI)*.42*weight
  var knee=legs[i].get_node_or_null("knee")
  if knee: knee.rotation.x=0
 upper.rotation.x=clampf(float(pose.get("pitch",0)),-1.1,1.1)
 head.rotation.x=upper.rotation.x
 rig.rotation.z=lerpf(rig.rotation.z,PI/2 if health<=0 else 0.0,1-exp(-delta*8))
 label.modulate=Color("#d5b89a") if health<=0 else Color("#bbd9c9")
 beam.visible=bool(pose.get("light",false)) and health>0
