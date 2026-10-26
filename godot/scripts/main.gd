extends Node3D
const Bridge=preload("res://scripts/bridge.gd")
const Soldier=preload("res://scripts/player.gd")
var bridge=Bridge.new()
var player: CharacterBody3D
var menu: PanelContainer
var hud: Label
var armory: OptionButton
var fullscreen=false

func _ready() -> void:
 setup_inputs()
 bridge.load_data()
 var world=Node3D.new()
 world.name="City"
 add_child(world)
 bridge.build_world(world)
 setup_environment()
 player=Soldier.new()
 add_child(player)
 reset_position()
 equip("Pistol")
 add_targets()
 setup_ui()
 set_playing(false)

func setup_inputs() -> void:
 var keys={"forward":KEY_W,"back":KEY_S,"left":KEY_A,"right":KEY_D,"jump":KEY_SPACE,"sprint":KEY_SHIFT,"reload":KEY_R,"flashlight":KEY_F,"pause_game":KEY_ESCAPE}
 for action in keys:
  InputMap.add_action(action)
  var event=InputEventKey.new()
  event.physical_keycode=keys[action]
  InputMap.action_add_event(action,event)
 InputMap.add_action("fire")
 var mouse=InputEventMouseButton.new()
 mouse.button_index=MOUSE_BUTTON_LEFT
 InputMap.action_add_event("fire",mouse)

func reset_position() -> void:
 var spawn: Dictionary=bridge.data.spawn
 player.position=Vector3(spawn.x,spawn.get("y",0)+.1,spawn.z)
 player.velocity=Vector3.ZERO
 player.rotation.y=0
 player.camera.rotation.x=0

func equip(id: String) -> void:
 for child in player.gun.get_children():
  player.gun.remove_child(child)
  child.queue_free()
 bridge.build_weapon(player.gun,id)
 player.config=bridge.data.weapons[id]
 player.ammo=int(player.config.capacity)
 player.reserve=int(player.config.reserve)
 player.reloading=0
 player.cooldown=0
 player.gun.scale=Vector3.ONE*float(player.config.get("viewZoom",1.0))
 player.base_gun=Vector3(.22,-.22-float(player.config.get("viewLower",0))*.05,-.38)
 player.gun.position=player.base_gun

func setup_environment() -> void:
 var world_environment=WorldEnvironment.new()
 var env=Environment.new()
 env.background_mode=Environment.BG_SKY
 var sky=Sky.new()
 var sky_material=ProceduralSkyMaterial.new()
 sky_material.sky_top_color=Color("#18232c")
 sky_material.sky_horizon_color=Color("#59636a")
 sky_material.ground_bottom_color=Color("#151d19")
 sky_material.ground_horizon_color=Color("#59636a")
 sky.sky_material=sky_material
 env.sky=sky
 env.ambient_light_source=Environment.AMBIENT_SOURCE_COLOR
 env.ambient_light_color=Color("#87959c")
 env.ambient_light_energy=.7
 env.tonemap_mode=Environment.TONE_MAPPER_FILMIC
 env.fog_enabled=true
 env.fog_light_color=Color("#34434a")
 env.fog_density=.006
 world_environment.environment=env
 add_child(world_environment)
 var sun=DirectionalLight3D.new()
 sun.rotation_degrees=Vector3(-38,-28,0)
 sun.light_color=Color("#bac9d2")
 sun.light_energy=1.1
 sun.shadow_enabled=true
 sun.directional_shadow_max_distance=90
 add_child(sun)

func add_targets() -> void:
 for i in range(3):
  var target=StaticBody3D.new()
  target.set_meta("target",true)
  target.set_meta("hp",250)
  target.position=player.position+Vector3(2+i*1.8,.9,-5)
  var mesh=MeshInstance3D.new()
  var box=BoxMesh.new()
  box.size=Vector3(.6,1.4,.3)
  mesh.mesh=box
  var mat=StandardMaterial3D.new()
  mat.albedo_color=Color("#a45b4c")
  mesh.material_override=mat
  target.add_child(mesh)
  var collision=CollisionShape3D.new()
  var shape=BoxShape3D.new()
  shape.size=box.size
  collision.shape=shape
  target.add_child(collision)
  add_child(target)

