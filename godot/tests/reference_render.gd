extends SceneTree
func _initialize() -> void: call_deferred("run")
func capture(name: String) -> void:
 for i in range(5): await process_frame
 await RenderingServer.frame_post_draw
 root.get_texture().get_image().save_png("user://reference-"+name+".png")
func run() -> void:
 var main=load("res://main.tscn").instantiate()
 root.add_child(main)
 await capture("menu")
 main.open_settings()
 await capture("settings")
 main.settings_panel.hide()
 main.open_store()
 await capture("locked")
 main.battle.money=100000
 main.battle.completed=25
 main.render_store()
 await capture("unlocked")
 main.store_tab="supplies"
 main.render_store()
 await capture("supplies")
 main.close_store()
 main.battle.time=-1000
 main.battle.notice_time=0
 main.player.position=Vector3(24,.05,30)
 main.player.rotation.y=-.4
 main.climate.kind="overcast"
 await capture("hud")
 main.set_playing(false)
 await capture("pause")
 print("PASS REFERENCE RENDER: menu, settings, both store tabs, HTML pistol and pause/HUD")
 quit()
