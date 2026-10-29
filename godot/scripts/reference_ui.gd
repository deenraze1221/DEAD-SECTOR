extends RefCounted
# Browser CSS uses Arial for branding and system monospace for terminal UI.
var main
var mono: SystemFont
var heading: SystemFont
var sans: SystemFont
var hud_root: Control
var hit_marker: Label
var mission_status: Label
var sector_subtitle: Label
var weather_line: Label
var health: Label
var condition: Label
var reserve: Label
var weapon: Label
var total_ammo: Label
var slots: Label
var heading_ticks: Label
var compass_widget: Control
var notice_box: PanelContainer
var prompt_box: PanelContainer
var interaction_label: Label
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
 store_shade.color=Color(.027,.063,.043,.84)
 main.screen.add_child(store_shade)
 store_shade.hide()
 menu_camera=Camera3D.new()
 menu_camera.fov=70
 menu_camera.far=650
 main.add_child(menu_camera)
 main.screen.resized.connect(layout)
 layout()

# The browser HUD is drawn at 1.25x of its CSS sizes (#hud transform: scale(1.25)); these numbers are the scaled values.
const ACID=Color("#cddaa1")

func tracked(spacing: int) -> FontVariation:
 var font=FontVariation.new()
 font.base_font=mono
 font.spacing_glyph=spacing
 return font

func tracked_text(value: String, size: int, color: Color, spacing: int) -> Label:
 var label=text(value,size,color)
 label.add_theme_font_override("font",tracked(spacing))
 return label

# Flat HTML-style panel: square corners and a single accent edge (top, left or right) instead of a full border.
func hud_box(edge: String, background: Color, line: Color, width: int, pad: Vector2) -> StyleBoxFlat:
 var style=StyleBoxFlat.new()
 style.bg_color=background
 style.border_color=line
 style.set_corner_radius_all(0)
 style.content_margin_left=pad.x
 style.content_margin_right=pad.x
 style.content_margin_top=pad.y
 style.content_margin_bottom=pad.y
 match edge:
  "top": style.border_width_top=width
  "left": style.border_width_left=width
  "right": style.border_width_right=width
  "all": style.set_border_width_all(width)
 return style

func hud_panel(parent: Node, edge: String, background: Color, line: Color, width: int, pad: Vector2, size: Vector2, at: Vector2, anchor: int) -> PanelContainer:
 var node=PanelContainer.new()
 node.set_anchors_and_offsets_preset(anchor)
 node.position=at
 node.custom_minimum_size=size
 node.add_theme_stylebox_override("panel",hud_box(edge,background,line,width,pad))
 node.mouse_filter=Control.MOUSE_FILTER_IGNORE
 parent.add_child(node)
 return node

var slot_boxes: Array=[]
var slot_labels: Array=[]

