import * as T from 'three';
import {mergeGeometries} from 'three/addons/utils/BufferGeometryUtils.js';

function batchStaticParts(group,owner){
 for(const child of [...group.children])if(child.isGroup)batchStaticParts(child,owner);
 const byMaterial=new Map();
 for(const mesh of group.children.filter(c=>c.isMesh)){const key=mesh.material;if(!byMaterial.has(key))byMaterial.set(key,[]);byMaterial.get(key).push(mesh);}
 owner.visualGeometry??=new Map();
 for(const [material,parts] of byMaterial){
  if(parts.length<2)continue;
  parts.forEach(p=>p.updateMatrix());
  const key=parts.map(p=>p.geometry.uuid+':'+p.matrix.elements.join(',')).join('|');
  if(!owner.visualGeometry.has(key)){
   const geometries=parts.map(p=>{const g=p.geometry.index?p.geometry.toNonIndexed():p.geometry.clone();return g.applyMatrix4(p.matrix);});
   owner.visualGeometry.set(key,mergeGeometries(geometries));geometries.forEach(g=>g.dispose());
  }
  parts.forEach(p=>p.visible=false);
  const combined=new T.Mesh(owner.visualGeometry.get(key),material);combined.name='batched-static-parts';combined.castShadow=true;group.add(combined);
 }
}


export const ZOMBIE_SKINS = Object.freeze([
 {id:'civilian',skin:'#9a917a',clothes:'#465d72',pants:'#303c4a',accent:'#b0a18b'},
 {id:'worker',skin:'#a18a65',clothes:'#b77932',pants:'#46463b',accent:'#e0b84c'},
 {id:'medic',skin:'#a8a493',clothes:'#c0bba2',pants:'#596c60',accent:'#8e292c'},
 {id:'soldier',skin:'#818e70',clothes:'#4a5840',pants:'#3d4634',accent:'#7c7954'},
 {id:'hoodie',skin:'#9a817b',clothes:'#703c46',pants:'#38373b',accent:'#99908a'},
]);

