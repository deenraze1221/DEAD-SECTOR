extends RefCounted
# Lobby window laid out like the HTML game: four player cards, a PLAY / OPEN TO NETWORK / BACK row,
# and a collapsible network section (LAN / server) that keeps every existing connection feature.
var main
var panel: PanelContainer
var status: Label
var cards: Array=[]
var card_names: Array=[]
var card_notes: Array=[]
var address: LineEdit
var invite: LineEdit
var lan_ip: LineEdit
var start: Button
var network_button: Button
var leave_button: Button
var network_box: VBoxContainer
var tabs: TabContainer
var code_label: Label
var copy_button: Button
var countdown_label: Label

func card_style(occupied: bool) -> StyleBoxFlat:
 var ui=main.reference_ui
 var style=ui.box(Color("#17251f") if occupied else Color(.045,.075,.09,.7),16,Color("#cddaa1") if occupied else Color("#2a393d"))
 style.border_width_top=3 if occupied else 1
 return style

func build(game) -> void:
 main=game
 var ui=main.reference_ui
 panel=ui.panel(main.screen,Vector2(900,0),Vector2(-450,-265),Control.PRESET_CENTER)
 panel.add_theme_stylebox_override("panel",ui.box(Color(.04,.06,.08,.97),28,Color("#34434a")))
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",14)
 panel.add_child(column)
 column.add_child(ui.text("LOBBY · DEAD SECTOR",22,Color("#e6ece8")))
 column.add_child(ui.text("UP TO 4 PLAYERS",10,Color("#aebca0")))
 var row=HBoxContainer.new()
 row.add_theme_constant_override("separation",12)
 row.custom_minimum_size.y=150
 column.add_child(row)
 for i in range(4):
  var card=PanelContainer.new()
  card.size_flags_horizontal=Control.SIZE_EXPAND_FILL
  card.custom_minimum_size=Vector2(190,150)
  var inner=VBoxContainer.new()
  inner.alignment=BoxContainer.ALIGNMENT_CENTER
  inner.add_theme_constant_override("separation",14)
  var name_label=ui.text("",12,Color("#e6ece8"))
  var note_label=ui.text("",10,Color("#aebca0"))
  inner.add_child(name_label)
  inner.add_child(note_label)
  card.add_child(inner)
  row.add_child(card)
  cards.append(card)
  card_names.append(name_label)
  card_notes.append(note_label)
 countdown_label=ui.text("",30,Color("#cddaa1"))
 column.add_child(countdown_label)
 column.add_child(HSeparator.new())
 var actions=HBoxContainer.new()
 actions.add_theme_constant_override("separation",12)
 column.add_child(actions)
 start=ui.action("PLAY",func():
  if not main.net.active: main.net.seed_value=int(main.generation.seed)
  main.net.start_match()
 ,true,40)
 start.custom_minimum_size.x=96
 actions.add_child(start)
 network_button=ui.action("OPEN TO NETWORK",func(): set_network_open(not network_box.visible),false,40)
 actions.add_child(network_button)
 leave_button=ui.action("LEAVE LOBBY",func(): main.net.close();panel.hide();main.menu.show(),false,40)
 actions.add_child(leave_button)
 actions.add_child(ui.action("BACK",func(): panel.hide();main.menu.show(),false,40))
 status=ui.text("",10,Color("#b4c5c2"))
 status.autowrap_mode=TextServer.AUTOWRAP_WORD_SMART
 status.custom_minimum_size=Vector2(820,18)
 column.add_child(status)
 network_box=VBoxContainer.new()
 network_box.add_theme_constant_override("separation",10)
 column.add_child(network_box)
 var code_row=HBoxContainer.new()
 code_label=ui.text("",18,Color("#dec995"))
 code_label.size_flags_horizontal=Control.SIZE_EXPAND_FILL
 code_row.add_child(code_label)
 copy_button=ui.action("COPY",func(): DisplayServer.clipboard_set(main.net.code),false,26)
 code_row.add_child(copy_button)
 network_box.add_child(code_row)
 tabs=TabContainer.new()
 network_box.add_child(tabs)
 var lan=VBoxContainer.new();lan.name="LAN";lan.add_theme_constant_override("separation",8);tabs.add_child(lan)
 lan.add_child(ui.text("Direct LAN · no external server · UDP 3001",11))
 var ips=[]
 for ip in IP.get_local_addresses():
  if "." in ip and not ip.begins_with("127.") and not ip.begins_with("169.254."): ips.append(ip)
 var local_ip=ui.text("YOUR LAN IP: "+(", ".join(ips) if not ips.is_empty() else "Check your network adapter"),10)
 local_ip.autowrap_mode=TextServer.AUTOWRAP_WORD_SMART
 local_ip.custom_minimum_size.x=820
 lan.add_child(local_ip)
 lan.add_child(ui.action("HOST LAN LOBBY",func(): main.net.host_lan()))
 lan_ip=LineEdit.new();lan_ip.placeholder_text="Host LAN IP, e.g. 192.168.1.10";lan_ip.custom_minimum_size.y=32;lan.add_child(lan_ip)
 lan.add_child(ui.action("JOIN LAN",func(): main.net.join_lan(lan_ip.text)))
 var server=VBoxContainer.new();server.name="SERVER";server.add_theme_constant_override("separation",8);tabs.add_child(server)
 server.add_child(ui.text("Invite friends with the same server and room code.",10))
 address=LineEdit.new();address.placeholder_text="wss://your-service.onrender.com";address.text=main.net.endpoint.trim_suffix("/godot");address.custom_minimum_size.y=32;server.add_child(address)
 server.add_child(ui.action("OPEN SERVER LOBBY",func(): main.net.connect_server(address.text)))
 invite=LineEdit.new();invite.placeholder_text="8-digit invitation code";invite.max_length=8;invite.custom_minimum_size.y=32;server.add_child(invite)
 server.add_child(ui.action("JOIN BY CODE",func():
  if invite.text.length()!=8 or not invite.text.is_valid_int(): main.net.status="Enter all 8 digits of the invitation code.";main.net.changed.emit()
  else: main.net.connect_server(address.text,invite.text)))
 network_box.hide()
 main.net.changed.connect(refresh)
 panel.resized.connect(layout)
 panel.hide()
 refresh()
 layout()

