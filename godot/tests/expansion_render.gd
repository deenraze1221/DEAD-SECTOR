extends SceneTree
func _initialize() -> void: call_deferred("run")
func shot(name: String) -> void:
 for i in range(4): await process_frame
 await RenderingServer.frame_post_draw
 root.get_texture().get_image().save_png("user://"+name+".png")
func run() -> void:
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 await shot("expansion-menu")
 scene.open_settings()
 await shot("expansion-settings")
 scene.set_playing(true)
 scene.player.position=Vector3(24,.05,30)
 scene.player.rotation.y=-.4
 scene.battle.time=60
 scene.battle.phase="ASSAULT"
 scene.climate.kind="overcast"
 scene.climate.fog="light"
 await shot("expansion-day")
 scene.player.camera.rotation.x=-PI/3
 await shot("expansion-body")
 scene.player.camera.rotation.x=0
 scene.climate.kind="rain"
 scene.climate.fog="medium"
 await shot("expansion-rain")
 scene.battle.number=3
 scene.climate.roll(3)
 scene.climate.kind="storm"
 scene.climate.flash_timer=.2
 scene.battle.time=60
 await shot("expansion-night")
 print("PASS VISUAL: generated city, settings, soldier body, rain and night/storm shaders")
 quit()
