extends CharacterBody3D
var camera: Camera3D
var weapon_camera: Camera3D
var weapon_viewport: SubViewport
var mouse_guard=0.0
var gun: Node3D
var stamina=5.0
var health=100
var battle
var switch_time=0.0
var pending_weapon=""
var ammo=8
var reserve=84
var reloading=0.0
var cooldown=0.0
var distance_walked=0.0
var exhausted=false
var active=false
var base_gun=Vector3(.22,-.22,-.38)
var recoil=0.0
var config: Dictionary
var shots=0
var last_hit=false
var shot_sound: AudioStreamPlayer
var flash: OmniLight3D

func _ready() -> void:
 collision_layer=8
 collision_mask=1|4
 var collider=CollisionShape3D.new()
 var capsule=CapsuleShape3D.new()
 capsule.radius=.32
 capsule.height=1.8
 collider.shape=capsule
 collider.position.y=.9
 add_child(collider)
 camera=Camera3D.new()
 camera.fov=73
 camera.cull_mask=1
 camera.near=.04
 camera.far=600
 camera.position.y=1.65
 add_child(camera)
 setup_weapon_view()
 gun=Node3D.new()
 gun.position=base_gun
 camera.add_child(gun)
 flash=OmniLight3D.new()
 flash.omni_range=3
 flash.light_color=Color(1,.7,.35)
 flash.light_energy=0
 flash.position=Vector3(.22,-.15,-.65)
 camera.add_child(flash)
 var lamp=SpotLight3D.new()
 lamp.name="Flashlight"
 lamp.spot_range=30
 lamp.spot_angle=24
 lamp.light_energy=3
 lamp.visible=false
 camera.add_child(lamp)
 shot_sound=AudioStreamPlayer.new()
 var wav=AudioStreamWAV.new()
 wav.format=AudioStreamWAV.FORMAT_16_BITS
 wav.mix_rate=22050
 var bytes=PackedByteArray()
 bytes.resize(6615*2)
 var rng=RandomNumberGenerator.new()
 rng.seed=123
 for i in range(6615):
  var t=float(i)/22050
  var signal_value=(rng.randf_range(-1,1)*.6+sin(t*TAU*(160-t*240))*.4)*exp(-t*30)*.65
  bytes.encode_s16(i*2,int(signal_value*32767))
 wav.data=bytes
 shot_sound.stream=wav
 add_child(shot_sound)

func _unhandled_input(event: InputEvent) -> void:
 if not active or health<=0: return
 if event is InputEventMouseMotion and Input.mouse_mode==Input.MOUSE_MODE_CAPTURED and mouse_guard<=0:
  # Relative input is used exactly once, independent of frame rate.
  var motion=event.screen_relative
  if abs(motion.x)>600 or abs(motion.y)>600: return
  look(motion)
 if event.is_action_pressed("fire"): fire()
 if event.is_action_pressed("reload") and switch_time<=0 and reloading<=0 and ammo<int(config.capacity) and reserve>0: reloading=float(config.reload)
 if event.is_action_pressed("flashlight") and (battle==null or battle.flashlight and battle.battery>0):
  camera.get_node("Flashlight").visible=not camera.get_node("Flashlight").visible
 if battle!=null:
  if event.is_action_pressed("primary") and battle.primary!="": battle.switch_to(battle.primary)
  if event.is_action_pressed("pistol"): battle.switch_to("Pistol")
  if event.is_action_pressed("trap") and battle.traps>0: battle.trap_selected=true
  if event.is_action_pressed("small_medkit"): battle.use_medkit(0)
  if event.is_action_pressed("large_medkit"): battle.use_medkit(1)
  if event.is_action_pressed("battery") and battle.batteries>0 and battle.flashlight and battle.battery<120:
   battle.batteries-=1
   battle.battery=120

