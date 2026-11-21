// Shared by the client and room server. Intervals resolve qualitative rate requests.
export const WEAPONS = Object.freeze([
  {id:'Pistol',unlockAfter:0,label:'G17',capacity:8,reserve:84,damage:45,interval:.28,reload:1.15,automatic:false,reloadStyle:'magazine'},
  {id:'Kureyka',unlockAfter:2,label:'KUREYKA-12',capacity:2,reserve:80,damage:100,interval:.4,reload:2.2,automatic:false,reloadStyle:'breech'},
  {id:'USP',unlockAfter:3,label:'USP-S',capacity:12,reserve:96,damage:55,interval:.3,reload:1.3,automatic:false,reloadStyle:'magazine'},
  {id:'Blackthorn',unlockAfter:4,label:'BLACKTHORN .357',capacity:6,reserve:48,damage:95,interval:.5,reload:2.6,automatic:false,reloadStyle:'shells'},
  {id:'Kemchug',viewZoom:1.2,viewLower:1.2,unlockAfter:5,label:'KEMCHUG K-3',capacity:50,reserve:300,damage:20,interval:.067,reload:3.3,automatic:true,reloadStyle:'action'},
  {id:'Ironhawk',unlockAfter:6,label:'IRONHAWK .50',capacity:7,reserve:56,damage:130,interval:.6,reload:2,automatic:false,reloadStyle:'magazine'},
  {id:'Biryusa',viewZoom:1.3,unlockAfter:7,label:'BIRYUSA BK-10',capacity:30,reserve:240,damage:32,interval:.08,reload:2.5,automatic:true,reloadStyle:'action'},
  {id:'Taiga',viewZoom:1.4,unlockAfter:8,label:'TAIGA-12',capacity:2,reserve:70,damage:105,interval:.4,reload:2.4,automatic:false,reloadStyle:'breech'},
  {id:'Angara',viewZoom:1.4,unlockAfter:12,label:'ANGARA-12',capacity:6,reserve:60,damage:75,interval:.7,reload:3,automatic:false,reloadStyle:'shells'},
  {id:'Bereg',viewZoom:1.35,unlockAfter:10,label:'BEREG A-24',capacity:30,reserve:240,damage:38,interval:.09,reload:2.6,automatic:true,reloadStyle:'action'},
  {id:'Sisim',viewZoom:1.35,unlockAfter:14,label:'SISIM SV-9',capacity:20,reserve:200,damage:34,interval:.075,reload:2.4,automatic:true,reloadStyle:'action'},
  {id:'Chulym',viewZoom:1.4,unlockAfter:15,label:'CHULYM C-5',capacity:30,reserve:240,damage:46,interval:.1,reload:2.8,automatic:true,reloadStyle:'action'},
  {id:'Sayan',viewZoom:1.4,unlockAfter:16,label:'SAYAN-12',capacity:8,reserve:64,damage:62,interval:.22,reload:3.2,automatic:false,reloadStyle:'action'},
  {id:'Vetluga',viewZoom:1.3,unlockAfter:18,label:'VETLUGA B-3',capacity:30,reserve:240,damage:55,interval:.15,reload:2.5,automatic:false,reloadStyle:'action'},
  {id:'Kazyr',viewZoom:1.5,unlockAfter:22,label:'KAZYR BR-62',capacity:20,reserve:160,damage:82,interval:.125,reload:3.2,automatic:true,reloadStyle:'action'},
  {id:'Katanga',viewZoom:1.6,unlockAfter:25,label:'KATANGA MG-1',capacity:100,reserve:400,damage:46,interval:.11,reload:5.5,automatic:true,reloadStyle:'action'},
]);
export const WEAPON_BY_ID = Object.freeze(Object.fromEntries(WEAPONS.map(w=>[w.id,w])));
