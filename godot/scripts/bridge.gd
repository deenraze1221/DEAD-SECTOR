extends RefCounted
# Three.js and Godot both use a right-handed Y-up world. Godot triangles are clockwise.
var materials: Array = []
var mesh_cache: Dictionary = {}
var data: Dictionary

func matrix(values: Array) -> Transform3D:
 return Transform3D(Basis(Vector3(values[0],values[1],values[2]),Vector3(values[4],values[5],values[6]),Vector3(values[8],values[9],values[10])),Vector3(values[12],values[13],values[14]))

func load_data() -> void:
 data=JSON.parse_string(FileAccess.get_file_as_string("res://assets/city.json"))
 for record in data.materials:
  var material: Material
  if record.surface>0:
   var shader_material=ShaderMaterial.new()
   shader_material.shader=load("res://shaders/surface.gdshader")
   shader_material.set_shader_parameter("tint",Color(record.color))
   shader_material.set_shader_parameter("surface_kind",int(record.surface))
   material=shader_material
  else:
   var standard=StandardMaterial3D.new()
   standard.albedo_color=Color(record.color)
   if record.get("transparent",false):
    standard.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA
    standard.albedo_color.a=record.get("opacity",1.0)
   if record.get("side",0)==2: standard.cull_mode=BaseMaterial3D.CULL_DISABLED
   standard.roughness=record.roughness
   standard.metallic=record.metalness
   standard.vertex_color_use_as_albedo=true
   standard.vertex_color_is_srgb=false
   if record.unlit: standard.shading_mode=BaseMaterial3D.SHADING_MODE_UNSHADED
   if record.texture>=0:
    standard.albedo_texture=load("res://assets/label-%d.png" % int(record.texture))
    standard.transparency=BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
    standard.texture_filter=BaseMaterial3D.TEXTURE_FILTER_NEAREST
   if record.energy>0:
    standard.emission_enabled=true
    standard.emission=Color(record.emissive)
    standard.emission_energy_multiplier=record.energy
   material=standard
  materials.append(material)

func make_mesh(record: Dictionary) -> ArrayMesh:
 var key=JSON.stringify([record.geometry,record.materials])
 if mesh_cache.has(key): return mesh_cache[key]
 var raw: Dictionary=data.geometries[int(record.geometry)]
 var mesh=ArrayMesh.new()
 var vertices=PackedVector3Array()
 var normals=PackedVector3Array()
 var uvs=PackedVector2Array()
 var colors=PackedColorArray()
 for i in range(0,raw.position.size(),3): vertices.append(Vector3(raw.position[i],raw.position[i+1],raw.position[i+2]))
 for i in range(0,raw.normal.size(),3): normals.append(Vector3(raw.normal[i],raw.normal[i+1],raw.normal[i+2]))
 for i in range(0,raw.uv.size(),2): uvs.append(Vector2(raw.uv[i],1.0-raw.uv[i+1]))
 for i in range(0,raw.color.size(),3): colors.append(Color(raw.color[i],raw.color[i+1],raw.color[i+2]))
 var original: Array=raw.index
 if original.is_empty(): original=range(vertices.size())
 var groups: Array=raw.groups if record.materials.size()>1 and not raw.groups.is_empty() else [{"start":0,"count":original.size(),"materialIndex":0}]
 for group in groups:
  var arrays=[]
  arrays.resize(Mesh.ARRAY_MAX)
  arrays[Mesh.ARRAY_VERTEX]=vertices
  if not normals.is_empty(): arrays[Mesh.ARRAY_NORMAL]=normals
  if not uvs.is_empty(): arrays[Mesh.ARRAY_TEX_UV]=uvs
  if not colors.is_empty(): arrays[Mesh.ARRAY_COLOR]=colors
  var indices=PackedInt32Array()
  for i in range(int(group.start),int(group.start+group.count),3):
   indices.append_array(PackedInt32Array([original[i],original[i+2],original[i+1]]))
  arrays[Mesh.ARRAY_INDEX]=indices
  mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES,arrays)
  mesh.surface_set_material(mesh.get_surface_count()-1,materials[int(record.materials[int(group.materialIndex)])])
 mesh_cache[key]=mesh
 return mesh

