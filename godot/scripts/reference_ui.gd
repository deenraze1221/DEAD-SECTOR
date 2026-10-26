extends RefCounted
# Browser CSS uses Arial for branding and system monospace for terminal UI.
var main
var mono: SystemFont
var heading: SystemFont
var sans: SystemFont
var hud_root: Control
var health: Label
var condition: Label
var reserve: Label
var weapon: Label
var total_ammo: Label
var slots: Label
var heading_ticks: Label
var health_bar: ProgressBar
var store_balance: Label
var owned_row: HBoxContainer
var store_tabs: Dictionary={}
var menu_hint: Label
var menu_shade: ColorRect
var store_shade: ColorRect
var play_button: Button
var restart_button: Button
var menu_camera: Camera3D
var started=false
var orbit=0.0

func fonts() -> void:
 mono=SystemFont.new()
 mono.font_names=PackedStringArray(["Courier New","Liberation Mono"])
 mono.fallbacks=[load("res://assets/terminal.ttf")]
 heading=SystemFont.new()
 heading.font_names=PackedStringArray(["Arial","Liberation Sans"])
 heading.font_weight=900
 heading.fallbacks=[load("res://assets/title.ttf")]
 sans=SystemFont.new()
 sans.font_names=PackedStringArray(["Arial","Liberation Sans"])
 sans.fallbacks=[load("res://assets/sans.ttf")]

func text(value: String, size=11, color=Color("#d8e1dc")) -> Label:
 var label=Label.new()
 label.text=value
 label.add_theme_font_size_override("font_size",size)
 label.add_theme_color_override("font_color",color)
 label.mouse_filter=Control.MOUSE_FILTER_IGNORE
 return label

func box(color: Color, margin=12, line=Color("#344148")) -> StyleBoxFlat:
 var style=StyleBoxFlat.new()
 style.bg_color=color
 style.border_color=line
 style.set_border_width_all(1)
 style.set_corner_radius_all(4)
 style.set_content_margin_all(margin)
 return style

func panel(parent: Node, size: Vector2, at: Vector2, anchor=Control.PRESET_TOP_LEFT) -> PanelContainer:
 var node=PanelContainer.new()
 node.set_anchors_and_offsets_preset(anchor)
 node.position=at
 node.custom_minimum_size=size
 node.add_theme_stylebox_override("panel",box(Color(.035,.063,.082,.64)))
 parent.add_child(node)
 return node

func action(value: String, callback: Callable, primary=false, height=34) -> Button:
 var node=Button.new()
 node.text=value
 node.custom_minimum_size.y=height
 node.pressed.connect(callback)
 node.pressed.connect(func(): if main.sound!=null: main.sound.play("ui-click"))
 node.mouse_entered.connect(func(): if main.sound!=null: main.sound.play("ui-hover",null,.5))
 if primary:
  node.add_theme_stylebox_override("normal",box(Color("#cddaa1"),8,Color("#cddaa1")))
  node.add_theme_stylebox_override("hover",box(Color("#e0edb5"),8,Color("#d4e2ae")))
  node.add_theme_color_override("font_color",Color("#172019"))
  node.add_theme_color_override("font_hover_color",Color("#172019"))
 return node

