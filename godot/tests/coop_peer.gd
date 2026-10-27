extends SceneTree
var role="host"
var mode="lan"
var tag="/tmp/dead-sector-coop-lan"
var net
var main
var failures=[]
var port=3031
func check(value: bool,message: String) -> void:
 if not value: failures.append(message);push_error(message)
func wait_until(predicate: Callable,seconds=25.0) -> bool:
 var deadline=Time.get_ticks_msec()+seconds*1000
 while Time.get_ticks_msec()<deadline:
  if predicate.call(): return true
  await process_frame
 return false
func write_stage(stage: String,data: Dictionary={}) -> void:
 var file=FileAccess.open(tag+"-"+stage+".json",FileAccess.WRITE)
 file.store_string(JSON.stringify(data));file.close()
func stage_exists(stage: String) -> bool: return FileAccess.file_exists(tag+"-"+stage+".json")
func read_stage(stage: String) -> Dictionary: return JSON.parse_string(FileAccess.get_file_as_string(tag+"-"+stage+".json"))
func _initialize() -> void: call_deferred("run")
func run() -> void:
 var args=OS.get_cmdline_user_args()
 role=args[0];mode=args[1];tag=args[2]
 if args.size()>3: port=int(args[3])
 elif mode=="ws": port=3030
 net=root.get_node("CoopNetwork")
 if role=="host":
  if mode=="lan": check(net.host_lan(port)==OK,"Host LAN socket")
  else:
   net.connect_server("ws://127.0.0.1:"+str(port))
   check(await wait_until(func(): return net.active),"Host WS connected")
  write_stage("room",{"code":net.code})
  check(await wait_until(func(): return net.members.size()==2),"Two seats filled")
  net.start_match()
 else:
  check(await wait_until(func(): return stage_exists("room")),"Host room available")
  if mode=="lan": check(net.join_lan("127.0.0.1",port)==OK,"Guest LAN socket")
  else: net.connect_server("ws://127.0.0.1:"+str(port),read_stage("room").code)
 check(await wait_until(func(): return net.match_active),"Match starts after countdown")
 if not failures.is_empty(): finish();return
 main=load("res://main.tscn").instantiate()
 root.add_child(main)
 current_scene=main
 main.battle.time=-9999
 main.objectives.elapsed=-9999
 await physics_frame
 await physics_frame
 if role=="host": await host_checks()
 else: await guest_checks()
 finish()
func host_checks() -> void:
 check(await wait_until(func(): return main.coop.peers.size()==1),"Remote avatar received")
 var actor=main.coop.peers.values()[0]
 check(actor.rig.find_child("soldier-neck",true,false)!=null,"Remote neck exists")
 check(actor.mount.get_node_or_null("SupportHand")!=null,"Remote support grip")
 for weapon in main.bridge.data.weapons:
  actor.equip(weapon)
  var grip=main.bridge.data.weapons[weapon].grips.trigger
  check(actor.mount.get_node("TriggerHand").position.distance_to(Vector3(grip[0],grip[1],grip[2]))<.001,"Remote grip "+weapon)
 actor.equip("Pistol")
 var point=main.objectives.road_point(919)
 main.player.position=point+Vector3(20,.05,0)
 main.battle.spawn_enemy("zombie")
 var enemy=main.battle.enemies.back()
 enemy.set_physics_process(false)
 enemy.position=point+Vector3(0,.05,0)
 enemy.hp=1
 main.battle.add_trap(point+Vector3(5,0,0))
 main.battle.projectile(enemy,false)
 var projectile=main.battle.projectiles.back()
 projectile.mesh.position=point+Vector3.UP*10
 projectile.velocity=Vector3(.01,0,0)
 projectile.life=20
 write_stage("shot",{"point":main.coop.v(point),"seed":main.generation.seed})
 check(await wait_until(func(): return main.battle.enemies.is_empty()),"Guest shot kills authoritative zombie")
 check(await wait_until(func(): return main.battle.money>=15),"Guest collects cash for host")
 write_stage("cash",{"money":main.battle.money})
 main.objectives.elapsed=0
 main.objectives.start_mission("generator",400)
 main.objectives.mission.position=point
 main.objectives.mission_model.position=point
 var before=main.battle.money
 write_stage("generator")
 check(await wait_until(func(): return main.objectives.mission.status=="complete",30),"Guest holds E 15s on generator")
 check(main.battle.money==before+400,"Mission rewards host exactly once")
 main.player.hurt(100)
 write_stage("revive",{"p":main.coop.v(main.player.position)})
 check(await wait_until(func(): return main.player.health==50),"Guest revives host")
 check(await wait_until(func(): return stage_exists("guest-ok")),"Guest verifies synchronized state")
 write_stage("host-ok")
