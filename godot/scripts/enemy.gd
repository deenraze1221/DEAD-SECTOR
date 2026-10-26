extends CharacterBody3D
var battle
var type="zombie"
var stats: Dictionary
var skin=0
var hp=0
var rig: Node3D
var hips: Array=[]
var arms: Array=[]
var path=PackedVector2Array()
var repath=0.0
var cooldown=1.0
var windup=0.0
var phase=0.0
var stuck=0.0
var voice=3.0
var dead=false
var stagger=0.0
var warning: MeshInstance3D

func _ready() -> void:
 hp=int(stats.hp)
 collision_layer=4
 collision_mask=1|4|8
 var collision=CollisionShape3D.new()
 var shape=CapsuleShape3D.new()
 shape.radius=stats.radius
 shape.height=maxf(stats.height,stats.radius*2)
 collision.shape=shape
 collision.position.y=shape.height/2 if type!="eye" else 0
 add_child(collision)
 rig=battle.main.bridge.creature_node(battle.main.bridge.data.creatures[type][skin])
 rig.scale*=float(stats.scale)
 add_child(rig)
 find_joints(rig)
 if type=="boss":
  warning=MeshInstance3D.new()
  var ring= TorusMesh.new()
  ring.inner_radius=3.1
  ring.outer_radius=3.3
  warning.mesh=ring
  var material=StandardMaterial3D.new()
  material.albedo_color=Color("#c75b35")
  material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
  warning.material_override=material
  warning.position.y=.06
  add_child(warning)
  warning.hide()

func find_joints(node: Node) -> void:
 if "hip" in str(node.name): hips.append(node)
 if "shoulder" in str(node.name): arms.append(node)
 for child in node.get_children(): find_joints(child)

func take_damage(amount: int) -> void:
 if dead: return
 hp-=amount
 stagger=.12
 repath=0
 if hp<=0:
  dead=true
  battle.killed(self)
  queue_free()

func _physics_process(delta: float) -> void:
 if dead or not battle.main.player.active: return
 var player=battle.main.player
 if player.health<=0: return
 cooldown-=delta
 repath-=delta
 voice-=delta
 if voice<=0:
  voice=4+battle.rng.randf()*4
  if position.distance_to(player.position)<18: battle.tone(70+skin*7,.15)
 var target=player.position
 var distance=Vector2(position.x-target.x,position.z-target.z).length()
 var origin=position+Vector3(0,1.6 if type!="eye" else 0,0)
 var ray=PhysicsRayQueryParameters3D.create(origin,target+Vector3(0,1,0),1)
 var clear=get_world_3d().direct_space_state.intersect_ray(ray).is_empty()
 var ratio=float(hp)/float(stats.hp)
 var stage=1 if ratio>.65 else 2 if ratio>.3 else 3
 if warning!=null: warning.visible=windup>0 and stage>=2
 if windup>0:
  windup=maxf(0,windup-delta)
  rig.rotation.x=-.12*sin(windup*8)
  if windup==0:
   rig.rotation.x=0
   if type=="vomiter":
    if distance<=8 and clear: battle.projectile(self,true)
    cooldown=battle.rng.randf_range(1,2)
   elif type=="eye":
    if clear: battle.projectile(self,false)
    cooldown=2.2
   else:
    var reach=stats.radius+(3 if type=="boss" and stage>=2 else 1)
    if distance<reach and clear: player.hurt(int(stats.damage))
    cooldown=1.4 if type=="boss" and stage==3 else 1.8 if type=="boss" else 1.05
 var can_attack=distance<stats.radius+1
 if type=="vomiter": can_attack=distance<=8
 if type=="eye": can_attack=distance<=40
 if can_attack and clear and cooldown<=0 and windup<=0:
  windup=.25 if type=="vomiter" else .55 if type=="eye" else 1.2 if type=="boss" and stage>=2 else .6 if type=="brute" else .18
  cooldown=windup+1
 var pursuit=target
 if distance>180:
  pursuit=Vector3(battle.main.bridge.data.terminal.x,0,battle.main.bridge.data.terminal.z)
 elif type in ["zombie","runner"] and distance>8 and distance<28:
  var side=float(get_instance_id()%7)*TAU/7
  pursuit+=Vector3(cos(side),0,sin(side))*3
 stagger=maxf(0,stagger-delta)
 var speed=float(stats.speed)*(1+(stage-1)*.2 if type=="boss" else 1)
 var direction=Vector3.ZERO
 if type=="eye":
  var difference=target-position
  difference.y=0
  direction=difference.normalized()*(1 if distance>20 else -1 if distance<12 else 0)
  if not clear:
   if repath<=0: path=battle.route(position,pursuit,type);repath=1
   while not path.is_empty() and Vector2(position.x,position.z).distance_to(path[0])<.7: path.remove_at(0)
   if not path.is_empty(): direction=Vector3(path[0].x-position.x,0,path[0].y-position.z).normalized()
  velocity.y=(4.5-position.y)*2
 else:
  if repath<=0:
   path=battle.route(position,pursuit,type)
   repath=.8+battle.rng.randf()*.3
  while not path.is_empty() and Vector2(position.x,position.z).distance_to(path[0])<.6: path.remove_at(0)
  if not path.is_empty(): direction=Vector3(path[0].x-position.x,0,path[0].y-position.z).normalized()
  elif distance>stats.radius+.8: direction=position.direction_to(target);direction.y=0
  if is_on_floor():
   velocity.y=-.1
   if target.y>position.y+.5 and distance<8 or stuck>.8: velocity.y=7;stuck=0
  else: velocity.y-=20*delta
 if windup>0 or can_attack and clear or stagger>0: direction=Vector3.ZERO
 var old=position
 velocity.x=direction.x*speed
 velocity.z=direction.z*speed
 move_and_slide()
 if direction.length()>.1 and position.distance_to(old)<speed*delta*.12: stuck+=delta
 else: stuck=0
 if distance>.1: rotation.y=atan2(target.x-position.x,target.z-position.z)
 phase+=position.distance_to(old)*5
 for i in range(hips.size()): hips[i].rotation.x=sin(phase+i*PI)*.4
 for i in range(arms.size()): arms[i].rotation.x=-.4+(sin(windup*6)*.7 if windup>0 else sin(phase+i*PI)*.12)