func build() -> void:
 fonts()
 var effects=CanvasLayer.new()
 effects.layer=1
 main.add_child(effects)
 var post=ColorRect.new()
 post.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 post.mouse_filter=Control.MOUSE_FILTER_IGNORE
 var post_material=ShaderMaterial.new()
 post_material.shader=load("res://shaders/pixels.gdshader")
 post.material=post_material
 effects.add_child(post)
 var overlay=CanvasLayer.new()
 overlay.layer=2
 main.add_child(overlay)
 main.screen=Control.new()
 main.screen.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 main.screen.mouse_filter=Control.MOUSE_FILTER_IGNORE
 main.screen.set_meta("post_material",post_material)
 var theme=Theme.new()
 theme.default_font=mono
 theme.default_font_size=11
 var track=StyleBoxFlat.new()
 track.bg_color=Color("#26343a")
 track.content_margin_top=2
 track.content_margin_bottom=2
 var fill=track.duplicate()
 fill.bg_color=Color("#b8c796")
 theme.set_stylebox("slider","HSlider",track)
 theme.set_stylebox("grabber_area","HSlider",fill)
 theme.set_stylebox("grabber_area_highlight","HSlider",fill)
 theme.set_icon("grabber","HSlider",load("res://assets/icons/slider-knob.svg"))
 theme.set_icon("grabber_highlight","HSlider",load("res://assets/icons/slider-knob.svg"))
 theme.set_color("font_color","Label",Color("#d8e1dc"))
 theme.set_color("font_color","Button",Color("#d8e1dc"))
 theme.set_color("font_disabled_color","Button",Color("#7e8b83"))
 theme.set_color("font_shadow_color","Label",Color(0,0,0,.7))
 theme.set_constant("shadow_offset_y","Label",1)
 for state in ["normal","hover","pressed","disabled","focus"]:
  theme.set_stylebox(state,"Button",box(Color("#29373b") if state in ["hover","pressed"] else Color("#101a21"),7,Color("#9aab92") if state=="focus" else Color("#34434a")))
 theme.set_stylebox("panel","TooltipPanel",box(Color("#10191f"),10))
 theme.set_font("font","TooltipLabel",mono)
 theme.set_font_size("font_size","TooltipLabel",11)
 theme.set_stylebox("panel","TabContainer",box(Color("#0b1218"),10))
 theme.set_stylebox("tab_selected","TabContainer",box(Color("#2b3839"),8,Color("#6d806e")))
 theme.set_stylebox("tab_unselected","TabContainer",box(Color("#0b1218"),8))
 main.screen.theme=theme
 overlay.add_child(main.screen)
 menu_shade=ColorRect.new()
 menu_shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 menu_shade.color=Color(.01,.025,.035,.3)
 menu_shade.mouse_filter=Control.MOUSE_FILTER_IGNORE
 main.screen.add_child(menu_shade)
 build_hud()
 build_menu()
 store_shade=ColorRect.new()
 store_shade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 store_shade.color=Color(.01,.025,.035,.82)
 main.screen.add_child(store_shade)
 store_shade.hide()
 menu_camera=Camera3D.new()
 menu_camera.fov=70
 menu_camera.far=650
 main.add_child(menu_camera)
 main.screen.resized.connect(layout)
 layout()

