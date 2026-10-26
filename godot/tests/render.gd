extends SceneTree
func _initialize() -> void:
 call_deferred("run")
func run() -> void:
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 for i in range(8): await process_frame
 await RenderingServer.frame_post_draw
 root.get_texture().get_image().save_png("user://godot-menu.png")
 scene.set_playing(true)
 scene.player.rotation.y=-.4
 for id in scene.bridge.data.weapons:
  scene.equip(id)
  for i in range(3): await process_frame
  await RenderingServer.frame_post_draw
  var image=root.get_texture().get_image()
  image.save_png("user://godot-"+id+".png")
  print("Rendered weapon: ",id)
 quit()
