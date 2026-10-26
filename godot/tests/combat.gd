extends SceneTree
var failures=[]
func check(ok: bool,text: String) -> void:
 if not ok: failures.append(text);push_error(text)
func _initialize() -> void: call_deferred("run")
func run() -> void:
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 await process_frame
 var b=scene.battle
 var p=scene.player
 scene.set_playing(true)
 b.time=-99999
 p.position=Vector3(24,.05,24)
 p.health=10000
 for i in range(4):
  b.spawn_enemy("zombie")
  b.enemies.back().position=Vector3(23+i*.6,.05,34+i*.5)
 for i in range(1800): await physics_frame
 check(p.health<10000,"Ground crowd must navigate to and damage player")
 for enemy in b.enemies.duplicate(): enemy.take_damage(100000)
 # Cancel incidental AI to isolate projectile sweep and wall collision.
 p.position=Vector3(24,0,24)
 p.health=100
 b.spawn_enemy("vomiter")
 var v=b.enemies.back()
 v.position=Vector3(24,0,29)
 b.projectile(v,true)
 v.take_damage(100000)
 for i in range(160): await physics_frame
 check(p.health<=90,"Vomiter projectile must actually hit the player")
 check(b.poison_time>0,"Vomit hit must apply poison")
 p.health=100
 b.poison_time=0
 b.spawn_enemy("eye")
 var eye=b.enemies.back()
 eye.position=Vector3(24,4.5,34)
 b.projectile(eye,false)
 eye.take_damage(100000)
 for i in range(100): await physics_frame
 check(p.health==70,"Eye projectile must deal30HP")
 if failures.is_empty(): print("PASS COMBAT: actual crowd approach/melee and swept vomit/eye hits with poison")
 quit(0 if failures.is_empty() else 1)
