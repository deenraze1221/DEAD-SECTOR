extends SceneTree
var errors=[]
func check(value: bool,message: String) -> void:
 if not value: errors.append(message);push_error(message)
func _initialize() -> void: call_deferred("run")
func run() -> void:
 var main=load("res://main.tscn").instantiate()
 root.add_child(main)
 await process_frame
 print("Fresh interior report: ",main.interior_report)
 check(main.interior_report.rooms>=198,"Rooms regenerated for all non-store buildings")
 check(main.interior_report.furniture>350,"Buildings receive fresh furniture")
 check(main.objectives!=null,"Objectives connected")
 for wave in range(1,3): check(main.objectives.roll(123,wave).is_empty(),"No missions beforewave3")
 var count=0
 for seed_value in range(500):
  if not main.objectives.roll(seed_value,8).is_empty(): count+=1
 check(count>45 and count<110,"Mission chance approximately15%")
 check(is_equal_approx(main.bridge.data.enemy_types.zombie.speed,2.42*1.05),"Normal zombie5% speed increase")
 check(is_equal_approx(main.bridge.data.enemy_types.runner.speed,3.78*1.05),"Runner5% speed increase")
 var ops=main.objectives
 main.set_playing(true)
 main.battle.time=-9999
 ops.start_mission("generator",400)
 main.player.position=ops.mission.position
 var before=main.battle.money
 Input.action_press("interact")
 ops._physics_process(14)
 check(ops.mission.status=="active" and ops.mission.progress==14,"Generator requires full15seconds")
 Input.action_release("interact")
 ops._physics_process(.1)
 check(ops.mission.progress==0,"Generator release resets")
 Input.action_press("interact")
 ops._physics_process(15)
 Input.action_release("interact")
 check(ops.mission.status=="complete" and main.battle.money==before+400,"15second generator reward")
 ops.complete()
 check(main.battle.money==before+400,"Reward exactly once")
 ops.start_mission("zone",500)
 main.player.position=ops.mission.position
 ops._physics_process(29)
 check(ops.mission.status=="active","Zone requires30seconds")
 main.player.position+=Vector3(7,0,0)
 ops._physics_process(.1)
 check(ops.mission.progress==0,"Leaving zone resets")
 main.player.position=ops.mission.position
 ops._physics_process(30)
 check(ops.mission.status=="complete","Zone complete")
 main.battle.number=9
 ops.start_mission("growth",300)
 check(ops.mission.hp==500,"Growth500 beforewave10")
 main.battle.number=10
 ops._physics_process(.1)
 check(ops.mission.hp==1000,"Growth scales by500 onwave10")
 ops.hit_growth(1000)
 check(ops.mission.status=="complete","Growth shot completion")
 ops.elapsed=479
 ops.drop_index=0
 ops._physics_process(1)
 check(ops.drops.size()==1 and ops.plane.visible,"Plane starts exactly480")
 ops._physics_process(6)
 check(ops.drops[0].status=="falling","Sixsecond flight")
 ops._physics_process(8)
 check(ops.drops[0].status=="landed","Eightsecond parachute descent")
 main.player.position=ops.drops[0].position
 Input.action_press("interact")
 ops._physics_process(9)
 check(ops.drops[0].status=="landed","Drop needs10seconds")
 Input.action_release("interact")
 ops._physics_process(.01)
 check(ops.drops[0].progress==0,"Drop release resets")
 Input.action_press("interact")
 ops._physics_process(10)
 Input.action_release("interact")
 check(ops.drops[0].status=="opened","Ten second drop unlock")
 main.battle.spawn_enemy("zombie")
 var enemy=main.battle.enemies.back()
 enemy.position=Vector3(24,.05,30)
 enemy.take_damage(45,enemy.position+Vector3.UP,Vector3.FORWARD)
 check(enemy.hp==205 and enemy.recoil_time>0 and enemy.stagger>0,"Hit reaction without incorrect damage")
 check(enemy.impulse.length()>0,"Collision-constrained hit impulse")
 enemy.take_damage(1000)
 check(enemy.dead,"Death is immediate")
 var hud_font=main.reference_ui.hud_root.theme.default_font
 check(hud_font is SystemFont and not str(hud_font.resource_path).ends_with("hud-user.ttf"),"HUD uses the HTML monospace stack, not the pixel font")
 check(hud_font.fallbacks.size()>0 and str(hud_font.fallbacks[0].resource_path).ends_with("terminal.ttf"),"Bundled monospace fallback present")
 if errors.is_empty(): print("PASS NEXT PORT: new rooms/furniture, speeds, missions,15s generator, growth, plane/drop and zombie hit reactions")
 quit(0 if errors.is_empty() else 1)
