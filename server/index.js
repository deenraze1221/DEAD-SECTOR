import {createServer} from 'node:http';import {readFile} from 'node:fs/promises';import {resolve,extname} from 'node:path';import {randomBytes} from 'node:crypto';import {WebSocketServer,WebSocket} from 'ws';
import {WEAPON_BY_ID} from '../weapon-config.js';
import {createInventory,syncCombat,buy,equipWeapon,useMedkit,nearTerminal,rewardForHP} from '../economy.js';
import {ENEMY_TYPES} from '../waves.js';
const GRACE=120000;
export function createGameServer(){const root=resolve('dist');const server=createServer(async(req,res)=>{try{const pathname=new URL(req.url,'http://local').pathname,file=resolve(root,'.'+decodeURIComponent(pathname==='/'?'/index.html':pathname));if(!file.startsWith(root+'/'))return res.writeHead(403).end();const bytes=await readFile(file);res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.css':'text/css'})[extname(file)]||'application/octet-stream');res.end(bytes);}catch{res.writeHead(404).end('Build the game with npm run build.');}});
const wss=new WebSocketServer({server,maxPayload:524288}),rooms=new Map(),send=(s,m)=>{if(s?.readyState===WebSocket.OPEN)s.send(JSON.stringify(m));};
const other=(r,role)=>r[role==='host'?'guest':'host'];
function remove(s,explicit=false){const r=s.room,role=s.role;if(!r)return;const seat=r[role];if(seat?.socket!==s)return;seat.socket=null;seat.until=Date.now()+GRACE;s.room=null;if(explicit&&role==='host'){send(r.guest?.socket,{type:'closed'});if(r.guest?.socket)r.guest.socket.room=null;rooms.delete(r.code);}else if(explicit){r.guest=null;send(r.host.socket,{type:'left'});}else send(other(r,role)?.socket,{type:'away',role});}
function attach(s,r,role,resume=false){const seat=r[role];seat.socket=s;seat.until=0;s.room=r;s.role=role;send(s,{type:'room',code:r.code,host:role==='host',seed:r.seed,token:seat.token,resume,snapshot:r.snapshot,selfPose:seat.pose,account:seat.account,result:r.result});if(other(r,role)?.pose)send(s,other(r,role).pose);send(other(r,role)?.socket,{type:'joined',role});}
const timer=setInterval(()=>{for(const r of rooms.values()){if(!r.host.socket&&r.host.until<Date.now()){send(r.guest?.socket,{type:'closed'});rooms.delete(r.code);}else if(r.guest&&!r.guest.socket&&r.guest.until<Date.now())r.guest=null;}},5000);timer.unref();wss.on('close',()=>clearInterval(timer));
wss.on('connection',s=>{s.on('message',data=>{try{const m=JSON.parse(data),r=s.room;if(m.type==='create'){remove(s,true);if(rooms.size>=100)return send(s,{type:'error',message:'Server full'});let code;do{code=randomBytes(3).toString('hex').toUpperCase();}while(rooms.has(code));const room={code,host:{token:randomBytes(24).toString('hex'),account:createInventory()},guest:null,seed:m.seed>>>0,snapshot:null,cash:new Map(),rewarded:new Set()};rooms.set(code,room);attach(s,room,'host');}
else if(m.type==='join'){const room=rooms.get(String(m.code).toUpperCase());if(!room||room.guest)return send(s,{type:'error',message:'Room not found or slot reserved'});remove(s,true);room.guest={token:randomBytes(24).toString('hex'),account:createInventory()};attach(s,room,'guest');}
else if(m.type==='resume'){const room=rooms.get(String(m.code).toUpperCase()),role=room&&['host','guest'].find(key=>room[key]?.token===m.token);if(!role||room[role].socket)return send(s,{type:'error',message:'Resume expired or already connected'});attach(s,room,role,true);}
else if(r&&m.type==='leave')remove(s,true);
else if(r&&m.type==='pose'&&[m.x,m.y,m.z,m.yaw,m.hp].every(Number.isFinite)&&Math.abs(m.x)<101&&Math.abs(m.z)<101&&m.y>=0&&m.y<30){const own=r[s.role];if(!m.inventory||m.inventory.revision===own.account.revision)own.account.health=m.hp;else m.hp=own.account.health;own.pose=m;syncCombat(own.account,m.inventory);send(other(r,s.role)?.socket,m);}
else if(r&&m.type==='state'&&s.role==='host'&&Array.isArray(m.enemies)&&m.enemies.length<=1000){for(const e of m.enemies){const id='cash-'+e.id;if(e.hp<=0&&ENEMY_TYPES[e.type]&&!r.cash.has(id))r.cash.set(id,{id,x:e.x,z:e.z,value:rewardForHP(ENEMY_TYPES[e.type].hp),taken:false});}m.cash=[...r.cash.values()].filter(c=>!c.taken);r.snapshot=m;
const reward=(key,amount)=>{if(r.rewarded.has(key))return;r.rewarded.add(key);for(const role of ['host','guest']){const seat=r[role];if(seat){seat.account.money+=amount;seat.account.revision++;send(seat.socket,{type:'economy',account:seat.account,reason:'Награда получена'});}}};
if(m.event?.state?.status==='complete'){const e=m.event.state,amount={generator:150,defend:250,growth:350}[e.id];if(amount&&Number.isInteger(e.wave))reward('event-'+e.wave,amount);}
if(m.wave?.rest&&Number.isInteger(m.wave.number))reward('wave-'+m.wave.number,m.wave.number%5===0?500:75+m.wave.number*25);
send(r.guest?.socket,m);}
else if(r&&m.type==='shot'&&s.role==='guest'&&r.guest.pose?.hp>0&&Array.isArray(m.direction)&&m.direction.length===3&&m.direction.every(Number.isFinite)){const damage=WEAPON_BY_ID[m.weapon]?.damage,p=r.guest.pose;if(damage)send(r.host.socket,{type:'shot',origin:[p.x,p.y+1.65,p.z],direction:m.direction,damage});}
else if(r&&m.type==='damage'&&s.role==='host')send(r.guest?.socket,{type:'damage',amount:[15,25,30,40,55].includes(m.amount)?m.amount:25});
else if(r&&m.type==='end'&&s.role==='host'){r.result=m.result;send(r.guest?.socket,m);}
else if(r&&m.type==='event-interact'&&s.role==='guest'&&r.guest.pose?.hp>0)send(r.host.socket,{type:'event-interact',position:{x:r.guest.pose.x,y:r.guest.pose.y,z:r.guest.pose.z}});
else if(r&&m.type==='pickup'){
 const own=r[s.role],cash=r.cash.get(m.id),p=own.pose;
 if(cash&&!cash.taken&&p?.hp>0&&Math.hypot(p.x-cash.x,p.z-cash.z)<1.6&&p.y<1.3){syncCombat(own.account,m.inventory);if(m.inventory?.revision===own.account.revision&&Number.isFinite(m.health))own.account.health=Math.max(0,Math.min(100,m.health));cash.taken=true;own.account.money+=cash.value;own.account.revision++;send(s,{type:'economy',account:own.account,reason:'Деньги подобраны'});const all=[...r.cash.values()].filter(c=>!c.taken);if(r.snapshot)r.snapshot.cash=all;send(r.host.socket,{type:'loot-state',cash:all});send(r.guest?.socket,{type:'loot-state',cash:all});}
}
else if(r&&m.type==='market'){
 const own=r[s.role],a=own.account,p=own.pose;if(m.inventory&&m.inventory.revision!==a.revision)return send(s,{type:'economy',account:a,reason:'Состояние обновлено',ok:false});syncCombat(a,m.inventory);
 if(Number.isFinite(m.health)&&p){p.hp=Math.max(0,Math.min(100,m.health));a.health=p.hp;}
 let result={ok:false,reason:'Подойдите к терминалу'};
 if(p?.hp>0){if(m.action==='buy'&&nearTerminal(p))result=buy(a,m.id);else if(m.action==='equip')result={ok:equipWeapon(a,m.id),reason:''};else if(m.action==='use')result={ok:useMedkit(a,m.id),reason:''};}
 if(result.ok)a.revision++;if(p)p.hp=a.health;send(s,{type:'economy',account:a,reason:result.reason,ok:result.ok});
}
else if(r&&m.type==='revive'){const own=r[s.role],peer=other(r,s.role);if(own.pose?.hp>0&&peer?.pose?.hp<=0&&Math.hypot(own.pose.x-peer.pose.x,own.pose.z-peer.pose.z)<2.5){peer.pose.hp=50;peer.account.health=50;send(peer.socket,{type:'revived',hp:50});send(own.socket,{type:'revive-success'});}}

}catch{send(s,{type:'error',message:'Invalid packet'});}});s.on('close',()=>remove(s));});return {server,wss,rooms};}
if(process.argv[1]===resolve('server/index.js')){const {server}=createGameServer();server.listen(Number(process.env.PORT)||3000,'0.0.0.0',()=>console.log('DEAD SECTOR server on port '+(process.env.PORT||3000)));}
