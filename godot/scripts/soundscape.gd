extends Node
var streams: Dictionary={}
var calm: AudioStreamPlayer
var tense: AudioStreamPlayer
var rain: AudioStreamPlayer
var wind: AudioStreamPlayer
var library: Dictionary
var index={"calm":0,"tense":0}
var main
var terminal_timer=0.0

func stream(id: String) -> AudioStream:
 if not streams.has(id):
  var path="res://assets/audio/"+id+".wav"
  if not ResourceLoader.exists(path): return null
  streams[id]=load(path)
 return streams[id]

func play(id: String, at: Variant=null, volume=1.0) -> void:
 var source=stream(id)
 if source==null: return
 var node
 if at is Vector3:
  node=AudioStreamPlayer3D.new()
  node.max_distance=15 if id=="trap" else 8 if id=="terminal" else 25
  node.unit_size=4
 else: node=AudioStreamPlayer.new()
 node.stream=source
 node.volume_db=linear_to_db(maxf(.0001,volume))
 add_child(node)
 if at is Vector3: node.global_position=at
 node.finished.connect(node.queue_free)
 node.play()

func _ready() -> void:
 library=JSON.parse_string(FileAccess.get_file_as_string("res://assets/audio/library.json"))
 for mood in ["calm","tense"]:
  var node=AudioStreamPlayer.new()
  add_child(node)
  node.volume_db=-60
  node.finished.connect(func(): next_track(mood))
  if mood=="calm": calm=node
  else: tense=node
 next_track("calm")
 next_track("tense")
 rain=bed("rain")
 wind=bed("wind")

func bed(id: String) -> AudioStreamPlayer:
 var node=AudioStreamPlayer.new()
 var wave=stream(id).duplicate()
 wave.loop_mode=AudioStreamWAV.LOOP_FORWARD
 wave.loop_begin=0
 wave.loop_end=wave.data.size()/2
 node.stream=wave
 node.volume_db=-60
 add_child(node)
 node.play()
 return node

func next_track(mood: String) -> void:
 var node=calm if mood=="calm" else tense
 node.stream=stream(mood+"-"+str(index[mood]%5))
 node.play()
 index[mood]+=1

func _process(delta: float) -> void:
 if main.battle==null or main.climate==null: return
 var active=main.player.active
 var assault=active and main.battle.phase in ["ASSAULT","CLEARANCE"]
 var factor=1-exp(-delta/1.2)
 calm.volume_db=lerpf(calm.volume_db,linear_to_db(maxf(.0001,main.music_volume)) if not assault else -60,factor)
 tense.volume_db=lerpf(tense.volume_db,linear_to_db(maxf(.0001,main.music_volume)) if assault else -60,factor)
 var wet=active and main.climate.kind in ["rain","storm"]
 rain.volume_db=lerpf(rain.volume_db,linear_to_db(maxf(.0001,main.weather_volume)) if wet else -60,factor)
 wind.volume_db=lerpf(wind.volume_db,linear_to_db(maxf(.0001,main.weather_volume*.55)) if active else -60,factor)
 terminal_timer-=delta
 if active and terminal_timer<=0:
  terminal_timer=2.4
  var terminal=Vector3(main.bridge.data.terminal.x,1.4,main.bridge.data.terminal.z)
  if terminal.distance_to(main.player.position)<8: play("terminal",terminal)
