export class Session{
 constructor(onMessage,onStatus){this.onMessage=onMessage;this.onStatus=onStatus;this.active=false;this.host=false;}
 connect(url,mode,seed,code){this.close();let socket;try{socket=new WebSocket(url);}catch{this.onStatus('Connection failed / Ошибка подключения');return;}this.socket=socket;this.onStatus('Connecting… / Подключение…');socket.onopen=()=>this.send({type:mode,seed,code});socket.onmessage=e=>{const m=JSON.parse(e.data);if(m.type==='room'){this.active=true;this.host=m.host;this.code=m.code;this.onStatus('Room / Комната: '+m.code);}if(m.type==='error')this.onStatus(m.message);if(m.type==='closed'){this.active=false;this.onStatus('Host disconnected / Хозяин отключился');}this.onMessage(m);};socket.onerror=()=>this.onStatus('Server unavailable / Сервер недоступен');socket.onclose=()=>{this.active=false;this.onMessage({type:'closed'});this.onStatus('Disconnected / Отключено');};}
 send(data){if(this.socket?.readyState===WebSocket.OPEN)this.socket.send(JSON.stringify(data));}
 close(){if(this.socket){this.socket.onclose=null;this.socket.close();}this.active=false;}
}
