extends Node3D
const Bridge=preload("res://scripts/bridge.gd")
const Soldier=preload("res://scripts/player.gd")
const Battle=preload("res://scripts/battle.gd")
var bridge=Bridge.new()
var battle
var screen: Control
var wave_label: Label
var notice_label: Label
var wallet_label: Label
var vignette: ColorRect
var store_panel: PanelContainer
var store_grid: GridContainer
var store_message: Label
var store_tab="weapon"
var store_feedback=""
var settings_panel: PanelContainer
var master_volume=1.0
var music_volume=.5
var calm_music: AudioStreamPlayer
var tense_music: AudioStreamPlayer
var sensitivity=1.0
var graphics_quality=1
var player: CharacterBody3D
var menu: PanelContainer
var hud: Label
var armory: OptionButton
var fullscreen=false
var stamina_bar: ProgressBar
var ammo_label: Label
var fps_label: Label
var bearing_label: Label
var compass_markers: Control
var compass_timer=0.0

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
 battle=Battle.new()
 battle.main=self
 add_child(battle)
 player.battle=battle
 setup_ui()
 setup_store()
 setup_music()
 set_playing(false)

func setup_inputs() -> void:
 var keys={"forward":KEY_W,"back":KEY_S,"left":KEY_A,"right":KEY_D,"jump":KEY_SPACE,"sprint":KEY_SHIFT,"reload":KEY_R,"flashlight":KEY_F,"pause_game":KEY_ESCAPE,"interact":KEY_E,"primary":KEY_1,"pistol":KEY_2,"trap":KEY_3,"small_medkit":KEY_H,"large_medkit":KEY_J,"battery":KEY_B}
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
 player.shot_sound.pitch_scale={"Pistol":1.1,"Shotgun":.65,"Vector":1.35,"Shambler":.8,"Tikhar":.9,"Kalash":.95,"Scorpion":1.25,"G36":1.0,"VSV":.85,"Helsing":1.4}.get(id,1.0)
 player.ammo=int(player.config.capacity)
 player.reserve=int(player.config.reserve)
 player.reloading=0
 player.cooldown=0
 player.gun.scale=Vector3.ONE*(.64 if id=="Pistol" else minf(1.15,float(player.config.get("viewZoom",1.0)))*.82)
 player.base_gun=Vector3(.19,-.19-float(player.config.get("viewLower",0))*.025,-.48)
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
 env.ambient_light_energy=.38
 env.tonemap_mode=Environment.TONE_MAPPER_FILMIC
 env.fog_enabled=true
 env.fog_light_color=Color("#34434a")
 env.fog_density=.009
 world_environment.environment=env
 add_child(world_environment)
 var sun=DirectionalLight3D.new()
 sun.name="Sun"
 sun.rotation_degrees=Vector3(-38,-28,0)
 sun.light_color=Color("#9daeb9")
 sun.light_cull_mask=1
 sun.light_energy=.65
 sun.shadow_enabled=true
 sun.directional_shadow_max_distance=90
 add_child(sun)
 var store_light=OmniLight3D.new()
 store_light.position=Vector3(bridge.data.terminal.x,2.5,bridge.data.terminal.z)
 store_light.light_color=Color("#d5b882")
 store_light.omni_range=9
 store_light.light_energy=1.3
 store_light.light_cull_mask=1
 add_child(store_light)

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
 node.pressed.connect(func(): if is_instance_valid(battle): battle.tone(600,.05))
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
 set_meta("post_material",material)
 effects.add_child(post)
 var overlay=CanvasLayer.new()
 overlay.layer=2
 add_child(overlay)
 screen=Control.new()
 var theme=Theme.new()
 theme.default_font=load("res://assets/terminal.ttf")
 theme.default_font_size=16
 theme.set_color("font_color","Label",Color("#d4dfba"))
 theme.set_color("font_color","Button",Color("#d4dfba"))
 theme.set_color("font_disabled_color","Button",Color("#67735e"))
 for state in ["normal","hover","pressed","disabled"]:
  var style=StyleBoxFlat.new()
  style.bg_color=Color("#24352a") if state=="hover" else Color("#0e1914") if state=="disabled" else Color("#18251d")
  style.border_color=Color("#a8b879") if state=="hover" else Color("#53664b")
  style.set_border_width_all(1)
  style.set_content_margin_all(12)
  theme.set_stylebox(state,"Button",style)
 screen.theme=theme
 screen.set_meta("post_material",get_meta("post_material"))
 screen.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 screen.mouse_filter=Control.MOUSE_FILTER_IGNORE
 overlay.add_child(screen)
 var crosshair=label("+",20)
 crosshair.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 crosshair.position-=Vector2(7,14)
 screen.add_child(crosshair)
 hud=label("SECTOR 00
COMBAT ZONE",18)
 hud.position=Vector2(28,28)
 screen.add_child(hud)
 setup_battle_hud()
 var panel=PanelContainer.new()
 panel.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_LEFT)
 panel.position=Vector2(24,-128)
 panel.custom_minimum_size=Vector2(300,100)
 var backing=StyleBoxFlat.new()
 backing.bg_color=Color(.035,.045,.05,.78)
 backing.set_content_margin_all(16)
 backing.set_corner_radius_all(5)
 panel.add_theme_stylebox_override("panel",backing)
 screen.add_child(panel)
 var vitals=VBoxContainer.new()
 vitals.add_theme_constant_override("separation",10)
 panel.add_child(vitals)
 vitals.add_child(label("STAMINA",16))
 stamina_bar=ProgressBar.new()
 var bar_fill=StyleBoxFlat.new()
 bar_fill.bg_color=Color("#b8bd87")
 bar_fill.set_corner_radius_all(3)
 stamina_bar.add_theme_stylebox_override("fill",bar_fill)
 var bar_back=StyleBoxFlat.new()
 bar_back.bg_color=Color("#303736")
 bar_back.set_corner_radius_all(3)
 stamina_bar.add_theme_stylebox_override("background",bar_back)
 stamina_bar.max_value=5
 stamina_bar.value=5
 stamina_bar.show_percentage=false
 stamina_bar.custom_minimum_size=Vector2(270,8)
 vitals.add_child(stamina_bar)
 ammo_label=label("",22)
 ammo_label.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_RIGHT)
 ammo_label.position=Vector2(-285,-125)
 ammo_label.custom_minimum_size=Vector2(255,90)
 ammo_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 screen.add_child(ammo_label)
 fps_label=label("",14)
 fps_label.set_anchors_and_offsets_preset(Control.PRESET_TOP_RIGHT)
 fps_label.position=Vector2(-130,12)
 fps_label.custom_minimum_size.x=115
 fps_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 screen.add_child(fps_label)
 bearing_label=label("",17)
 bearing_label.set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
 bearing_label.position=Vector2(-175,24)
 bearing_label.custom_minimum_size.x=350
 bearing_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
 screen.add_child(bearing_label)
 compass_markers=Control.new()
 compass_markers.set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
 compass_markers.position=Vector2(-250,54)
 compass_markers.mouse_filter=Control.MOUSE_FILTER_IGNORE
 compass_markers.add_to_group("match_hud")
 screen.add_child(compass_markers)
 var status=label("F · FLASHLIGHT   B · BATTERY   3 · TRAP   H / J · MEDKITS",15)
 status.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_LEFT)
 status.position=Vector2(24,-40)
 screen.add_child(status)
 menu=PanelContainer.new()
 menu.set_anchors_and_offsets_preset(Control.PRESET_CENTER_LEFT)
 menu.position=Vector2(64,-260)
 menu.custom_minimum_size=Vector2(560,520)
 var style=StyleBoxFlat.new()
 style.bg_color=Color(.035,.065,.05,.96)
 style.border_color=Color("#9ea971")
 style.set_border_width_all(1)
 style.set_content_margin_all(24)
 menu.add_theme_stylebox_override("panel",style)
 screen.add_child(menu)
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",14)
 menu.add_child(column)
 column.add_child(label("DEAD SECTOR",38))
 column.add_child(label("FIELD SYSTEMS / BUILD 0.2",16))
 column.add_child(label("Behind the perimeter — silence.\nWASD Move / Shift Sprint / Space Jump\nMouse Look / LMB Fire / R Reload\nF Flashlight / Esc Pause",17))
 column.add_child(button("PLAY / RESUME",func(): set_playing(true)))
 column.add_child(button("NEW MATCH",func(): get_tree().reload_current_scene()))
 column.add_child(button("SETTINGS",open_settings))
 column.add_child(button("FULLSCREEN / WINDOW",toggle_fullscreen))
 column.add_child(button("QUIT",func(): get_tree().quit()))
 for child in screen.get_children():
  if child!=menu: child.add_to_group("match_hud")