func build_hud() -> void:
 hud_root=Control.new()
 hud_root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 hud_root.mouse_filter=Control.MOUSE_FILTER_IGNORE
 hud_root.add_to_group("match_hud")
 main.screen.add_child(hud_root)
 var cross=text("▪",10)
 cross.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 cross.position=Vector2(-3,-7)
 hud_root.add_child(cross)
 main.hud=text("",11)
 main.hud.position=Vector2(24,90)
 hud_root.add_child(main.hud)
 var vitals=panel(hud_root,Vector2(400,76),Vector2(24,-94),Control.PRESET_BOTTOM_LEFT)
 var row=HBoxContainer.new()
 row.add_theme_constant_override("separation",16)
 vitals.add_child(row)
 for title in ["HEALTH","STAMINA","MONEY","CONDITION"]:
  var column=VBoxContainer.new()
  column.add_theme_constant_override("separation",4)
  column.custom_minimum_size.x=78
  row.add_child(column)
  column.add_child(text(title,8,Color("#b2c0b9")))
  var value=text("100",21)
  column.add_child(value)
  if title=="HEALTH":
   health=value
   health_bar=bar(100,Color("#cddaa1"))
   column.add_child(health_bar)
  elif title=="STAMINA":
   value.name="StaminaValue"
   main.stamina_bar=bar(5,Color("#8dbbc3"))
   main.stamina_bar.set_meta("number",value)
   column.add_child(main.stamina_bar)
  elif title=="MONEY":
   main.wallet_label=value
   value.add_theme_color_override("font_color",Color("#dec995"))
  else: condition=value
 var ammo=panel(hud_root,Vector2(150,92),Vector2(-174,-110),Control.PRESET_BOTTOM_RIGHT)
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",5)
 ammo.add_child(column)
 weapon=text("PISTOL",9)
 weapon.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 column.add_child(weapon)
 var numbers=HBoxContainer.new()
 numbers.alignment=BoxContainer.ALIGNMENT_END
 column.add_child(numbers)
 main.ammo_label=text("8",30)
 reserve=text("/ 84",14,Color("#a2b7b0"))
 reserve.vertical_alignment=VERTICAL_ALIGNMENT_BOTTOM
 numbers.add_child(main.ammo_label)
 numbers.add_child(reserve)
 total_ammo=text("92/500 · AMMO",9)
 total_ammo.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 column.add_child(total_ammo)
 slots=text("",9)
 slots.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_RIGHT)
 slots.position=Vector2(-280,-138)
 slots.custom_minimum_size.x=255
 slots.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 hud_root.add_child(slots)
 main.gear_label=text("",8)
 main.gear_label.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_LEFT)
 main.gear_label.position=Vector2(24,-118)
 hud_root.add_child(main.gear_label)
 var waves=panel(hud_root,Vector2(250,74),Vector2(-274,88),Control.PRESET_TOP_RIGHT)
 main.wave_label=text("",10,Color("#d4dfb3"))
 main.wave_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 waves.add_child(main.wave_label)
 main.fps_label=text("",9)
 main.fps_label.set_anchors_and_offsets_preset(Control.PRESET_TOP_RIGHT)
 main.fps_label.position=Vector2(-110,8)
 main.fps_label.custom_minimum_size.x=100
 main.fps_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 hud_root.add_child(main.fps_label)
 heading_ticks=text("",9,Color("#aabdb5"))
 heading_ticks.set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
 heading_ticks.position=Vector2(-220,7)
 heading_ticks.custom_minimum_size.x=440
 heading_ticks.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
 hud_root.add_child(heading_ticks)
 main.bearing_label=text("",11)
 main.bearing_label.set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
 main.bearing_label.position=Vector2(-100,26)
 main.bearing_label.custom_minimum_size.x=200
 main.bearing_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
 hud_root.add_child(main.bearing_label)
 main.compass_markers=Control.new()
 main.compass_markers.set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
 main.compass_markers.position=Vector2(-250,46)
 hud_root.add_child(main.compass_markers)
 main.notice_label=text("",15)
 main.notice_label.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 main.notice_label.position=Vector2(-400,-150)
 main.notice_label.custom_minimum_size.x=800
 main.notice_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
 hud_root.add_child(main.notice_label)
 main.vignette=ColorRect.new()
 main.vignette.mouse_filter=Control.MOUSE_FILTER_IGNORE
 main.vignette.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 hud_root.add_child(main.vignette)

func bar(maximum: float, color: Color) -> ProgressBar:
 var node=ProgressBar.new()
 node.max_value=maximum
 node.show_percentage=false
 node.custom_minimum_size=Vector2(78,3)
 node.add_theme_stylebox_override("fill",box(color,0,color))
 node.add_theme_stylebox_override("background",box(Color("#354941"),0,Color("#354941")))
 return node

