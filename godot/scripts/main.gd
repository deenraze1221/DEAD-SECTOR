extends Node3D
const Generator=preload("res://scripts/city_generator.gd")
const Climate=preload("res://scripts/climate.gd")
const Soundscape=preload("res://scripts/soundscape.gd")
const Body=preload("res://scripts/local_body.gd")
const Bridge=preload("res://scripts/bridge.gd")
const Soldier=preload("res://scripts/player.gd")
const Battle=preload("res://scripts/battle.gd")
var bridge=Bridge.new()
var battle
var climate
var sound
var body
var generation: Dictionary
var map_seed=0
var weather_volume=.7
var show_fps=true
var screen: Control
var wave_label: Label
var notice_label: Label
var wallet_label: Label
var vignette: ColorRect
var gear_label: Label
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
 load_user_settings()
 setup_inputs()
 bridge.load_data()
 var seed_value=map_seed if map_seed!=0 else int(Time.get_unix_time_from_system()*1000)%2147483647
 generation=Generator.new().apply(bridge.data,seed_value,bridge)
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
 climate=Climate.new()
 climate.main=self
 add_child(climate)
 body=Body.new()
 player.add_child(body)
 body.setup(player,bridge)
 apply_quality()
 set_playing(false)

func setup_inputs() -> void:
 var keys={"forward":KEY_W,"back":KEY_S,"left":KEY_A,"right":KEY_D,"jump":KEY_SPACE,"sprint":KEY_SHIFT,"reload":KEY_R,"flashlight":KEY_F,"pause_game":KEY_ESCAPE,"interact":KEY_E,"primary":KEY_1,"pistol":KEY_2,"trap":KEY_3,"small_medkit":KEY_H,"large_medkit":KEY_J,"battery":KEY_B}
 for action in keys:
  if not InputMap.has_action(action): InputMap.add_action(action)
  InputMap.action_erase_events(action)
  var event=InputEventKey.new()
  event.physical_keycode=keys[action]
  InputMap.action_add_event(action,event)
 if not InputMap.has_action("fire"): InputMap.add_action("fire")
 InputMap.action_erase_events("fire")
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
 player.gun.scale=Vector3.ONE*(.64 if id=="Pistol" else minf(1.15,float(player.config.get("viewZoom",1.0)))*.82)
 player.base_gun=Vector3(.19,-.19-float(player.config.get("viewLower",0))*.025,-.48)
 player.gun.position=player.base_gun

