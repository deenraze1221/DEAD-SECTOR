extends RefCounted
# The ten weapons from Weapons/Guns.txt. Model frame: pivot at the grip, barrel along -Z after the 180 degree turn.
# bounds = [min z, max z] of the model after the turn (barrel tip is min z); scale = viewmodel scale.
# shots / gap: rounds per trigger pull and delay between them. sound: caliber class chosen by damage.
const ORDER=["Pistol","Diaglo","Shotgun","SMG","Disorate","Kapano","Arben","Well-255","APVS","MDR-ROSE50"]
const DEFS={
 "Pistol":{"label":"PISTOL","price":0,"unlock":0,"damage":35,"capacity":8,"reserve":84,"reload":2.5,"interval":.28,"automatic":false,"shots":1,"gap":0.0,"sound":"Pistol","style":"slide","scale":.62,"tip":-.327,"rear":.057,"kick":.07,
  "support":[-.01,-.04,-.05],"trigger":[0.0,-.01,.0],"mag":[0.0,-.045,.0]},
 "Diaglo":{"label":"DIAGLO","price":1000,"unlock":1,"damage":50,"capacity":6,"reserve":48,"reload":3.0,"interval":.4,"automatic":false,"shots":1,"gap":0.0,"sound":"Shambler","style":"revolver","scale":.58,"tip":-.53,"rear":.059,"kick":.1,
  "support":[-.02,-.07,-.07],"trigger":[0.0,-.02,.0],"mag":[0.0,.03,-.08]},
 "Shotgun":{"label":"SHOTGUN","price":2000,"unlock":2,"damage":100,"capacity":2,"reserve":24,"reload":2.0,"interval":.6,"automatic":false,"shots":2,"gap":.07,"sound":"Shotgun","style":"break","scale":.5,"tip":-.814,"rear":.072,"kick":.14,
  "support":[0.0,-.07,-.34],"trigger":[0.0,-.02,.0],"mag":[0.0,.0,-.2]},
 "SMG":{"label":"SMG","price":2500,"unlock":4,"damage":20,"capacity":30,"reserve":240,"reload":2.5,"interval":.11,"automatic":true,"shots":1,"gap":0.0,"sound":"Vector","style":"smg","scale":.42,"tip":-.794,"rear":.361,"kick":.04,
  "support":[0.0,-.06,-.32],"trigger":[0.0,-.03,.0],"mag":[0.0,-.14,-.1]},
 "Disorate":{"label":"DISORATE","price":3000,"unlock":6,"damage":30,"capacity":24,"reserve":192,"reload":3.0,"interval":.36,"automatic":false,"shots":3,"gap":.055,"sound":"Scorpion","style":"smg","scale":.42,"tip":-.711,"rear":.412,"kick":.06,
  "support":[0.0,-.06,-.3],"trigger":[0.0,-.03,.0],"mag":[0.0,-.12,-.1]},
 "Kapano":{"label":"KAPANO","price":4000,"unlock":8,"damage":45,"capacity":30,"reserve":240,"reload":3.5,"interval":.105,"automatic":true,"shots":1,"gap":0.0,"sound":"Kalash","style":"rifle","scale":.4,"tip":-.821,"rear":.399,"kick":.06,
  "support":[0.0,-.07,-.36],"trigger":[0.0,-.04,.0],"mag":[0.0,-.14,-.12]},
 "Arben":{"label":"ARBEN","price":4500,"unlock":10,"damage":200,"capacity":1,"reserve":16,"reload":2.0,"interval":.9,"automatic":false,"shots":1,"gap":0.0,"sound":"Tikhar","style":"bolt","scale":.44,"tip":-.82,"rear":.201,"kick":.16,
  "support":[0.0,-.08,-.34],"trigger":[0.0,-.03,.0],"mag":[0.0,.0,-.1]},
 "Well-255":{"label":"WELL-255","price":5000,"unlock":13,"damage":40,"capacity":35,"reserve":280,"reload":3.0,"interval":.13,"automatic":true,"shots":1,"gap":0.0,"sound":"G36","style":"rifle","scale":.42,"tip":-.693,"rear":.345,"kick":.05,
  "support":[0.0,-.08,-.3],"trigger":[0.0,-.05,.0],"mag":[0.0,-.15,-.1]},
 "APVS":{"label":"APVS","price":6000,"unlock":16,"damage":140,"capacity":5,"reserve":40,"reload":4.0,"interval":.7,"automatic":false,"shots":1,"gap":0.0,"sound":"Tikhar","style":"bolt","scale":.4,"tip":-.905,"rear":.295,"kick":.13,
  "support":[0.0,-.07,-.4],"trigger":[0.0,-.03,.0],"mag":[0.0,-.06,-.1]},
 "MDR-ROSE50":{"label":"MDR-ROSE50","price":6500,"unlock":20,"damage":50,"capacity":50,"reserve":400,"reload":5.5,"interval":.075,"automatic":true,"shots":1,"gap":0.0,"sound":"VSV","style":"lmg","scale":.34,"tip":-.978,"rear":.452,"kick":.045,
  "support":[0.0,-.1,-.4],"trigger":[0.0,-.06,.0],"mag":[0.0,-.2,-.05]}
}

static func weapons() -> Dictionary:
 var result={}
 for id in ORDER:
  var d: Dictionary=DEFS[id]
  var s=float(d.scale)
  result[id]={"id":id,"label":d.label,"unlockAfter":d.unlock,"capacity":d.capacity,"reserve":d.reserve,"damage":d.damage,"interval":d.interval,"reload":d.reload,"automatic":d.automatic,"shots":d.shots,"gap":d.gap,"sound":d.sound,"reloadStyle":d.style,"kick":d.kick,"viewScale":s,
   "grips":{"support":scaled(d.support,s),"trigger":scaled(d.trigger,s)},"mag":scaled(d.mag,s),"tip":d.tip*s,"rear":d.rear*s}
 return result

static func scaled(v: Array, s: float) -> Array: return [v[0]*s,v[1]*s,v[2]*s]

static func shop_items() -> Array:
 var list=[]
 for id in ORDER:
  if id!="Pistol": list.append({"id":id,"kind":"weapon","name":DEFS[id].label,"price":DEFS[id].price})
 return list
