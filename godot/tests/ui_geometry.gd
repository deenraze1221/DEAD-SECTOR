extends SceneTree
var failures=[]
func check(ok: bool,message: String) -> void:
 if not ok: failures.append(message);push_error(message)
func _initialize() -> void: call_deferred("run")
func run() -> void:
 var main=load("res://main.tscn").instantiate()
 root.add_child(main)
 for i in range(4): await process_frame
 var ui=main.reference_ui
 check(ui.mono.font_names[0]=="Courier New","Terminal uses browser monospace family on Windows")
 check(ui.heading.font_names[0]=="Arial","Brand uses browser Arial family")
 check(is_equal_approx(main.player.weapon_camera.fov,45),"Pistol uses original HTML viewmodel projection")
 check(main.player.gun.scale==Vector3.ONE,"Weapon uses source model scale")
 check(ui.menu_camera.current and not ui.started,"Fresh launch shows real cinematic city camera")
 check(not ui.hud_root.visible,"Gameplay HUD hidden in menu")
 for size in [Vector2(960,600),Vector2(1280,720),Vector2(1920,1080)]:
  main.screen.size=size
  ui.layout()
  for i in range(3): await process_frame
  var bounds=Rect2(Vector2.ZERO,size)
  check(bounds.encloses(main.menu.get_global_rect()),"Menu in bounds "+str(size))
  main.open_settings()
  for i in range(3): await process_frame
  check(bounds.encloses(main.settings_panel.get_global_rect()),"Settings in bounds "+str(size))
  main.settings_panel.hide()
  main.open_store()
  for i in range(3): await process_frame
  check(bounds.encloses(main.store_panel.get_global_rect()),"Store in bounds "+str(size))
  check(main.store_grid.get_child_count()==9,"Nine weapon cards")
  check(main.store_grid.get_child(0).name=="CardShotgun","Weapons sorted by price")
  check(main.store_grid.get_child(8).name=="CardHelsing","Most expensive weapon last")
  var bottom=main.store_grid.get_child(8).get_global_rect().end.y
  check(bottom<main.store_panel.get_global_rect().end.y-20,"All nine cards fit without scrolling "+str(size))
  main.store_tab="supplies"
  main.render_store()
  for i in range(2): await process_frame
  check(main.store_grid.get_child_count()==7,"Seven supplies with descriptions")
  main.store_tab="weapon"
  main.close_store()
  main.set_playing(false)
  check(main.player.camera.current and ui.started,"Pause preserves player camera")
 main.screen.size=Vector2(1280,720)
 main.battle.money=100000
 main.battle.completed=25
 main.open_store()
 main.render_store()
 var buy=main.store_grid.get_node("CardShotgun").find_children("*","Button",true,false)[0]
 buy.pressed.emit()
 check(main.battle.primary=="Shotgun","Actual weapon purchase button buys its own card")
 check(main.battle.money==98000,"Correct UI purchase price")
 check(ui.carried_ammo()==92,"HUD counts holstered pistol ammo")
 main.close_store()
 for i in range(3): await process_frame
 check(main.reference_ui.hud_root.visible,"HUD visible in match")
 check(main.ammo_label.text==str(main.player.ammo),"Ammo widget updates live")
 if failures.is_empty(): print("PASS UI: browser font families, cinematic/pause cameras, responsive bounds, all cards, real purchase and live ammo")
 quit(0 if failures.is_empty() else 1)
