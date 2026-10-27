extends RefCounted
var centers: Array[Vector2]=[]
var pinned: Dictionary={}
var owner_zone: Dictionary={}
var geometry_bounds: Array[AABB]=[]
var seed_value=0

func zone_for(box: AABB) -> int:
 var mid=box.get_center()
 var x=roundi((mid.x+108)/24)
 var z=roundi((mid.z+108)/24)
 if x<0 or x>9 or z<0 or z>9: return -1
 var c=centers[z*10+x]
 if box.position.x<c.x-11.8 or box.end.x>c.x+11.8 or box.position.z<c.y-11.8 or box.end.z>c.y+11.8: return -1
 return z*10+x

func pin_intrusions(box: AABB) -> void:
 if box.end.y<.2 or box.size.x>25 or box.size.z>25: return
 for i in range(100):
  var c=centers[i]
  if box.position.x<c.x+9.3 and box.end.x>c.x-9.3 and box.position.z<c.y+9.3 and box.end.z>c.y-9.3: pinned[i]=true

func apply(data: Dictionary, value: int, bridge) -> Dictionary:
 seed_value=value
 for z in range(10):
  for x in range(10): centers.append(Vector2(-108+x*24,-108+z*24))
 for fixed in [0,9,90,99,44,45,54,55]: pinned[fixed]=true
 for geometry in data.geometries:
  var raw: Array=geometry.position
  var box=AABB(Vector3(raw[0],raw[1],raw[2]),Vector3.ZERO)
  for i in range(3,raw.size(),3): box=box.expand(Vector3(raw[i],raw[i+1],raw[i+2]))
  geometry_bounds.append(box)
 for key in data.source_bounds:
  var bounds: Dictionary=data.source_bounds[key]
  var a=Vector3(bounds.min[0],bounds.min[1],bounds.min[2])
  var b=Vector3(bounds.max[0],bounds.max[1],bounds.max[2])
  var box=AABB(a,b-a)
  var zone=zone_for(box)
  owner_zone[str(key)]=zone
  if zone<0: pin_intrusions(box)
 for batch in data.batches:
  var zones=[]
  for i in range(batch.transforms.size()):
   var box=bridge.matrix(batch.transforms[i])*geometry_bounds[int(batch.geometry)]
   var key=str(batch.owners[i]) if batch.get("owners",[]).size()>i else ""
   var zone=int(owner_zone.get(key,zone_for(box)))
   if zone<0: pin_intrusions(box)
   zones.append(zone)
  batch["zones"]=zones
 var groups: Dictionary={}
 for building in data.buildings:
  var index=roundi((building.cz+108)/24)*10+roundi((building.cx+108)/24)
  if pinned.has(index): continue
  if not groups.has(building.district): groups[building.district]=[]
  groups[building.district].append(index)
 var rng=RandomNumberGenerator.new()
 rng.seed=value
 var building_by_zone={}
 for building in data.buildings:
  building_by_zone[roundi((building.cz+108)/24)*10+roundi((building.cx+108)/24)]=building
 var source_for={}
 for zone in range(100): source_for[zone]=zone
 # Cover every available architectural family, then choose additional variants
 # with replacement. This changes building counts, not only plot order.
 for group in groups.values():
  var by_type={}
  for zone in group:
   var type=building_by_zone[zone].type
   if not by_type.has(type): by_type[type]=[]
   by_type[type].append(zone)
  var targets=group.duplicate()
  for i in range(targets.size()-1,0,-1):
   var j=rng.randi_range(0,i)
   var old=targets[i]
   targets[i]=targets[j]
   targets[j]=old
  var types=by_type.keys()
  for i in range(targets.size()):
   var type=types[i] if i<types.size() else types[rng.randi_range(0,types.size()-1)]
   var choices=by_type[type]
   source_for[targets[i]]=choices[rng.randi_range(0,choices.size()-1)]
 var destinations={}
 var moved=0
 for target in source_for:
  var source_zone=int(source_for[target])
  if target!=source_zone: moved+=1
  if not destinations.has(source_zone): destinations[source_zone]=[]
  destinations[source_zone].append(int(target))
 for batch in data.batches:
  var transforms=[]
  var colors=[]
  var owners=[]
  var roles=[]
  for i in range(batch.transforms.size()):
   var zone=int(batch.zones[i])
   var owner=batch.owners[i] if batch.get("owners",[]).size()>i else null
   var role=batch.roles[i] if batch.get("roles",[]).size()>i else "model"
   for target in destinations.get(zone,[]) if zone>=0 else [-1]:
    var values: Array=batch.transforms[i].duplicate()
    var shift=Vector3.ZERO if target<0 else Vector3(centers[target].x-centers[zone].x,0,centers[target].y-centers[zone].y)
    values[12]+=shift.x
    values[14]+=shift.z
    transforms.append(values)
    owners.append(remap_owner(owner,target))
    roles.append(role)
    colors.append_array(batch.colors.slice(i*3,i*3+3) if batch.colors.size()>i*3+2 else [1,1,1])
  batch.transforms=transforms
  batch.colors=colors
  batch.owners=owners
  batch.roles=roles
 var colliders=[]
 for box in data.colliders:
  var bounds=AABB(Vector3(box.minX,box.minY,box.minZ),Vector3(box.maxX-box.minX,box.maxY-box.minY,box.maxZ-box.minZ))
  var zone=int(owner_zone.get(str(box.get("owner","")),zone_for(bounds)))
  for target in destinations.get(zone,[]) if zone>=0 else [-1]:
   var copy=box.duplicate()
   var shift=Vector2.ZERO if target<0 else centers[target]-centers[zone]
   copy.minX+=shift.x
   copy.maxX+=shift.x
   copy.minZ+=shift.y
   copy.maxZ+=shift.y
   copy.owner=remap_owner(box.get("owner",null),target)
   colliders.append(copy)
 data.colliders=colliders
 var buildings=[]
 for target in range(100):
  var building: Dictionary=building_by_zone[source_for[target]].duplicate(true)
  var source_zone=int(source_for[target])
  var shift=centers[target]-centers[source_zone]
  building.x+=shift.x
  building.z+=shift.y
  building.cx=centers[target].x
  building.cz=centers[target].y
  building.id=target_id(target)
  buildings.append(building)
 data.buildings=buildings
 var assets=[]
 for asset in data.source_assets:
  var zone=int(owner_zone.get(str(asset.id),-1))
  for target in destinations.get(zone,[]) if zone>=0 else [-1]:
   var copy: Dictionary=asset.duplicate()
   var shift=Vector2.ZERO if target<0 else centers[target]-centers[zone]
   copy.x+=shift.x
   copy.z+=shift.y
   copy.id=remap_owner(asset.id,target)
   if target>=0 and copy.get("building")!=null: copy.building=target_id(target)
   assets.append(copy)
 data.source_assets=assets
 data.seed=value
 return {"seed":value,"moved_blocks":moved,"pinned_blocks":pinned.size(),"plan":source_for.duplicate(),"fresh_selection":true}

func target_id(zone: int) -> int: return (zone%10)*10+int(zone/10)

func remap_owner(owner,target: int):
 if target<0 or owner==null: return owner
 var id=str(owner)
 if id.begins_with("interior-"):
  var tokens=id.split("-")
  tokens[1]=str(target_id(target))
  return "-".join(tokens)
 if id.is_valid_int(): return target_id(target)
 return id+"@"+str(target)
