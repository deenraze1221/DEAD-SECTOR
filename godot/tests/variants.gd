extends SceneTree
var failures=[]
func check(ok: bool,text: String) -> void:
 if not ok: failures.append(text);push_error(text)
func _initialize() -> void: call_deferred("run")
func run() -> void:
 var scene=load("res://main.tscn").instantiate()
 root.add_child(scene)
 await process_frame
 var b=scene.battle
 var expected={1:"standard",2:"siege",3:"rush",4:"blackout",5:"boss",6:"rush",8:"blackout",9:"rush",10:"boss",11:"siege",12:"blackout",20:"boss"}
 for n in expected:
  b.number=n
  check(b.variant()==expected[n],"wave %d should be %s, got %s" % [n,expected[n],b.variant()])
 b.number=4;b.phase="PREPARATION"
 check(b.blackout_district()==-1,"no blackout during preparation")
 b.phase="ASSAULT"
 check(b.blackout_district()==0,"wave 4 blackout district 0, got %d" % b.blackout_district())
 b.number=8
 check(b.blackout_district()==1,"wave 8 district 1")
 b.number=12
 check(b.blackout_district()==2,"wave 12 district 2")
 b.number=16
 check(b.blackout_district()==3,"wave 16 district 3")
 b.phase="INTERMISSION"
 check(b.blackout_district()==-1,"power restored at rest")
 b.number=3;b.phase="ASSAULT"
 check(b.blackout_district()==-1,"rush wave has no blackout")
 check(b.district_of(-10,-10)==0 and b.district_of(-10,10)==1 and b.district_of(10,-10)==2 and b.district_of(10,10)==3,"district quadrants")
 # Rush wave: at least half runners on average.
 b.number=3
 var runners=0
 for i in range(400): if b.choose_type()=="runner": runners+=1
 check(runners>=200,"rush wave should be runner heavy, got %d/400" % runners)
 # Siege wave: every spawn comes from hive number%4.
 b.number=2
 for i in range(6): b.spawn_enemy("zombie")
 var hive=scene.bridge.data.hives[2%4]
 for e in b.enemies: check(e.position.distance_to(Vector3(hive.x,0,hive.z))<30,"siege spawns near hive 2")
 # Blackout applies shader parameter and removes lamp lights.
 for e in b.enemies.duplicate(): e.queue_free()
 b.enemies.clear()
 scene.set_playing(true)
 b.number=4;b.phase="ASSAULT"
 for i in range(6): await physics_frame
 for i in range(6): await process_frame
 check(scene.climate.applied_cut==0,"climate applies blackout district")
 check(scene.atmosphere.applied_cut==0,"atmosphere applies blackout district")
 b.phase="INTERMISSION"
 for i in range(6): await process_frame
 check(scene.climate.applied_cut==-1,"climate restores power")
 # Flying eyes appear only after wave 10.
 for n in [1,6,10]:
  b.number=n;b.vomiters=999
  var eyes=0
  for i in range(600): if b.choose_type()=="eye": eyes+=1
  check(eyes==0,"no eyes on wave %d, got %d" % [n,eyes])
 b.number=11
 var eyes_late=0
 for i in range(600): if b.choose_type()=="eye": eyes_late+=1
 check(eyes_late>0,"eyes must appear on wave 11")
 if failures.is_empty(): print("PASS VARIANTS: wave variants, blackout districts, rush mix, siege hive, power restore")
 else: for f in failures: print("FAIL ",f)
 quit(0 if failures.is_empty() else 1)
