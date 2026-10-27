extends Node
const Avatar=preload("res://scripts/coop_avatar.gd")
var main
var peers: Dictionary={}
var replicas: Dictionary={}
var cash_models: Dictionary={}
var tick=0.0
var snapshot_clock=0.0
var serial=0
var credit_totals: Dictionary={}
var last_notice=""
var last_state: Dictionary={}
var paused_for_disconnect=false

func _ready() -> void:
 main.net.packet.connect(receive)

func actors() -> Array:
 var out=[]
 if main.player.health>0: out.append(main.player)
 for actor in peers.values():
  if actor.health>0 and actor.age<3: out.append(actor)
 return out

func nearest(point: Vector3):
 var best=main.player
 var distance=INF
 for actor in actors():
  var d=actor.position.distance_squared_to(point)
  if d<distance: distance=d;best=actor
 return best

func interactors() -> Array:
 var out=[{"id":main.net.peer_id,"position":main.player.position,"held":Input.is_action_pressed("interact") and main.player.active,"hp":main.player.health}]
 for actor in peers.values():
  if actor.age<.5: out.append({"id":actor.id,"position":actor.position,"held":actor.held,"hp":actor.health})
 return out

func grant(amount: int,target="all") -> void:
 if target=="all" or target==main.net.peer_id: main.battle.money+=amount
 if main.net.active:
  for member in main.net.members:
   if target=="all" or target==member.id: credit_totals[member.id]=int(credit_totals.get(member.id,0))+amount
  main.net.send({"type":"grant","credits":credit_totals})

func apply_credits(totals: Dictionary) -> void:
 var amount=int(totals.get(main.net.peer_id,0))
 if amount>main.net.credit_received:
  main.battle.money+=amount-main.net.credit_received
  main.net.credit_received=amount
  main.sound.play("cash")

func _process(delta: float) -> void:
 if not main.net.active or not main.net.match_active: return
 tick+=delta
 snapshot_clock+=delta
 if tick>=.05:
  tick=0
  var p=main.player
  main.net.send({"type":"pose","p":v(p.position),"yaw":p.rotation.y,"pitch":p.camera.rotation.x,"hp":p.health,"weapon":"trap" if main.battle.trap_selected else p.config.id,"held":p.active and Input.is_action_pressed("interact"),"light":p.camera.get_node("Flashlight").visible})
 if main.net.hosting and snapshot_clock>=.1:
  snapshot_clock=0
  main.net.send(snapshot())
 if not main.net.hosting:
  for id in replicas:
   var entry=replicas[id]
   var node=entry.node
   var previous=node.position
   node.position=node.position.lerp(vec(entry.p),1-exp(-delta*14))
   node.rotation.y=lerp_angle(node.rotation.y,float(entry.yaw),1-exp(-delta*14))
   node.voice-=delta
   if node.voice<=0:
    node.voice=4+main.battle.rng.randf()*4
    if node.position.distance_to(main.player.position)<18: main.sound.play(node.type+"-living",node.position)
   var travel=node.position.distance_to(previous)
   node.phase+=travel*5
   var stride=clampf(travel/maxf(delta,.001)/maxf(.1,float(node.stats.speed)),0,1)
   var windup=float(entry.get("windup",0))
   for i in range(node.hips.size()): node.hips[i].rotation.x=lerpf(node.hips[i].rotation.x,sin(node.phase+i*PI)*.4*stride,1-exp(-delta*18))
   for i in range(node.arms.size()): node.arms[i].rotation.x=lerpf(node.arms[i].rotation.x,-.4+(sin(windup*6)*.7 if windup>0 else sin(node.phase+i*PI)*.12*stride),1-exp(-delta*18))
   if node.warning!=null:
    node.warning.visible=entry.get("warning",false)
    node.warning.scale=Vector3.ONE*(4.5/3.2 if float(node.hp)/float(node.stats.hp)<=.3 else 1.0)

static func v(p: Vector3) -> Array: return [p.x,p.y,p.z]
static func vec(p: Array) -> Vector3: return Vector3(p[0],p[1],p[2])

