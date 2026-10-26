import * as T from 'three';
// Local view only: the full soldier rig remains unchanged for other players.
export class FirstPersonBody {
 constructor(avatar){this.avatar=avatar;this.phase=0;this.weight=0;this.distance=0;this.root=avatar.root;this.root.scale.set(.72,1,.72);avatar.head.visible=false;avatar.upper.visible=false;for(const child of this.root.children)if(child.isMesh)child.visible=false;for(const leg of avatar.legs){leg.hip.position.y=.8575;leg.hip.position.z=-.10;leg.foot.position.z=0;}}
 update(player,dt){const travel=Math.max(0,(player.distance||0)-this.distance);this.distance=player.distance||0;this.phase+=travel*Math.PI*2/(player.sprinting?2.3:1.65);const moving=player.grounded&&player.speed>.08;this.weight=T.MathUtils.damp(this.weight,moving?Math.min(1,player.speed/4.5):0,12,dt);const yaw=player.yaw||0,forward=-(player.velocityX||0)*Math.sin(yaw)-(player.velocityZ||0)*Math.cos(yaw),direction=forward<-.1?-1:1;
  for(const [i,leg]of this.avatar.legs.entries()){const phase=this.phase+i*Math.PI,z=-.18+Math.cos(phase)*.14*this.weight*direction,lift=Math.max(0,Math.sin(phase))*.13*this.weight,down=Math.sqrt(.74*.74-z*z)-lift,distance=Math.min(.7399,Math.hypot(down,z)),a=.38,b=.36;leg.hip.position.y=.1175+Math.sqrt(.74*.74-z*z);leg.hip.rotation.set(Math.atan2(-z,down)-Math.acos(T.MathUtils.clamp((a*a+distance*distance-b*b)/(2*a*distance),-1,1)),0,0);leg.knee.rotation.x=Math.PI-Math.acos(T.MathUtils.clamp((a*a+b*b-distance*distance)/(2*a*b),-1,1));if(!player.grounded){leg.hip.rotation.x-=.12;leg.knee.rotation.x+=.3;}leg.foot.rotation.x=-leg.hip.rotation.x-leg.knee.rotation.x;}
  this.root.position.set(player.position.x+Math.sin(yaw)*.18,player.position.y,player.position.z+Math.cos(yaw)*.18);this.root.rotation.set(0,yaw,0);
 }
}
