extends RefCounted
var centers: Array[Vector2]=[]
var pinned: Dictionary={}
var owner_zone: Dictionary={}
var geometry_bounds: Array[AABB]=[]
var zone_map: Dictionary={}
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

func offset(zone: int) -> Vector3:
 if zone<0: return Vector3.ZERO
 var a=centers[zone]
 var b=centers[zone_map.get(zone,zone)]
 return Vector3(b.x-a.x,0,b.y-a.y)

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
 for group in groups.values():
  var shuffled=group.duplicate()
  for i in range(shuffled.size()-1,0,-1):
   var j=rng.randi_range(0,i)
   var tmp=shuffled[i]
   shuffled[i]=shuffled[j]
   shuffled[j]=tmp
  for i in range(group.size()): zone_map[group[i]]=shuffled[i]
 var moved=0
 for zone in zone_map:
  if zone_map[zone]!=zone: moved+=1
 for batch in data.batches:
  for i in range(batch.transforms.size()):
   var shift=offset(batch.zones[i])
   batch.transforms[i][12]+=shift.x
   batch.transforms[i][14]+=shift.z
 for box in data.colliders:
  var bounds=AABB(Vector3(box.minX,box.minY,box.minZ),Vector3(box.maxX-box.minX,box.maxY-box.minY,box.maxZ-box.minZ))
  var zone=int(owner_zone.get(str(box.get("owner","")),zone_for(bounds)))
  var shift=offset(zone)
  box.minX+=shift.x
  box.maxX+=shift.x
  box.minZ+=shift.z
  box.maxZ+=shift.z
 for building in data.buildings:
  var zone=roundi((building.cz+108)/24)*10+roundi((building.cx+108)/24)
  var shift=offset(zone)
  building.x+=shift.x
  building.z+=shift.z
  building.cx+=shift.x
  building.cz+=shift.z
 data.seed=value
 return {"seed":value,"moved_blocks":moved,"pinned_blocks":pinned.size(),"plan":zone_map.duplicate()}