func build_menu() -> void:
 main.menu=PanelContainer.new()
 main.menu.set_anchors_and_offsets_preset(Control.PRESET_CENTER_LEFT)
 main.menu.position=Vector2(84,-205)
 main.menu.custom_minimum_size=Vector2(360,0)
 var style=StyleBoxFlat.new()
 style.bg_color=Color(0,0,0,0)
 style.border_color=Color(.8,.86,.64,.3)
 style.border_width_left=1
 style.content_margin_left=18
 main.menu.add_theme_stylebox_override("panel",style)
 main.screen.add_child(main.menu)
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",10)
 main.menu.add_child(column)
 column.add_child(text("▪ CONNECTION ESTABLISHED / SECTOR 00",8))
 var logo=VBoxContainer.new()
 logo.add_theme_constant_override("separation",-18)
 for line in ["DEAD","SECTOR"]:
  var title=text(line,72,Color("#e8eeeb") if line=="DEAD" else Color("#cddaa1"))
  title.add_theme_font_override("font",heading)
  logo.add_child(title)
 column.add_child(logo)
 var subtitle=text("Beyond the perimeter — silence.\nCheck your gear. Look around. Keep moving.",12,Color("#bcc6c4"))
 subtitle.add_theme_font_override("font",sans)
 column.add_child(subtitle)
 var actions=VBoxContainer.new()
 actions.custom_minimum_size.x=255
 actions.size_flags_horizontal=Control.SIZE_SHRINK_BEGIN
 actions.add_theme_constant_override("separation",6)
 column.add_child(actions)
 play_button=action("PLAY                        ↗",func(): main.set_playing(true),true)
 play_button.add_theme_font_override("font",heading)
 actions.add_child(play_button)
 var join=action("JOIN BY CODE",func(): pass)
 join.disabled=true
 join.tooltip_text="Network play is not available in the Godot port yet."
 actions.add_child(join)
 actions.add_child(action("SETTINGS                    +",main.open_settings))
 restart_button=action("NEW MATCH",func(): main.get_tree().reload_current_scene(),false,26)
 restart_button.hide()
 actions.add_child(restart_button)
 for choice in actions.get_children(): choice.add_theme_font_override("font",heading)
 menu_hint=text("60-second preparation · Clear all enemies\nESC · Resume / pause",8,Color("#a3b3b6"))
 column.add_child(menu_hint)
 main.menu.resized.connect(layout)
 var quit=action("QUIT",func(): main.get_tree().quit(),false,22)
 quit.size_flags_horizontal=Control.SIZE_SHRINK_BEGIN
 column.add_child(quit)
 var top=text("DS / FIELD SYSTEMS",8,Color("#aab9bc"))
 top.position=Vector2(40,22)
 top.add_to_group("menu_chrome")
 main.screen.add_child(top)
 var bottom=text("RESTRICTED AREA / AUTHORIZED PERSONNEL ONLY",8,Color("#aab9bc"))
 bottom.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_LEFT)
 bottom.position=Vector2(40,-28)
 bottom.add_to_group("menu_chrome")
 main.screen.add_child(bottom)

func build_store() -> void:
 main.store_panel=panel(main.screen,Vector2(800,580),Vector2(-400,-290),Control.PRESET_CENTER)
 main.store_panel.add_theme_stylebox_override("panel",box(Color(.047,.07,.09,.98),16,Color("#73816c")))
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",8)
 main.store_panel.add_child(column)
 var header=HBoxContainer.new()
 column.add_child(header)
 var title=text("TERMINAL · SECTOR MART",14)
 title.size_flags_horizontal=Control.SIZE_EXPAND_FILL
 header.add_child(title)
 header.add_child(action("CLOSE · ESC",main.close_store,false,26))
 store_balance=text("",10,Color("#dec995"))
 column.add_child(store_balance)
 owned_row=HBoxContainer.new()
 column.add_child(owned_row)
 var tabs=HBoxContainer.new()
 column.add_child(tabs)
 for tab in ["weapon","supplies"]:
  var choice=action("WEAPONS" if tab=="weapon" else "GEAR",func(): main.store_tab=tab;render_store(),false,28)
  choice.size_flags_horizontal=Control.SIZE_EXPAND_FILL
  choice.toggle_mode=true
  choice.add_theme_stylebox_override("pressed",box(Color("#2b3839"),7,Color("#9eae8a")))
  store_tabs[tab]=choice
  tabs.add_child(choice)
 var scroll=ScrollContainer.new()
 scroll.vertical_scroll_mode=ScrollContainer.SCROLL_MODE_SHOW_NEVER
 scroll.horizontal_scroll_mode=ScrollContainer.SCROLL_MODE_DISABLED
 scroll.size_flags_vertical=Control.SIZE_EXPAND_FILL
 column.add_child(scroll)
 main.store_grid=GridContainer.new()
 main.store_grid.columns=3
 main.store_grid.size_flags_horizontal=Control.SIZE_EXPAND_FILL
 main.store_grid.add_theme_constant_override("h_separation",8)
 main.store_grid.add_theme_constant_override("v_separation",8)
 scroll.add_child(main.store_grid)
 main.store_message=text("",9,Color("#dec995"))
 column.add_child(main.store_message)
 main.store_panel.hide()

