import * as THREE from 'three';
export class Rifle {
  constructor(audio) {
    this.audio=audio;this.scene=new THREE.Scene();this.camera=new THREE.PerspectiveCamera(65,1,.01,10);
    this.scene.add(new THREE.HemisphereLight('#d5dfcb','#494739',3));
    const light=new THREE.DirectionalLight('#ece2c4',3);light.position.set(-2,3,2);this.scene.add(light);
    this.model=new THREE.Group();this.model.scale.setScalar(.7);this.scene.add(this.model);
    const steel=new THREE.MeshStandardMaterial({color:'#596260',metalness:.65,roughness:.6});
    const black=new THREE.MeshStandardMaterial({color:'#1b2424',roughness:.8});
    const wood=new THREE.MeshStandardMaterial({color:'#63543b',roughness:.9});
    const box=(w,h,d,x,y,z,mat=steel)=>{const m=new THREE.Mesh(new THREE.BoxGeometry(w,h,d),mat);m.position.set(x,y,z);this.model.add(m);return m;};
    box(.13,.16,.38,0,0,-.22);box(.1,.11,.32,0,0,-.53,wood);
    const barrel=new THREE.Mesh(new THREE.CylinderGeometry(.023,.023,.3,10),steel);barrel.rotation.x=Math.PI/2;barrel.position.set(0,.015,-.83);this.model.add(barrel);
    box(.08,.12,.09,0,.095,-.92,black);box(.025,.055,.025,0,.175,-.92);box(.06,.035,.03,0,.12,-.12);
    const mag=box(.08,.23,.13,0,-.16,-.28,black);mag.rotation.x=-.15;
    const grip=box(.07,.18,.09,0,-.14,-.04,black);grip.rotation.x=-.25;
    box(.12,.13,.27,0,-.01,.08,wood);box(.1,.025,.35,0,.095,-.26,black);
    for(let i=0;i<6;i++)box(.135,.018,.018,0,.04,-.43-i*.04,black);
    for(const x of [-.15,.15])box(.07,.13,.16,x,-.1,-.42,new THREE.MeshStandardMaterial({color:'#455242'}));
    this.flash=new THREE.Mesh(new THREE.ConeGeometry(.085,.24,5),new THREE.MeshBasicMaterial({color:'#ffe790'}));this.flash.rotation.x=-Math.PI/2;this.flash.position.set(0,.015,-1.07);this.model.add(this.flash);
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
  pose(time,moving,aim){this.model.position.set(aim?0:.23,aim?-.18:-.23-(this.reloadTime?Math.sin(this.reloadTime/1.7*Math.PI)*.24:0),-.35+this.recoil);this.model.rotation.set(this.recoil*.8,0,this.reloadTime?-.35:0);if(moving){this.model.position.y+=Math.sin(time*10)*.007;this.model.position.x+=Math.cos(time*5)*.006;}}
}
