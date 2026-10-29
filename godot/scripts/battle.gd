extends Node3D
const Enemy=preload("res://scripts/enemy.gd")
var main
var number=1
var phase="PREPARATION"
var time=0.0
var spawned=0
var vomiters=0
var money=0
var completed=0
var primary=""
var inventory: Dictionary={}
var medkits=[0,0]
var flashlight=false
var battery=0.0
var batteries=0
var traps=0
var trap_selected=false
var enemies: Array=[]
var pickups: Array=[]
var projectiles: Array=[]
const VOMIT_GRAVITY=5.0
var placed_traps: Array=[]
var grid=AStarGrid2D.new()
var large_grid: AStarGrid2D
var air_grid: AStarGrid2D
var notice=""
var notice_time=0.0
var poison_time=0.0
var poison_tick=0.0
var rng=RandomNumberGenerator.new()
var store_open=false
var smoke_texture: ImageTexture
# Kill Zone: a 300 x 300 m box whose top face is 5 m under the map. Any enemy that falls into it dies at once.
const KILL_ZONE_SIZE=300.0
const KILL_ZONE_TOP=-5.0
const KILL_ZONE_DEPTH=20.0
var kill_zone: Area3D
var void_kills=0

func _ready() -> void:
 rng.randomize()
 build_kill_zone()
 grid=make_grid(.48,1.5,.25)
 large_grid=make_grid(2.1,6.6,.25)
 air_grid=make_grid(.7,5.2,3.5)
 inventory.Pistol={"ammo":8,"durability":100.0,"shots":0}
 announce("PREPARE · FIRST WAVE IN 60 SECONDS")

func build_kill_zone() -> void:
 kill_zone=Area3D.new()
 kill_zone.name="KillZone"
 kill_zone.collision_layer=0
 kill_zone.collision_mask=4
 kill_zone.monitoring=true
 var shape=CollisionShape3D.new()
 var box=BoxShape3D.new()
 box.size=Vector3(KILL_ZONE_SIZE,KILL_ZONE_DEPTH,KILL_ZONE_SIZE)
 shape.shape=box
 kill_zone.add_child(shape)
 kill_zone.position=Vector3(0,KILL_ZONE_TOP-KILL_ZONE_DEPTH*.5,0)
 add_child(kill_zone)
 kill_zone.body_entered.connect(void_kill)

# Removes an enemy that left the map. No cash, blood or sound: it died where nobody can see it.
func void_kill(body: Node) -> void:
 if not enemies.has(body) or body.dead: return
 if main.net.active and not main.net.hosting: return
 enemies.erase(body)
 body.dead=true
 body.set_physics_process(false)
 body.queue_free()
 void_kills+=1

func make_grid(radius: float, height: float, bottom: float) -> AStarGrid2D:
 var nav=AStarGrid2D.new()
 nav.region=Rect2i(0,0,498,498)
 nav.cell_size=Vector2(.5,.5)
 nav.offset=Vector2(-124.25,-124.25)
 nav.diagonal_mode=AStarGrid2D.DIAGONAL_MODE_ONLY_IF_NO_OBSTACLES
 nav.update()
 for box in main.bridge.data.colliders:
  if box.minY>=height or box.maxY<=bottom: continue
  var a=cell(Vector3(box.minX-radius,0,box.minZ-radius))
  var b=cell(Vector3(box.maxX+radius,0,box.maxZ+radius))
  for x in range(maxi(0,a.x),mini(497,b.x)+1):
   for y in range(maxi(0,a.y),mini(497,b.y)+1): nav.set_point_solid(Vector2i(x,y))
 return nav

func cell(p: Vector3) -> Vector2i:
 return Vector2i(roundi((p.x+124.25)/.5),roundi((p.z+124.25)/.5))

