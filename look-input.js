// Pointer-lock re-entry and OS cursor warps can emit enormous relative deltas.
export class LookInput{
 constructor(){this.reset(0);}
 reset(now){this.x=0;this.y=0;this.readyAt=now+80;}
 add(x,y,now){if(now<this.readyAt||!Number.isFinite(x)||!Number.isFinite(y)||Math.abs(x)>2000||Math.abs(y)>2000)return;this.x=Math.max(-180,Math.min(180,this.x+x));this.y=Math.max(-180,Math.min(180,this.y+y));}
 consume(dt,sensitivity=1){const fraction=1-Math.exp(-24*Math.min(.1,Math.max(0,dt))),limit=.28/(.002*Math.max(.2,sensitivity)),x=Math.max(-limit,Math.min(limit,this.x*fraction)),y=Math.max(-limit,Math.min(limit,this.y*fraction));this.x-=x;this.y-=y;if(Math.abs(this.x)<.01)this.x=0;if(Math.abs(this.y)<.01)this.y=0;return {x,y};}
}
