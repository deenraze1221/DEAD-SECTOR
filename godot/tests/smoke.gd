extends SceneTree
var failures: Array[String]=[]

func check(condition: bool, message: String) -> void:
 if not condition:
  failures.append(message)
  push_error(message)

func _initialize() -> void:
 call_deferred("run")

func run() -> void:
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 await process_frame
 check(scene.bridge.data.weapons.size()==10,"Ten weapons must be available")
 check(scene.bridge.data.batches.size()>100,"Original city must be loaded")
 check(scene.bridge.data.colliders.size()>7000,"City collision volumes must be preserved")
 check(scene.player.camera.fov==73,"World FOV must be73")
 var p=scene.player
 scene.set_playing(true)
 for i in range(60): await physics_frame
 check(p.is_on_floor(),"Spawn must settle on a physical floor")
 check(abs(p.position.y-scene.bridge.data.spawn.get("y",0))<.2,"Spawn must not float or fall")
 # Actual CharacterBody physics: no movement from Shift alone.
 var initial=p.position
 Input.action_press("sprint")
 for i in range(30): await physics_frame
 Input.action_release("sprint")
 check(p.position.distance_to(initial)<.01,"Stationary Shift must not move the player")
 check(p.stamina==5,"Stationary Shift must not consume stamina")
 Input.action_press("forward")
 for i in range(60): await physics_frame
 Input.action_release("forward")
 check(p.position.distance_to(initial)>1,"WASD must move the physical character")
 scene.reset_position()
 for id in scene.bridge.data.weapons:
  scene.equip(id)
  check(p.gun.get_child_count()>0,id+" model missing")
  check(p.gun.has_node("SupportHand") and p.gun.has_node("TriggerHand"),id+" grip hands missing")
  var grips: Dictionary=p.config.grips
  for name_and_grip in [["SupportHand","support"],["TriggerHand","trigger"]]:
   var point: Array=grips[name_and_grip[1]]
   check(p.gun.get_node(name_and_grip[0]).position.distance_to(Vector3(point[0],point[1],point[2]))<.00001,id+" hand misses grip")
  check(p.camera.cull_mask==1 and p.weapon_camera.cull_mask==2,"World/weapon cameras must be isolated")
  check(p.ammo==int(p.config.capacity),id+" magazine incorrect")
  p.fire()
  check(p.ammo==int(p.config.capacity)-1,id+" shot did not consume one round")
  check(p.cooldown==float(p.config.interval),id+" fire rate mismatch")
  check(p.reloading==0,id+" unexpected reload")
  var before=p.reserve
  var reload_event=InputEventAction.new()
  reload_event.action="reload"
  reload_event.pressed=true
  p._unhandled_input(reload_event)
  check(is_equal_approx(p.reloading,float(p.config.reload)),id+" reload duration mismatch")
  # Disable automatic held input; allow the actual countdown to complete.
  p._physics_process(float(p.config.reload)+.001)
  check(p.ammo==int(p.config.capacity),id+" reload failed")
  check(p.reserve==before-1,id+" reserve accounting failed")
 scene.equip("Pistol")
 var down=InputEventMouseMotion.new()
 down.relative=Vector2(0,500)
 p.look(down.relative)
 p.look(down.relative)
 check(is_equal_approx(p.camera.rotation.x,-PI/3),"Downward look limit must be60 degrees")
 scene.set_playing(false)
 check(not p.active and scene.menu.visible,"Pause must release gameplay")
 if failures.is_empty(): print("PASS GODOT: city, colliders, physical spawn/movement, stamina, all10 weapons, reloads, camera and pause")
 quit(0 if failures.is_empty() else 1)