func set_network_open(value: bool) -> void:
 network_box.visible=value
 network_button.text="HIDE NETWORK" if value else "OPEN TO NETWORK"
 panel.reset_size()
 layout()

func layout() -> void:
 if not is_instance_valid(panel) or panel.size.y<1: return
 var viewport_size=main.screen.size
 var fit=minf(1,minf((viewport_size.x-40)/panel.size.x,(viewport_size.y-32)/panel.size.y))
 panel.scale=Vector2.ONE*fit
 panel.position=(viewport_size-panel.size*fit)*.5

func open(join=false) -> void:
 main.menu.hide()
 panel.show()
 if join:
  set_network_open(true)
  tabs.current_tab=1
 else: set_network_open(false)
 layout()
 refresh()

func refresh() -> void:
 if not is_instance_valid(panel): return
 var net=main.net
 for i in range(4):
  var occupied=i<net.members.size() or (i==0 and not net.active)
  cards[i].add_theme_stylebox_override("panel",card_style(occupied))
  if i<net.members.size():
   var member=net.members[i]
   var mine=member.id==net.peer_id
   card_names[i].text="PLAYER %d%s" % [i+1," · YOU" if mine else ""]
   card_notes[i].text=("HOST" if i==0 else "IN LOBBY")+(" · DISCONNECTED" if not member.get("connected",true) else "")
  elif i==0 and not net.active:
   card_names[i].text="PLAYER 1 · YOU"
   card_notes[i].text="HOST"
  else:
   card_names[i].text="EMPTY"
   card_notes[i].text="INVITE A FRIEND"
 copy_button.disabled=net.code.is_empty()
 code_label.text="INVITE CODE  "+net.code if not net.code.is_empty() else "LOCAL SQUAD" if net.transport=="lan" else "SOLO / COOPERATIVE"
 status.text=net.status
 countdown_label.text="%d" % ceili(maxf(0,net.countdown)) if net.countdown>=0 else ""
 countdown_label.visible=net.countdown>=0
 leave_button.visible=net.active
 start.disabled=(net.active and not net.hosting) or net.countdown>=0 or (not net.active and net.transport!="")
 start.text="WAITING FOR HOST" if net.active and not net.hosting else "STARTING…" if net.countdown>=0 else "PLAY"
