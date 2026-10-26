extends SceneTree
var failures: Array=[]
func check(ok: bool, text: String) -> void:
 if not ok: failures.append(text);push_error(text)
func _initialize() -> void: call_deferred("run")
func run() -> void:
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 await process_frame
 var b=scene.battle
 var p=scene.player
 scene.set_playing(true)
 check(b.inventory.size()==1 and b.inventory.has("Pistol"),"Pistol-only starter")
 check(b.money==0 and p.ammo+p.reserve==92,"Starting money/ammo")
 b._physics_process(59)
 check(b.phase=="PREPARATION" and b.enemies.is_empty(),"First wave must wait60s")
 b._physics_process(1)
 check(b.phase=="ASSAULT","First wave starts at60s")
 b._physics_process(60)
 check(b.spawned==7 and b.enemies.size()==7,"First wave must spawn gradually")
 b._physics_process(60)
 check(b.spawned==15 and b.enemies.size()==15,"Wave1 has15 enemies")
 for e in b.enemies: check(e.type in ["zombie","runner"],"First5 waves only normal/runner")
 check(b.phase=="CLEARANCE","Wave cannot advance with survivors")
 b._physics_process(200)
 check(b.phase=="CLEARANCE" and b.number==1,"Survivors block next wave")
 for e in b.enemies.duplicate(): e.take_damage(100000)
 b._physics_process(0)
 check(b.phase=="INTERMISSION","Last kill starts intermission")
 b._physics_process(29)
 check(b.number==1,"Break must last30 seconds")
 b._physics_process(1)
 check(b.number==2 and b.total()==20,"Next wave adds5")
 b.money=100000
 b.completed=0
 check("LOCKED" in b.buy("Helsing"),"Helsing unlock gate")
 b.completed=25
 for item in scene.bridge.data.shop_items:
  if item.kind=="weapon":
   check(b.buy(item.id)=="PURCHASE COMPLETE","Purchase "+item.id)
   check(b.inventory.size()==2 and b.inventory.has("Pistol"),"Primary replaces old gun")
 p.health=40
 check(b.buy("smallMedkit")=="PURCHASE COMPLETE" and p.health==65,"Small medkit heals25")
 check(b.buy("largeMedkit")=="PURCHASE COMPLETE" and p.health==100,"Large medkit caps health")
 b.inventory[p.config.id].durability=40
 check(b.buy("repair")=="PURCHASE COMPLETE" and b.inventory[p.config.id].durability==90,"Repair restores50")
 p.reserve=499
 check("LIMIT" in b.buy("ammo"),"Ammo cap enforced")
 p.reserve=0
 b.save_gun()
 check(b.buy("ammo")=="PURCHASE COMPLETE" and p.reserve==50,"Ammo pack50")
 check(b.buy("flashlight")=="PURCHASE COMPLETE" and b.battery==120,"Light battery120")
 for wave in [6,10,20,21,30]:
  b.number=wave
  b.vomiters=0
  var counts={}
  for i in range(b.total()):
   var type=b.choose_type()
   counts[type]=counts.get(type,0)+1
   if wave<=20: check(type!="brute","Brute unlock21")
  check(counts.get("vomiter",0)<=floori(b.total()*.15),"Vomiter15% cap")
 for type in scene.bridge.data.enemy_types:
  b.spawn_enemy(type)
  var e=b.enemies.back()
  check(e.rig!=null and e.hp>0,"Model and HP "+type)
  if type=="eye": check(e.hp==200,"Eye200")
  if type=="brute": check(e.hp==800,"Brute800")
  if type=="boss": check(e.hp==4800,"Sixth boss4800")
  e.take_damage(100000)
 check(scene.bridge.data.creatures.zombie.size()==9,"Nine zombie skins")
 check(scene.bridge.data.creatures.boss.size()==5,"Five boss skins")
 b.poison_time=5
 p.health=100
 for i in range(5): b._physics_process(1)
 check(p.health==75,"Poison5x5")
 var cash=b.pickups[0]
 p.position=cash.mesh.position-Vector3(0,.2,0)
 var before=b.money
 b._physics_process(0)
 check(b.money>before,"Automatic shared-style cash collection")
 if failures.is_empty(): print("PASS GAMEPLAY: wave timing, clearance, cash, nine weapons, slot replacement, unlocks, medicine, ammo, repairs, battery, poison, creatures and skins")
 scene.set_playing(false)
 quit(0 if failures.is_empty() else 1)
