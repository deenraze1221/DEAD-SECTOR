extends Node3D
const ReferenceUI=preload("res://scripts/reference_ui.gd")
var reference_ui
const Interiors=preload("res://scripts/interior_layout.gd")
const Objectives=preload("res://scripts/objectives.gd")
var objectives
var net
var coop
var network_ui
var atmosphere
var combat_fx
var interior_report: Dictionary
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
 net=get_node("/root/CoopNetwork")
 load_user_settings()
 setup_inputs()
 bridge.load_data()
 for type in ["zombie","runner","vomiter","brute","boss"]: bridge.data.enemy_types[type].speed*=1.05
 var seed_value=net.seed_value if net.match_active else map_seed if map_seed!=0 else int(Time.get_unix_time_from_system()*1000)%2147483647
 var interiors=Interiors.new()
 interiors.capture(bridge.data,bridge)
 generation=Generator.new().apply(bridge.data,seed_value,bridge)
 interior_report=interiors.rebuild(bridge.data,seed_value)
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
 atmosphere=preload("res://scripts/atmosphere.gd").new()
 atmosphere.main=self
 add_child(atmosphere)
 objectives=Objectives.new()
 objectives.main=self
 add_child(objectives)
 combat_fx=preload("res://scripts/combat_fx.gd").new()
 combat_fx.main=self
 add_child(combat_fx)
 apply_quality()
 coop=preload("res://scripts/coop.gd").new()
 coop.main=self
 add_child(coop)
 network_ui=preload("res://scripts/network_ui.gd").new()
 network_ui.build(self)
 if net.match_active:
  var slot=maxi(0,net.members.find_custom(func(m): return str(m.id)==net.peer_id))
  player.position+=Vector3(slot*1.1,0,0)
  set_playing(true)
 else: set_playing(false)

func world_running() -> bool:
 if net.match_active and net.active: return coop!=null and not coop.paused_for_disconnect
 return player.active

func new_match() -> void:
 net.close()
 get_tree().reload_current_scene()

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
 for node in player.gun.find_children("*","MeshInstance3D",true,false): node.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_ON
 player.config=bridge.data.weapons[id]
 player.ammo=int(player.config.capacity)
 player.reserve=int(player.config.reserve)
 player.reloading=0
 player.cooldown=0
 var zoom=float(player.config.get("viewZoom",1.0))
 player.weapon_camera.fov=rad_to_deg(2*atan(tan(deg_to_rad(45)*.5)/zoom))
 player.gun.scale=Vector3.ONE
 player.base_gun=Vector3(.14/zoom,-.115/zoom,-.5)
 player.gun.position=player.base_gun

func setup_environment() -> void:
 var world_environment=WorldEnvironment.new()
 world_environment.name="WorldEnvironment"
 var env=Environment.new()
 env.background_mode=Environment.BG_SKY
 var sky=Sky.new()
 var sky_material=ProceduralSkyMaterial.new()
 sky_material.sky_top_color=Color("#0f171d")
 sky_material.sky_horizon_color=Color("#424b50")
 sky_material.ground_bottom_color=Color("#0c1210")
 sky_material.ground_horizon_color=Color("#424b50")
 sky_material.sky_curve=.12
 sky.sky_material=sky_material
 env.sky=sky
 env.ambient_light_source=Environment.AMBIENT_SOURCE_COLOR
 env.ambient_light_color=Color("#74828a")
 env.ambient_light_energy=.46
 env.tonemap_mode=Environment.TONE_MAPPER_FILMIC
 env.tonemap_exposure=1.0
 env.tonemap_white=5.0
 env.adjustment_enabled=true
 env.adjustment_contrast=1.1
 env.adjustment_saturation=.8
 env.adjustment_brightness=1.0
 env.glow_enabled=true
 env.glow_intensity=.55
 env.glow_strength=.9
 env.glow_bloom=.04
 env.glow_hdr_threshold=1.05
 env.fog_enabled=true
 env.fog_light_color=Color("#2e3b42")
 env.fog_density=.012
 env.fog_sky_affect=1.0
 world_environment.environment=env
 add_child(world_environment)
 var sun=DirectionalLight3D.new()
 sun.name="Sun"
 sun.rotation_degrees=Vector3(-38,-28,0)
 sun.light_color=Color("#8d9fab")
 sun.light_cull_mask=1
 sun.light_energy=.72
 sun.shadow_enabled=true
 sun.shadow_blur=1.4
 sun.directional_shadow_mode=DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
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
 reference_ui=ReferenceUI.new()
 reference_ui.main=self
 reference_ui.build()