func label(text: String, size: int) -> Label:
 var node=Label.new()
 node.text=text
 node.add_theme_font_size_override("font_size",size)
 return node

func button(text: String, callback: Callable) -> Button:
 var node=Button.new()
 node.text=text
 node.custom_minimum_size=Vector2(0,48)
 node.pressed.connect(callback)
 return node

func setup_ui() -> void:
 var effects=CanvasLayer.new()
 effects.layer=1
 add_child(effects)
 var post=ColorRect.new()
 post.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 post.mouse_filter=Control.MOUSE_FILTER_IGNORE
 var material=ShaderMaterial.new()
 material.shader=load("res://shaders/pixels.gdshader")
 post.material=material
 effects.add_child(post)
 var overlay=CanvasLayer.new()
 overlay.layer=2
 add_child(overlay)
 var screen=Control.new()
 screen.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 screen.mouse_filter=Control.MOUSE_FILTER_IGNORE
 overlay.add_child(screen)
 var crosshair=label("+",20)
 crosshair.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 crosshair.position-=Vector2(7,14)
 screen.add_child(crosshair)
 hud=label("",20)
 hud.position=Vector2(24,24)
 screen.add_child(hud)
 var status=label("GODOT PORT · STAGE 1 / TRAINING BUILD",15)
 status.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_LEFT)
 status.position=Vector2(24,-40)
 screen.add_child(status)
 menu=PanelContainer.new()
 menu.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 menu.position=Vector2(-280,-260)
 menu.custom_minimum_size=Vector2(560,520)
 var style=StyleBoxFlat.new()
 style.bg_color=Color(.035,.045,.05,.97)
 style.border_color=Color("#9ea971")
 style.set_border_width_all(1)
 style.set_content_margin_all(24)
 menu.add_theme_stylebox_override("panel",style)
 screen.add_child(menu)
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",14)
 menu.add_child(column)
 column.add_child(label("DEAD SECTOR",38))
 column.add_child(label("NATIVE GODOT · FIRST PLAYABLE PORT",16))
 column.add_child(label("Original city · 250 × 250 m · 10 weapons\nWASD Move / Shift Sprint / Space Jump\nMouse Look / LMB Fire / R Reload\nF Test flashlight / Esc Pause",17))
 column.add_child(button("PLAY / RESUME",func(): set_playing(true)))
 column.add_child(label("TEST ARMORY — NOT THE FINAL SHOP",14))
 armory=OptionButton.new()
 for id in bridge.data.weapons: armory.add_item(id)
 armory.item_selected.connect(func(index): equip(armory.get_item_text(index)))
 column.add_child(armory)
 column.add_child(button("RETURN TO STORE SPAWN",func(): reset_position();set_playing(true)))
 column.add_child(button("FULLSCREEN / WINDOW",toggle_fullscreen))
 column.add_child(button("QUIT",func(): get_tree().quit()))

func toggle_fullscreen() -> void:
 fullscreen=not fullscreen
 DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN if fullscreen else DisplayServer.WINDOW_MODE_WINDOWED)

func set_playing(value: bool) -> void:
 player.active=value
 menu.visible=not value
 Input.mouse_mode=Input.MOUSE_MODE_CAPTURED if value else Input.MOUSE_MODE_VISIBLE

func _unhandled_input(event: InputEvent) -> void:
 if event.is_action_pressed("pause_game"): set_playing(not player.active)

func _notification(what: int) -> void:
 if what==NOTIFICATION_APPLICATION_FOCUS_OUT and is_instance_valid(menu): set_playing(false)

func _process(_delta: float) -> void:
 if not is_instance_valid(player) or not is_instance_valid(hud): return
 hud.text="SECTOR 00   |   %s\nHEALTH 100   STAMINA %d%%\nAMMO %d / %d   %s\n%d FPS" % [player.config.id,roundi(player.stamina*20),player.ammo,player.reserve,"RELOADING" if player.reloading>0 else "",Engine.get_frames_per_second()]
 if player.position.y < -10: reset_position()
