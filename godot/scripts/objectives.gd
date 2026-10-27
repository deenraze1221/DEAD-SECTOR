extends Node3D
var main
var elapsed=0.0
var rolled=0
var mission: Dictionary={}
var mission_model: Node3D
var drops: Array=[]
var plane: Node3D
var plane_sound: AudioStreamPlayer3D
var plane_rotors: Array=[]
var drop_index=0
var interaction=""
var altitude=50.0

static func hash32(value: int) -> int:
 value=((value^(value>>16))*0x7feb352d)&0xffffffff
 value=((value^(value>>15))*0x846ca68b)&0xffffffff
 return (value^(value>>16))&0xffffffff

static func roll(seed_value: int,wave: int) -> Dictionary:
 if wave<=2: return {}
 var h=hash32((seed_value^((wave*0x9e3779b1)&0xffffffff))&0xffffffff)
 if float(h)/4294967296>=.15: return {}
 return {"kind":["package","zone","generator","growth"][hash32(h^0x85ebca6b)%4],"reward":300+hash32(h^0xc2b2ae35)%301}

func _ready() -> void:
 for building in main.bridge.data.buildings: altitude=maxf(altitude,building.h+12)
 plane=main.bridge.creature_node(main.bridge.data.mission_models.plane)
 add_child(plane)
 plane.hide()
 for child in plane.get_children():
  if child.get_child_count()==2: plane_rotors.append(child)

func free_ground(point: Vector3,radius=.5) -> bool:
 if absf(point.x)>119 or absf(point.z)>119: return false
 for box in main.bridge.data.colliders:
  if box.minY<1.5 and box.maxY>0 and point.x>box.minX-radius and point.x<box.maxX+radius and point.z>box.minZ-radius and point.z<box.maxZ+radius: return false
 return true

func road_point(seed_value: int) -> Vector3:
 var roads=[-96,-72,-48,-24,0,24,48,72,96]
 for i in range(150):
  var p=Vector3(roads[posmod(seed_value+i*13,roads.size())],0,posmod(seed_value+i*37,237)-118)
  if free_ground(p,1.1): return p
 return Vector3(24,0,24)

func package_point() -> Vector3:
 for offset in range(main.bridge.data.buildings.size()):
  var building=main.bridge.data.buildings[(offset+int(main.generation.seed)%100)%100]
  if building.type=="grocery": continue
  for dz in [1.0,1.8,2.5]:
   for dx in [0,.8,-.8,1.5,-1.5]:
    var point=Vector3(building.x+dx,0,building.z+building.d/2-dz)
    if free_ground(point,.35): return point
 return road_point(int(main.generation.seed))

func start_mission(kind: String,reward=450,silent=false) -> void:
 if not mission.is_empty() and mission.status=="active": return
 var where=package_point() if kind=="package" else road_point(int(main.generation.seed)+main.battle.number*193)
 if kind=="zone":
  var roads=[-96,-72,-48,-24,0,24,48,72,96]
  var n=posmod(int(main.generation.seed)+main.battle.number*13,81)
  where=Vector3(roads[n%9],0,roads[n/9])
 mission={"key":str(main.battle.number)+"-"+kind,"kind":kind,"position":where,"reward":reward,"status":"active","progress":0.0,"beep":0.0,"hp":500+int(main.battle.number/10)*500,"paid":false,"max_hp":500+int(main.battle.number/10)*500}
 if is_instance_valid(mission_model): mission_model.queue_free()
 mission_model=main.bridge.creature_node(main.bridge.data.mission_models[kind])
 add_child(mission_model)
 mission_model.position=where
 if kind=="growth":
  var target=StaticBody3D.new()
  target.set_script(load("res://scripts/mission_target.gd"))
  target.objectives=self
  target.collision_layer=4
  var collision=CollisionShape3D.new()
  var shape=BoxShape3D.new()
  shape.size=Vector3(1.8,1.65,1.8)
  collision.shape=shape
  collision.position.y=.825
  target.add_child(collision)
  mission_model.add_child(target)
 if not silent: main.battle.announce("NEW MISSION · "+kind.to_upper(),"mission")

func hit_growth(damage: int) -> void:
 if mission.is_empty() or mission.kind!="growth" or mission.status!="active": return
 mission.hp=maxi(0,int(mission.hp)-damage)
 main.battle.burst(mission.position+Vector3(0,.8,0),false,8)
 if mission.hp<=0: complete()

func complete() -> void:
 if mission.is_empty() or mission.status!="active" or mission.paid: return
 mission.status="complete"
 mission.paid=true
 main.coop.grant(int(mission.reward))
 mission_model.queue_free()
 main.battle.announce("MISSION COMPLETE · +$%d" % mission.reward,"mission")
 main.sound.play("cash")

func create_drop(index: int,silent=false) -> void:
 var where=road_point(int(main.generation.seed)+index*113)
 var model=main.bridge.creature_node(main.bridge.data.mission_models.crate)
 add_child(model)
 model.hide()
 drops.append({"position":where,"model":model,"flight":float(index*480),"status":"approaching","progress":0.0,"reward":100+posmod(int(main.generation.seed)+index*197,401)})
 if drops.size()>8:
  var oldest=drops.pop_front()
  oldest.model.queue_free()
 if not silent: main.battle.announce("AIRCRAFT APPROACHING","airdrop")