func free_cell(p: Vector3, nav: AStarGrid2D=null) -> Vector2i:
 if nav==null: nav=grid
 var c=cell(p)
 for radius in range(18):
  for x in range(-radius,radius+1):
   for y in range(-radius,radius+1):
    if maxi(abs(x),abs(y))!=radius: continue
    var v=c+Vector2i(x,y)
    if nav.is_in_boundsv(v) and not nav.is_point_solid(v): return v
 return Vector2i(-1,-1)

func route(from: Vector3, to: Vector3, type="zombie") -> PackedVector2Array:
 var nav=large_grid if type in ["boss","brute"] else air_grid if type=="eye" else grid
 var a=free_cell(from,nav)
 var b=free_cell(to,nav)
 if a.x<0 or b.x<0: return PackedVector2Array()
 return nav.get_point_path(a,b,true)

func total() -> int: return 15+(number-1)*5

# Wave variants match the HTML game: every 5th boss, 4th blackout, 3rd rush, others 2 mod 3 siege.
func variant() -> String:
 if number%5==0: return "boss"
 if number%4==0: return "blackout"
 if number%3==0: return "rush"
 if number%3==2: return "siege"
 return "standard"

func variant_label() -> String:
 return {"boss":"WARDEN","blackout":"BLACKOUT","rush":"RUSH","siege":"SIEGE"}.get(variant(),"")

# District (0..3) that loses power during a blackout assault; -1 otherwise. Quadrants follow the HTML grid.
func blackout_district() -> int:
 if variant()!="blackout" or phase not in ["ASSAULT","CLEARANCE"]: return -1
 return (floori(number/4.0)-floori(number/20.0)-1)%4

static func district_of(x: float, z: float) -> int:
 return (0 if z<0 else 1) if x<0 else (2 if z<0 else 3)

func choose_type() -> String:
 if number>=6 and vomiters<floori(total()*.15) and rng.randf()<.15:
  vomiters+=1
  return "vomiter"
 if variant()=="rush" and rng.randf()<.5: return "runner"
 var weights={"zombie":55,"runner":20}
 if number>10: weights.eye=20
 if number>=21: weights.brute=5
 var sum=0
 for weight in weights.values(): sum+=weight
 var roll=rng.randf()*sum
 for type in weights:
  roll-=weights[type]
  if roll<0: return type
 return "zombie"

func spawn_enemy(type: String) -> void:
 var hive=main.bridge.data.hives[number%4 if variant()=="siege" else rng.randi_range(0,3)]
 var nav=large_grid if type in ["boss","brute"] else air_grid if type=="eye" else grid
 var c=free_cell(Vector3(hive.x,0,hive.z),nav)
 var node=Enemy.new()
 node.battle=self
 node.type=type
 node.stats=main.bridge.data.enemy_types[type].duplicate()
 if type=="boss": node.stats.hp=1800+maxi(0,number/5-1)*600
 node.skin=rng.randi_range(0,main.bridge.data.creatures[type].size()-1)
 add_child(node)
 var location=grid.get_point_position(c)
 node.position=Vector3(location.x,4.5 if type=="eye" else .05,location.y)
 enemies.append(node)

