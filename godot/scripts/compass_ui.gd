extends Control
var main
static func bearing(point: Vector3,origin: Vector3) -> float:
 var offset=point-origin
 return fposmod(rad_to_deg(atan2(offset.x,-offset.z)),360)
func x_for(angle: float) -> float:
 var heading=fposmod(-rad_to_deg(main.player.rotation.y),360)
 return size.x*.5+wrapf(angle-heading,-180,180)/150*size.x
func _process(_delta: float) -> void:
 if is_visible_in_tree(): queue_redraw()
func centered(font: Font,at: Vector2,value: String,size_value: int,color: Color) -> void:
 var width=font.get_string_size(value,HORIZONTAL_ALIGNMENT_LEFT,-1,size_value).x
 draw_string(font,at-Vector2(width*.5,0),value,HORIZONTAL_ALIGNMENT_LEFT,-1,size_value,color)
func marker(point: Vector3,value: String,color: Color,enemy=false) -> void:
 var x=x_for(bearing(point,main.player.position))
 if x<4 or x>size.x-4: return
 if enemy: draw_rect(Rect2(Vector2(x-2,43),Vector2(4,4)),color)
 else:
  draw_polygon(PackedVector2Array([Vector2(x,40),Vector2(x-3,46),Vector2(x+3,46)]),PackedColorArray([color]))
  centered(main.reference_ui.hud_root.theme.default_font,Vector2(x,59),value,8,color)
func _draw() -> void:
 if main==null: return
 var font=main.reference_ui.hud_root.theme.default_font
 for degree in range(0,360,5):
  var x=x_for(degree)
  if x<0 or x>size.x: continue
  var major=degree%15==0
  draw_line(Vector2(x,2),Vector2(x,8 if major else 5),Color("#9dac98"),1)
  if major:
   var name={0:"N",90:"E",180:"S",270:"W"}.get(degree,str(degree))
   centered(font,Vector2(x,21),name,9,Color("#dce6c4") if degree%90==0 else Color("#aebcb4"))
 var center=size.x*.5
 draw_polygon(PackedVector2Array([Vector2(center-3,1),Vector2(center+3,1),Vector2(center,6)]),PackedColorArray([Color("#cddaa1")]))
 var heading=fposmod(-rad_to_deg(main.player.rotation.y),360)
 centered(font,Vector2(center,36),"%d°" % int(round(heading)),11,Color("#e5edda"))
 var terminal=main.bridge.data.terminal
 var store=Vector3(terminal.x,0,terminal.z)
 marker(store,"MART · %dm" % int(store.distance_to(main.player.position)),Color("#ddc58a"))
 for enemy in main.battle.enemies:
  if main.battle.phase=="CLEARANCE" or enemy.position.distance_to(main.player.position)<25:
   marker(enemy.position,"",Color("#cf574a"),true)
 if main.objectives!=null:
  for record in main.objectives.markers(): marker(record.position,record.text,record.color)

 if main.coop!=null:
  for actor in main.coop.peers.values(): marker(actor.position,"SQUAD · %dm" % int(actor.position.distance_to(main.player.position)),Color("#8fc9d2"))
