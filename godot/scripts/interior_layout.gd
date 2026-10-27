extends RefCounted
var library: Dictionary={}
var rng=RandomNumberGenerator.new()
var data: Dictionary
var bridge
var occupied: Array[AABB]=[]
var reserved: Array[AABB]=[]
var rooms: Array=[]
var generated=0
var unit_box=0
var plaster=0

func capture(source: Dictionary, converter) -> void:
 bridge=converter
 var assets={}
 for item in source.source_assets:
  if str(item.id).begins_with("interior-"): assets[str(item.id)]=item
 var records={}
 for batch in source.batches:
  for i in range(batch.get("owners",[]).size()):
   var id=str(batch.owners[i])
   if not assets.has(id): continue
   if not records.has(id): records[id]=[]
   var item=assets[id]
   var anchor=Transform3D(Basis(Vector3.UP,item.yaw),Vector3(item.x,item.y,item.z))
   var color=Color.WHITE
   if batch.colors.size()>i*3+2: color=Color(batch.colors[i*3],batch.colors[i*3+1],batch.colors[i*3+2])
   records[id].append({"batch":batch,"local":anchor.affine_inverse()*bridge.matrix(batch.transforms[i]),"color":color})
 for id in records:
  var item=assets[id]
  if not library.has(item.type): library[item.type]=[]
  if library[item.type].size()>=5: continue
  var boxes=[]
  var inverse=Transform3D(Basis(Vector3.UP,item.yaw),Vector3(item.x,item.y,item.z)).affine_inverse()
  for box in source.colliders:
   if str(box.get("owner",""))==id: boxes.append(inverse*AABB(Vector3(box.minX,box.minY,box.minZ),Vector3(box.maxX-box.minX,box.maxY-box.minY,box.maxZ-box.minZ)))
  library[item.type].append({"item":item,"parts":records[id],"boxes":boxes})

func rebuild(source: Dictionary, seed_value: int) -> Dictionary:
 data=source
 rooms=[]
 generated=0
 rng.seed=seed_value^0x524f4f4d
 for i in range(data.geometries.size()):
  var raw=data.geometries[i]
  if raw.position.size()==72 and raw.index.size()==36: unit_box=i;break
 for i in range(data.materials.size()):
  if data.materials[i].color=="#a6a58c": plaster=i;break
 for batch in data.batches:
  var transforms=[]
  var colors=[]
  var owners=[]
  var roles=[]
  for i in range(batch.transforms.size()):
   var owner=str(batch.owners[i]) if batch.owners.size()>i else ""
   var role=batch.roles[i] if batch.get("roles",[]).size()>i else ""
   if (owner.begins_with("interior-") and not owner.begins_with("interior-55-")) or (role=="partition" and owner!="55"): continue
   transforms.append(batch.transforms[i])
   owners.append(owner)
   roles.append(role)
   if batch.colors.size()>i*3+2: colors.append_array(batch.colors.slice(i*3,i*3+3))
   else: colors.append_array([1,1,1])
  batch.transforms=transforms
  batch.colors=colors
  batch.owners=owners
  batch.roles=roles
 data.colliders=data.colliders.filter(func(box): return not ((box.kind=="furniture" and not str(box.owner).begins_with("interior-55-")) or (box.kind=="partition" and int(box.owner)!=55)))
 for building in data.buildings:
  if building.type=="grocery": continue
  occupied=[]
  reserved=[]
  var left=building.x-building.w/2+.3
  var right=building.x+building.w/2-.3
  var back=building.z-building.d/2+.3
  var front=building.z+building.d/2-.3
  var plan=rng.randi_range(0,2)
  building["floorplan"]=plan
  var workrooms=[]
  if plan==0:
   var opening=building.z+rng.randf_range(-.8,.8)
   for side in [-1,1]:
    var x=building.x+side*.95
    wall(x,(back+opening-.8)/2,.12,opening-.8-back,building.id)
    wall(x,(opening+.8+front)/2,.12,front-opening-.8,building.id)
    var room=Rect2(Vector2(left if side<0 else x+.1,back),Vector2((x-.1 if side<0 else right)-(left if side<0 else x+.1),front-back))
    workrooms.append(room)
    reserved.append(AABB(Vector3(room.position.x,0,opening-.6),Vector3(room.size.x,3,1.2)))
   reserved.append(AABB(Vector3(building.x-.8,0,back+1.75),Vector3(1.6,3,front-back-1.75+.8)))
   workrooms.append(Rect2(Vector2(building.x-.85,back),Vector2(1.7,1.6)))
   wall(building.x-.7,back+1.7,.5,.12,building.id)
   wall(building.x+.7,back+1.7,.5,.12,building.id)
  else:
   var split=back+(front-back)*(.47 if plan==1 else .32)
   var gap_x=building.x+rng.randf_range(-.6,.6)
   wall((left+gap_x-.85)/2,split,gap_x-.85-left,.12,building.id)
   wall((right+gap_x+.85)/2,split,right-gap_x-.85,.12,building.id)
   workrooms=[Rect2(Vector2(left,back),Vector2(right-left,split-back-.1)),Rect2(Vector2(left,split+.1),Vector2(right-left,front-split-.1))]
   reserved.append(AABB(Vector3(gap_x-.75,0,back),Vector3(1.5,3,front-back+.6)))
   if plan==1:
    var original: Rect2=workrooms[0]
    workrooms[0]=Rect2(Vector2(left+2.1,back),Vector2(right-left-2.1,original.size.y))
    workrooms.append(Rect2(Vector2(left,back),Vector2(1.95,minf(1.8,original.size.y))))
    wall(left+2.05,back+.9,.12,1.8,building.id)
    wall(left+.25,back+1.85,.5,.12,building.id)
    wall(left+1.8,back+1.85,.5,.12,building.id)
  for index in range(workrooms.size()):
   var r=workrooms[index]
   rooms.append({"building":building.id,"minX":r.position.x,"maxX":r.end.x,"minZ":r.position.y,"maxZ":r.end.y,"name":"room-"+str(index)})
   var residential=building.type in ["apartment","hotel","motel","residentialHighrise"]
   var service=building.type in ["garage","warehouse","workshop","utility","firestation","freightDepot","observatory","communicationsTower"]
   var program=["bed","cabinet","tvstand","stool"] if residential and index==0 else ["sofa","counter","fridge","table","sink"] if residential else ["workbench","shelf","locker","crate"] if service else ["desk","shelf","cabinet","table","sofa"]
   if index==2: program=["toilet","sink"]
   for type in program:
    for attempt in range(12):
     if place(type,r,building.id): break
 data["rooms"]=rooms
 return {"rooms":rooms.size(),"furniture":generated,"native":true}