func toggle_fullscreen() -> void:
 fullscreen=not fullscreen
 DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN if fullscreen else DisplayServer.WINDOW_MODE_WINDOWED)

func set_playing(value: bool) -> void:
 if value and player.health<=0: return
 player.active=value
 player.gun.visible=value
 player.mouse_guard=.1
 if not value: player.flash.light_energy=0
 menu.visible=not value
 if is_instance_valid(settings_panel): settings_panel.visible=false
 Input.mouse_mode=Input.MOUSE_MODE_CAPTURED if value else Input.MOUSE_MODE_VISIBLE

func _unhandled_input(event: InputEvent) -> void:
 if event.is_action_pressed("pause_game"):
  if store_panel.visible: close_store()
  else: set_playing(not player.active)
 if event.is_action_pressed("interact") and player.active and near_store(): open_store()

func _notification(what: int) -> void:
 if what==NOTIFICATION_APPLICATION_FOCUS_OUT and is_instance_valid(menu): set_playing(false)

func _process(_delta: float) -> void:
 if not is_instance_valid(player) or not is_instance_valid(hud): return
 for control in get_tree().get_nodes_in_group("match_hud"): control.visible=player.active
 if player.health<=0: notice_label.show()
 if is_instance_valid(calm_music):
  var tense=player.active and battle.phase in ["ASSAULT","CLEARANCE"]
  calm_music.volume_db=lerpf(calm_music.volume_db,-60 if tense else linear_to_db(maxf(.001,music_volume)),minf(1,_delta*1.5))
  tense_music.volume_db=lerpf(tense_music.volume_db,linear_to_db(maxf(.001,music_volume)) if tense else -60,minf(1,_delta*1.5))
 stamina_bar.value=player.stamina
 wallet_label.text="HEALTH %d HP   $%d   CONDITION %d%%" % [player.health,battle.money,int(battle.inventory.get(player.config.id,{}).get("durability",100))]
 wave_label.text="WAVE %d · %s
