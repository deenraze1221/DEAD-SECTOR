extends SceneTree
func _initialize() -> void: call_deferred("run")
func shot(name: String) -> void:
 for i in range(3): await process_frame
 await RenderingServer.frame_post_draw
 root.get_texture().get_image().save_png("user://next-"+name+".png")
func run() -> void:
 var main=load("res://main.tscn").instantiate()
 root.add_child(main)
 await shot("menu-font")
 main.set_playing(true)
 main.player.position=Vector3(24,.05,30)
 main.player.rotation.y=0
 main.battle.time=5
 main.battle.announce("WAVE 1 · ASSAULT")
 await shot("hud-compass-notice")
 main.battle.notice_time=0
 main.objectives.start_mission("generator",450)
 main.player.position=main.objectives.mission.position+Vector3(0,0,2)
 main.player.rotation.y=0
 Input.action_press("interact")
 await shot("generator")
 Input.action_release("interact")
 main.objectives.complete()
 main.objectives.start_mission("growth",450)
 main.player.position=main.objectives.mission.position+Vector3(0,0,3)
 await shot("growth")
 main.objectives.complete()
 main.objectives.elapsed=479
 main.objectives._physics_process(1)
 main.player.position=main.objectives.drops[0].position+Vector3(0,0,20)
 main.climate.kind="clear"
 main.climate.fog="light"
 main.climate._process(1)
 main.player.camera.look_at(main.objectives.plane.global_position)
 await shot("aircraft")
 main.player.camera.rotation=Vector3.ZERO
 main.battle.spawn_enemy("zombie")
 var enemy=main.battle.enemies.back()
 enemy.position=main.player.position+Vector3(0,0,-3)
 Engine.time_scale=.05
 enemy.take_damage(45,enemy.position+Vector3.UP,Vector3(0,0,-1))
 for emitter in main.battle.get_children():
  if emitter is CPUParticles3D: emitter.preprocess=.1;emitter.speed_scale=.02;emitter.restart()
 await shot("hit")
 enemy.take_damage(500)
 for emitter in main.battle.get_children():
  if emitter is CPUParticles3D: emitter.preprocess=.1;emitter.speed_scale=.02;emitter.restart()
 await shot("death")
 Engine.time_scale=1
 main.battle.spawn_enemy("vomiter")
 var vomiter=main.battle.enemies.back()
 vomiter.position=main.player.position+Vector3(0,0,-3)
 vomiter.take_damage(1000)
 await shot("spores")
 print("PASS VISUAL: uploaded fonts, HTML-style compass/notices, missions, aircraft, hit/death/spore effects")
 quit()