// Separate shoulders, elbows, hips, knees and ankles; all measurements are local metres.
export function createMutantVisuals(owner,type,skinIndex){
 const root=new T.Group();root.name='anatomy';const limbs=[],legs=[],arms=[];
 const brute=type==='brute',runner=type==='runner',skin=ZOMBIE_SKINS[skinIndex];
 const palette={skin:brute?'#938270':runner?'#a0967c':skin.skin,cloth:brute?'#625445':runner?'#4b5750':skin.clothes,pants:brute?'#4b4236':skin.pants,accent:skin.accent,boot:'#292d29',wound:'#823c40',bone:'#bfb497',eye:'#e4c46b',black:'#252a23'};
 const materials=Object.fromEntries(Object.entries(palette).map(([key,color])=>[key,owner.material(color,key==='eye')]));
 function part(parent,name,kind,size,material,x,y,z){const p=owner.mesh(parent,kind,size,materials[material],x,y,z);p.name=name;p.castShadow=true;return p;}
 if(type==='eye'){
  part(root,'flesh-orb','sphere',[.8,2],'wound',0,0,0).scale.set(1,.94,.92);
  part(root,'sclera','sphere',[.52,2],'bone',0,.03,.48);
  part(root,'iris','sphere',[.31,2],'eye',0,.03,.89);
  part(root,'pupil','sphere',[.15,2],'black',0,.03,1.12);
  for(let n=0;n<9;n++){const a=n*Math.PI*2/9;part(root,'vein','capsule',[.035,.28,2,5],'skin',Math.cos(a)*.63,Math.sin(a)*.55,.12).rotation.z=a;}
  for(let n=0;n<6;n++){const a=n*Math.PI/3;const stalk=new T.Group();stalk.position.set(Math.cos(a)*.6,Math.sin(a)*.6,-.18);root.add(stalk);part(stalk,'tendril','cone',[.09,.48,6],'skin',0,0,-.2).rotation.x=Math.PI/2;arms.push({pivot:stalk,elbow:null,side:n%2?1:-1});}
  batchStaticParts(root,owner);return {root,limbs,legs,arms,eye:true};
 }
 const torso=new T.Group();torso.name='torso';torso.position.y=.96;root.add(torso);
 function upper(name,kind,size,material,x,y,z){return part(torso,name,kind,size,material,x,y-.96,z);}
 part(root,'pelvis','capsule',[brute?.29:.22,.13,4,8],'pants',0,.90,0);
 upper('chest','capsule',[brute?.30:.255,.27,4,8],'cloth',0,1.23,0);
 if(brute){
  upper('belly','sphere',[.35,2],'skin',0,1.08,.09).scale.set(1.1,.91,1.04);
  for(const side of [-1,1])upper('suspender','box',[.075,.46,.065],'cloth',side*.22,1.38,.30).rotation.z=side*-.17;
  for(let n=0;n<6;n++)upper('lesion','sphere',[.038+(n%2)*.015,1],'wound',Math.sin(n*2.4)*.30,1.02+n*.055,.45-Math.abs(Math.sin(n*2.4))*.10);
 }else{
  upper('torn-hem','box',[.35,.09,.27],'cloth',0,.94,.01);
  upper('shirt-seam','box',[.022,.42,.018],'accent',0,1.25,.257);
  for(const side of [-1,1])upper('pocket','box',[.095,.105,.035],'accent',side*.12,1.37,.24);
  upper('chest-wound','sphere',[.06,1],'wound',-.13,1.19,.235).scale.set(1,1.45,.38);
 }
 upper('neck','capsule',[.085,.10,3,7],'skin',0,1.57,0);
 const head=new T.Group();head.name='head';head.position.set(0,1.72-.96,.025);torso.add(head);
 part(head,'skull','sphere',[.215,2],'skin',0,0,0).scale.set(.92,1.08,.91);
 part(head,'jaw','box',[.255,.12,.20],'skin',0,-.16,.048);
 part(head,'mouth','box',[.19,.055,.035],'wound',0,-.125,.164);
 for(let n=0;n<5;n++)part(head,'tooth','box',[.023,.027,.015],'bone',(n-2)*.033,-.106,.185);
 part(head,'nose','cone',[.046,.11,5],'skin',0,-.018,.20).rotation.x=-Math.PI/2;
 for(const side of [-1,1]){
  part(head,'eye-socket','sphere',[.058,1],'black',side*.085,.047,.165).scale.set(1,.7,.55);
  part(head,'eye','sphere',[.027,1],'eye',side*.085,.049,.19);
  part(head,'brow','box',[.10,.027,.04],'skin',side*.083,.09,.166).rotation.z=side*-.17;
  part(head,'ear','sphere',[.045,1],'skin',side*.207,-.008,0).scale.set(.55,1,.7);
 }
 part(head,'temple-wound','sphere',[.05,1],'wound',.148,.105,.085).scale.set(.5,1,.55);
 if(!brute&&!runner){
  if(skin.id==='worker'){
   part(head,'hardhat','sphere',[.234,1],'accent',0,.148,-.01).scale.set(1,.43,1);
   part(head,'hardhat-brim','box',[.49,.025,.39],'accent',0,.128,.015);
   for(const side of [-1,1])upper('reflector','box',[.045,.31,.03],'bone',side*.18,1.25,.25);
  }else if(skin.id==='medic'){
   upper('coat-lapel','box',[.055,.30,.035],'bone',-.07,1.40,.25).rotation.z=.22;
   upper('medical-patch','box',[.095,.10,.022],'bone',.13,1.36,.268);
   upper('cross-vertical','box',[.024,.075,.01],'accent',.13,1.36,.285);
   upper('cross-horizontal','box',[.065,.023,.01],'accent',.13,1.36,.285);
  }else if(skin.id==='soldier'){
   upper('vest','box',[.41,.33,.12],'accent',0,1.30,.24);
   for(const side of [-1,1])upper('ammo-pouch','box',[.12,.14,.07],'cloth',side*.10,1.22,.33);
   part(head,'field-cap','sphere',[.225,1],'cloth',0,.16,0).scale.set(1,.30,1);
   part(head,'cap-visor','box',[.32,.025,.19],'cloth',0,.143,.15);
  }else if(skin.id==='hoodie'){
   upper('hood','sphere',[.22,1],'cloth',0,1.55,-.15).scale.set(1,.65,.55);
   for(const side of [-1,1])upper('drawstring','box',[.012,.16,.016],'accent',side*.08,1.40,.25);
  }else upper('jacket-collar','box',[.32,.065,.15],'accent',0,1.48,.11);
 }
 for(const side of [-1,1]){
  const shoulderMass=upper('shoulder-joint','capsule',[brute?.13:.09,brute?.24:.06,3,7],'cloth',side*(brute?.40:.275),1.43,0);shoulderMass.rotation.z=Math.PI/2;
  const hip=new T.Group();hip.name=side<0?'left-hip':'right-hip';hip.position.set(side*(brute?.25:.145),.85,0);root.add(hip);
  part(hip,'thigh','capsule',[brute?.13:.10,.19,3,7],'pants',0,-.195,0);
  const knee=new T.Group();knee.name='knee';knee.position.y=-.39;hip.add(knee);
  part(knee,'shin','capsule',[brute?.115:.085,.22,3,7],'pants',0,-.195,0);
  const ankle=new T.Group();ankle.name='ankle';ankle.position.y=-.39;knee.add(ankle);
  part(ankle,'boot','box',[brute?.24:.18,.14,.29],'boot',0,-.03,.065);
  part(ankle,'boot-sole','box',[brute?.245:.185,.035,.30],'black',0,-.0825,.065);
  legs.push({hip,knee,ankle,side});limbs.push(hip,knee,ankle);
  const shoulder=new T.Group();shoulder.name=side<0?'left-shoulder':'right-shoulder';
  // Wide shoulders keep both arms outside the bloated torso at every gait phase.
  shoulder.position.set(side*(brute?.53:.34),1.43-.96,0);torso.add(shoulder);
  shoulder.rotation.z=side*(brute?.09:.10);
  part(shoulder,'upper-arm','capsule',[brute?.105:.075,.19,3,7],'cloth',0,-.17,0);
  const elbow=new T.Group();elbow.name='elbow';elbow.position.y=-.34;shoulder.add(elbow);
  part(elbow,'forearm','capsule',[brute?.09:.063,.21,3,7],'skin',0,-.17,0);
  part(elbow,'hand','box',[brute?.15:.105,.15,.10],'skin',0,-.38,.012);
  for(let f=0;f<3;f++)part(elbow,'claw','cone',[.014,.13,5],'bone',(f-1)*.033,-.49,.036).rotation.z=Math.PI;
  arms.push({pivot:shoulder,elbow,side});limbs.push(shoulder,elbow);
 }
 batchStaticParts(root,owner);return {root,torso,head,limbs,legs,arms,brute,runner,eye:false};
}