%ds   ALIVE %d   SPAWNED %d/%d" % [battle.number,battle.phase,int(maxf(0,(60 if battle.phase=="PREPARATION" else 30 if battle.phase=="INTERMISSION" else 120)-battle.time)),battle.enemies.size(),battle.spawned,battle.total()]
 notice_label.text=battle.notice if battle.notice_time>0 else "E · ACCESS STORE" if player.active and near_store() else ""
 vignette.color=Color(0,0,0,0)
 screen.get_meta("post_material").set_shader_parameter("low_health",player.health>0 and player.health<25)
 ammo_label.text="%s
%d / %d
%s" % [player.config.id.to_upper(),player.ammo,player.reserve,"RELOADING" if player.reloading>0 else "R · RELOAD"]
 fps_label.text="%d FPS" % Engine.get_frames_per_second()
 var azimuth=fposmod(-rad_to_deg(player.rotation.y),360)
 var direction=["N","NE","E","SE","S","SW","W","NW"][int(round(azimuth/45))%8]
 bearing_label.text="·     %s    %03d°     ·" % [direction,int(azimuth)]
 compass_timer-=_delta
 if compass_timer<=0: compass_timer=.1;update_markers(azimuth)
 if player.position.y < -10: reset_position()

func setup_battle_hud() -> void:
 wave_label=label("",15)
 wave_label.set_anchors_and_offsets_preset(Control.PRESET_TOP_RIGHT)
 wave_label.position=Vector2(-380,95)
 screen.add_child(wave_label)
 wallet_label=label("",15)
 wallet_label.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_LEFT)
 wallet_label.position=Vector2(24,-165)
 screen.add_child(wallet_label)
 notice_label=label("",21)
 notice_label.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 notice_label.position=Vector2(-450,-150)
 notice_label.custom_minimum_size.x=900
 notice_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
 screen.add_child(notice_label)
 vignette=ColorRect.new()
 vignette.mouse_filter=Control.MOUSE_FILTER_IGNORE
 vignette.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 screen.add_child(vignette)

func near_store() -> bool:
 var terminal: Dictionary=bridge.data.terminal
 return Vector2(player.position.x-terminal.x,player.position.z-terminal.z).length()<3 and abs(player.position.y)<2