func build_hud() -> void:
 hud_root=Control.new()
 hud_root.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 hud_root.mouse_filter=Control.MOUSE_FILTER_IGNORE
 hud_root.add_to_group("match_hud")
 var hud_theme=main.screen.theme.duplicate()
 hud_theme.default_font=mono
 hud_root.theme=hud_theme
 main.screen.add_child(hud_root)
 # Crosshair: 5 px light square with a dark halo, like the browser's .crosshair.
 var halo=ColorRect.new()
 halo.color=Color(0,0,0,.33)
 halo.mouse_filter=Control.MOUSE_FILTER_IGNORE
 halo.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 halo.custom_minimum_size=Vector2(10,10)
 halo.size=Vector2(10,10)
 halo.position=Vector2(-5,-5)
 hud_root.add_child(halo)
 var cross=ColorRect.new()
 cross.color=Color("#e8f4d8")
 cross.mouse_filter=Control.MOUSE_FILTER_IGNORE
 cross.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 cross.custom_minimum_size=Vector2(5,5)
 cross.size=Vector2(5,5)
 cross.position=Vector2(-2.5,-2.5)
 hud_root.add_child(cross)
 hit_marker=text("×",40,Color("#f3d19a"))
 hit_marker.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
 hit_marker.position=Vector2(-11,-26)
 hud_root.add_child(hit_marker)
 mission_status=text("",12,Color("#dec995"))
 mission_status.position=Vector2(35,196)
 mission_status.custom_minimum_size.x=300
 hud_root.add_child(mission_status)
 main.hud=tracked_text("",14,Color("#e5e7d9"),3)
 main.hud.position=Vector2(35,102)
 hud_root.add_child(main.hud)
 sector_subtitle=text("COMBAT ZONE",11,Color("#aab79d"))
 sector_subtitle.position=Vector2(35,130)
 hud_root.add_child(sector_subtitle)
 weather_line=text("",10,Color("#aab79d"))
 weather_line.position=Vector2(35,150)
 hud_root.add_child(weather_line)
 # Bottom-left status block (.hud-bottom): accent line on top, 9 px labels, 24 px values.
 var vitals=hud_panel(hud_root,"top",Color("#111c19de"),Color(.627,.729,.451,.27),1,Vector2(22,17),Vector2(0,0),Vector2(35,-128),Control.PRESET_BOTTOM_LEFT)
 var row=HBoxContainer.new()
 row.add_theme_constant_override("separation",30)
 vitals.add_child(row)
 for title in ["HEALTH","STAMINA","MONEY","CONDITION"]:
  var column=VBoxContainer.new()
  column.add_theme_constant_override("separation",7)
  column.custom_minimum_size.x=112 if title=="CONDITION" else 125
  row.add_child(column)
  column.add_child(tracked_text(title,11,Color("#aebba0"),1))
  var value=text("100",30,Color("#e5e7d9"))
  column.add_child(value)
  if title=="HEALTH":
   health=value
   health_bar=bar(100,ACID)
   column.add_child(health_bar)
  elif title=="STAMINA":
   value.name="StaminaValue"
   main.stamina_bar=bar(5,Color("#8dbbc3"))
   main.stamina_bar.set_meta("number",value)
   column.add_child(main.stamina_bar)
  elif title=="MONEY":
   main.wallet_label=value
   value.add_theme_color_override("font_color",Color("#d7e9a5"))
  else: condition=value
 # Bottom-right ammunition block (.ammo-panel): accent line on the right edge.
 var ammo=hud_panel(hud_root,"right",Color("#111c19de"),ACID,4,Vector2(22,17),Vector2(225,0),Vector2(-260,-130),Control.PRESET_BOTTOM_RIGHT)
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",6)
 ammo.add_child(column)
 weapon=tracked_text("PISTOL",12,Color("#bbc9aa"),1)
 weapon.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 column.add_child(weapon)
 var numbers=HBoxContainer.new()
 numbers.alignment=BoxContainer.ALIGNMENT_END
 column.add_child(numbers)
 main.ammo_label=text("8",37,Color("#e5e7d9"))
 reserve=text("/ 84",19,Color("#a4b794"))
 reserve.vertical_alignment=VERTICAL_ALIGNMENT_BOTTOM
 numbers.add_child(main.ammo_label)
 numbers.add_child(reserve)
 total_ammo=text("92/500 · AMMO",12,Color("#bbc9aa"))
 total_ammo.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 column.add_child(total_ammo)
 # Weapon slots (.weapon-slots): two small boxes above the status block; the equipped one gets the accent border.
 var slot_row=HBoxContainer.new()
 slot_row.add_theme_constant_override("separation",8)
 slot_row.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_LEFT)
 slot_row.position=Vector2(35,-198)
 slot_row.mouse_filter=Control.MOUSE_FILTER_IGNORE
 hud_root.add_child(slot_row)
 for i in range(2):
  var slot_box=PanelContainer.new()
  slot_box.mouse_filter=Control.MOUSE_FILTER_IGNORE
  var slot_label=text("",12,Color("#a5b29a"))
  slot_box.add_child(slot_label)
  slot_row.add_child(slot_box)
  slot_boxes.append(slot_box)
  slot_labels.append(slot_label)
 slots=slot_labels[0]
 main.gear_label=text("",10,Color("#aebba0"))
 main.gear_label.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_LEFT)
 main.gear_label.position=Vector2(35,-150)
 hud_root.add_child(main.gear_label)
 # Wave panel (.wave-panel): accent line on the left edge, right-aligned text.
 var waves=hud_panel(hud_root,"left",Color("#111c19db"),ACID,4,Vector2(22,17),Vector2(330,0),Vector2(-365,102),Control.PRESET_TOP_RIGHT)
 main.wave_label=text("",13,Color("#d4dfb3"))
 main.wave_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 waves.add_child(main.wave_label)
 main.fps_label=text("",9)
 main.fps_label.set_anchors_and_offsets_preset(Control.PRESET_TOP_RIGHT)
 main.fps_label.position=Vector2(-110,8)
 main.fps_label.custom_minimum_size.x=100
 main.fps_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_RIGHT
 hud_root.add_child(main.fps_label)
 main.bearing_label=text("",11)
 main.bearing_label.hide()
 hud_root.add_child(main.bearing_label)
 main.compass_markers=Control.new()
 main.compass_markers.hide()
 hud_root.add_child(main.compass_markers)
 compass_widget=preload("res://scripts/compass_ui.gd").new()
 compass_widget.main=main
 compass_widget.mouse_filter=Control.MOUSE_FILTER_IGNORE
 hud_root.add_child(compass_widget)
 # Wave notice (#wave-notice): green top line, thin lower line, wide letter spacing.
 notice_box=PanelContainer.new()
 var notice_style=hud_box("top",Color("#142119e8"),ACID,2,Vector2(37,22))
 notice_style.border_width_bottom=1
 notice_style.border_color=ACID
 notice_style.expand_margin_bottom=0
 notice_box.add_theme_stylebox_override("panel",notice_style)
 notice_box.mouse_filter=Control.MOUSE_FILTER_IGNORE
 hud_root.add_child(notice_box)
 main.notice_label=tracked_text("",22,Color("#dfedb6"),3)
 main.notice_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
 main.notice_label.autowrap_mode=TextServer.AUTOWRAP_WORD_SMART
 notice_box.add_child(main.notice_label)
 # Interaction prompt (.interaction): flat dark strip, no border.
 prompt_box=PanelContainer.new()
 prompt_box.add_theme_stylebox_override("panel",hud_box("none",Color("#111c19d1"),Color(0,0,0,0),0,Vector2(15,10)))
 prompt_box.mouse_filter=Control.MOUSE_FILTER_IGNORE
 hud_root.add_child(prompt_box)
 interaction_label=text("",14,Color("#e2eccb"))
 interaction_label.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
 prompt_box.add_child(interaction_label)
 main.vignette=ColorRect.new()
 main.vignette.mouse_filter=Control.MOUSE_FILTER_IGNORE
 main.vignette.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 hud_root.add_child(main.vignette)