func setup_environment() -> void:
 var world_environment=WorldEnvironment.new()
 world_environment.name="WorldEnvironment"
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
 node.pressed.connect(func(): if is_instance_valid(sound): sound.play("ui-click"))
 node.mouse_entered.connect(func(): if is_instance_valid(sound): sound.play("ui-hover",null,.5))
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
 theme.set_color("font_shadow_color","Label",Color(0,0,0,.8))
 theme.set_constant("shadow_offset_x","Label",1)
 theme.set_constant("shadow_offset_y","Label",1)
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
 status.position=Vector2(24,-220)
 status.add_theme_font_size_override("font_size",12)
 gear_label=status
 screen.add_child(status)
 menu=PanelContainer.new()
 menu.set_anchors_and_offsets_preset(Control.PRESET_CENTER_LEFT)
 menu.position=Vector2(64,-300)
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
 var title=label("DEAD
SECTOR®",64)
 title.add_theme_font_override("font",load("res://assets/title.ttf"))
 title.add_theme_constant_override("line_spacing",-8)
 title.add_theme_color_override("font_color",Color("#c6db78"))
 column.add_child(title)
 column.add_child(label("FIELD SYSTEMS / BUILD 0.3",16))
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
 stamina_bar.value=player.stamina
 gear_label.text="1 · %s   2 · PISTOL   3 · TRAPS ×%d   H/J · MEDKITS %d/%d
F · LIGHT %d%%   B · BATTERIES ×%d" % [battle.primary.to_upper() if battle.primary!="" else "EMPTY",battle.traps,battle.medkits[0],battle.medkits[1],int(battle.battery/120*100),battle.batteries]
 wallet_label.text="HEALTH %d HP   $%d   CONDITION %d%%" % [player.health,battle.money,int(battle.inventory.get(player.config.id,{}).get("durability",100))]
 wave_label.text="WAVE %d · %s
%ds   ALIVE %d   SPAWNED %d/%d" % [battle.number,battle.phase,int(maxf(0,(60 if battle.phase=="PREPARATION" else 30 if battle.phase=="INTERMISSION" else 120)-battle.time)),battle.enemies.size(),battle.spawned,battle.total()]
 notice_label.text=battle.notice if battle.notice_time>0 else "E · ACCESS STORE" if player.active and near_store() else ""
 vignette.color=Color(0,0,0,0)
 screen.get_meta("post_material").set_shader_parameter("low_health",player.health>0 and player.health<25)
 ammo_label.text="%s
%d / %d
%s" % [player.config.id.to_upper(),player.ammo,player.reserve,"RELOADING" if player.reloading>0 else "R · RELOAD"]
 fps_label.text="%d FPS" % Engine.get_frames_per_second() if show_fps else ""
 if climate!=null: hud.text="SECTOR 00
COMBAT ZONE
%02d:%02d · %s
%s · %s fog" % [int(climate.hour),int(fmod(climate.hour,1)*60),"NIGHT" if climate.night else "DAY",climate.kind.to_upper(),climate.fog.to_upper()]
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
  var display_name={"ammo":"UNIVERSAL AMMO","smallMedkit":"SMALL MEDKIT","largeMedkit":"LARGE MEDKIT","flashlight":"FLASHLIGHT","battery":"BATTERY","trap":"BEAR TRAP","repair":"REPAIR WEAPON"}.get(item.id,item.id.to_upper())
  var text="%s · $%d" % [display_name,int(item.price)]
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
 settings_panel.position=Vector2(-570,-300)
 settings_panel.custom_minimum_size=Vector2(540,600)
 var backing=StyleBoxFlat.new()
 backing.bg_color=Color(.035,.06,.045,.98)
 backing.set_content_margin_all(20)
 backing.set_border_width_all(1)
 backing.border_color=Color("#75875b")
 settings_panel.add_theme_stylebox_override("panel",backing)
 screen.add_child(settings_panel)
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",16)
 settings_panel.add_child(column)
 column.add_child(label("SETTINGS",26))
 var tabs=TabContainer.new()
 tabs.size_flags_vertical=Control.SIZE_EXPAND_FILL
 column.add_child(tabs)
 var panels={}
 for title in ["Audio","Controls","Graphics","World"]:
  var page=VBoxContainer.new()
  page.name=title
  page.add_theme_constant_override("separation",18)
  tabs.add_child(page)
  panels[title]=page
 for setting in [["MASTER VOLUME","master_volume"],["MUSIC VOLUME","music_volume"],["WEATHER VOLUME","weather_volume"]]:
  panels.Audio.add_child(label(setting[0],16))
  var slider=HSlider.new()
  slider.min_value=0
  slider.max_value=1
  slider.step=.01
  slider.value=get(setting[1])
  slider.value_changed.connect(func(value): set(setting[1],value);save_user_settings())
  panels.Audio.add_child(slider)
 panels.Controls.add_child(label("MOUSE SENSITIVITY",16))
 var mouse=HSlider.new()
 mouse.min_value=.2
 mouse.max_value=3
 mouse.step=.05
 mouse.value=sensitivity
 mouse.value_changed.connect(func(value): sensitivity=value;save_user_settings())
 panels.Controls.add_child(mouse)
 panels.Controls.add_child(label("WASD · MOVE
SHIFT · SPRINT
SPACE · JUMP
1 / 2 · PRIMARY / PISTOL
3 / LMB · TRAP
R · RELOAD
H / J · MEDKITS
E · STORE
F / B · FLASHLIGHT / BATTERY
ESC · PAUSE",16))
 panels.Graphics.add_child(label("GRAPHICS QUALITY",16))
 var quality=OptionButton.new()
 for text in ["LOW","MEDIUM","HIGH"]: quality.add_item(text)
 quality.select(graphics_quality)
 quality.item_selected.connect(func(index): graphics_quality=index;apply_quality();save_user_settings())
 panels.Graphics.add_child(quality)
 var metrics=CheckButton.new()
 metrics.text="SHOW FPS"
 metrics.button_pressed=show_fps
 metrics.toggled.connect(func(value): show_fps=value;save_user_settings())
 panels.Graphics.add_child(metrics)
 panels.Graphics.add_child(label("PSX COLOR / PIXEL EFFECT: ALWAYS ON",14))
 panels.Graphics.add_child(button("FULLSCREEN / WINDOW",toggle_fullscreen))
 panels.World.add_child(label("CITY SEED (0 = RANDOM EACH MATCH)",15))
 var seed_input=LineEdit.new()
 seed_input.text=str(map_seed)
 seed_input.text_changed.connect(func(value):
  if value.is_valid_int(): map_seed=int(value);save_user_settings())
 panels.World.add_child(seed_input)
 panels.World.add_child(label("CURRENT SEED: %d
%d BLOCKS REASSEMBLED

Seed changes apply to NEW MATCH.
Roads and central store stay fixed.
Buildings and interiors move together.

Interface language: English" % [generation.seed,generation.moved_blocks],15))
 column.add_child(button("CLOSE",func(): settings_panel.hide()))

func load_user_settings() -> void:
 var config=ConfigFile.new()
 if config.load("user://settings.cfg")!=OK: return
 for key in ["master_volume","music_volume","weather_volume"]:
  set(key,clampf(float(config.get_value("settings",key,get(key))),0,1))
 sensitivity=clampf(float(config.get_value("settings","sensitivity",1.0)),.2,3)
 graphics_quality=clampi(int(config.get_value("settings","graphics_quality",1)),0,2)
 map_seed=int(config.get_value("settings","map_seed",0))
 show_fps=bool(config.get_value("settings","show_fps",true))
 AudioServer.set_bus_volume_db(0,linear_to_db(maxf(.0001,master_volume)))

func save_user_settings() -> void:
 var config=ConfigFile.new()
 for key in ["master_volume","music_volume","weather_volume","sensitivity","graphics_quality","map_seed","show_fps"]:
  config.set_value("settings",key,get(key))
 config.save("user://settings.cfg")
 AudioServer.set_bus_volume_db(0,linear_to_db(maxf(.0001,master_volume)))

func apply_quality() -> void:
 get_node("Sun").shadow_enabled=graphics_quality>0
 get_node("Sun").directional_shadow_max_distance=[60,90,120][graphics_quality]
 player.camera.far=[350,600,650][graphics_quality]

func setup_music() -> void:
 sound=Soundscape.new()
 sound.main=self
 add_child(sound)

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