func _physics_process(delta: float) -> void:
 if not active or health<=0: return
 if switch_time>0:
  var before=switch_time
  switch_time=maxf(0,switch_time-delta)
  if before>.75 and switch_time<=.75 and battle!=null: battle.equip_owned(pending_weapon)
 mouse_guard=maxf(0,mouse_guard-delta)
 cooldown=maxf(0,cooldown-delta)
 if config.automatic and Input.is_action_pressed("fire"): fire()
 flash.light_energy=maxf(0,flash.light_energy-delta*80)
 recoil=move_toward(recoil,0,delta*4)
 if reloading>0:
  reloading=maxf(0,reloading-delta)
  if reloading==0:
   var amount=mini(int(config.capacity)-ammo,reserve)
   ammo+=amount
   reserve-=amount
 var input=Input.get_vector("left","right","forward","back")
 var moving=input.length()>.01
 if exhausted and stamina>=5: exhausted=false
 var sprint=Input.is_action_pressed("sprint") and moving and stamina>0 and not exhausted
 stamina=clampf(stamina+(-delta if sprint else delta),0,5)
 if stamina==0: exhausted=true
 var direction=transform.basis*Vector3(input.x,0,input.y)
 var speed=7.5 if sprint else 4.5
 var acceleration=(36.0 if moving else 55.0)/1.15
 var horizontal=Vector2(velocity.x,velocity.z).move_toward(Vector2(direction.x,direction.z)*speed,acceleration*delta)
 velocity.x=horizontal.x
 velocity.z=horizontal.y
 if not is_on_floor(): velocity.y-=20*delta
 elif Input.is_action_just_pressed("jump"): velocity.y=7/sqrt(2.0)
 else: velocity.y=-.1
 move_and_slide()
 var actual=Vector2(get_real_velocity().x,get_real_velocity().z).length()
 distance_walked+=actual*delta
 var bob=sin(distance_walked*TAU/1.65)*minf(actual/4.5,1)*.006
 gun.position=base_gun+Vector3(0,bob-(sin((float(config.reload)-reloading)/float(config.reload)*PI)*.16 if reloading>0 else 0),recoil*.07)
 gun.position.y-=sin((1.5-switch_time)/1.5*PI)*.35 if switch_time>0 else 0
 gun.rotation.x=recoil*.15

func fire() -> void:
 if not active or health<=0 or switch_time>0: return
 if battle!=null and battle.trap_selected: battle.place_trap();return
 if cooldown>0 or reloading>0 or ammo<=0: return
 if battle!=null:
  var state=battle.inventory.get(config.id,{})
  if state.get("durability",100)<=0: return
  state.shots=state.get("shots",0)+1
  if state.shots>=100: state.durability=maxf(0,state.durability-5);state.shots=0
 ammo-=1
 cooldown=float(config.interval)
 recoil=1
 shots+=1
 shot_sound.play()
 flash.light_energy=5
 var from=camera.global_position
 var query=PhysicsRayQueryParameters3D.create(from,from-camera.global_basis.z*100)
 query.exclude=[get_rid()]
 var hit=get_world_3d().direct_space_state.intersect_ray(query)
 last_hit=not hit.is_empty()
 if last_hit and hit.collider.has_method("take_damage"): hit.collider.take_damage(int(config.damage))
 elif last_hit and hit.collider.has_meta("target"):
  var target=hit.collider
  target.set_meta("hp",target.get_meta("hp")-int(config.damage))
  if target.get_meta("hp")<=0: target.queue_free()

func look(relative: Vector2) -> void:
 var sensitivity=get_parent().sensitivity
 rotate_y(-relative.x*.002*sensitivity)
 camera.rotation.x=clamp(camera.rotation.x-relative.y*.002*sensitivity,-PI/3,PI/2-.02)

func setup_weapon_view() -> void:
 var layer=CanvasLayer.new()
 layer.layer=0
 add_child(layer)
 var container=SubViewportContainer.new()
 container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
 container.stretch=true
 container.mouse_filter=Control.MOUSE_FILTER_IGNORE
 layer.add_child(container)
 weapon_viewport=SubViewport.new()
 weapon_viewport.size=Vector2i(1280,720)
 weapon_viewport.transparent_bg=true
 weapon_viewport.world_3d=get_world_3d()
 weapon_viewport.render_target_update_mode=SubViewport.UPDATE_ALWAYS
 container.add_child(weapon_viewport)
 weapon_camera=Camera3D.new()
 weapon_camera.cull_mask=2
 weapon_camera.fov=55
 weapon_camera.near=.015
 var environment=Environment.new()
 environment.background_mode=Environment.BG_CLEAR_COLOR
 environment.ambient_light_source=Environment.AMBIENT_SOURCE_COLOR
 environment.ambient_light_color=Color("#9da8ad")
 environment.ambient_light_energy=.65
 weapon_camera.environment=environment
 weapon_viewport.add_child(weapon_camera)
 var key_light=OmniLight3D.new()
 key_light.light_cull_mask=2
 key_light.omni_range=4
 key_light.light_energy=1.2
 key_light.position=Vector3(-.2,.45,.1)
 weapon_camera.add_child(key_light)
 var fill=DirectionalLight3D.new()
 fill.light_cull_mask=2
 fill.light_energy=.5
 fill.rotation_degrees=Vector3(-35,-25,0)
 weapon_viewport.add_child(fill)

func _process(_delta: float) -> void:
 if is_instance_valid(weapon_camera):
  weapon_camera.global_transform=camera.global_transform

func hurt(amount: int) -> void:
 if health<=0: return
 health=maxi(0,health-amount)
 if health==0: get_parent().show_death()
