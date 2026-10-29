extends SceneTree
func _initialize() -> void:
 var scene=load("res://assets/weapons/Guns.fbx").instantiate()
 for n in scene.get_children():
  var mesh: ArrayMesh=n.mesh
  var arrays=mesh.surface_get_arrays(0)
  var verts: PackedVector3Array=arrays[Mesh.ARRAY_VERTEX]
  var idx: PackedInt32Array=arrays[Mesh.ARRAY_INDEX]
  var t: Transform3D=n.transform
  var pts=[]
  var minp=Vector2(1e9,1e9);var maxp=Vector2(-1e9,-1e9)
  for v in verts:
   var w=t*v
   var q=Vector2(w.z,-w.y)
   pts.append(q)
   minp=minp.min(q);maxp=maxp.max(q)
  var size=maxp-minp
  var scale=240.0/size.x
  var h=int(size.y*scale)+8
  var img=Image.create(256,h,false,Image.FORMAT_RGBA8)
  var col=Color("#c7d9a0")
  var tris=idx.size()/3 if idx.size()>0 else verts.size()/3
  for ti in range(tris):
   var ia=idx[ti*3] if idx.size()>0 else ti*3
   var ib=idx[ti*3+1] if idx.size()>0 else ti*3+1
   var ic=idx[ti*3+2] if idx.size()>0 else ti*3+2
   var a=(pts[ia]-minp)*scale+Vector2(8,4)
   var b=(pts[ib]-minp)*scale+Vector2(8,4)
   var c=(pts[ic]-minp)*scale+Vector2(8,4)
   var lo=a.min(b).min(c);var hi=a.max(b).max(c)
   var d=(b.y-c.y)*(a.x-c.x)+(c.x-b.x)*(a.y-c.y)
   for y in range(maxi(0,int(lo.y)-1),mini(h-1,int(hi.y)+1)+1):
    for x in range(maxi(0,int(lo.x)-1),mini(255,int(hi.x)+1)+1):
     var p=Vector2(x+.5,y+.5)
     if absf(d)<.00001:
      continue
     var l1=((b.y-c.y)*(p.x-c.x)+(c.x-b.x)*(p.y-c.y))/d
     var l2=((c.y-a.y)*(p.x-c.x)+(a.x-c.x)*(p.y-c.y))/d
     var l3=1-l1-l2
     var e=-.12
     if l1>=e and l2>=e and l3>=e: img.set_pixel(x,y,col)
  # flip: y of image grows down, our q.y = -w.y so already image-down = -y... (up is smaller y)
  img.save_png("res://assets/icons/%s.png" % n.name)
  print("icon ",n.name,img.get_size())
 quit()