func serialize(t: Transform3D) -> Array:
 return [t.basis.x.x,t.basis.x.y,t.basis.x.z,0,t.basis.y.x,t.basis.y.y,t.basis.y.z,0,t.basis.z.x,t.basis.z.y,t.basis.z.z,0,t.origin.x,t.origin.y,t.origin.z,1]

func append_part(record: Dictionary,t: Transform3D,color: Color,id: String) -> void:
 var batch={"geometry":record.geometry,"materials":record.materials,"transforms":[serialize(t)],"colors":[color.r,color.g,color.b],"owners":[id],"roles":["fresh"]}
 # Accumulate per material/geometry rather than creating one draw call per object.
 var key=str(record.geometry)+":"+str(record.materials)
 if not data.has("fresh_batches"): data.fresh_batches={}
 if data.fresh_batches.has(key):
  var target=data.fresh_batches[key]
  target.transforms.append(batch.transforms[0])
  target.colors.append_array(batch.colors)
  target.owners.append(id)
  target.roles.append("fresh")
 else: data.fresh_batches[key]=batch;data.batches.append(batch)

func collider(box: AABB,kind: String,id) -> void:
 data.colliders.append({"minX":box.position.x,"maxX":box.end.x,"minY":box.position.y,"maxY":box.end.y,"minZ":box.position.z,"maxZ":box.end.z,"kind":kind,"owner":id})

func wall(x: float,z: float,w: float,d: float,id) -> void:
 if w<=.01 or d<=.01: return
 var t=Transform3D(Basis().scaled(Vector3(w,3.12,d)),Vector3(x,1.56,z))
 append_part({"geometry":unit_box,"materials":[plaster]},t,Color.WHITE,"partition-"+str(id))
 collider(AABB(Vector3(x-w/2,0,z-d/2),Vector3(w,3.12,d)),"partition",id)

func place(type: String,room: Rect2,building) -> bool:
 if not library.has(type): return false
 var prefab=library[type][rng.randi_range(0,library[type].size()-1)]
 var item=prefab.item
 var side=rng.randi_range(0,3)
 var yaw=side*PI/2
 var size=Vector2(item.w,item.d) if side%2==0 else Vector2(item.d,item.w)
 if size.x>room.size.x-.3 or size.y>room.size.y-.3: return false
 var x=rng.randf_range(room.position.x+size.x/2+.12,room.end.x-size.x/2-.12)
 var z=rng.randf_range(room.position.y+size.y/2+.12,room.end.y-size.y/2-.12)
 if side==0: z=room.position.y+size.y/2+.12
 elif side==1: x=room.position.x+size.x/2+.12
 elif side==2: z=room.end.y-size.y/2-.12
 else: x=room.end.x-size.x/2-.12
 var footprint=AABB(Vector3(x-size.x/2-.08,0,z-size.y/2-.08),Vector3(size.x+.16,3,size.y+.16))
 for box in occupied+reserved:
  if box.intersects(footprint): return false
 occupied.append(footprint)
 var anchor=Transform3D(Basis(Vector3.UP,yaw),Vector3(x,0,z))
 var id="fresh-"+str(building)+"-"+str(generated)
 for part in prefab.parts: append_part(part.batch,anchor*part.local,part.color,id)
 for box in prefab.boxes: collider(anchor*box,"furniture",id)
 generated+=1
 if type in ["desk","table","workbench","counter"] and library.has("chair"):
  var chair_at=anchor*Vector3(0,0,item.d/2+.52)
  place_chair(room,building,chair_at,yaw+PI)
 return true

func place_chair(room: Rect2,building,point: Vector3,yaw: float) -> void:
 var prefab=library.chair[rng.randi_range(0,library.chair.size()-1)]
 var item=prefab.item
 var size=Vector2(item.d,item.w) if absf(sin(yaw))>.5 else Vector2(item.w,item.d)
 var box=AABB(Vector3(point.x-size.x/2-.06,0,point.z-size.y/2-.06),Vector3(size.x+.12,3,size.y+.12))
 if box.position.x<room.position.x+.08 or box.end.x>room.end.x-.08 or box.position.z<room.position.y+.08 or box.end.z>room.end.y-.08: return
 for other in occupied+reserved:
  if box.intersects(other): return
 occupied.append(box)
 var anchor=Transform3D(Basis(Vector3.UP,yaw),point)
 var id="fresh-"+str(building)+"-"+str(generated)
 for part in prefab.parts: append_part(part.batch,anchor*part.local,part.color,id)
 for shape in prefab.boxes: collider(anchor*shape,"furniture",id)
 generated+=1
