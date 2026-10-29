extends CharacterBody3D
var camera: Camera3D
var weapon_camera: Camera3D
var weapon_viewport: SubViewport
var weapon_env: Environment
var weapon_key: OmniLight3D
var weapon_fill: DirectionalLight3D
var weapon_sky: ProceduralSkyMaterial
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
var hit_time=0.0
var hurt_time=0.0
var actions
var flash: OmniLight3D
var muzzle: MeshInstance3D
var muzzle_time=0.0
var muzzle_offsets: Dictionary={}
var flicker=0.0

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
 actions=preload("res://scripts/weapon_actions.gd").new()
 add_child(actions)
 actions.setup(self)
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
 lamp.spot_range=34
 lamp.spot_angle=31
 lamp.spot_attenuation=.75
 lamp.light_energy=4.2
 lamp.light_color=Color("#e9f0ff")
 lamp.position=Vector3(.17,-.09,-.1)
 lamp.shadow_enabled=true
 lamp.shadow_bias=.06
 lamp.visible=false
 camera.add_child(lamp)
 muzzle=MeshInstance3D.new()
 var star=QuadMesh.new()
 star.size=Vector2(.34,.34)
 var star_material=StandardMaterial3D.new()
 star_material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 star_material.blend_mode=BaseMaterial3D.BLEND_MODE_ADD
 star_material.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA
 star_material.albedo_texture=muzzle_texture()
 star_material.albedo_color=Color(1,.78,.42,1)
 star_material.depth_draw_mode=BaseMaterial3D.DEPTH_DRAW_DISABLED
 star_material.no_depth_test=true
 star_material.render_priority=5
 star.material=star_material
 muzzle.mesh=star
 muzzle.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
 muzzle.layers=2
 muzzle.visible=false
 camera.add_child(muzzle)

# Barrel tip in gun space: front of the weapon mesh bounds, measured once per weapon.
func muzzle_offset() -> Vector3:
 var id=str(config.id)
 if muzzle_offsets.has(id): return muzzle_offsets[id]
 var bounds=AABB()
 var first=true
 for node in gun.find_children("WeaponBody","MeshInstance3D",true,false):
  var local_box=(gun.global_transform.affine_inverse()*node.global_transform)*node.get_aabb()
  bounds=local_box if first else bounds.merge(local_box)
  first=false
 var offset=Vector3(0,.03,-.45)
 if not first: offset=Vector3(0,float(config.get("muzzleY",.02)),float(config.tip)-.01)
 muzzle_offsets[id]=offset
 return offset

func muzzle_texture() -> ImageTexture:
 var image=Image.create(48,48,false,Image.FORMAT_RGBA8)
 for y in range(48):
  for x in range(48):
   var d=Vector2(x-23.5,y-23.5)
   var angle=atan2(d.y,d.x)
   var spikes=.55+.45*absf(cos(angle*2.5))
   var radius=d.length()/23.5/spikes
   image.set_pixel(x,y,Color(1,1,1,pow(maxf(0,1-radius),1.6)))
 return ImageTexture.create_from_image(image)