func show_death() -> void:
 set_playing(false)
 battle.notice="YOU DIED · SELECT NEW MATCH"
 battle.notice_time=99999

func setup_store() -> void:
 store_panel=PanelContainer.new()
 store_panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 store_panel.position=Vector2(-530,-325)
 store_panel.custom_minimum_size=Vector2(1060,650)
 var backing=StyleBoxFlat.new()
 backing.bg_color=Color(.035,.06,.045,.98)
 backing.set_content_margin_all(20)
 backing.border_color=Color("#8d9c69")
 backing.set_border_width_all(1)
 store_panel.add_theme_stylebox_override("panel",backing)
 screen.add_child(store_panel)
 var column=VBoxContainer.new()
 store_panel.add_child(column)
 column.add_child(label("SECTOR MART / SUPPLY TERMINAL",26))
 var tabs=HBoxContainer.new()
 column.add_child(tabs)
 tabs.add_child(button("WEAPONS",func(): store_tab="weapon";render_store()))
 tabs.add_child(button("SUPPLIES & REPAIR",func(): store_tab="supplies";render_store()))
 tabs.add_child(button("CLOSE",close_store))
 var scroll=ScrollContainer.new()
 scroll.vertical_scroll_mode=ScrollContainer.SCROLL_MODE_SHOW_NEVER
 scroll.horizontal_scroll_mode=ScrollContainer.SCROLL_MODE_DISABLED
 scroll.size_flags_vertical=Control.SIZE_EXPAND_FILL
 column.add_child(scroll)
 store_grid=GridContainer.new()
 store_grid.columns=3
 store_grid.add_theme_constant_override("h_separation",12)
 store_grid.add_theme_constant_override("v_separation",12)
 scroll.add_child(store_grid)
 store_message=label("",16)
 column.add_child(store_message)
 store_panel.hide()

func open_store() -> void:
 set_playing(false)
 menu.hide()
 store_panel.show()
 battle.store_open=true
 render_store()

func close_store() -> void:
 store_panel.hide()
 battle.store_open=false
 set_playing(true)

func render_store() -> void:
 for child in store_grid.get_children(): store_grid.remove_child(child);child.queue_free()
 var items: Array=bridge.data.shop_items.duplicate()
 items.sort_custom(func(a,b): return a.price<b.price)
 for item in items:
  if (item.kind=="weapon")!=(store_tab=="weapon"): continue
  var text="%s · $%d" % [item.id.to_upper(),int(item.price)]
  var locked=false
  if item.kind=="weapon":
   var config: Dictionary=bridge.data.weapons[item.id]
   text+="
DAMAGE %d / MAG %d
%.1f ROUNDS/SEC" % [config.damage,config.capacity,1.0/config.interval]
   locked=battle.completed<int(config.unlockAfter)
   if locked: text+="
LOCKED · AFTER WAVE %d" % int(config.unlockAfter)
  elif item.id=="ammo": text+="
50 UNIVERSAL ROUNDS / MAX 500"
  elif item.id=="smallMedkit": text+="
+25 HEALTH"
  elif item.id=="largeMedkit": text+="
+75 HEALTH"
  elif item.id=="repair": text+="
+50 WEAPON CONDITION"
  else: text+="
"+("120 SECONDS" if item.id in ["battery","flashlight"] else "300 DAMAGE / ENEMIES ONLY")
  var card=button(text,func(): store_feedback=battle.buy(item.id);render_store())
  var icon_id=item.id if item.kind=="weapon" else "medkit" if item.kind=="medkit" else "ammo" if item.id=="ammo" else "repair" if item.id=="repair" else "equipment"
  card.icon=load("res://assets/icons/"+icon_id+".svg")
  card.expand_icon=true
  card.add_theme_constant_override("icon_max_width",105)
  card.custom_minimum_size=Vector2(330,140)
  card.disabled=locked or battle.money<int(item.price)
  store_grid.add_child(card)
 store_message.text=store_feedback+"
BALANCE $%d · COMPLETE WAVE %d · 1 PRIMARY / 1 PISTOL" % [battle.money,battle.completed]

