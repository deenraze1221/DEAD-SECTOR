extends SceneTree
var failures=[]
func check(ok: bool,text: String) -> void:
 if not ok: failures.append(text);push_error(text)
func _initialize() -> void: call_deferred("run")
func contacts(boxes: Array) -> Dictionary:
 var buckets={}
 var pairs={}
 for i in range(boxes.size()):
  var a=boxes[i]
  if a.minY>=1.8 or a.maxY<=.25: continue
  for x in range(floori(a.minX/5),floori(a.maxX/5)+1):
   for z in range(floori(a.minZ/5),floori(a.maxZ/5)+1):
    var key=Vector2i(x,z)
    for j in buckets.get(key,[]):
     var b=boxes[j]
     if str(a.get("owner"))==str(b.get("owner")): continue
     if minf(a.maxX,b.maxX)-maxf(a.minX,b.minX)>.01 and minf(a.maxZ,b.maxZ)-maxf(a.minZ,b.minZ)>.01 and minf(a.maxY,b.maxY)-maxf(a.minY,b.minY)>.01: pairs[contact_signature(a,b)]=true
    if not buckets.has(key): buckets[key]=[]
    buckets[key].append(i)
 return pairs
func signature(box: Dictionary) -> String:
 return str([box.kind,snappedf(fposmod(box.minX,24),.001),snappedf(fposmod(box.minZ,24),.001),snappedf(box.maxX-box.minX,.001),snappedf(box.maxZ-box.minZ,.001),snappedf(box.minY,.001),snappedf(box.maxY,.001)])
func contact_signature(a: Dictionary,b: Dictionary) -> String:
 var values=[signature(a),signature(b)]
 values.sort()
 return str(values)
func run() -> void:
 var Bridge=load("res://scripts/bridge.gd")
 var Generator=load("res://scripts/city_generator.gd")
 var bridge=Bridge.new()
 bridge.load_data()
 var source=JSON.parse_string(FileAccess.get_file_as_string("res://assets/city.json"))
 var original_contacts=contacts(source.colliders)
 var plans=[]
 var interior_hashes=[]
 var interior_factory=load("res://scripts/interior_layout.gd").new()
 interior_factory.capture(source,bridge)
 for seed_value in [123,123,456]:
  var data=source.duplicate(true)
  var result=Generator.new().apply(data,seed_value,bridge)
  print("Generation: ",result.seed," moved=",result.moved_blocks," pinned=",result.pinned_blocks)
  check(result.moved_blocks>10,"Generator must move substantial blocks")
  check(data.spawn==source.spawn and data.terminal==source.terminal,"Store/spawn remain fixed")
  var new_contacts=contacts(data.colliders)
  var extra=0
  for key in new_contacts:
   if not original_contacts.has(key): extra+=1
  check(extra==0,"No new overlapping solid collider pairs; found "+str(extra))
  plans.append(result.plan)
  interior_factory.rebuild(data,seed_value)
  interior_hashes.append(hash(data.fresh_batches))
  var fresh_pairs=contacts(data.colliders)
  var extra_rooms=0
  for key in fresh_pairs:
   if not original_contacts.has(key): extra_rooms+=1
  check(extra_rooms==0,"Fresh rooms have no inter-object overlaps seed"+str(seed_value)+": "+str(extra_rooms))
 check(interior_hashes[0]==interior_hashes[1] and interior_hashes[0]!=interior_hashes[2],"Fresh furniture and rooms repeat by seed and vary across seeds")
 check(plans[0]==plans[1] and plans[0]!=plans[2],"Seeds repeat and differ")
 var Climate=load("res://scripts/climate.gd")
 var counts={}
 for seed_value in range(1000):
  var state=Climate.selection((seed_value*2654435761)&0xffffffff,7)
  counts[state.weather]=counts.get(state.weather,0)+1
 check(counts.size()==4,"All weather types possible")
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 await process_frame
 var fresh_contacts=contacts(scene.bridge.data.colliders)
 var fresh_extra=0
 for key in fresh_contacts:
  if not original_contacts.has(key): fresh_extra+=1
 check(fresh_extra==0,"Fresh interiors must not overlap other objects: "+str(fresh_extra))
 check(scene.body.legs.size()==2,"Local soldier has two animated legs")
 check(scene.sound.library.calm.size()==5 and scene.sound.library.tense.size()==5,"All10 original scores")
 for id in scene.bridge.data.weapons: check(scene.sound.stream("shot-"+id)!=null,"Original sound "+id)
 var saved=scene.master_volume
 scene.master_volume=.42
 scene.save_user_settings()
 scene.master_volume=1
 scene.load_user_settings()
 check(is_equal_approx(scene.master_volume,.42),"Volume persists between launches")
 scene.master_volume=saved
 scene.save_user_settings()
 var calm_index=scene.sound.index.calm
 scene.sound.calm.finished.emit()
 check(scene.sound.index.calm==calm_index+1,"Playlist advances")
 scene.set_playing(true)
 for wave in [1,2,3,4,5]:
  scene.battle.number=wave
  scene.battle.phase="ASSAULT"
  scene.battle.time=60
  scene.climate._process(.1)
  check(scene.climate.night==((wave-1)%4>=2),"Day/night sequence")
 scene.player.camera.rotation.x=0
 scene.body._process(1)
 check(not scene.body.visible,"Body hidden when looking ahead")
 scene.player.camera.rotation.x=-PI/3
 scene.body._process(.1)
 check(scene.body.visible and scene.body.alpha<.86,"Body fades into compact lookdown view")
 scene.open_settings()
 check(scene.settings_panel.visible,"Settings opens")
 scene.set_playing(true)
 check(not scene.settings_panel.visible,"Resume closes settings")
 if failures.is_empty(): print("PASS EXPANSION: seed generation, collision provenance, original audio, weather, day/night, body and settings")
 quit(0 if failures.is_empty() else 1)