func _unhandled_input(event: InputEvent) -> void:
 if not active or health<=0: return
 if event is InputEventMouseMotion and Input.mouse_mode==Input.MOUSE_MODE_CAPTURED and mouse_guard<=0:
  # Relative input is used exactly once, independent of frame rate.
  var motion=event.screen_relative
  if abs(motion.x)>600 or abs(motion.y)>600: return
  look(motion)
 if event.is_action_pressed("fire"): fire()
 if event.is_action_pressed("reload") and switch_time<=0 and reloading<=0 and ammo<int(config.capacity) and reserve>0:
  reloading=float(config.reload)
  if get_parent().sound!=null: get_parent().sound.play("reload")
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
 if not active or health<=0 or not get_parent().world_running(): return
 if switch_time>0:
  var before=switch_time
  switch_time=maxf(0,switch_time-delta)
  if before>.75 and switch_time<=.75 and battle!=null: battle.equip_owned(pending_weapon)
 mouse_guard=maxf(0,mouse_guard-delta)
 hit_time=maxf(0,hit_time-delta)
 hurt_time=maxf(0,hurt_time-delta)
 cooldown=maxf(0,cooldown-delta)
 if config.automatic and Input.is_action_pressed("fire"): fire()
 flash.light_energy=maxf(0,flash.light_energy-delta*80)
 recoil=move_toward(recoil,0,delta*4)
 # View kick and damage shake shift only the projection, so aim and hit scan stay exact.
 camera.v_offset=recoil*.016
 camera.h_offset=randf_range(-1,1)*(recoil*.004+hurt_time*.05)
 muzzle_time=maxf(0,muzzle_time-delta)
 muzzle.visible=muzzle_time>0
 if muzzle.visible:
  muzzle.position=gun.transform*muzzle_offset()
  muzzle.rotation.z=randf()*TAU
  muzzle.scale=Vector3.ONE*randf_range(.7,1.1)*(.6 if str(config.id) in ["Pistol","Diaglo"] else .9)
 var lamp_node=camera.get_node("Flashlight")
 if lamp_node.visible:
  flicker=maxf(0,flicker-delta)
  var weak=battle!=null and battle.battery<12
  if weak and flicker<=0 and randf()<.08: flicker=randf_range(.04,.18)
  lamp_node.light_energy=(.35 if flicker>0 else lerpf(1.2,4.2,clampf(battle.battery/12.0,0,1)) if weak else 4.2)
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
 var previous_step=floori(distance_walked/1.7)
 distance_walked+=actual*delta
 if is_on_floor() and actual>.5 and floori(distance_walked/1.7)!=previous_step and get_parent().sound!=null: get_parent().sound.play("step",null,(.6*(1.2 if get_parent().climate!=null and get_parent().climate.wetness>.5 else 1.0)),randf_range(.88,1.08)*(.93 if get_parent().climate!=null and get_parent().climate.wetness>.5 else 1.0))
 var bob=sin(distance_walked*TAU/1.65)*minf(actual/4.5,1)*.006
 gun.position=base_gun+Vector3(0,bob-(sin((float(config.reload)-reloading)/float(config.reload)*PI)*.045 if reloading>0 else 0),recoil*.07)
 gun.position.y-=sin((1.5-switch_time)/1.5*PI)*.35 if switch_time>0 else 0
 gun.rotation.x=recoil*.15

func fire() -> void:
 if not active or health<=0 or switch_time>0 or not get_parent().world_running(): return
 if battle!=null and battle.trap_selected: battle.place_trap();return
 if cooldown>0 or reloading>0 or ammo<=0: return
 if battle!=null:
  var state=battle.inventory.get(config.id,{})
  if state.get("durability",100)<=0: return
  state.shots=state.get("shots",0)+1
  if state.shots>=100: state.durability=maxf(0,state.durability-5);state.shots=0
 cooldown=float(config.interval)
 var id=config.id
 for i in range(int(config.shots)):
  if i>0:
   await get_tree().create_timer(float(config.gap)).timeout
   if not is_instance_valid(self) or config.id!=id or ammo<=0 or reloading>0 or health<=0 or switch_time>0: return
  shoot_round()

