extends Node3D
var player
var gun_id=""
var flash_mesh: MeshInstance3D
var casing_count=0
var base_hands: Dictionary={}
func setup(owner_player) -> void: player=owner_player

func bind() -> void:
 gun_id=player.config.id
 base_hands={}
 for name in ["SupportHand","TriggerHand"]:
  var hand=player.gun.get_node_or_null(name)
  if hand: base_hands[name]=hand.position
 var mesh=MeshInstance3D.new()
 mesh.name="MuzzleFlash"
 var cone=CylinderMesh.new()
 cone.top_radius=0
 cone.bottom_radius=.055
 cone.height=.16
 cone.radial_segments=5
 mesh.mesh=cone
 var material=StandardMaterial3D.new()
 material.albedo_color=Color("#ffe790")
 material.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
 mesh.material_override=material
 mesh.layers=2
 mesh.rotation.x=PI/2
 mesh.position=Vector3(0,.0,float(player.config.tip)-.06)
 player.gun.add_child(mesh)
 flash_mesh=mesh
 flash_mesh.hide()

func _process(_delta: float) -> void:
 if player==null: return
 if player.config.id!=gun_id or not is_instance_valid(flash_mesh): bind()
 flash_mesh.visible=player.flash.light_energy>1 and player.active
 flash_mesh.rotation.z=randf()*TAU
 var progress=1-player.reloading/float(player.config.reload) if player.reloading>0 else 0.0
 var body=player.gun.get_node_or_null("WeaponBody")
 var pose=reload_pose(str(player.config.reloadStyle),progress,player.reloading>0)
 var xf: Transform3D=pose.body
 if body:
  if not body.has_meta("rest"): body.set_meta("rest",body.transform)
  body.transform=xf*(body.get_meta("rest") as Transform3D)
 var grips=player.config.grips
 var mag=Vector3(player.config.mag[0],player.config.mag[1],player.config.mag[2])
 for name in base_hands:
  var hand=player.gun.get_node_or_null(name)
  if not hand: continue
  var support=name=="SupportHand"
  var rest: Vector3=base_hands[name]
  if support:
   var target=mag+pose.hand_extra
   hand.position=(xf*rest).lerp(xf*target,pose.mag)+pose.support_extra
  else:
   hand.position=xf*rest+pose.trigger_extra
  var arm=player.gun.get_node_or_null("SupportForearm" if support else "TriggerForearm")
  if arm:
   var elbow=Vector3(-.29 if support else .23,-.28,.27)
   var delta=hand.position-elbow
   arm.position=(hand.position+elbow)*.5
   arm.quaternion=Quaternion(Vector3.UP,delta.normalized())
   arm.scale.y=delta.length()/arm.mesh.height

func ramp(a: float, b: float, x: float) -> float: return smoothstep(a,b,x)

# Magazines are part of the model, so reloads hide the magazine well by turning the weapon away
# and moving the support hand to the right spot instead of showing a detached magazine.
func reload_pose(style: String, p: float, active: bool) -> Dictionary:
 var r={"body":Transform3D.IDENTITY,"mag":0.0,"hand_extra":Vector3.ZERO,"support_extra":Vector3.ZERO,"trigger_extra":Vector3.ZERO}
 if not active: return r
 var tilt=ramp(0,.14,p)-ramp(.88,1,p)
 var roll=0.0
 var pitch=0.0
 var lift=Vector3.ZERO
 match style:
  "slide","smg","rifle","lmg":
   roll=-.55*tilt
   pitch=.16*tilt+.07*sin(clampf((p-.58)/.12,0,1)*PI)
   lift=Vector3(.03,.025,.03)*tilt
   r.mag=ramp(.14,.3,p)-ramp(.64,.74,p)
   r.hand_extra=Vector3(0,-.11*(ramp(.3,.46,p)-ramp(.5,.58,p)),0)
   var rack=sin(clampf((p-.8)/.15,0,1)*PI)
   r.support_extra=Vector3(0,0,.1*rack)*(1.0-r.mag)
   if style=="slide": r.support_extra*=.6
  "revolver":
   roll=-.85*tilt
   pitch=.35*tilt
   lift=Vector3(.02,.03,.05)*tilt
   r.mag=ramp(.12,.28,p)-ramp(.78,.9,p)
   r.hand_extra=Vector3(0,.02*sin(p*TAU*3)*ramp(.3,.4,p),0)
  "break":
   roll=-.35*tilt
   pitch=.7*(ramp(.08,.3,p)-ramp(.74,.8,p))-.12*sin(clampf((p-.8)/.1,0,1)*PI)
   lift=Vector3(.02,.02,.05)*tilt
   r.mag=ramp(.2,.36,p)-ramp(.7,.78,p)
   r.hand_extra=Vector3(0,.04*maxf(0,sin(p*TAU*2.0)),0)
  "bolt":
   roll=-.6*tilt
   pitch=.12*tilt
   lift=Vector3(.03,.03,.03)*tilt
   var loads=clampf(float(player.config.capacity),1,5)
   r.mag=ramp(.3,.4,p)-ramp(.82,.9,p)
   r.hand_extra=Vector3(0,.035*maxf(0,sin(clampf((p-.4)/.4,0,1)*PI*loads)),0)
   var bolt=ramp(.12,.24,p)-ramp(.26,.36,p)+ramp(.84,.9,p)-ramp(.9,.96,p)
   r.trigger_extra=Vector3(0,.03*bolt,.09*bolt)
 r.body=Transform3D(Basis.from_euler(Vector3(pitch,0,roll)),lift)
 return r

func fired() -> void:
 if casing_count>=24: return
 casing_count+=1
 var casing=RigidBody3D.new()
 casing.collision_layer=16
 casing.collision_mask=1
 casing.mass=.01
 var mesh=MeshInstance3D.new()
 var cylinder=CylinderMesh.new()
 cylinder.top_radius=.006
 cylinder.bottom_radius=.006
 cylinder.height=.025
 cylinder.radial_segments=6
 mesh.mesh=cylinder
 var material=StandardMaterial3D.new()
 material.albedo_color=Color("#b99b55")
 material.metallic=.5
 mesh.material_override=material
 casing.add_child(mesh)
 var shape=CollisionShape3D.new()
 var box=BoxShape3D.new()
 box.size=Vector3(.012,.025,.012)
 shape.shape=box
 casing.add_child(shape)
 player.get_parent().add_child(casing)
 casing.position=player.camera.global_position-player.camera.global_basis.z*.35+player.camera.global_basis.x*.15
 casing.linear_velocity=player.camera.global_basis.x*1.2+Vector3.UP*.8+player.velocity*.2
 casing.angular_velocity=Vector3(8,6,3)
 get_tree().create_timer(.9).timeout.connect(func(): if is_instance_valid(casing): casing.queue_free();casing_count-=1)