func guest_checks() -> void:
 check(await wait_until(func(): return stage_exists("shot")),"Host provides target")
 var data=read_stage("shot")
 check(int(data.seed)==int(main.generation.seed),"Same generated city seed")
 var point=main.coop.vec(data.point)
 main.player.position=point+Vector3(0,.05,5)
 check(await wait_until(func(): return main.battle.enemies.size()==1),"Enemy replica appears")
 check(await wait_until(func(): return main.coop.prop_pools.traps.size()==1),"Trap model replicated")
 check(main.coop.prop_pools.traps.values()[0].get_child_count()>0,"Remote trap uses original detailed model")
 check(await wait_until(func(): return main.coop.prop_pools.shots.size()==1),"Projectile stable identity replicated")
 await create_timer(1).timeout
 net.send({"type":"shot","weapon":"Pistol","direction":[0,-.08,-1]})
 check(await wait_until(func(): return main.battle.enemies.is_empty()),"Replica disappears after host kill")
 main.player.position=point+Vector3(0,.05,0)
 check(await wait_until(func(): return stage_exists("cash")),"Shared pickup confirmed")
 var cash=read_stage("cash")
 check(await wait_until(func(): return main.battle.money==int(cash.money)),"Same cash reward on guest")
 if mode=="ws":
  net.socket.close(1001,"test interruption")
  check(await wait_until(func(): return main.coop.paused_for_disconnect),"WebSocket interruption detected")
  check(await wait_until(func(): return not main.coop.paused_for_disconnect and net.was_open),"Automatic WebSocket seat resume")
  check(main.battle.money==int(cash.money),"Reconnect does not duplicate cash")
 check(await wait_until(func(): return stage_exists("generator")),"Generator announced")
 check(await wait_until(func(): return not main.objectives.mission.is_empty() and main.objectives.mission.kind=="generator"),"Mission replica appears")
 Input.action_press("interact")
 check(await wait_until(func(): return main.objectives.mission.status=="complete",30),"Mission completion sync")
 Input.action_release("interact")
 check(main.battle.money==int(cash.money)+400,"Shared mission reward exactly once")
 # Reapplying snapshots cannot duplicate credits.
 main.coop.apply_snapshot(main.coop.last_state)
 check(main.battle.money==int(cash.money)+400,"Snapshot reward deduplication")
 check(main.battle.buy("ammo")=="PURCHASE COMPLETE","Guest buys personal supplies")
 main.coop.apply_snapshot(main.coop.last_state)
 check(main.battle.money==int(cash.money)+350,"Snapshot does not refund purchases")
 check(await wait_until(func(): return stage_exists("revive")),"Host downed")
 main.player.position=main.coop.vec(read_stage("revive").p)+Vector3(.8,0,0)
 await create_timer(.5).timeout
 net.send({"type":"revive","target":"1" if mode=="lan" else "host"})
 write_stage("guest-ok")
 check(await wait_until(func(): return stage_exists("host-ok")),"Host result acknowledged")
 if mode=="ws": check(await wait_until(func(): return not net.active),"Host exit closes guest lobby cleanly")
func finish() -> void:
 print("PASS COOP "+mode+" "+role if failures.is_empty() else "FAIL COOP "+str(failures))
 net.close()
 if is_instance_valid(main): main.queue_free()
 await process_frame
 quit(0 if failures.is_empty() else 1)