func snapshot() -> Dictionary:
 var b=main.battle
 var enemies=[]
 for e in b.enemies:
  if not e.has_meta("net_id"): serial+=1;e.set_meta("net_id",str(serial))
  enemies.append({"key":e.get_meta("net_id"),"type":e.type,"skin":e.skin,"hp":e.hp,"max_hp":e.stats.hp,"p":v(e.position),"yaw":e.rotation.y,"windup":e.windup,"warning":e.windup>0 and e.attack_kind=="slam"})
 var cash=[]
 for item in b.pickups:
  if not item.has("key"): serial+=1;item.key="cash-"+str(serial)
  cash.append({"key":item.key,"p":v(item.mesh.position),"value":item.value})
 var projectiles=[]
 for shot in b.projectiles:
  if not shot.has("key"): serial+=1;shot.key="shot-"+str(serial)
  projectiles.append({"key":shot.key,"p":v(shot.mesh.position),"poison":shot.poison})
 var mission=main.objectives.mission.duplicate(true)
 if not mission.is_empty(): mission.position=v(mission.position)
 var drops=[]
 for drop in main.objectives.drops:
  var record=drop.duplicate()
  record.erase("model")
  record.position=v(drop.position)
  drops.append(record)
 var traps=[]
 for trap in b.placed_traps:
  if not trap.has_meta("net_id"): serial+=1;trap.set_meta("net_id","trap-"+str(serial))
  traps.append({"key":trap.get_meta("net_id"),"p":v(trap.position)})
 return {"type":"snapshot","credits":credit_totals,"wave":b.number,"phase":b.phase,"time":b.time,"spawned":b.spawned,"completed":b.completed,"notice":b.notice if b.notice_time>0 else "","enemies":enemies,"cash":cash,"projectiles":projectiles,"mission":mission,"drops":drops,"elapsed":main.objectives.elapsed,"traps":traps}

func receive(m: Dictionary) -> void:
 var type=str(m.get("type",""))
 if type=="start":
  main.get_tree().reload_current_scene()
  return
 if type=="transport-lost":
  paused_for_disconnect=true
  main.battle.announce("CONNECTION LOST · WAITING FOR HOST","")
  return
 if type=="room" and m.get("phase","")=="playing":
  paused_for_disconnect=false
  main.battle.announce("CONNECTION RESTORED","")
 if type=="away":
  var id=str(m.id)
  if id in ["host","1"]:
   paused_for_disconnect=true
   main.battle.announce("HOST DISCONNECTED · WAITING FOR RECONNECT","")
  if peers.has(id): peers[id].queue_free();peers.erase(id)
 if type=="closed": main.get_tree().reload_current_scene();return
 if not main.net.match_active: return
 if type=="pose":
  var id=str(m.id)
  if id==main.net.peer_id: return
  if not peers.has(id):
   var avatar=Avatar.new()
   main.add_child(avatar)
   avatar.setup(main,id)
   peers[id]=avatar
  peers[id].update_pose(m)
 elif type=="snapshot" and not main.net.hosting:
  paused_for_disconnect=false
  apply_snapshot(m)
 elif type=="grant" and not main.net.hosting:
  apply_credits(m.credits)
 elif type=="damage" and m.target==main.net.peer_id:
  main.player.hurt(int(m.amount))
  if m.get("poison",false): main.battle.poison_time=5;main.battle.poison_tick=0
 elif type=="revived" and m.target==main.net.peer_id:
  main.player.health=50
  main.set_playing(true)
  main.battle.announce("REVIVED BY SQUAD","")
 elif type=="revive" and main.net.hosting and peers.has(str(m.id)):
  var actor=peers[str(m.id)]
  var target=main.player if m.target==main.net.peer_id else peers.get(str(m.target))
  if target!=null and actor.health>0 and target.health<=0 and actor.position.distance_to(target.position)<2.5:
   target.health=50
   if target==main.player: main.set_playing(true)
   main.net.send({"type":"revived","target":m.target})
 elif type=="shot" and main.net.hosting and peers.has(str(m.id)):
  host_shot(m)
 elif type=="trap" and main.net.hosting and peers.has(str(m.id)):
  var actor=peers[str(m.id)]
  if actor.health>0: main.battle.add_trap(actor.position)
 elif type=="effect":
  if peers.has(str(m.get("shooter",""))): peers[str(m.shooter)].kick=.1
  if m.get("sound","")!="" and m.get("shooter","")!=main.net.peer_id: main.sound.play(m.sound,vec(m.p))
  if m.has("end") and m.get("shooter","")!=main.net.peer_id: main.combat_fx.tracer(vec(m.p),vec(m.end))
  if m.get("burst",false): main.battle.burst(vec(m.p),m.get("green",false),int(m.get("amount",8)),vec(m.get("direction",[0,1,0])),m.get("hit",false),m.get("dust",false))

func host_shot(m: Dictionary) -> void:
 var actor=peers[str(m.id)]
 if actor.health<=0 or not main.bridge.data.weapons.has(str(m.weapon)): return
 var interval=float(main.bridge.data.weapons[m.weapon].interval)
 var now=Time.get_ticks_msec()/1000.0
 if now-float(actor.get_meta("last_shot",-100.0))<interval*.85: return
 actor.set_meta("last_shot",now)
 var direction=vec(m.direction).normalized()
 actor.kick=.1
 var origin=actor.position+Vector3.UP*1.65
 var query=PhysicsRayQueryParameters3D.create(origin,origin+direction*100,1|4)
 var hit=main.get_world_3d().direct_space_state.intersect_ray(query)
 var damage=int(main.bridge.data.weapons[m.weapon].damage)
 if not hit.is_empty() and hit.collider.has_method("take_damage"):
  if hit.collider.has_meta("mutant"): hit.collider.take_damage(damage,hit.position,direction)
  else: hit.collider.take_damage(damage)
 main.combat_fx.tracer(origin,hit.position if not hit.is_empty() else origin+direction*100)
 main.sound.play("shot-"+str(m.weapon),origin)
 main.net.send({"type":"effect","sound":"shot-"+str(m.weapon),"p":v(origin),"end":v(hit.position if not hit.is_empty() else origin+direction*100),"shooter":m.id})
 for enemy in main.battle.enemies: enemy.hear(origin)

