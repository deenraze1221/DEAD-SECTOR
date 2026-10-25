import * as THREE from 'three';
import weaponModels from './assets/models/weapons.json' with { type: 'json' };
export const WEAPONS = Object.freeze([
  {id:'RifleGod',label:'GOD / ШТУРМОВАЯ',capacity:30,reserve:90,damage:34,interval:.11,reload:1.7,automatic:true},
  {id:'RifleBran',label:'BRAN / ВИНТОВКА',capacity:20,reserve:80,damage:50,interval:.2,reload:2,automatic:true},
  {id:'SMG',label:'SMG / ПИСТОЛЕТ-ПУЛЕМЁТ',capacity:32,reserve:128,damage:25,interval:.075,reload:1.5,automatic:true},
  {id:'Pistol',label:'ПИСТОЛЕТ',capacity:12,reserve:60,damage:50,interval:.25,reload:1.15,automatic:false},
]);
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
          color:m.name==='Gun_metal'?'#495654':i===1?'#77816e':'#bd9254',
          metalness:m.name==='Gun_metal'?.65:.25,roughness:m.name==='Gun_metal'?.5:.8,flatShading:true,
        }));
        return new THREE.Mesh(new THREE.BufferGeometryLoader().parse(part.geometry),materials);
      };
      const body=make(record.body),magazine=make(record.magazine);body.name=record.id;magazine.name=record.id+'Magazine';
      group.add(body,magazine);this.model.add(group);return {group,body,magazine,record};
    });
    this.flash=new THREE.Mesh(new THREE.ConeGeometry(.065,.18,5),new THREE.MeshBasicMaterial({color:'#ffe790'}));
    this.flash.rotation.x=-Math.PI/2;this.model.add(this.flash);this.reset();
  }
  reset(){
    const index=Math.min(3,Math.max(0,Math.floor(this.random()*4)));
    this.config=WEAPONS[index];this.variant=this.variants[index];this.gun=this.variant.body;this.magazine=this.variant.magazine;
    this.variants.forEach(v=>{v.group.visible=v===this.variant;v.magazine.position.set(0,0,0);v.magazine.rotation.set(0,0,0);});
    this.flash.position.set(0,this.variant.record.muzzleY,-this.variant.record.length/2-.08);
    this.ammo=this.config.capacity;this.reserve=this.config.reserve;this.cooldown=0;this.reloadTime=0;this.recoil=0;this.flashTime=0;this.flash.visible=false;
  }
  reload(){if(this.reloadTime||this.ammo===this.config.capacity||!this.reserve)return;this.reloadTime=this.config.reload;this.audio.play('reload');}
  step(dt){
    this.cooldown=Math.max(0,this.cooldown-dt);this.flashTime=Math.max(0,this.flashTime-dt);this.flash.visible=this.flashTime>0;this.recoil=Math.max(0,this.recoil-dt*1.8);
    if(this.reloadTime>0){this.reloadTime=Math.max(0,this.reloadTime-dt);if(!this.reloadTime){const n=Math.min(this.config.capacity-this.ammo,this.reserve);this.ammo+=n;this.reserve-=n;this.audio.play('reload');}}
  }
  fire(){if(this.reloadTime||this.cooldown)return false;if(!this.ammo){this.cooldown=.25;this.audio.play('empty');return false;}this.ammo--;this.cooldown=this.config.interval;this.recoil=.09;this.flashTime=.045;this.audio.play('shot');return true;}
  pose(time,moving,aim){
    const progress=this.reloadTime?1-this.reloadTime/this.config.reload:0,drop=this.reloadTime?Math.sin(progress*Math.PI):0;
    this.model.position.set(aim?0:.23,(aim?-.14:-.22)-drop*.15,-.6+this.recoil);this.model.rotation.set(this.recoil*.8,0,-drop*.25);
    // Animate the matching, separately authored magazine; top-fed BRAN lifts upwards.
    this.magazine.position.y=(this.config.id==='RifleBran'?1:-1)*drop*.22;this.magazine.rotation.x=drop*.2;
    if(moving){this.model.position.y+=Math.sin(time*10)*.007;this.model.position.x+=Math.cos(time*5)*.006;}
  }
}