func build_world(parent: Node3D) -> void:
 for batch in data.batches:
  var instance=MultiMeshInstance3D.new()
  var multi=MultiMesh.new()
  multi.transform_format=MultiMesh.TRANSFORM_3D
  multi.mesh=make_mesh(batch)
  multi.use_colors=true
  multi.instance_count=batch.transforms.size()
  for i in range(multi.instance_count):
   multi.set_instance_transform(i,matrix(batch.transforms[i]))
   var colors: Array=batch.get("colors",[])
   if colors.size()>=i*3+3: multi.set_instance_color(i,Color(colors[i*3],colors[i*3+1],colors[i*3+2]))
  instance.multimesh=multi
  parent.add_child(instance)
 # Compound static body avoids thousands of separate physics bodies.
 var body=StaticBody3D.new()
 parent.add_child(body)
 for box in data.colliders:
  var size=Vector3(box.maxX-box.minX,box.maxY-box.minY,box.maxZ-box.minZ)
  if size.x<=0 or size.y<=0 or size.z<=0: continue
  var shape=CollisionShape3D.new()
  var resource=BoxShape3D.new()
  resource.size=size
  shape.shape=resource
  shape.position=Vector3(box.minX,box.minY,box.minZ)+size*.5
  body.add_child(shape)
 var ground=CollisionShape3D.new()
 var floor_shape=BoxShape3D.new()
 floor_shape.size=Vector3(450,.2,450)
 ground.shape=floor_shape
 ground.position.y=-.1
 body.add_child(ground)

func build_weapon(parent: Node3D, id: String) -> void:
 var part_index=0
 for part in data.weapons[id].parts:
  var instance=MeshInstance3D.new()
  instance.name="WeaponBody" if part_index==0 else "WeaponMagazine"
  part_index+=1
  instance.mesh=make_mesh(part)
  if id=="Pistol":
   instance.mesh=instance.mesh.duplicate()
   for surface in range(instance.mesh.get_surface_count()):
    var source: StandardMaterial3D=instance.mesh.surface_get_material(surface)
    var finish=source.duplicate()
    finish.albedo_color=[Color("#495654"),Color("#77816e"),Color("#bd9254")][mini(surface,2)]
    finish.roughness=.72
    instance.mesh.surface_set_material(surface,finish)
  instance.transform=matrix(part.transform)
  instance.layers=2
  instance.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
  parent.add_child(instance)

 build_hands(parent,data.weapons[id].grips)

func build_hands(parent: Node3D, grips: Dictionary) -> void:
 var sleeve=StandardMaterial3D.new()
 sleeve.albedo_color=Color("#677359")
 sleeve.roughness=.95
 var glove=StandardMaterial3D.new()
 glove.albedo_color=Color("#535d4d")
 glove.roughness=.9
 for side in [-1,1]:
  var raw: Array=grips.support if side<0 else grips.trigger
  var wrist=Vector3(raw[0],raw[1],raw[2])
  var elbow=Vector3(-.29 if side<0 else .23,-.28,.27)
  var arm=MeshInstance3D.new()
  var cylinder=CylinderMesh.new()
  cylinder.top_radius=.035
  cylinder.bottom_radius=.049
  cylinder.height=wrist.distance_to(elbow)
  arm.name="SupportForearm" if side<0 else "TriggerForearm"
  arm.mesh=cylinder
  arm.material_override=sleeve
  parent.add_child(arm)
  arm.position=(wrist+elbow)*.5
  var axis=(wrist-elbow).normalized()
  arm.quaternion=Quaternion(Vector3.UP,axis)
  arm.layers=2
  arm.cast_shadow=GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
  var hand=Node3D.new()
  hand.name="SupportHand" if side<0 else "TriggerHand"
  hand.position=wrist
  parent.add_child(hand)
  var palm=MeshInstance3D.new()
  var box=BoxMesh.new()
  box.size=Vector3(.062,.078,.068)
  palm.mesh=box
  palm.material_override=glove
  palm.layers=2
  hand.add_child(palm)
  for finger in range(4):
   var mesh=MeshInstance3D.new()
   var shape=BoxMesh.new()
   shape.size=Vector3(.027,.014,.04)
   mesh.mesh=shape
   mesh.position=Vector3(-side*.025,-.026+finger*.017,-.025)
   mesh.material_override=glove
   mesh.layers=2
   hand.add_child(mesh)

func creature_node(record: Dictionary) -> Node3D:
 var node: Node3D
 if record.has("geometry"):
  var mesh=MeshInstance3D.new()
  mesh.mesh=make_mesh(record)
  node=mesh
 else: node=Node3D.new()
 node.name=record.name if not record.name.is_empty() else "Joint"
 node.transform=matrix(record.transform)
 for child in record.children: node.add_child(creature_node(child))
 return node