func _physics_process(delta: float) -> void:
 if not main.world_running(): return
 # Safety net for enemies that fall past the edge of the zone, so a wave can never wait for a lost enemy.
 for enemy in enemies.duplicate():
  if is_instance_valid(enemy) and enemy.position.y<KILL_ZONE_TOP: void_kill(enemy)
 notice_time=maxf(0,notice_time-delta)
 if main.player.health<=0 and not main.net.active: return
 if poison_time>0:
  poison_time=maxf(0,poison_time-delta)
  poison_tick+=delta
  if poison_tick>=1:
   poison_tick-=1
   main.player.hurt(5)
 if main.player.camera.get_node("Flashlight").visible:
  battery=maxf(0,battery-delta)
  if battery==0: main.player.camera.get_node("Flashlight").visible=false
 if not main.net.authority(): return
 time+=delta
 if phase=="PREPARATION" and time>=60:
  phase="ASSAULT"
  time=0
  announce("WAVE %d · ASSAULT%s" % [number,(" · "+variant_label()) if variant_label()!="" else ""])
 elif phase=="ASSAULT":
  var due=mini(total(),floori(time*total()/120))
  while spawned<due:
   spawn_enemy("boss" if number%5==0 and spawned==total()-1 else choose_type())
   spawned+=1
  if time>=120: phase="CLEARANCE"
 if phase=="CLEARANCE" and enemies.is_empty():
  completed=number
  if number%5==0: main.coop.grant(500)
  phase="INTERMISSION"
  time=0
  announce("WAVE %d COMPLETE · 30 SECOND BREAK" % number)
 elif phase=="INTERMISSION" and time>=30:
  number+=1
  phase="ASSAULT"
  time=0
  spawned=0
  vomiters=0
  announce("WAVE %d · ASSAULT%s" % [number,(" · "+variant_label()) if variant_label()!="" else ""])
 for pickup in pickups.duplicate():
  pickup.mesh.rotation.y+=delta
  var collector=main.coop.nearest(pickup.mesh.position) if main.coop!=null else main.player
  if collector.health>0 and pickup.mesh.position.distance_to(collector.position+Vector3(0,.2,0))<1.2:
   main.coop.grant(pickup.value)
   pickup.mesh.queue_free()
   pickups.erase(pickup)
   main.sound.play("cash")
 for shot in projectiles.duplicate():
  if shot.poison:
   shot.velocity.y-=VOMIT_GRAVITY*delta
   if shot.velocity.length()>.1: shot.mesh.look_at(shot.mesh.position+shot.velocity,Vector3.UP)
  var next=shot.mesh.position+shot.velocity*delta
  var query=PhysicsRayQueryParameters3D.create(shot.mesh.position,next,1|8)
  var hit=get_world_3d().direct_space_state.intersect_ray(query)
  shot.life-=delta
  if not hit.is_empty():
   if hit.collider==main.player:
    main.player.hurt(10 if shot.poison else 30)
    if shot.poison: poison_time=5;poison_tick=0
   shot.life=0
  if main.net.active and main.net.hosting:
   var end=hit.position if not hit.is_empty() else next
   for actor in main.coop.peers.values():
    if actor.health<=0: continue
    var closest=Geometry3D.get_closest_point_to_segment(actor.position+Vector3.UP,shot.mesh.position,end)
    if closest.distance_to(actor.position+Vector3.UP)<.55:
     actor.health=maxi(0,actor.health-(10 if shot.poison else 30))
     main.net.send({"type":"damage","target":actor.id,"amount":10 if shot.poison else 30,"poison":shot.poison})
     shot.life=0
     break
  shot.mesh.position=next
  if shot.life<=0:
   if shot.poison: burst(shot.mesh.position,true,10,Vector3.ZERO,false,false)
   shot.mesh.queue_free();projectiles.erase(shot)
 for trap in placed_traps.duplicate():
  for enemy in enemies.duplicate():
   if enemy.type!="eye" and enemy.position.distance_to(trap.position)<enemy.stats.radius+.4:
    enemy.take_damage(300)
    main.sound.play("trap",trap.position)
    trap.queue_free()
    placed_traps.erase(trap)
    break

func announce(text: String,sound_name="auto") -> void:
 notice=text
 notice_time=5
 if main.sound!=null and sound_name!="": main.sound.play(("wave-clear" if "COMPLETE" in text else "wave") if sound_name=="auto" else sound_name)