export function poseMutant(enemy){
 const v=enemy.visual,phase=enemy.phase,motion=enemy.motion||0,attack=Math.sin(Math.min(1,(enemy.attackTime||0)/.42)*Math.PI);
 if(v.eye){v.arms.forEach((a,i)=>a.pivot.rotation.z=Math.sin(phase+i)*.14);return;}
 v.torso.rotation.x=(v.runner?.15:.045)+Math.sin(phase*2)*.018*motion;
 v.torso.rotation.z=Math.sin(phase)*.018*motion;
 v.head.rotation.y=Math.sin(phase*.5)*.035;
 for(const leg of v.legs){
  const p=phase+(leg.side<0?0:Math.PI),z=Math.cos(p)*.18*motion,lift=Math.max(0,Math.sin(p))*.12*motion;
  const down=.75-lift,distance=Math.min(.779,Math.hypot(down,z));
  leg.hip.rotation.x=Math.atan2(-z,down)-Math.acos(T.MathUtils.clamp(distance/.78,-1,1));
  leg.knee.rotation.x=2*Math.acos(T.MathUtils.clamp(distance/.78,-1,1));
  leg.ankle.rotation.x=-leg.hip.rotation.x-leg.knee.rotation.x;
 }
 for(const arm of v.arms){
  const stride=Math.cos(phase+(arm.side<0?0:Math.PI));
  arm.pivot.rotation.x=-.12+stride*.22*motion-attack*.85;
  arm.pivot.rotation.z=arm.side*(v.brute?.09:.10);
  arm.elbow.rotation.x=-.16-attack*.32-Math.max(0,-stride)*.10*motion;
 }
}