func toggle_fullscreen() -> void:
 fullscreen=not fullscreen
 DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN if fullscreen else DisplayServer.WINDOW_MODE_WINDOWED)

func set_playing(value: bool) -> void:
 if value and player.health<=0: return
 if value:
  reference_ui.started=true
  player.camera.make_current()
 player.active=value
 player.gun.visible=value or reference_ui.started
 player.mouse_guard=.1
 if not value: player.flash.light_energy=0
 menu.visible=not value
 if is_instance_valid(settings_panel): settings_panel.visible=false
 Input.mouse_mode=Input.MOUSE_MODE_CAPTURED if value else Input.MOUSE_MODE_VISIBLE

func _unhandled_input(event: InputEvent) -> void:
 if event.is_action_pressed("pause_game"):
  if network_ui!=null and network_ui.panel.visible: network_ui.panel.hide();menu.show()
  elif store_panel.visible: close_store()
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
 notice_label.text=battle.notice if battle.notice_time>0 else ""
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
 reference_ui.refresh(_delta)

func near_store() -> bool:
 var terminal: Dictionary=bridge.data.terminal
 return Vector2(player.position.x-terminal.x,player.position.z-terminal.z).length()<3 and abs(player.position.y)<2

func show_death() -> void:
 set_playing(false)
 battle.notice="YOU ARE DOWN · A SQUAD MEMBER CAN REVIVE YOU" if net.active else "YOU DIED · SELECT NEW MATCH"
 battle.notice_time=99999

func setup_store() -> void:
 reference_ui.build_store()

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
 reference_ui.render_store()

