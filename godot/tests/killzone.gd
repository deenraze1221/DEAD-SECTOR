extends SceneTree
var failures=[]
func check(ok: bool,text: String) -> void:
 if not ok: failures.append(text);push_error(text)
func _initialize() -> void: call_deferred("run")
func gone(e) -> bool: return not is_instance_valid(e) or e.is_queued_for_deletion()
func run() -> void:
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 await process_frame
 var b=scene.battle
 scene.set_playing(true)
 b.time=-99999
 # Geometry: 300 x 300 m, top face 5 m under the map.
 var zone=b.get_node("KillZone")
 var box: BoxShape3D=zone.get_child(0).shape
 check(box.size.x==300.0 and box.size.z==300.0,"kill zone must be 300 x 300 m, got %s" % box.size)
 check(is_equal_approx(zone.position.y+box.size.y*.5,-5.0),"kill zone top must be at y=-5, got %f" % (zone.position.y+box.size.y*.5))
 # Enemy that falls into the zone dies at once, without cash or blood.
 var pickups_before=b.pickups.size()
 b.spawn_enemy("zombie")
 var fallen=b.enemies.back()
 fallen.collision_mask=0
 fallen.position=Vector3(10,-6,10)
 for i in range(6): await physics_frame
 check(gone(fallen),"zombie below the map must be removed")
 check(not b.enemies.has(fallen),"fallen zombie must leave the enemy list")
 check(b.void_kills==1,"void kill counter should be 1, got %d" % b.void_kills)
 check(b.pickups.size()==pickups_before,"no cash may drop under the map")
 # Every enemy type is caught, large ones too.
 for type in ["runner","brute","eye","vomiter","boss"]:
  b.spawn_enemy(type)
  var e=b.enemies.back()
  e.collision_mask=0
  e.position=Vector3(-20,-7,15)
  for i in range(6): await physics_frame
  check(gone(e),type+" below the map must be removed")
 # Safety net: far outside the 300 m square the wave must still not wait for a lost enemy.
 b.spawn_enemy("zombie")
 var lost=b.enemies.back()
 lost.collision_mask=0
 lost.position=Vector3(400,-8,0)
 for i in range(6): await physics_frame
 check(gone(lost),"enemy outside the zone square must still be removed")
 # Enemies on the ground are untouched.
 b.spawn_enemy("zombie")
 var safe=b.enemies.back()
 var kills=b.void_kills
 for i in range(30): await physics_frame
 check(not gone(safe) and b.enemies.has(safe),"enemy on the map must survive")
 check(b.void_kills==kills,"no void kill for a normal enemy")
 if failures.is_empty(): print("PASS KILLZONE: 300x300 zone at y=-5, all enemy types, no cash, safety net, ground enemies safe")
 else: for f in failures: print("FAIL ",f)
 quit(0 if failures.is_empty() else 1)
