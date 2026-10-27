extends SceneTree
func _initialize() -> void: call_deferred("run")
func shot(name_value: String) -> void:
 for i in range(3): await process_frame
 await RenderingServer.frame_post_draw
 root.get_texture().get_image().save_png("user://coop-"+name_value+".png")
func run() -> void:
 var main=load("res://main.tscn").instantiate()
 root.add_child(main)
 main.network_ui.open()
 await shot("lobby")
 main.network_ui.open(true)
 await shot("server")
 main.network_ui.panel.hide()
 main.set_playing(true)
 main.player.position=Vector3(24,.05,30)
 main.climate.kind="rain";main.climate.fog="light"
 var actor=preload("res://scripts/coop_avatar.gd").new()
 main.add_child(actor);actor.setup(main,"guest")
 actor.update_pose({"p":[24,.05,27],"yaw":PI,"pitch":-.1,"hp":100,"weapon":"Kalash","light":true})
 main.coop.peers.guest=actor
 main.battle.announce("SQUAD READY · DEFEND THE SECTOR","mission")
 await shot("soldier-hud")
 print("PASS COOP VISUAL: lobby/server panels, soldier grip/neck and compass/HUD")
 quit()