func shoot_round() -> void:
 ammo-=1
 recoil=1
 shots+=1
 if get_parent().sound!=null: get_parent().sound.play("shot-"+str(config.sound))

 flash.light_energy=5
 muzzle_time=.045
 actions.fired()
 if not get_parent().net.active or get_parent().net.hosting:
  for enemy in battle.enemies: enemy.hear(global_position)
 var from=camera.global_position
 var query=PhysicsRayQueryParameters3D.create(from,from-camera.global_basis.z*100)
 query.collision_mask=1|4
 query.exclude=[get_rid()]
 var hit=get_world_3d().direct_space_state.intersect_ray(query)
 last_hit=not hit.is_empty()
 var main=get_parent()
 main.combat_fx.tracer(from-camera.global_basis.z*.5,hit.position if last_hit else from-camera.global_basis.z*100)
 if main.net.active and not main.net.hosting:
  main.net.send({"type":"shot","direction":main.coop.v(-camera.global_basis.z),"weapon":config.id})
  if last_hit and hit.collider.has_method("take_damage"): hit_time=.16
  return
 elif main.net.active:
  main.net.send({"type":"effect","sound":"shot-"+str(config.sound),"p":main.coop.v(global_position),"shooter":main.net.peer_id,"end":main.coop.v(hit.position if last_hit else from-camera.global_basis.z*100)})
 if last_hit and hit.collider.has_method("take_damage"):
  if hit.collider.has_meta("mutant"): hit.collider.take_damage(int(config.damage),hit.position,-camera.global_basis.z)
  else: hit.collider.take_damage(int(config.damage))
  hit_time=.16
  if get_parent().sound!=null: get_parent().sound.play("hit")
 elif last_hit and hit.collider.has_meta("target"):
  var target=hit.collider
  target.set_meta("hp",target.get_meta("hp")-int(config.damage))
  if target.get_meta("hp")<=0: target.queue_free()

 if last_hit and not hit.collider.has_method("take_damage"): battle.burst(hit.position,false,5,hit.normal,true,true)

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
 environment.ambient_light_energy=.9
 # Painted metal is dark (vertex colours around 0.2) and metallic, so without a sky to reflect it renders black.
 weapon_sky=ProceduralSkyMaterial.new()
 weapon_sky.sky_top_color=Color("#7d8f9a")
 weapon_sky.sky_horizon_color=Color("#c2ccd0")
 weapon_sky.ground_horizon_color=Color("#8b9591")
 weapon_sky.ground_bottom_color=Color("#3b4440")
 var weapon_sky_resource=Sky.new()
 weapon_sky_resource.sky_material=weapon_sky
 environment.sky=weapon_sky_resource
 environment.reflected_light_source=Environment.REFLECTION_SOURCE_SKY
 weapon_env=environment
 weapon_camera.environment=environment
 weapon_viewport.add_child(weapon_camera)
 # Soft top-left key light gives the gun volume; it has no shadow so it never flickers.
 var key_light=OmniLight3D.new()
 key_light.light_cull_mask=2
 key_light.omni_range=4
 key_light.light_energy=.7
 key_light.position=Vector3(-.2,.45,.1)
 weapon_camera.add_child(key_light)
 weapon_key=key_light
 # The directional light follows the real sun and casts shadows, so the gun and hands shade each other.
 var fill=DirectionalLight3D.new()
 fill.light_cull_mask=2
 fill.light_energy=.9
 fill.rotation_degrees=Vector3(-35,-25,0)
 fill.shadow_enabled=true
 fill.shadow_bias=.015
 fill.shadow_normal_bias=.8
 fill.shadow_blur=1.6
 fill.directional_shadow_mode=DirectionalLight3D.SHADOW_ORTHOGONAL
 fill.directional_shadow_max_distance=3.0
 weapon_viewport.add_child(fill)
 weapon_fill=fill

# The viewmodel is lit by its own lights, so they must follow the world: dark at night, grey in storms.
func update_weapon_light() -> void:
 var main=get_parent()
 if weapon_env==null or main==null or not main.has_node("WorldEnvironment") or not main.has_node("Sun"): return
 var world_env=main.get_node("WorldEnvironment").environment
 var sun=main.get_node("Sun")
 var ratio=clampf(world_env.ambient_light_energy/.46,0.0,1.0)
 weapon_env.ambient_light_energy=lerpf(.28,1.7,ratio)
 weapon_env.ambient_light_color=world_env.ambient_light_color.lerp(Color("#9da8ad"),.35)
 weapon_env.reflected_light_source=Environment.REFLECTION_SOURCE_SKY
 var shade=lerpf(.2,1.3,ratio)
 weapon_sky.sky_top_color=Color("#7d8f9a")*shade
 weapon_sky.sky_horizon_color=Color("#c2ccd0")*shade
 weapon_sky.ground_horizon_color=Color("#8b9591")*shade
 weapon_sky.ground_bottom_color=Color("#3b4440")*shade
 weapon_key.light_energy=lerpf(.35,1.9,ratio)
 weapon_fill.light_color=sun.light_color.lerp(Color.WHITE,.25)
 weapon_fill.light_energy=lerpf(.4,2.8,clampf(sun.light_energy/.75,0.0,1.0))
 weapon_fill.basis=sun.global_transform.basis

func _process(_delta: float) -> void:
 if is_instance_valid(weapon_camera):
  weapon_camera.global_transform=camera.global_transform
  update_weapon_light()

func hurt(amount: int) -> void:
 if health<=0: return
 hurt_time=.35
 health=maxi(0,health-amount)
 if get_parent().sound!=null: get_parent().sound.play("hurt")
 if health==0: get_parent().show_death()