func render_store() -> void:
 for tab in store_tabs: store_tabs[tab].set_pressed_no_signal(tab==main.store_tab)
 for parent in [main.store_grid,owned_row]:
  for child in parent.get_children(): parent.remove_child(child);child.queue_free()
 store_balance.text="$%d  ·  AMMO %d/500" % [main.battle.money,carried_ammo()]
 for id in main.battle.inventory:
  owned_row.add_child(action(("2 · PISTOL · FREE" if id=="Pistol" else "1 · "+id.to_upper()),func(): main.battle.equip_owned(id);render_store(),false,24))
 var items: Array=main.bridge.data.shop_items.duplicate()
 items.sort_custom(func(a,b): return a.price<b.price)
 for item in items:
  if (item.kind=="weapon")!=(main.store_tab=="weapon"): continue
  var locked=item.kind=="weapon" and main.battle.completed<int(main.bridge.data.weapons[item.id].unlockAfter)
  var card=PanelContainer.new()
  card.name="Card"+item.id
  card.size_flags_horizontal=Control.SIZE_EXPAND_FILL
  card.add_theme_stylebox_override("panel",box(Color("#10181d") if locked else Color("#19252c"),8,Color("#34434a")))
  main.store_grid.add_child(card)
  var column=VBoxContainer.new()
  column.add_theme_constant_override("separation",3)
  card.add_child(column)
  var names={"ammo":"50 universal rounds","smallMedkit":"Small medkit · +25 HP","largeMedkit":"Large medkit · +75 HP","flashlight":"Flashlight · 120 seconds","battery":"Battery · 120 seconds","trap":"Bear trap · 300 damage","repair":"Repair · +50 condition"}
  column.add_child(text(names.get(item.id,item.id.to_upper()),9))
  var preview=Control.new()
  preview.custom_minimum_size=Vector2(0,40)
  column.add_child(preview)
  var icon=TextureRect.new()
  icon.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
  icon.expand_mode=TextureRect.EXPAND_IGNORE_SIZE
  icon.stretch_mode=TextureRect.STRETCH_KEEP_ASPECT_CENTERED
  var icon_id=item.id
  icon.texture=load("res://assets/icons/"+icon_id+".svg")
  icon.modulate.a=.23 if locked else 1
  preview.add_child(icon)
  var info=""
  if item.kind=="weapon":
   var config: Dictionary=main.bridge.data.weapons[item.id]
   info="Damage: %d HP\nMagazine: %d rounds\nRate: %.1f rounds/sec\nReload: %.1f seconds" % [config.damage,config.capacity,1.0/config.interval,config.reload]
   if locked:
    var lock_icon=TextureRect.new()
    lock_icon.texture=load("res://assets/icons/lock.svg")
    lock_icon.expand_mode=TextureRect.EXPAND_IGNORE_SIZE
    lock_icon.stretch_mode=TextureRect.STRETCH_KEEP_ASPECT_CENTERED
    lock_icon.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
    lock_icon.position=Vector2(-10,-12)
    lock_icon.size=Vector2(20,24)
    preview.add_child(lock_icon)
  else:
   info="Carry up to 500 rounds, including magazines." if item.id=="ammo" else "Heals immediately when injured; otherwise stored (H/J)." if item.kind=="medkit" else "B · Replace battery" if item.id=="battery" else "F · Toggle flashlight" if item.id=="flashlight" else "Enemies only. 3 + LMB to place." if item.id=="trap" else "Restore equipped weapon condition."
   var description=text(info,8,Color("#a9bcb2"))
   description.autowrap_mode=TextServer.AUTOWRAP_WORD_SMART
   description.custom_minimum_size.y=24
   column.add_child(description)
  card.tooltip_text=info
  var purchase=action("$%d" % int(item.price),func(): main.store_feedback=main.battle.buy(item.id);render_store(),false,24)
  purchase.disabled=locked or main.battle.money<int(item.price)
  purchase.tooltip_text=info
  column.add_child(purchase)
  if item.kind=="weapon":
   var unlock=text("Available after completing wave %d" % int(main.bridge.data.weapons[item.id].unlockAfter) if locked else "Hover for weapon statistics",8,Color("#b7c5ac"))
   unlock.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
   column.add_child(unlock)
 main.store_message.text=main.store_feedback if main.store_feedback!="" else "H · Small medkit ×%d / J · Large medkit ×%d" % main.battle.medkits
 layout()

