import * as THREE from 'three';
import weaponModels from './assets/models/weapons.json' with { type: 'json' };
import {WEAPONS} from './weapon-config.js';
export {WEAPONS};
export class Rifle {
  constructor(audio, random = Math.random) {
    this.audio=audio;this.random=random;this.scene=new THREE.Scene();this.camera=new THREE.PerspectiveCamera(65,1,.01,10);
    this.scene.add(new THREE.HemisphereLight('#d5dfcb','#494739',3));
    const light=new THREE.DirectionalLight('#ece2c4',3);light.position.set(-2,3,2);this.scene.add(light);
    this.model=new THREE.Group();this.model.scale.setScalar(.8);this.scene.add(this.model);
    this.variants=weaponModels.map(record=>{
      const group=new THREE.Group();group.name=record.id;
      const make=part=>{
        const materials=part.materials.map((m,i)=>new THREE.MeshStandardMaterial({
          color:record.id==='Pistol'?(m.name==='Gun_metal'?'#495654':i===1?'#77816e':'#bd9254'):new THREE.Color(m.color).lerp(new THREE.Color('#73776d'),.08),
          metalness:.5,roughness:.7,flatShading:true,
        }));
        return new THREE.Mesh(new THREE.BufferGeometryLoader().parse(part.geometry),materials.length===1?materials[0]:materials);
      };
      // gunsv2 supplies single-piece guns; never detach an invented magazine.
      const body=make(record.body),magazine=record.magazine?make(record.magazine):new THREE.Group();body.name=record.id;magazine.name=record.id+'Magazine';
      group.add(body,magazine);this.model.add(group);return {group,body,magazine,record};
    });
    this.hands=[-1,1].map(side=>{
      const g=new THREE.Group();g.name=side<0?'support-hand':'trigger-hand';
      const glove=new THREE.MeshStandardMaterial({color:'#59604a',roughness:.95}),sleeve=new THREE.MeshStandardMaterial({color:'#303d32',roughness:1});
      const palm=new THREE.Mesh(new THREE.BoxGeometry(.095,.08,.13),glove);g.add(palm);
      for(let i=0;i<4;i++){const finger=new THREE.Mesh(new THREE.CapsuleGeometry(.014,.055,2,5),glove);finger.rotation.x=Math.PI/2;finger.position.set((i-1.5)*.022,-.025,-.055);g.add(finger);}
      const thumb=new THREE.Mesh(new THREE.CapsuleGeometry(.02,.04,2,5),glove);thumb.rotation.z=side*.65;thumb.position.set(side*.055,.01,0);g.add(thumb);
      const arm=new THREE.Mesh(new THREE.CylinderGeometry(.055,.085,.32,8),sleeve);arm.rotation.x=Math.PI/2;arm.position.set(side*.035,-.045,.2);g.add(arm);this.model.add(g);return g;
    });
    this.flash=new THREE.Mesh(new THREE.ConeGeometry(.065,.18,5),new THREE.MeshBasicMaterial({color:'#ffe790'}));
    this.flash.rotation.x=-Math.PI/2;this.model.add(this.flash);this.reset();
  }
  reset(id){
    const selected=WEAPONS.findIndex(w=>w.id===id);
    const index=selected>=0?selected:Math.min(WEAPONS.length-1,Math.max(0,Math.floor(this.random()*WEAPONS.length)));
    this.config=WEAPONS[index];this.variant=this.variants.find(v=>v.record.id===this.config.id);this.gun=this.variant.body;this.magazine=this.variant.magazine;
    this.variants.forEach(v=>{v.group.visible=v===this.variant;v.magazine.position.set(0,0,0);v.magazine.rotation.set(0,0,0);});
    this.flash.position.set(0,this.variant.record.muzzleY,-this.variant.record.length/2-.08);
    this.ammo=this.config.capacity;this.reserve=this.config.reserve;this.cooldown=0;this.reloadTime=0;this.recoil=0;this.flashTime=0;this.flash.visible=false;
  }
  reload(){if(this.reloadTime||this.ammo===this.config.capacity||!this.reserve)return;this.reloadTime=this.config.reload;this.audio.play('reload');}
  step(dt){
    this.cooldown=Math.max(0,this.cooldown-dt);this.flashTime=Math.max(0,this.flashTime-dt);this.flash.visible=this.flashTime>0;this.recoil=Math.max(0,this.recoil-dt*1.8);
    if(this.reloadTime>0){this.reloadTime=Math.max(0,this.reloadTime-dt);if(!this.reloadTime){const n=Math.min(this.config.capacity-this.ammo,this.reserve);this.ammo+=n;this.reserve-=n;this.audio.play('reload');}}
  }
  fire(){if(this.reloadTime||this.cooldown)return false;if(!this.ammo){this.cooldown=.25;this.audio.play('empty');return false;}this.ammo--;this.cooldown=this.config.interval;this.recoil=.09;this.flashTime=.045;this.audio.shot?.(this.config.id);return true;}
  pose(time,moving){
    const progress=this.reloadTime?1-this.reloadTime/this.config.reload:0,drop=this.reloadTime?Math.sin(progress*Math.PI):0;
    this.model.position.set(.23,(-.22)-drop*.15,-.48+this.recoil);this.model.rotation.set(this.recoil*.8,0,-drop*.25);
    this.magazine.position.y=-drop*.22;this.magazine.rotation.x=drop*.2;
    const loading=this.reloadTime&&this.config.reloadStyle!=='magazine'?Math.sin(progress*Math.PI*6)*drop:0;
    if(this.reloadTime)this.model.rotation.x+=drop*(this.config.reloadStyle==='breech'?.4:.12);
    const length=this.variant.record.length;
    this.hands[1].position.set(.055,-.12,length*.23);
    this.hands[0].position.set(-.065-drop*.1,-.09+this.magazine.position.y+loading*.025,-length*.13+drop*.16+loading*.04);
    this.hands[0].rotation.z=drop*.4;
    if(moving){this.model.position.y+=Math.sin(time*10)*.007;this.model.position.x+=Math.cos(time*5)*.006;}
  }
}
