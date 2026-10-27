extends RefCounted
var main
var panel: PanelContainer
var status: Label
var seats: Label
var address: LineEdit
var invite: LineEdit
var lan_ip: LineEdit
var start: Button
var code_label: Label
var copy_button: Button

func build(game) -> void:
 main=game
 var ui=main.reference_ui
 panel=ui.panel(main.screen,Vector2(520,0),Vector2(-260,-265),Control.PRESET_CENTER)
 panel.add_theme_stylebox_override("panel",ui.box(Color(.035,.063,.082,.98),20,Color("#7e907d")))
 var column=VBoxContainer.new()
 column.add_theme_constant_override("separation",10)
 panel.add_child(column)
 column.add_child(ui.text("SQUAD LOBBY · 4 PLAYERS",18,Color("#cddaa1")))
 seats=ui.text("",12)
 column.add_child(seats)
 code_label=ui.text("",18,Color("#dec995"))
 var code_row=HBoxContainer.new()
 code_label.size_flags_horizontal=Control.SIZE_EXPAND_FILL
 code_row.add_child(code_label)
 copy_button=ui.action("COPY",func(): DisplayServer.clipboard_set(main.net.code),false,26)
 code_row.add_child(copy_button)
 column.add_child(code_row)
 var tabs=TabContainer.new()
 column.add_child(tabs)
 var lan=VBoxContainer.new();lan.name="LAN";lan.add_theme_constant_override("separation",8);tabs.add_child(lan)
 lan.add_child(ui.text("Direct LAN · no external server · UDP 3001",11))
 var ips=[]
 for ip in IP.get_local_addresses():
  if "." in ip and not ip.begins_with("127.") and not ip.begins_with("169.254."): ips.append(ip)
 var local_ip=ui.text("YOUR LAN IP: "+(", ".join(ips) if not ips.is_empty() else "Check your network adapter"),10)
 local_ip.autowrap_mode=TextServer.AUTOWRAP_WORD_SMART
 local_ip.custom_minimum_size.x=460
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
 status=ui.text("",10,Color("#b4c5c2"));status.autowrap_mode=TextServer.AUTOWRAP_WORD_SMART;status.custom_minimum_size=Vector2(460,30);column.add_child(status)
 start=ui.action("START MATCH · 5 SECOND COUNTDOWN",func():
  if not main.net.active: main.net.seed_value=int(main.generation.seed)
  main.net.start_match()
 ,true)
 column.add_child(start)
 column.add_child(ui.action("LEAVE LOBBY",func(): main.net.close();panel.hide();main.menu.show()))
 column.add_child(ui.action("BACK",func(): panel.hide();main.menu.show(),false,26))
 main.net.changed.connect(refresh)
 panel.resized.connect(layout)
 panel.hide()
 refresh()
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
 layout()
 if join: panel.get_child(0).get_child(3).current_tab=1
 refresh()

func refresh() -> void:
 if not is_instance_valid(panel): return
 var net=main.net
 var rows=[]
 for i in range(4):
  if i<net.members.size():
   var member=net.members[i]
   rows.append("%d  %s%s%s" % [i+1,"YOU" if member.id==net.peer_id else "SQUAD "+str(member.id).to_upper()," · HOST" if i==0 else ""," · DISCONNECTED" if not member.get("connected",true) else ""])
  else: rows.append("%d  %s" % [i+1,"YOU · HOST" if i==0 and not net.active else "EMPTY SLOT"])
 seats.text="\n".join(rows)
 copy_button.disabled=net.code.is_empty()
 code_label.text="INVITE CODE  "+net.code if not net.code.is_empty() else "LOCAL SQUAD" if net.transport=="lan" else "SOLO / COOPERATIVE"
 status.text=net.status
 start.disabled=(net.active and not net.hosting) or net.countdown>=0 or (not net.active and net.transport!="")
 start.text="WAITING FOR HOST" if net.active and not net.hosting else "STARTING…" if net.countdown>=0 else "START MATCH · 5 SECOND COUNTDOWN"