func update_slots(primary: String, equipped: String) -> void:
 var names=[("1 · "+primary.to_upper()) if primary!="" else "1 · EMPTY","2 · PISTOL"]
 var active=0 if primary!="" and equipped==primary else 1
 for i in range(2):
  slot_labels[i].text=names[i]
  slot_labels[i].add_theme_color_override("font_color",Color("#deefb7") if i==active else Color("#a5b29a"))
  slot_boxes[i].add_theme_stylebox_override("panel",hud_box("all",Color("#111c19ce"),ACID if i==active else Color(.4,.463,.302,.4),1,Vector2(12,10)))

func bar(maximum: float, color: Color) -> ProgressBar:
 var node=ProgressBar.new()
 node.max_value=maximum
 node.show_percentage=false
 node.custom_minimum_size=Vector2(112,4)
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
 var brand_font=load("res://assets/title-user.ttf")
 brand_font.fallbacks=[heading]
 var logo=VBoxContainer.new()
 logo.add_theme_constant_override("separation",-18)
 for line in ["DEAD","SECTOR"]:
  var title=text(line,72,Color("#e8eeeb") if line=="DEAD" else Color("#cddaa1"))
  title.add_theme_font_override("font",brand_font)
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
 play_button=action("PLAY                        ↗",func():
  if started: main.set_playing(true)
  else: main.network_ui.open()
 ,true)
 play_button.add_theme_font_override("font",heading)
 actions.add_child(play_button)
 var join=action("JOIN BY CODE",func(): main.network_ui.open(true))
 actions.add_child(join)
 actions.add_child(action("SETTINGS                    +",main.open_settings))
 restart_button=action("NEW MATCH",func(): main.new_match(),false,26)
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
 var build_tag=text("BUILD 0.4 · WAVE SURVIVAL",8,Color("#aab9bc"))
 build_tag.set_anchors_preset(Control.PRESET_TOP_RIGHT)
 build_tag.grow_horizontal=Control.GROW_DIRECTION_BEGIN
 build_tag.offset_left=-40
 build_tag.offset_right=-40
 build_tag.offset_top=22
 build_tag.add_to_group("menu_chrome")
 main.screen.add_child(build_tag)
 var operation=text("OPERATION / CLEARANCE",8,Color("#aab9bc"))
 operation.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
 operation.grow_horizontal=Control.GROW_DIRECTION_BEGIN
 operation.offset_left=-40
 operation.offset_right=-40
 operation.offset_top=-28
 operation.offset_bottom=-16
 operation.add_to_group("menu_chrome")
 main.screen.add_child(operation)
 var rule=ColorRect.new()
 rule.color=Color(.8,.86,.64,.16)
 rule.mouse_filter=Control.MOUSE_FILTER_IGNORE
 rule.set_anchors_preset(Control.PRESET_TOP_WIDE)
 rule.offset_left=40
 rule.offset_right=-40
 rule.offset_top=44
 rule.offset_bottom=45
 rule.add_to_group("menu_chrome")
 main.screen.add_child(rule)