func open_settings() -> void:
 if is_instance_valid(settings_panel): settings_panel.queue_free()
 settings_panel=PanelContainer.new()
 settings_panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER_RIGHT)
 settings_panel.position=Vector2(-570,-320)
 settings_panel.custom_minimum_size=Vector2(540,0)
 var theme=screen.theme.duplicate()
 theme.default_font_size=16
 settings_panel.theme=theme
 # HTML look: dark glass panel, soft outline, thin accent line on top, 14 px corners.
 var backing=StyleBoxFlat.new()
 backing.bg_color=Color(.047,.07,.09,.96)
 backing.set_content_margin_all(26)
 backing.set_border_width_all(1)
 backing.border_width_top=2
 backing.border_color=Color(.7,.78,.78,.22)
 backing.set_corner_radius_all(14)
 settings_panel.add_theme_stylebox_override("panel",backing)
 screen.add_child(settings_panel)
 settings_panel.resized.connect(func(): if reference_ui!=null: reference_ui.layout())
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",16)
 settings_panel.add_child(column)
 var header=HBoxContainer.new()
 var title_label=reference_ui.tracked_text("SETTINGS",26,Color("#e8eeeb"),3)
 title_label.size_flags_horizontal=Control.SIZE_EXPAND_FILL
 header.add_child(title_label)
 var close_button=Button.new()
 close_button.text="×"
 close_button.custom_minimum_size=Vector2(44,44)
 close_button.add_theme_font_size_override("font_size",26)
 close_button.pressed.connect(func(): settings_panel.hide())
 close_button.pressed.connect(func(): if is_instance_valid(sound): sound.play("ui-click"))
 var close_style=reference_ui.box(Color("#1d2b33"),4,Color("#44575e"))
 close_style.set_corner_radius_all(8)
 for state in ["normal","hover","pressed","focus"]: close_button.add_theme_stylebox_override(state,close_style)
 header.add_child(close_button)
 column.add_child(header)
 var divider=ColorRect.new()
 divider.color=Color(.7,.78,.78,.16)
 divider.custom_minimum_size.y=1
 column.add_child(divider)
 # Segmented tab bar like the browser's AUDIO / CONTROLS / GRAPHICS switch.
 var tab_frame=PanelContainer.new()
 var frame_style=reference_ui.box(Color(.035,.055,.07,.9),4,Color(.7,.78,.78,.2))
 frame_style.set_corner_radius_all(10)
 tab_frame.add_theme_stylebox_override("panel",frame_style)
 column.add_child(tab_frame)
 var tab_row=HBoxContainer.new()
 tab_row.add_theme_constant_override("separation",4)
 tab_frame.add_child(tab_row)
 var pages=VBoxContainer.new()
 pages.size_flags_vertical=Control.SIZE_EXPAND_FILL
 pages.custom_minimum_size.y=380
 column.add_child(pages)
 var panels={}
 var tab_group=ButtonGroup.new()
 for title in ["Audio","Controls","Graphics","World"]:
  var page=VBoxContainer.new()
  page.name=title
  page.add_theme_constant_override("separation",18)
  page.visible=(title=="Audio")
  pages.add_child(page)
  panels[title]=page
  var tab_button=Button.new()
  tab_button.text=title.to_upper()
  tab_button.toggle_mode=true
  tab_button.button_group=tab_group
  tab_button.button_pressed=(title=="Audio")
  tab_button.size_flags_horizontal=Control.SIZE_EXPAND_FILL
  tab_button.custom_minimum_size.y=40
  tab_button.add_theme_font_size_override("font_size",13)
  var plain=reference_ui.box(Color(0,0,0,0),6,Color(0,0,0,0))
  plain.set_corner_radius_all(8)
  var selected=reference_ui.box(Color("#2b3839"),6,Color("#2b3839"))
  selected.set_corner_radius_all(8)
  selected.border_width_bottom=2
  selected.border_color=Color("#cddaa1")
  var hover=reference_ui.box(Color("#1b282e"),6,Color("#1b282e"))
  hover.set_corner_radius_all(8)
  tab_button.add_theme_stylebox_override("normal",plain)
  tab_button.add_theme_stylebox_override("hover",hover)
  tab_button.add_theme_stylebox_override("pressed",selected)
  tab_button.add_theme_stylebox_override("hover_pressed",selected)
  tab_button.add_theme_stylebox_override("focus",plain)
  tab_button.pressed.connect(func():
   for key in panels: panels[key].visible=(key==title)
   if is_instance_valid(sound): sound.play("ui-click"))
  tab_row.add_child(tab_button)
 for setting in [["Master volume","master_volume"],["Music volume","music_volume"],["Weather volume","weather_volume"]]:
  var row=HBoxContainer.new()
  var caption=label(setting[0],16)
  caption.add_theme_font_override("font",reference_ui.sans)
  caption.size_flags_horizontal=Control.SIZE_EXPAND_FILL
  row.add_child(caption)
  var amount=label("%d%%" % int(get(setting[1])*100),16)
  amount.add_theme_font_override("font",reference_ui.sans)
  row.add_child(amount)
  panels.Audio.add_child(row)
  var slider=HSlider.new()
  slider.min_value=0
  slider.max_value=1
  slider.step=.01
  slider.value=get(setting[1])
  slider.value_changed.connect(func(value): set(setting[1],value);amount.text="%d%%" % int(value*100);save_user_settings())
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
 reference_ui.layout()

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
 if is_instance_valid(screen): screen.get_meta("post_material").set_shader_parameter("detail",[0.0,.65,1.0][graphics_quality])
 get_node("Sun").shadow_enabled=graphics_quality>0
 get_node("Sun").directional_shadow_max_distance=[60,90,120][graphics_quality]
 player.camera.far=[350,600,650][graphics_quality]

func setup_music() -> void:
 sound=Soundscape.new()
 sound.main=self
 add_child(sound)

func update_markers(_azimuth: float) -> void:
 if reference_ui!=null and is_instance_valid(reference_ui.compass_widget): reference_ui.compass_widget.queue_redraw()

func _exit_tree() -> void:
 if network_ui!=null and is_instance_valid(net) and net.changed.is_connected(network_ui.refresh): net.changed.disconnect(network_ui.refresh)
