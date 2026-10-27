extends Node
# Persistent transport; world authority always belongs to the lobby host.
signal packet(message: Dictionary)
signal changed
var active=false
var hosting=false
var match_active=false
var transport=""
var peer_id=""
var code=""
var token=""
var seed_value=42
var members: Array=[]
var status="Choose LAN or a WebSocket server."
var socket: WebSocketPeer
var endpoint=""
var pending: Dictionary={}
var was_open=false
var closed_handled=false
var reconnect_at=0.0
var retries=0
var credit_received=0
var ping_ms=0
var ping_clock=0.0
var enet: ENetMultiplayerPeer
var countdown=-1.0

func _ready() -> void:
 multiplayer.peer_connected.connect(_joined)
 multiplayer.peer_disconnected.connect(_left)
 multiplayer.connection_failed.connect(func(): status="LAN connection failed. Check address/firewall.";changed.emit())
 multiplayer.server_disconnected.connect(func(): call_deferred("lost_lan"))
 multiplayer.connected_to_server.connect(func(): peer_id=str(multiplayer.get_unique_id());status="Connected to LAN host";changed.emit())
 var settings=ConfigFile.new()
 if settings.load("user://network.cfg")==OK: endpoint=str(settings.get_value("network","endpoint",""))

func lost_lan() -> void:
 close()
 status="LAN host disconnected."
 packet.emit({"type":"closed"})
 changed.emit()

func authority() -> bool: return not active or hosting

func close() -> void:
 if socket!=null:
  if socket.get_ready_state()==WebSocketPeer.STATE_OPEN: socket.send_text(JSON.stringify({"type":"leave"}));socket.poll()
  socket.close()
 socket=null
 if enet!=null: multiplayer.multiplayer_peer=null;enet.close();enet=null
 active=false
 hosting=false
 match_active=false
 transport=""
 peer_id=""
 token=""
 code=""
 members=[]
 countdown=-1
 reconnect_at=0
 retries=0
 credit_received=0
 changed.emit()

func host_lan(port=3001) -> Error:
 close()
 enet=ENetMultiplayerPeer.new()
 var result=enet.create_server(port,3)
 if result!=OK: status="Cannot listen on UDP port %d" % port;changed.emit();return result
 multiplayer.multiplayer_peer=enet
 transport="lan"
 active=true
 hosting=true
 peer_id="1"
 seed_value=randi_range(1,2147483646)
 members=[{"id":"1","connected":true}]
 status="LAN host · UDP %d · invite with your LAN IP" % port
 changed.emit()
 return OK

func join_lan(address: String,port=3001) -> Error:
 close()
 enet=ENetMultiplayerPeer.new()
 var result=enet.create_client(address.strip_edges(),port)
 if result!=OK: status="Invalid LAN address";changed.emit();return result
 multiplayer.multiplayer_peer=enet
 transport="lan"
 status="Connecting to LAN…"
 changed.emit()
 return OK

func connect_server(url: String,join_code="") -> Error:
 close()
 endpoint=url.strip_edges().trim_suffix("/")
 if endpoint.begins_with("https://"): endpoint="wss://"+endpoint.substr(8)
 if endpoint.begins_with("http://"): endpoint="ws://"+endpoint.substr(7)
 if not endpoint.begins_with("ws://") and not endpoint.begins_with("wss://"):
  status="Use ws://address:3000 or wss://your-service.onrender.com";changed.emit();return ERR_INVALID_PARAMETER
 if not endpoint.ends_with("/godot"): endpoint+="/godot"
 transport="ws"
 seed_value=randi_range(1,2147483646)
 pending={"type":"create","seed":seed_value} if join_code.is_empty() else {"type":"join","code":join_code.strip_edges()}
 var settings=ConfigFile.new()
 settings.set_value("network","endpoint",endpoint)
 settings.save("user://network.cfg")
 return open_socket()

func open_socket() -> Error:
 socket=WebSocketPeer.new()
 socket.inbound_buffer_size=2097152
 socket.outbound_buffer_size=2097152
 was_open=false
 closed_handled=false
 var result=socket.connect_to_url(endpoint)
 status="Connecting… Render may take a minute to wake up."
 if result!=OK: status="Connection failed: "+error_string(result);socket=null
 changed.emit()
 return result

func send(message: Dictionary) -> void:
 if transport=="ws":
  if socket!=null and socket.get_ready_state()==WebSocketPeer.STATE_OPEN:
   if message.get("type","") in ["pose","snapshot"] and socket.get_current_outbound_buffered_amount()>262144: return
   socket.send_text(JSON.stringify(message))
 elif transport=="lan" and active:
  if hosting: relay(message,peer_id)
  else: receive_lan.rpc_id(1,message)