# HTML shop palette: deep green window, acid title, green terminal buttons.
func green_button(node: Button, selected=false) -> Button:
 var normal=box(Color("#33502d") if selected else Color("#283c25"),8,Color("#a3b878") if selected else Color("#80915e"))
 normal.set_corner_radius_all(3)
 var hover=box(Color("#375430"),8,Color("#a3b878"))
 hover.set_corner_radius_all(3)
 var off=box(Color("#1c2a1b"),8,Color("#46553a"))
 off.set_corner_radius_all(3)
 node.add_theme_stylebox_override("normal",normal)
 node.add_theme_stylebox_override("pressed",normal)
 node.add_theme_stylebox_override("hover",hover)
 node.add_theme_stylebox_override("hover_pressed",hover)
 node.add_theme_stylebox_override("disabled",off)
 node.add_theme_stylebox_override("focus",hover)
 node.add_theme_color_override("font_color",Color("#dae9bd"))
 node.add_theme_color_override("font_hover_color",Color("#eef7d6"))
 node.add_theme_color_override("font_pressed_color",Color("#eef7d6"))
 node.add_theme_color_override("font_disabled_color",Color("#6f7d5f"))
 node.add_theme_font_size_override("font_size",12)
 return node

func build_store() -> void:
 main.store_panel=panel(main.screen,Vector2(940,600),Vector2(-470,-300),Control.PRESET_CENTER)
 var window_style=box(Color("#16231b"),16,Color("#798d59"))
 window_style.set_corner_radius_all(2)
 window_style.shadow_color=Color(0,0,0,.5)
 window_style.shadow_size=28
 main.store_panel.add_theme_stylebox_override("panel",window_style)
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",8)
 main.store_panel.add_child(column)
 var header=HBoxContainer.new()
 column.add_child(header)
 var title=tracked_text("TERMINAL · SECTOR MART",22,Color("#d3e690"),1)
 title.size_flags_horizontal=Control.SIZE_EXPAND_FILL
 header.add_child(title)
 header.add_child(green_button(action("CLOSE · ESC",main.close_store,false,28)))
 store_balance=text("",13,Color("#bdcbb0"))
 column.add_child(store_balance)
 owned_row=HBoxContainer.new()
 column.add_child(owned_row)
 var tabs=HBoxContainer.new()
 column.add_child(tabs)
 for tab in ["weapon","supplies"]:
  var choice=green_button(action("WEAPONS" if tab=="weapon" else "GEAR",func(): main.store_tab=tab;render_store(),false,28))
  choice.size_flags_horizontal=Control.SIZE_EXPAND_FILL
  choice.toggle_mode=true
  choice.add_theme_stylebox_override("pressed",box(Color("#3a5832"),8,Color("#b4c985")))
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
 main.store_message=text("",12,Color("#d7e9a5"))
 column.add_child(main.store_message)
 main.store_panel.hide()