func open_settings() -> void:
 if is_instance_valid(settings_panel): settings_panel.queue_free()
 settings_panel=PanelContainer.new()
 settings_panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER_RIGHT)
 settings_panel.position=Vector2(-430,-230)
 settings_panel.custom_minimum_size=Vector2(410,460)
 var backing=StyleBoxFlat.new()
 backing.bg_color=Color(.035,.06,.045,.98)
 backing.set_content_margin_all(18)
 backing.border_color=Color("#75875b")
 backing.set_border_width_all(1)
 settings_panel.add_theme_stylebox_override("panel",backing)
 screen.add_child(settings_panel)
 var column=VBoxContainer.new()
 settings_panel.add_child(column)
 column.add_child(label("SETTINGS",26))
 column.add_child(label("MASTER VOLUME",16))
 var volume=HSlider.new()
 volume.min_value=0
 volume.max_value=1
 volume.step=.01
 volume.value=master_volume
 volume.value_changed.connect(func(value): master_volume=value;AudioServer.set_bus_volume_db(0,linear_to_db(maxf(.0001,value))))
 column.add_child(volume)
 column.add_child(label("MUSIC VOLUME",16))
 var music=HSlider.new()
 music.min_value=0
 music.max_value=1
 music.step=.01
 music.value=music_volume
 music.value_changed.connect(func(value): music_volume=value)
 column.add_child(music)
 column.add_child(label("MOUSE SENSITIVITY",16))
 var mouse=HSlider.new()
 mouse.min_value=.2
 mouse.max_value=3
 mouse.step=.05
 mouse.value=sensitivity
 mouse.value_changed.connect(func(value): sensitivity=value)
 column.add_child(mouse)
 column.add_child(label("GRAPHICS QUALITY",16))
 var quality=OptionButton.new()
 for text in ["LOW","MEDIUM","HIGH"]: quality.add_item(text)
 quality.select(graphics_quality)
 quality.item_selected.connect(func(index):
  graphics_quality=index
  get_node("Sun").shadow_enabled=index>0
  get_node("Sun").directional_shadow_max_distance=[60,90,120][index]
  player.camera.far=[350,600,650][index])
 column.add_child(quality)
 column.add_child(label("WASD · MOVE
SHIFT · SPRINT
SPACE · JUMP
1 / 2 · WEAPONS
3 / LMB · TRAP
H / J · MEDKITS
E · STORE
F / B · LIGHT / BATTERY",16))
 column.add_child(button("CLOSE",func(): settings_panel.hide()))

func setup_music() -> void:
 for tense in [false,true]:
  var stream=AudioStreamWAV.new()
  stream.mix_rate=22050
  stream.format=AudioStreamWAV.FORMAT_16_BITS
  stream.loop_mode=AudioStreamWAV.LOOP_FORWARD
  var samples=22050*8
  stream.loop_end=samples
  var bytes=PackedByteArray()
  bytes.resize(samples*2)
  for i in range(samples):
   var t=float(i)/22050
   var note=[55.0,65.406,49.0,58.27][int(t/2)%4]
   var value=(sin(t*TAU*note)+sin(t*TAU*note*1.5)*.3)*.04
   if tense: value+=(sin(t*TAU*110)*.05+sin(t*TAU*220)*.02)*exp(-fmod(t,.25)*22)
   var fade=minf(1,minf(t*8,(8-t)*8))
   bytes.encode_s16(i*2,int(value*fade*32767))
  stream.data=bytes
  var audio=AudioStreamPlayer.new()
  audio.stream=stream
  audio.volume_db=-60
  add_child(audio)
  audio.play()
  if tense: tense_music=audio
  else: calm_music=audio

func update_markers(azimuth: float) -> void:
 for marker in compass_markers.get_children(): compass_markers.remove_child(marker);marker.queue_free()
 var store_position=Vector3(bridge.data.terminal.x,0,bridge.data.terminal.z)
 var targets=[{"position":store_position,"store":true}]
 for enemy in battle.enemies:
  if battle.phase=="CLEARANCE" or enemy.position.distance_to(player.position)<25: targets.append({"position":enemy.position,"store":false})
 for target in targets:
  var offset=target.position-player.position
  var angle=wrapf(rad_to_deg(atan2(offset.x,-offset.z))-azimuth,-180,180)
  if absf(angle)>75: continue
  var marker=label("MART %dm" % int(offset.length()) if target.store else "■",12)
  marker.position=Vector2(250+angle*3,0)
  marker.add_theme_color_override("font_color",Color("#e0c37b") if target.store else Color("#d65e4b"))
  compass_markers.add_child(marker)