@rpc("any_peer","call_remote","reliable")
func receive_lan(message: Dictionary) -> void:
 if not hosting: return
 var sender=str(multiplayer.get_remote_sender_id())
 if not members.any(func(m): return m.id==sender): return
 if message.get("type","") in ["pose","shot","trap","revive"]: relay(message,sender)

func relay(message: Dictionary,sender: String) -> void:
 var m=message.duplicate(true)
 m.id=sender
 for member in members:
  if member.id!="1" and member.id!=sender:
   if m.type in ["pose","snapshot"]: deliver_stream.rpc_id(int(member.id),m)
   else: deliver_lan.rpc_id(int(member.id),m)
 if sender!="1": packet.emit(m)

@rpc("authority","call_remote","reliable")
func deliver_lan(message: Dictionary) -> void: accept(message)

@rpc("authority","call_remote","unreliable_ordered",1)
func deliver_stream(message: Dictionary) -> void: accept(message)

func _joined(id: int) -> void:
 if transport!="lan" or not hosting: return
 if match_active: enet.disconnect_peer(id);return
 members.append({"id":str(id),"connected":true})
 deliver_lan.rpc_id(id,{"type":"room","id":str(id),"host":false,"seed":seed_value,"members":members,"phase":"lobby"})
 relay({"type":"lobby","members":members},"1")
 changed.emit()

func _left(id: int) -> void:
 if transport!="lan" or not hosting: return
 members=members.filter(func(m): return m.id!=str(id))
 relay({"type":"away","id":str(id)},str(id))
 relay({"type":"lobby","members":members},"1")
 packet.emit({"type":"away","id":str(id)})
 changed.emit()

func start_match() -> void:
 if not active or hosting:
  if active: send({"type":"countdown","seconds":5})
  countdown=5.0
  status="Starting in 5…"
  changed.emit()

func accept(message: Dictionary) -> void:
 var type=str(message.get("type",""))
 if type=="room":
  active=true
  peer_id=str(message.id)
  hosting=bool(message.host)
  code=str(message.get("code",""))
  token=str(message.get("token",""))
  seed_value=int(message.seed)
  members=message.members
  retries=0
  status="Room "+code if transport=="ws" else "LAN lobby"
  if message.get("phase","")=="playing" and not match_active:
   match_active=true
   call_deferred("resume_started")
  elif hosting and message.get("phase","")=="countdown" and countdown<0: send({"type":"start"})
 elif type=="lobby": members=message.members
 elif type=="countdown": status="Host starting match in 5 seconds…"
 elif type=="start":
  match_active=true
  seed_value=int(message.seed)
 elif type=="error": status=str(message.get("message","Connection error"));reconnect_at=0;pending={}
 elif type=="closed": close();status="Host closed the lobby."
 elif type=="pong": ping_ms=Time.get_ticks_msec()-int(message.sent)
 packet.emit(message)
 changed.emit()

func resume_started() -> void: packet.emit({"type":"start","seed":seed_value})

func _process(delta: float) -> void:
 if countdown>=0:
  countdown-=delta
  status="Starting in %d…" % ceili(maxf(0,countdown))
  changed.emit()
  if countdown<=0:
   countdown=-1
   if active: send({"type":"start","seed":seed_value})
   if transport!="ws": accept({"type":"start","seed":seed_value})
 if socket==null: return
 socket.poll()
 var state=socket.get_ready_state()
 if state==WebSocketPeer.STATE_OPEN:
  if not was_open: was_open=true;socket.send_text(JSON.stringify(pending))
  while socket!=null and socket.get_available_packet_count()>0:
   var message=JSON.parse_string(socket.get_packet().get_string_from_utf8())
   if message is Dictionary: accept(message)
  ping_clock+=delta
  if active and ping_clock>3: ping_clock=0;send({"type":"ping","sent":Time.get_ticks_msec()})
 elif state==WebSocketPeer.STATE_CLOSED:
  if not closed_handled:
   closed_handled=true
   was_open=false
   status="Connection lost · reconnecting…" if not token.is_empty() else "Server unavailable. Check address and service logs."
   packet.emit({"type":"transport-lost"})
   reconnect_at=Time.get_ticks_msec()+minf(8000,pow(2,retries)*1000)
   changed.emit()
  if not token.is_empty() and retries<15 and Time.get_ticks_msec()>=reconnect_at:
   pending={"type":"resume","code":code,"token":token}
   retries+=1
   open_socket()
   reconnect_at=Time.get_ticks_msec()+minf(8000,pow(2,retries)*1000)

  elif token.is_empty() and not active and not pending.is_empty() and retries<8 and Time.get_ticks_msec()>=reconnect_at:
   retries+=1
   open_socket()
   status="Connecting to server · attempt %d/8…" % retries
   changed.emit()
