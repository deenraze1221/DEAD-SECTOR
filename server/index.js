import {createServer} from 'node:http';import {readFile} from 'node:fs/promises';import {resolve,extname} from 'node:path';import {randomBytes} from 'node:crypto';import {WebSocketServer,WebSocket} from 'ws';
import {WEAPON_BY_ID} from '../weapon-config.js';
const GRACE=120000;
export function createGameServer(){const root=resolve('dist');const server=createServer(async(req,res)=>{try{const pathname=new URL(req.url,'http://local').pathname,file=resolve(root,'.'+decodeURIComponent(pathname==='/'?'/index.html':pathname));if(!file.startsWith(root+'/'))return res.writeHead(403).end();const bytes=await readFile(file);res.setHeader('Content-Type',({'.html':'text/html','.js':'text/javascript','.css':'text/css'})[extname(file)]||'application/octet-stream');res.end(bytes);}catch{res.writeHead(404).end('Build the game with npm run build.');}});
const wss=new WebSocketServer({server,maxPayload:524288}),rooms=new Map(),send=(s,m)=>{if(s?.readyState===WebSocket.OPEN)s.send(JSON.stringify(m));};
const other=(r,role)=>r[role==='host'?'guest':'host'];
function remove(s,explicit=false){const r=s.room,role=s.role;if(!r)return;const seat=r[role];if(seat?.socket!==s)return;seat.socket=null;seat.until=Date.now()+GRACE;s.room=null;if(explicit&&role==='host'){send(r.guest?.socket,{type:'closed'});if(r.guest?.socket)r.guest.socket.room=null;rooms.delete(r.code);}else if(explicit){r.guest=null;send(r.host.socket,{type:'left'});}else send(other(r,role)?.socket,{type:'away',role});}
function attach(s,r,role,resume=false){const seat=r[role];seat.socket=s;seat.until=0;s.room=r;s.role=role;send(s,{type:'room',code:r.code,host:role==='host',seed:r.seed,token:seat.token,resume,snapshot:r.snapshot,selfPose:seat.pose,result:r.result});if(other(r,role)?.pose)send(s,other(r,role).pose);send(other(r,role)?.socket,{type:'joined',role});}
const timer=setInterval(()=>{for(const r of rooms.values()){if(!r.host.socket&&r.host.until<Date.now()){send(r.guest?.socket,{type:'closed'});rooms.delete(r.code);}else if(r.guest&&!r.guest.socket&&r.guest.until<Date.now())r.guest=null;}},5000);timer.unref();wss.on('close',()=>clearInterval(timer));
wss.on('connection',s=>{s.on('message',data=>{try{const m=JSON.parse(data),r=s.room;if(m.type==='create'){remove(s,true);if(rooms.size>=100)return send(s,{type:'error',message:'Server full'});let code;do{code=randomBytes(3).toString('hex').toUpperCase();}while(rooms.has(code));const room={code,host:{token:randomBytes(24).toString('hex')},guest:null,seed:m.seed>>>0,snapshot:null};rooms.set(code,room);attach(s,room,'host');}
else if(m.type==='join'){const room=rooms.get(String(m.code).toUpperCase());if(!room||room.guest)return send(s,{type:'error',message:'Room not found or slot reserved'});remove(s,true);room.guest={token:randomBytes(24).toString('hex')};attach(s,room,'guest');}
else if(m.type==='resume'){const room=rooms.get(String(m.code).toUpperCase()),role=room&&['host','guest'].find(key=>room[key]?.token===m.token);if(!role||room[role].socket)return send(s,{type:'error',message:'Resume expired or already connected'});attach(s,room,role,true);}
else if(r&&m.type==='leave')remove(s,true);
else if(r&&m.type==='pose'&&[m.x,m.y,m.z,m.yaw,m.hp].every(Number.isFinite)&&Math.abs(m.x)<101&&Math.abs(m.z)<101&&m.y>=0&&m.y<30){r[s.role].pose=m;send(other(r,s.role)?.socket,m);}
else if(r&&m.type==='state'&&s.role==='host'&&Array.isArray(m.enemies)&&m.enemies.length<=1000){r.snapshot=m;send(r.guest?.socket,m);}
else if(r&&m.type==='shot'&&s.role==='guest'&&r.guest.pose?.hp>0&&Array.isArray(m.direction)&&m.direction.length===3&&m.direction.every(Number.isFinite)){const damage=WEAPON_BY_ID[m.weapon]?.damage,p=r.guest.pose;if(damage)send(r.host.socket,{type:'shot',origin:[p.x,p.y+1.65,p.z],direction:m.direction,damage});}
else if(r&&m.type==='damage'&&s.role==='host')send(r.guest?.socket,{type:'damage',amount:[15,25,30,40].includes(m.amount)?m.amount:25});
else if(r&&m.type==='end'&&s.role==='host'){r.result=m.result;send(r.guest?.socket,m);}
else if(r&&m.type==='pickup'&&s.role==='guest')send(r.host.socket,{type:'pickup',id:m.id});
else if(r&&m.type==='grant'&&s.role==='host')send(r.guest?.socket,m);
else if(r&&m.type==='revive'){const own=r[s.role],peer=other(r,s.role);if(own.pose?.hp>0&&peer?.pose?.hp<=0&&Math.hypot(own.pose.x-peer.pose.x,own.pose.z-peer.pose.z)<2.5){peer.pose.hp=50;send(peer.socket,{type:'revived',hp:50});send(own.socket,{type:'revive-success'});}}

}catch{send(s,{type:'error',message:'Invalid packet'});}});s.on('close',()=>remove(s));});return {server,wss,rooms};}
if(process.argv[1]===resolve('server/index.js')){const {server}=createGameServer();server.listen(Number(process.env.PORT)||3000,'0.0.0.0',()=>console.log('DEAD SECTOR server on port '+(process.env.PORT||3000)));}
