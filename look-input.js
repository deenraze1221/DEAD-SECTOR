// Pointer-lock re-entry and OS cursor warps can emit enormous relative deltas.
export class LookInput{
 constructor(){this.reset(0);}
 reset(now){this.x=0;this.y=0;this.readyAt=now+80;}
 add(x,y,now){if(now<this.readyAt||!Number.isFinite(x)||!Number.isFinite(y)||Math.abs(x)>2000||Math.abs(y)>2000)return;this.x=Math.max(-1000,Math.min(1000,this.x+x));this.y=Math.max(-1000,Math.min(1000,this.y+y));}
 consume(dt,sensitivity=1){const limit=.28/(.002*Math.max(.2,sensitivity)),x=Math.max(-limit,Math.min(limit,this.x)),y=Math.max(-limit,Math.min(limit,this.y));this.x=0;this.y=0;return {x,y};}
}
