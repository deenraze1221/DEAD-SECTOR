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
var recoil_direction=Vector3.ZERO
var recoil_time=0.0
var impulse=Vector3.ZERO
var hearing_position=Vector3.ZERO
var hearing_time=0.0
var progress_time=0.0
var progress_anchor=Vector3.ZERO
var firing_goal=Vector3.ZERO
var attack_count=0
var attack_kind="melee"
var warning: MeshInstance3D

func _ready() -> void:
 set_meta("mutant",true)
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

func take_damage(amount: int,point=Vector3.INF,direction=Vector3.ZERO) -> void:
 if dead: return
 hp-=amount
 stagger=.06 if type=="boss" else .14
 recoil_time=.24
 recoil_direction=global_basis.inverse()*direction
 impulse=direction*minf(2.4,float(amount)*.025)/(2 if type in ["brute","boss"] else 1)
 impulse.y=0
 if point==Vector3.INF: point=position+Vector3(0,stats.height*.6,0)
 if hp>0: battle.burst(point,false,8,-direction+Vector3.UP*.5,true)
 repath=0
 if hp<=0:
  dead=true
  battle.killed(self)
  queue_free()

func _physics_process(delta: float) -> void:
 if dead or not battle.main.world_running(): return
 var player=battle.main.coop.nearest(position) if battle.main.coop!=null else battle.main.player
 if player.health<=0: return
 recoil_time=maxf(0,recoil_time-delta)
 impulse=impulse.move_toward(Vector3.ZERO,delta*12)
 cooldown-=delta
 repath-=delta
 voice-=delta
 if voice<=0:
  voice=4+battle.rng.randf()*4
  if position.distance_to(player.position)<18: battle.main.sound.play(type+"-living",position)
 var target=player.position
 var distance=Vector2(position.x-target.x,position.z-target.z).length()
 var origin=position+Vector3(0,1.6 if type!="eye" else 0,0)
 var ray=PhysicsRayQueryParameters3D.create(origin,target+Vector3(0,1,0),1)
 var clear=get_world_3d().direct_space_state.intersect_ray(ray).is_empty()
 var ratio=float(hp)/float(stats.hp)
 var stage=1 if ratio>.65 else 2 if ratio>.3 else 3
 if warning!=null: warning.visible=windup>0 and attack_kind=="slam"
 if warning!=null: warning.scale=Vector3.ONE*(4.5/3.2 if stage==3 else 1.0)
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
    var reach=(4.5 if stage==3 else 3.0) if attack_kind=="slam" else stats.radius+1
    if distance<reach and clear: player.hurt(int(stats.damage))
    cooldown=1.4 if type=="boss" and stage==3 else 1.8 if type=="boss" else 1.05
 var can_attack=distance<stats.radius+1
 if type=="vomiter": can_attack=distance<=8
 if type=="eye": can_attack=distance<=40
 if can_attack and clear and cooldown<=0 and windup<=0:
  attack_count+=1
  attack_kind="slam" if type=="boss" and stage>=2 and attack_count%3==0 else "melee"
  windup=.25 if type=="vomiter" else .55 if type=="eye" else 1.2 if attack_kind=="slam" else .9 if type=="boss" else .6 if type=="brute" else .18
  cooldown=windup+1
  if type=="vomiter": battle.main.sound.play("vomit-prepare",position)
 var pursuit=target
 if hearing_time>0: hearing_time=maxf(0,hearing_time-delta)
 if distance>180:
  pursuit=hearing_position if hearing_time>0 else Vector3(battle.main.bridge.data.terminal.x,0,battle.main.bridge.data.terminal.z)
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
   if repath<=0:
    firing_goal=pursuit
    for n in range(6):
     var angle=(n+get_instance_id()%6)*TAU/6
     var candidate=target+Vector3(cos(angle)*18,4.5-target.y,sin(angle)*18)
     if absf(candidate.x)>119 or absf(candidate.z)>119: continue
     var check_ray=PhysicsRayQueryParameters3D.create(candidate,target+Vector3.UP,1)
     if get_world_3d().direct_space_state.intersect_ray(check_ray).is_empty(): firing_goal=candidate;break
    path=battle.route(position,firing_goal,type)
    repath=1
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
 var hold=can_attack and clear and (type!="eye" or distance>=12 and distance<=24)
 if windup>0 or hold or stagger>0: direction=Vector3.ZERO
 var old=position
 velocity.x=direction.x*speed+impulse.x
 velocity.z=direction.z*speed+impulse.z
 move_and_slide()
 progress_time+=delta
 if progress_time>=3:
  if position.distance_to(progress_anchor)<.5: repath=0;path=PackedVector2Array();stuck=.9
  progress_anchor=position
  progress_time=0
 if direction.length()>.1 and position.distance_to(old)<speed*delta*.12: stuck+=delta
 else: stuck=0
 if distance>.1: rotation.y=atan2(target.x-position.x,target.z-position.z)
 rig.rotation.z=-recoil_direction.x*sin(recoil_time/.24*PI)*.14
 if windup<=0: rig.rotation.x=-absf(recoil_direction.z)*sin(recoil_time/.24*PI)*.12
 phase+=position.distance_to(old)*5
 for i in range(hips.size()): hips[i].rotation.x=sin(phase+i*PI)*.4
 for i in range(arms.size()): arms[i].rotation.x=-.4+(sin(windup*6)*.7 if windup>0 else sin(phase+i*PI)*.12)

func hear(point: Vector3) -> void:
 if position.distance_to(point)<90:
  hearing_position=point
  hearing_time=6
  repath=0
