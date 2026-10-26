// Shared by the client and room server. Intervals resolve qualitative rate requests.
export const WEAPONS = Object.freeze([
  {id:'Pistol',label:'ПИСТОЛЕТ',capacity:8,reserve:84,damage:45,interval:.28,reload:1.15,automatic:false,reloadStyle:'magazine'},
  {id:'Helsing',label:'HELSING',capacity:40,reserve:320,damage:30,interval:.075,reload:4,automatic:true,reloadStyle:'action'},
  {id:'Tikhar',label:'TIKHAR',capacity:1,reserve:60,damage:55,interval:.45,reload:3,automatic:false,reloadStyle:'breech'},
  {id:'Shambler',label:'SHAMBLER',capacity:5,reserve:100,damage:50,interval:.5,reload:3,automatic:false,reloadStyle:'shells'},
  {id:'Shotgun',label:'SHOTGUN',capacity:2,reserve:80,damage:80,interval:.6,reload:2,automatic:false,reloadStyle:'breech'},
  {id:'Kalash',label:'KALASH',capacity:30,reserve:240,damage:40,interval:.14,reload:3,automatic:true,reloadStyle:'action'},
  {id:'VSV',label:'VSV',capacity:12,reserve:120,damage:70,interval:.24,reload:6,automatic:true,reloadStyle:'action'},
  {id:'Scorpion',label:'SCORPION',capacity:35,reserve:280,damage:35,interval:.10,reload:4,automatic:true,reloadStyle:'action'},
  {id:'Vector',label:'VECTOR',capacity:20,reserve:200,damage:35,interval:.09,reload:3,automatic:true,reloadStyle:'action'},
  {id:'G36',label:'G36',capacity:30,reserve:240,damage:45,interval:.14,reload:4,automatic:true,reloadStyle:'action'},
]);
export const WEAPON_BY_ID = Object.freeze(Object.fromEntries(WEAPONS.map(w=>[w.id,w])));