func render_store() -> void:
 for tab in store_tabs: store_tabs[tab].set_pressed_no_signal(tab==main.store_tab)
 for parent in [main.store_grid,owned_row]:
  for child in parent.get_children(): parent.remove_child(child);child.queue_free()
 store_balance.text="$%d  ·  AMMO %d/500" % [main.battle.money,carried_ammo()]
 for id in main.battle.inventory:
  owned_row.add_child(green_button(action(("2 · PISTOL · FREE" if id=="Pistol" else "1 · "+id.to_upper()),func(): main.battle.equip_owned(id);render_store(),false,26)))
 var items: Array=main.bridge.data.shop_items.duplicate()
 items.sort_custom(func(a,b): return a.price<b.price)
 for item in items:
  if (item.kind=="weapon")!=(main.store_tab=="weapon"): continue
  var locked=item.kind=="weapon" and main.battle.completed<int(main.bridge.data.weapons[item.id].unlockAfter)
  var card=PanelContainer.new()
  card.name="Card"+item.id
  card.size_flags_horizontal=Control.SIZE_EXPAND_FILL
  card.add_theme_stylebox_override("panel",box(Color("#121d16") if locked else Color("#1d2e22"),10,Color("#3f5233") if locked else Color("#5b7048")))
  (card.get_theme_stylebox("panel") as StyleBoxFlat).set_content_margin_all(7)
  main.store_grid.add_child(card)
  var column=VBoxContainer.new()
  column.add_theme_constant_override("separation",3)
  card.add_child(column)
  var names={"ammo":"50 universal rounds","smallMedkit":"Small medkit · +25 HP","largeMedkit":"Large medkit · +75 HP","flashlight":"Flashlight · 120 seconds","battery":"Battery · 120 seconds","trap":"Bear trap · 300 damage","repair":"Repair · +50 condition"}
  column.add_child(text(names.get(item.id,item.id.to_upper()),12,Color("#d3e690")))
  var preview=Control.new()
  preview.custom_minimum_size=Vector2(0,32)
  column.add_child(preview)
  var icon=TextureRect.new()
  icon.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
  icon.expand_mode=TextureRect.EXPAND_IGNORE_SIZE
  icon.stretch_mode=TextureRect.STRETCH_KEEP_ASPECT_CENTERED
  var icon_id=item.id
  icon.texture=load("res://assets/icons/"+icon_id+(".png" if item.kind=="weapon" else ".svg"))
  icon.modulate.a=.23 if locked else 1
  preview.add_child(icon)
  var info=""
  if item.kind=="weapon":
   var config: Dictionary=main.bridge.data.weapons[item.id]
   var mode="Auto" if config.automatic else "Double shot" if int(config.shots)==2 else "Triple burst" if int(config.shots)==3 else "Semi-auto"
   info="Damage: %d HP · %s\nMagazine: %d rounds\nRate: %.1f rounds/sec\nReload: %.1f seconds" % [config.damage,mode,config.capacity,float(config.shots)/config.interval,config.reload]
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
   var description=text(info,10,Color("#bdcbb0"))
   description.autowrap_mode=TextServer.AUTOWRAP_WORD_SMART
   description.custom_minimum_size.y=24
   column.add_child(description)
  card.tooltip_text=info
  var purchase=action("$%d" % int(item.price),func(): main.store_feedback=main.battle.buy(item.id);render_store(),false,24)
  purchase.disabled=locked or main.battle.money<int(item.price)
  purchase.tooltip_text=info
  green_button(purchase)
  column.add_child(purchase)
  if item.kind=="weapon":
   var unlock=text("Available after completing wave %d" % int(main.bridge.data.weapons[item.id].unlockAfter) if locked else "Hover for weapon statistics",9,Color("#a9b99a"))
   unlock.horizontal_alignment=HORIZONTAL_ALIGNMENT_CENTER
   column.add_child(unlock)
 main.store_message.text=main.store_feedback if main.store_feedback!="" else "H · Small medkit ×%d / J · Large medkit ×%d" % main.battle.medkits
 layout()

func layout() -> void:
 if main.network_ui!=null: main.network_ui.layout()
 var size=main.screen.size
 if size.x<=0: return
 if is_instance_valid(compass_widget):
  compass_widget.position=Vector2((size.x-minf(400,size.x*.55))*.5,0)
  compass_widget.size=Vector2(minf(400,size.x*.55),68)
 if is_instance_valid(notice_box):
  var width=minf(780,size.x*.9)
  notice_box.custom_minimum_size=Vector2(width,56)
  notice_box.size.x=width
  notice_box.position=Vector2((size.x-width)*.5,size.y*.24)
 if is_instance_valid(prompt_box):
  prompt_box.position=Vector2((size.x-prompt_box.size.x)*.5,size.y-181-prompt_box.size.y)
 if is_instance_valid(main.store_panel):
  var target=Vector2(minf(940,size.x-40),minf(600,size.y-32))
  main.store_panel.custom_minimum_size=target
  main.store_panel.size=target
  main.store_panel.position=(size-target)*.5
 if is_instance_valid(main.settings_panel):
  var panel_scale=minf(1.0,minf((size.y-32)/maxf(1.0,main.settings_panel.size.y),(size.x*.55)/maxf(1.0,main.settings_panel.size.x)))
  main.settings_panel.scale=Vector2.ONE*panel_scale
  main.settings_panel.position=Vector2(size.x-main.settings_panel.size.x*panel_scale-28,maxf(16,(size.y-main.settings_panel.size.y*panel_scale)*.5))
 if is_instance_valid(main.menu): main.menu.position=Vector2(size.x*.065,maxf(65,(size.y-main.menu.size.y)*.5))

