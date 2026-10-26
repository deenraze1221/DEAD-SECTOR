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
 scene.player.position=Vector3(10.35,.05,24)
 scene.player.rotation.y=PI
 for i in range(3): await process_frame
 await RenderingServer.frame_post_draw
 root.get_texture().get_image().save_png("user://godot-street.png")
 scene.player.rotation.y=-.4
 for id in (["Pistol"] if "--quick" in OS.get_cmdline_user_args() else scene.bridge.data.weapons.keys()):
  scene.equip(id)
  for i in range(3): await process_frame
  await RenderingServer.frame_post_draw
  var image=root.get_texture().get_image()
  image.save_png("user://godot-"+id+".png")
  print("Rendered weapon: ",id)
 quit()