# Select a single living actor. Holding progress cannot transfer between players.
func actor_for(point: Vector3,radius: float,requires_key=false) -> Dictionary:
 var best={}
 var minimum=INF
 for actor in main.coop.interactors():
  if actor.hp<=0 or actor.position.y>=1.5 or (requires_key and not actor.held): continue
  var distance=Vector2(actor.position.x-point.x,actor.position.z-point.z).length()
  if distance<radius and distance<minimum: best=actor;minimum=distance
 return best

func progress_for(record: Dictionary,actor: Dictionary,delta: float) -> void:
 var id=str(actor.get("id","none"))
 if actor.is_empty(): record.progress=0.0;record.actor="";return
 if record.get("actor",id)!=id: record.progress=0.0
 record.actor=id
 record.progress+=delta

func _physics_process(delta: float) -> void:
 if not main.world_running(): return
 elapsed+=delta
 interaction=""
 var authoritative=main.net.authority()
 if authoritative and main.battle.phase=="ASSAULT" and main.battle.time>=20 and rolled!=main.battle.number:
  rolled=main.battle.number
  var choice=roll(int(main.generation.seed),rolled)
  if not choice.is_empty() and (mission.is_empty() or mission.status!="active"): start_mission(choice.kind,choice.reward)
 if not mission.is_empty() and mission.status=="active":
  var distance=Vector2(main.player.position.x-mission.position.x,main.player.position.z-mission.position.z).length()
  var nearby=main.player.health>0 and main.player.position.y<1.5
  if mission.kind=="zone":
   mission.beep+=delta
   if mission.beep>=5: mission.beep=0;main.sound.play("beacon",mission.position)
   if authoritative:
    var actor=actor_for(mission.position,6)
    # Zone can be held by any squad member; only a completely empty zone resets.
    mission.progress=minf(30,mission.progress+delta) if not actor.is_empty() else 0.0
    if mission.progress>=30: complete()
   if distance<=6 and nearby: interaction="HOLD ZONE · %d / 30 s" % int(mission.progress)
  elif mission.kind=="generator":
   if authoritative:
    progress_for(mission,actor_for(mission.position,2.4,true),delta)
    if mission.progress>=15: complete()
   if distance<2.4 and nearby: interaction="HOLD E · START GENERATOR · %.1f / 15 s" % mission.progress
  elif mission.kind=="package":
   if distance<2.4 and nearby: interaction="E · COLLECT PACKAGE"
   if authoritative and not actor_for(mission.position,2.4,true).is_empty(): complete()
  elif mission.kind=="growth" and is_instance_valid(mission_model):
   var core=mission_model.find_child("growth-core",true,false)
   if core: core.scale=Vector3(1,1.2,1)*(1+sin(elapsed*3)*.035)
   if authoritative:
    var maximum=500+int(main.battle.number/10)*500
    mission.hp+=maxi(0,maximum-int(mission.get("max_hp",maximum)))
    mission.max_hp=maximum
 var index=int(elapsed/480)
 if authoritative and index>drop_index: drop_index=index;create_drop(index)
 plane.visible=false
 for drop in drops:
  if drop.status=="opened": drop.model.hide();continue
  var flight_time=elapsed-drop.flight
  if flight_time<6:
   plane.visible=true
   plane.position=drop.position+Vector3((flight_time-6)*14,altitude,0)
   for rotor in plane_rotors: rotor.rotation.x+=delta*45
   if not is_instance_valid(plane_sound):
    plane_sound=AudioStreamPlayer3D.new()
    plane_sound.stream=main.sound.stream("aircraft")
    plane_sound.max_distance=250
    plane_sound.unit_size=70
    plane.add_child(plane_sound)
    plane_sound.play()
    plane_sound.finished.connect(func(): plane_sound.queue_free())
   continue
  drop.model.show()
  var falling=flight_time<14
  drop.status="falling" if falling else "landed"
  drop.model.position=drop.position+Vector3(0,maxf(0,altitude*(1-(flight_time-6)/8)),0)
  var chute=drop.model.find_child("parachute",true,false)
  if chute: chute.visible=falling
  if not falling:
   var distance=Vector2(main.player.position.x-drop.position.x,main.player.position.z-drop.position.z).length()
   if distance<2 and main.player.position.y<1 and main.player.health>0: interaction="HOLD E · OPEN SUPPLY CRATE · %d / 10 s" % int(drop.progress)
   if authoritative:
    var actor=actor_for(drop.position,2,true)
    progress_for(drop,actor,delta)
    if drop.progress>=10:
     drop.status="opened"
     main.coop.grant(int(drop.reward),str(actor.id))
     drop.model.hide()
     main.sound.play("cash")
     main.battle.announce("SUPPLY OPENED · +$%d" % drop.reward,"")
 if main.net.active:
  for actor in main.coop.peers.values():
   if actor.health<=0 and actor.position.distance_to(main.player.position)<2.5 and main.player.health>0:
    interaction="E · REVIVE SQUAD MEMBER"
    if Input.is_action_just_pressed("interact"):
     if main.net.hosting:
      actor.health=50;main.net.send({"type":"revived","target":actor.id})
     else: main.net.send({"type":"revive","target":actor.id})

func markers() -> Array:
 var out=[]
 if not mission.is_empty() and mission.status=="active": out.append({"position":mission.position,"text":mission.kind.to_upper(),"color":Color("#dec995")})
 for drop in drops:
  if drop.status!="opened": out.append({"position":drop.position,"text":"SUPPLY","color":Color("#9ac3cd")})
 return out

func _process(_delta: float) -> void:
 if not plane.visible and is_instance_valid(plane_sound):
  plane_sound.stop()
  plane_sound.queue_free()
  plane_sound=null