func refresh(delta: float) -> void:
 var p=main.player
 var b=main.battle
 hit_marker.visible=p.hit_time>0
 notice_box.visible=b.notice_time>0 and main.notice_label.text!=""
 notice_box.modulate.a=clampf(b.notice_time/.35,0,1)
 var prompt=main.objectives.interaction if main.objectives!=null else ""
 if prompt=="" and main.near_store(): prompt="E · ACCESS SUPPLY TERMINAL"
 interaction_label.text=prompt
 prompt_box.visible=prompt!=""
 prompt_box.position=Vector2((main.screen.size.x-prompt_box.size.x)*.5,main.screen.size.y-181-prompt_box.size.y)
 main.screen.get_meta("post_material").set_shader_parameter("hurt",p.hurt_time/.35)
 mission_status.text=""
 if main.objectives!=null and not main.objectives.mission.is_empty() and main.objectives.mission.status=="active":
  var m=main.objectives.mission
  mission_status.text="MISSION · "+str(m.kind).to_upper()+" · $"+str(m.reward)
  if m.kind=="growth": mission_status.text+="\nGROWTH HP "+str(m.hp)

 main.hud.text="▪ SECTOR 00"
 if main.climate!=null:
  var climate=main.climate
  weather_line.text="%02d:%02d · %s · %s · %s fog" % [int(climate.hour),int(fmod(climate.hour,1)*60),"Night" if climate.night else "Day",climate.kind.capitalize(),climate.fog.capitalize()]
 var countdown=maxf(0,(60 if b.phase=="PREPARATION" else 30 if b.phase=="INTERMISSION" else 120)-b.time)
 main.wave_label.text=("%s · %ds" % [("WAVE %d · ASSAULT%s" % [b.number,(" · "+b.variant_label()) if b.variant_label()!="" else ""]) if b.phase=="ASSAULT" else b.phase,int(countdown)])+"

ALIVE: %d · SPAWNED: %d/%d" % [b.enemies.size(),b.spawned,b.total()]
 if b.phase=="CLEARANCE": main.wave_label.text="CLEAR ALL SURVIVORS

ALIVE: %d" % b.enemies.size()
 if main.objectives!=null: main.wave_label.text+="
Next airdrop: %ds" % (480-int(fmod(main.objectives.elapsed,480)))
 health.text="%d HP" % p.health
 health_bar.value=p.health
 main.stamina_bar.get_meta("number").text="%d%%" % int(p.stamina*20)
 main.wallet_label.text="$%d" % b.money
 condition.text="%d%%" % int(b.inventory.get(p.config.id,{}).get("durability",100))
 weapon.text=p.config.id.to_upper()
 main.ammo_label.text=str(p.ammo)
 reserve.text="/ %d" % p.reserve
 total_ammo.text="RELOADING" if p.reloading>0 else "%d/500 · AMMO" % carried_ammo()
 update_slots(b.primary,str(p.config.id))
 main.gear_label.text="F · Flashlight %d%%  ·  B ×%d  ·  3 Traps ×%d  ·  H/J %d/%d" % [int(b.battery/120*100),b.batteries,b.traps,b.medkits[0],b.medkits[1]]
 menu_shade.visible=(main.network_ui!=null and main.network_ui.panel.visible) or main.menu.visible or is_instance_valid(main.settings_panel) and main.settings_panel.visible
 store_shade.visible=is_instance_valid(main.store_panel) and main.store_panel.visible
 for node in main.get_tree().get_nodes_in_group("menu_chrome"): node.visible=main.menu.visible or (main.network_ui!=null and main.network_ui.panel.visible)
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