func burst(at: Vector3, green=false,amount=22, direction=Vector3.ZERO, hit=false, dust=false) -> void:
 var particles=CPUParticles3D.new()
 particles.amount=amount
 particles.lifetime=.35 if hit else 2.5 if green else .65
 particles.one_shot=true
 particles.explosiveness=1
 particles.direction=direction.normalized() if direction.length()>.01 else Vector3.UP
 particles.spread=65 if hit else 180
 particles.initial_velocity_min=.5
 particles.initial_velocity_max=1.4 if hit else .8 if green else 4
 particles.gravity=Vector3(0,.2 if green else -5,0)
 particles.scale_amount_min=.06
 particles.scale_amount_max=.25 if green else .12
 var material=StandardMaterial3D.new()
 material.albedo_color=Color.WHITE
 material.vertex_color_use_as_albedo=true
 material.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA
 var ramp=Gradient.new()
 var tint=Color("#83887c") if dust else Color("#739543") if green else Color("#b63d31")
 ramp.colors=PackedColorArray([tint,tint,Color(tint.r,tint.g,tint.b,0)])
 ramp.offsets=PackedFloat32Array([0,.55,1])
 particles.color_ramp=ramp
 var mesh=SphereMesh.new()
 mesh.radius=.5
 mesh.height=1
 mesh.radial_segments=8
 mesh.rings=4
 material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 if green:
  if smoke_texture==null:
   var image=Image.create(32,32,false,Image.FORMAT_RGBA8)
   for y in range(32):
    for x in range(32): image.set_pixel(x,y,Color(1,1,1,pow(maxf(0,1-Vector2(x-15.5,y-15.5).length()/15.5),2)))
   smoke_texture=ImageTexture.create_from_image(image)
  var cloud=QuadMesh.new()
  cloud.size=Vector2(1.2,1.2)
  material.albedo_texture=smoke_texture
  material.billboard_mode=BaseMaterial3D.BILLBOARD_ENABLED
  cloud.material=material
  particles.mesh=cloud
  particles.scale_amount_min=.5
  particles.scale_amount_max=1.2
 else:
  mesh.material=material
  particles.mesh=mesh
 particles.local_coords=false
 if main.net.active and main.net.hosting:
  main.net.send({"type":"effect","p":main.coop.v(at),"burst":true,"green":green,"amount":amount,"direction":main.coop.v(direction),"hit":hit,"dust":dust})
 add_child(particles)
 particles.position=at
 particles.finished.connect(particles.queue_free)
 particles.emitting=true

func killed(enemy) -> void:
 enemies.erase(enemy)
 burst(enemy.position+Vector3(0,1.2,0),enemy.type=="vomiter",14 if enemy.type=="vomiter" else 40)
 var pickup=cash_mesh(enemy.position+Vector3(0,.2,0))
 pickup.position.y=.2
 var floor_query=PhysicsRayQueryParameters3D.create(enemy.position+Vector3(0,1,0),enemy.position-Vector3(0,12,0),1)
 var floor_hit=get_world_3d().direct_space_state.intersect_ray(floor_query)
 if not floor_hit.is_empty(): pickup.position.y=floor_hit.position.y+.2
 pickups.append({"mesh":pickup,"value":roundi(clampf(5+(float(enemy.stats.hp)-125)/5,15,100))})
 main.sound.play(str(enemy.type)+"-death",enemy.position)
 if main.net.active and main.net.hosting: main.net.send({"type":"effect","sound":str(enemy.type)+"-death","p":main.coop.v(enemy.position)})

func cash_mesh(at: Vector3) -> MeshInstance3D:
 var pickup=MeshInstance3D.new()
 var box=BoxMesh.new()
 box.size=Vector3(.22,.12,.15)
 pickup.mesh=box
 var material=StandardMaterial3D.new()
 material.albedo_color=Color("#b2bb73")
 material.emission_enabled=true
 material.emission=Color("#45522c")
 pickup.material_override=material
 add_child(pickup)
 pickup.position=at
 return pickup