func apply_snapshot(m: Dictionary) -> void:
 last_state=m
 apply_credits(m.get("credits",{}))
 var b=main.battle
 b.number=int(m.wave);b.phase=m.phase;b.time=float(m.time);b.spawned=int(m.spawned);b.completed=int(m.completed)
 if m.notice!=last_notice and m.notice!="": b.announce(m.notice,"mission" if "MISSION" in m.notice else "airdrop" if "AIRCRAFT" in m.notice else "auto")
 last_notice=m.notice
 var seen={}
 for e in m.enemies:
  seen[e.key]=true
  if not replicas.has(e.key):
   var enemy=b.Enemy.new()
   enemy.battle=b;enemy.type=e.type;enemy.skin=int(e.skin)
   enemy.stats=main.bridge.data.enemy_types[e.type].duplicate()
   enemy.stats.hp=e.max_hp
   b.add_child(enemy)
   enemy.set_physics_process(false)
   enemy.collision_layer=4
   enemy.collision_mask=0
   enemy.position=vec(e.p)
   b.enemies.append(enemy)
   replicas[e.key]={"node":enemy}
  var node=replicas[e.key].node
  node.hp=int(e.hp)
  replicas[e.key].merge(e,true)
 for id in replicas.keys():
  if not seen.has(id):
   var node=replicas[id].node
   b.enemies.erase(node);node.queue_free();replicas.erase(id)
 var cash_seen={}
 for item in m.cash:
  cash_seen[item.key]=true
  if not cash_models.has(item.key): cash_models[item.key]=b.cash_mesh(vec(item.p))
 for id in cash_models.keys():
  if not cash_seen.has(id): cash_models[id].queue_free();cash_models.erase(id)
 # Replica projectiles/traps use small pooled nodes; they do not deal local duplicate damage.
 update_props("shots",m.projectiles)
 update_props("traps",m.traps)
 var ops=main.objectives
 ops.elapsed=float(m.elapsed)
 var mission: Dictionary=m.mission.duplicate(true)
 if not mission.is_empty():
  mission.position=vec(mission.position)
  if ops.mission.is_empty() or ops.mission.kind!=mission.kind or ops.mission.position!=mission.position or ops.mission.get("key","")!=mission.get("key",""):
   ops.start_mission(mission.kind,int(mission.reward),true)
   ops.mission_model.position=mission.position
  ops.mission=mission
  if is_instance_valid(ops.mission_model):
   if mission.status!="active": ops.mission_model.queue_free()
   else: ops.mission_model.show()
 var old_drops=ops.drops.duplicate()
 var new_drops=[]
 for record_source in m.drops:
  var record=record_source.duplicate(true)
  record.position=vec(record.position)
  var existing={}
  for drop in old_drops:
   if is_equal_approx(float(drop.flight),float(record.flight)): existing=drop;break
  if existing.is_empty():
   ops.create_drop(int(float(record.flight)/480),true)
   existing=ops.drops.back()
  existing.merge(record,true)
  new_drops.append(existing)
 for drop in old_drops:
  if not new_drops.any(func(d): return d.model==drop.model): drop.model.queue_free()
 ops.drops=new_drops

var prop_pools: Dictionary={"shots":{},"traps":{}}
func update_props(kind: String,records: Array) -> void:
 var pool: Dictionary=prop_pools[kind]
 var seen={}
 for record in records:
  var id=str(record.key)
  seen[id]=true
  if not pool.has(id):
   var model: Node3D
   if kind=="traps": model=main.bridge.creature_node(main.bridge.data.mission_models.trap)
   else:
    var mesh=MeshInstance3D.new()
    var shape=SphereMesh.new();shape.radius=.18;shape.height=.36;mesh.mesh=shape
    var mat=StandardMaterial3D.new();mat.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
    mat.albedo_color=Color("#89a737") if record.poison else Color("#c76b3f")
    mesh.material_override=mat
    model=mesh
   main.add_child(model)
   model.position=vec(record.p)
   pool[id]=model
   if kind=="shots": main.combat_fx.trail(model,bool(record.poison))
  pool[id].position=vec(record.p)
 for id in pool.keys():
  if not seen.has(id): pool[id].queue_free();pool.erase(id)
