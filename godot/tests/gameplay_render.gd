extends SceneTree
func _initialize() -> void: call_deferred("run")
func shot(name: String) -> void:
 for i in range(3): await process_frame
 await RenderingServer.frame_post_draw
 root.get_texture().get_image().save_png("user://"+name+".png")
func run() -> void:
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 await shot("stage2-menu")
 scene.open_settings()
 await shot("stage2-settings")
 scene.settings_panel.hide()
 scene.open_store()
 await shot("stage2-store-locked")
 scene.battle.money=100000
 scene.battle.completed=25
 scene.render_store()
 await shot("stage2-store-unlocked")
 scene.store_tab="supplies"
 scene.render_store()
 await shot("stage2-supplies")
 scene.close_store()
 scene.player.active=false
 scene.player.gun.visible=true
 scene.player.position=Vector3(24,0,32)
 scene.player.rotation.y=0
 for type in scene.bridge.data.enemy_types:
  scene.battle.spawn_enemy(type)
  var enemy=scene.battle.enemies.back()
  enemy.position=Vector3(24,4.5 if type=="eye" else 0,13 if type in ["boss","brute"] else 26)
  enemy.rotation.y=0
  await shot("stage2-"+type)
  scene.battle.enemies.erase(enemy)
  enemy.queue_free()
 print("PASS RENDER: menu, locked/unlocked shop, supplies and six creature models")
 quit()