func projectile(enemy, poison: bool) -> void:
 var mesh=MeshInstance3D.new()
 var sphere=SphereMesh.new()
 sphere.radius=.14 if poison else .2
 sphere.height=sphere.radius*2
 mesh.mesh=sphere
 var material=StandardMaterial3D.new()
 material.albedo_color=Color("#89a737") if poison else Color("#c76b3f")
 material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 mesh.material_override=material
 add_child(mesh)
 mesh.position=enemy.position+Vector3(0,1.8 if poison else 0,0)
 main.combat_fx.trail(mesh,poison)
 var aim=(main.coop.nearest(enemy.position).position if main.coop!=null else main.player.position)+Vector3(0,1,0)
 # Vomit is thrown 3x faster than before (15 m/s) and arcs slightly under gravity; the aim is lifted so the arc lands on the target.
 var speed=15.0 if poison else 20.0
 var velocity=mesh.position.direction_to(aim)*speed
 if poison:
  velocity.y+=.5*VOMIT_GRAVITY*(mesh.position.distance_to(aim)/speed)
  sphere.radial_segments=8
  sphere.rings=4
  mesh.scale=Vector3(1,1,2.4)
 projectiles.append({"mesh":mesh,"velocity":velocity,"life":.85 if poison else 6.0,"poison":poison})
 main.sound.play("vomit" if poison else "eye-living",enemy.position)
 if main.net.active and main.net.hosting: main.net.send({"type":"effect","sound":"vomit" if poison else "eye-living","p":main.coop.v(enemy.position)})

func save_gun() -> void:
 var p=main.player
 if inventory.has(p.config.id): inventory[p.config.id].ammo=p.ammo

func switch_to(id: String) -> void:
 if not inventory.has(id) or main.player.switch_time>0: return
 save_gun()
 main.player.pending_weapon=id
 main.player.switch_time=1.5

func equip_owned(id: String) -> void:
 save_gun()
 var reserve=main.player.reserve
 main.equip(id)
 main.player.reserve=reserve
 main.player.ammo=inventory[id].ammo
 trap_selected=false

func buy(id: String) -> String:
 var item: Dictionary={}
 for record in main.bridge.data.shop_items:
  if record.id==id: item=record
 if item.is_empty(): return "UNKNOWN ITEM"
 if money<int(item.price): return "NOT ENOUGH MONEY"
 var p=main.player
 if item.kind=="weapon":
  var weapon=main.bridge.data.weapons[id]
  if completed<int(weapon.unlockAfter): return "LOCKED · COMPLETE WAVE %d" % int(weapon.unlockAfter)
  if primary==id: return "ALREADY OWNED"
  save_gun()
  if primary!="":
   p.reserve+=int(inventory[primary].ammo)
   inventory.erase(primary)
  primary=id
  inventory[id]={"ammo":0,"durability":100.0,"shots":0}
  equip_owned(id)
 elif id=="ammo":
  save_gun()
  var total_ammo=p.reserve
  for gun in inventory.values(): total_ammo+=int(gun.ammo)
  if total_ammo>450: return "AMMO LIMIT · 500"
  p.reserve+=50
 elif id=="smallMedkit" or id=="largeMedkit":
  if p.health<100: p.health=mini(100,p.health+(25 if id=="smallMedkit" else 75))
  else: medkits[0 if id=="smallMedkit" else 1]+=1
 elif id=="repair":
  var gun=inventory[p.config.id]
  if gun.durability>=100: return "WEAPON ALREADY REPAIRED"
  gun.durability=minf(100,gun.durability+50)
 elif id=="flashlight":
  if flashlight: return "ALREADY OWNED"
  flashlight=true
  battery=120
 elif id=="battery": batteries+=1
 elif id=="trap": traps+=1
 money-=int(item.price)
 return "PURCHASE COMPLETE"

func use_medkit(index: int) -> void:
 if medkits[index]>0 and main.player.health<100:
  medkits[index]-=1
  main.player.health=mini(100,main.player.health+(25 if index==0 else 75))

func place_trap() -> void:
 if traps<=0: trap_selected=false;return
 if main.net.active and not main.net.hosting:
  main.net.send({"type":"trap"})
 else: add_trap(main.player.position)
 traps-=1
 trap_selected=false

func add_trap(at: Vector3) -> void:
 var mesh=main.bridge.creature_node(main.bridge.data.mission_models.trap)
 add_child(mesh)
 mesh.position=at+Vector3(0,.06,0)
 placed_traps.append(mesh)
