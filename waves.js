export const ENEMY_TYPES={
 zombie:{hp:250,speed:2.2,damage:25,weight:55,radius:.5,height:2,scale:1},
 runner:{hp:125,speed:3.6,damage:15,weight:20,radius:.38,height:1.5,scale:.75},
 eye:{hp:300,speed:2.3,damage:30,weight:20,radius:.7,height:1.5,scale:1},
 brute:{hp:600,speed:1.1,damage:40,weight:5,radius:2.15,height:6,scale:3},
};
export function chooseEnemy(random=Math.random){let roll=random()*100;for(const [type,s]of Object.entries(ENEMY_TYPES)){roll-=s.weight;if(roll<0)return type;}return 'brute';}
export class Waves {
 constructor(seed=1){this.seed=seed>>>0;this.number=1;this.time=0;this.spawned=0;this.rest=false;this.clearing=false;}
 random(){this.seed=(1664525*this.seed+1013904223)>>>0;return this.seed/4294967296;}
 get total(){return 30+(this.number-1)*5;}
 step(dt,spawn,getAlive=()=>0){
  let remaining=dt;
  while(remaining>1e-9){
   if(this.clearing){
    if(getAlive()>0)return;
    this.clearing=false;this.rest=true;this.time=0;
   }
   const limit=this.rest?10:120,chunk=Math.min(remaining,limit-this.time);
   this.time+=chunk;remaining-=chunk;
   if(!this.rest){
    const due=Math.min(this.total,Math.floor((this.time+1e-8)*this.total/120));
    while(this.spawned<due){spawn(chooseEnemy(()=>this.random()),Math.floor(this.random()*4));this.spawned++;}
   }
   if(this.time>=limit-1e-9){
    if(this.rest){this.number++;this.spawned=0;this.rest=false;this.time=0;}
    else if(getAlive()>0){this.time=120;this.clearing=true;}
    else{this.rest=true;this.time=0;}
   }
  }
 }
 snapshot(){return {seed:this.seed,number:this.number,time:this.time,spawned:this.spawned,rest:this.rest,clearing:this.clearing};}
 restore(v){Object.assign(this,v);this.clearing=Boolean(v.clearing);}
}