func layout() -> void:
 var size=main.screen.size
 if size.x<=0: return
 if is_instance_valid(main.store_panel):
  var target=Vector2(minf(800,size.x-40),minf(580,size.y-32))
  main.store_panel.custom_minimum_size=target
  main.store_panel.size=target
  main.store_panel.position=(size-target)*.5
 if is_instance_valid(main.settings_panel):
  main.settings_panel.scale=Vector2(.72,.72)
  main.settings_panel.position=Vector2(size.x-main.settings_panel.size.x*.72-24,(size.y-main.settings_panel.size.y*.72)*.5)
 if is_instance_valid(main.menu): main.menu.position=Vector2(size.x*.065,maxf(65,(size.y-main.menu.size.y)*.5))

func refresh(delta: float) -> void:
 var p=main.player
 var b=main.battle
 health.text="%d HP" % p.health
 health_bar.value=p.health
 main.stamina_bar.get_meta("number").text="%d%%" % int(p.stamina*20)
 main.wallet_label.text="$%d" % b.money
 condition.text="%d%%" % int(b.inventory.get(p.config.id,{}).get("durability",100))
 weapon.text=p.config.id.to_upper()
 main.ammo_label.text=str(p.ammo)
 reserve.text="/ %d" % p.reserve
 total_ammo.text="RELOADING" if p.reloading>0 else "%d/500 · AMMO" % carried_ammo()
 slots.text="1 · %s    2 · PISTOL" % (b.primary.to_upper() if b.primary!="" else "EMPTY")
 main.gear_label.text="F · Flashlight %d%%  ·  B ×%d  ·  3 Traps ×%d  ·  H/J %d/%d" % [int(b.battery/120*100),b.batteries,b.traps,b.medkits[0],b.medkits[1]]
 var degrees=fposmod(-rad_to_deg(p.rotation.y),360)
 heading_ticks.text=""
 for n in range(-3,4):
  var bearing=int(fposmod(round(degrees/15)*15+n*15,360))
  heading_ticks.text+=({0:"N",90:"E",180:"S",270:"W"}.get(bearing,"%03d" % bearing))+"     "
 menu_shade.visible=main.menu.visible or is_instance_valid(main.settings_panel) and main.settings_panel.visible
 store_shade.visible=is_instance_valid(main.store_panel) and main.store_panel.visible
 for node in main.get_tree().get_nodes_in_group("menu_chrome"): node.visible=main.menu.visible
 play_button.text="RESUME                      ↗" if started else "PLAY                        ↗"
 restart_button.visible=started
 menu_hint.text="YOU DIED · START A NEW MATCH" if p.health<=0 else "60-second preparation · Clear all enemies\nESC · Resume / pause"
 if not started and not p.active and not store_shade.visible:
  orbit+=delta*.018
  menu_camera.position=Vector3(sin(orbit)*125,60,cos(orbit)*125)
  menu_camera.look_at(Vector3(0,4,0))
  menu_camera.make_current()

func carried_ammo() -> int:
 var result=main.player.reserve+main.player.ammo
 for id in main.battle.inventory:
  if id!=main.player.config.id: result+=int(main.battle.inventory[id].ammo)
 return result
