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
 for distance in [9.5]:
  p.position=Vector3(24,.05,24)
  p.health=100
  b.spawn_enemy("vomiter")
  var v=b.enemies.back()
  v.position=Vector3(24,.05,24+distance)
  var fastest=0.0
  var fired=false
  for i in range(360):
   await physics_frame
   for shot in b.projectiles:
    if shot.poison: fired=true;fastest=maxf(fastest,shot.velocity.length())
   if p.health<100: break
  if distance<10: check(fired and fastest>14.0 and fastest<16.5,"vomit speed ~15 m/s, got %.1f" % fastest)
  if distance<10: check(p.health<100,"vomit must reach a player 9.5 m away")
  for e in b.enemies.duplicate(): e.take_damage(100000)
  b.projectiles.clear()
 if failures.is_empty(): print("PASS VOMIT: 15 m/s arcing vomit reaches 9.5 m")
 else: for f in failures: print("FAIL ",f)
 quit(0 if failures.is_empty() else 1)
