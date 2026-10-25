import * as THREE from 'three';
import gunGeometry from './assets/models/gun.geometry.json' with { type: 'json' };
export class Rifle {
  constructor(audio) {
    this.audio=audio;this.scene=new THREE.Scene();this.camera=new THREE.PerspectiveCamera(65,1,.01,10);
    this.scene.add(new THREE.HemisphereLight('#d5dfcb','#494739',3));
    const light=new THREE.DirectionalLight('#ece2c4',3);light.position.set(-2,3,2);this.scene.add(light);
    this.model=new THREE.Group();this.model.scale.setScalar(.8);this.scene.add(this.model);
    // This geometry is converted from the user's original gun.fbx, not a substitute model.
    const geometry=new THREE.BufferGeometryLoader().parse(gunGeometry);
    const positions=geometry.attributes.position;
    const byMaterial=[[],[],[]];
    for(let i=0;i<positions.count;i+=3){
      const y=(positions.getY(i)+positions.getY(i+1)+positions.getY(i+2))/3;
      const z=(positions.getZ(i)+positions.getZ(i+1)+positions.getZ(i+2))/3;
      const material=z<-.33?2:y<-.04?1:0;
      byMaterial[material].push(i,i+1,i+2);
    }
    geometry.clearGroups();geometry.setIndex(byMaterial.flat());
    let start=0;byMaterial.forEach((indices,material)=>{geometry.addGroup(start,indices.length,material);start+=indices.length;});
    const materials=[
      new THREE.MeshStandardMaterial({color:'#657063',metalness:.4,roughness:.67,flatShading:true}),
      new THREE.MeshStandardMaterial({color:'#252c29',metalness:.05,roughness:.9,flatShading:true}),
      new THREE.MeshStandardMaterial({color:'#77807d',metalness:.75,roughness:.4,flatShading:true}),
    ];
    this.gun=new THREE.Mesh(geometry,materials);this.gun.name='user-gun-fbx';this.model.add(this.gun);
    this.flash=new THREE.Mesh(new THREE.ConeGeometry(.065,.18,5),new THREE.MeshBasicMaterial({color:'#ffe790'}));
    this.flash.rotation.x=-Math.PI/2;this.flash.position.set(0,.12,-.53);this.model.add(this.flash);
    this.reset();
  }
  reset(){this.ammo=30;this.reserve=90;this.cooldown=0;this.reloadTime=0;this.recoil=0;this.flashTime=0;this.flash.visible=false;}
  reload(){if(this.reloadTime||this.ammo===30||!this.reserve)return;this.reloadTime=1.7;this.audio.play('reload');}
  step(dt){
    this.cooldown=Math.max(0,this.cooldown-dt);this.flashTime=Math.max(0,this.flashTime-dt);this.flash.visible=this.flashTime>0;
    this.recoil=Math.max(0,this.recoil-dt*1.8);
    if(this.reloadTime>0){this.reloadTime=Math.max(0,this.reloadTime-dt);if(!this.reloadTime){const n=Math.min(30-this.ammo,this.reserve);this.ammo+=n;this.reserve-=n;this.audio.play('reload');}}
  }
  fire(){if(this.reloadTime||this.cooldown)return false;if(!this.ammo){this.cooldown=.25;this.audio.play('empty');return false;}this.ammo--;this.cooldown=.11;this.recoil=.09;this.flashTime=.045;this.audio.play('shot');return true;}
  pose(time,moving,aim){this.model.position.set(aim?0:.23,aim?-.143:-.22-(this.reloadTime?Math.sin(this.reloadTime/1.7*Math.PI)*.24:0),-.6+this.recoil);this.model.rotation.set(this.recoil*.8,0,this.reloadTime?-.35:0);if(moving){this.model.position.y+=Math.sin(time*10)*.007;this.model.position.x+=Math.cos(time*5)*.006;}}
}
