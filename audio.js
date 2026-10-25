// Original synthesized effects: no external files or downloads required.
export class GameAudio {
  unlock() {
    try { this.context ??= new AudioContext(); this.context.resume().catch(()=>{}); } catch { /* Audio is optional. */ }
  }
  shot(id){
    const c=this.context;if(!c||c.state!=='running')return;
    const p={Pistol:[.12,1700,160,.12],Helsing:[.09,2100,180,.13],Tikhar:[.21,620,95,.17],Shambler:[.25,1100,70,.2],Shotgun:[.32,800,55,.23],Kalash:[.17,1550,110,.18],VSV:[.23,760,80,.2],Scorpion:[.075,3300,230,.12],Vector:[.08,2750,210,.13],G36:[.16,2050,125,.17]}[id];if(!p)return;
    const [duration,frequency,bass,volume]=p,t=c.currentTime,g=c.createGain();g.connect(c.destination);g.gain.setValueAtTime(volume,t);g.gain.exponentialRampToValueAtTime(.001,t+duration);
    const buffer=c.createBuffer(1,Math.ceil(c.sampleRate*duration),c.sampleRate);const data=buffer.getChannelData(0);for(let i=0;i<data.length;i++)data[i]=Math.random()*2-1;
    const noise=c.createBufferSource();noise.buffer=buffer;const filter=c.createBiquadFilter();filter.type='lowpass';filter.frequency.value=frequency;noise.connect(filter);filter.connect(g);noise.start();noise.stop(t+duration);
    const osc=c.createOscillator();osc.type=['Scorpion','Vector','Helsing'].includes(id)?'square':'triangle';osc.frequency.setValueAtTime(bass,t);osc.frequency.exponentialRampToValueAtTime(35,t+duration);osc.connect(g);osc.start();osc.stop(t+duration);
  }
  play(kind, volume=1) {
    const c=this.context;if(!c||c.state!=='running')return;
    const duration=kind==='shot'?.16:kind==='hurt'?.22:kind==='growl'?.35:kind==='step'?.07:.09;
    const gain=c.createGain();gain.connect(c.destination);gain.gain.setValueAtTime((kind==='shot'?.15:.07)*Math.max(.01,Math.min(1,volume)),c.currentTime);gain.gain.exponentialRampToValueAtTime(.001,c.currentTime+duration);
    if(kind==='shot'||kind==='hurt'||kind==='growl'||kind==='step') {
      const buffer=c.createBuffer(1,Math.ceil(c.sampleRate*duration),c.sampleRate),data=buffer.getChannelData(0);
      for(let i=0;i<data.length;i++)data[i]=Math.random()*2-1;
      const source=c.createBufferSource();source.buffer=buffer;const filter=c.createBiquadFilter();filter.type='lowpass';filter.frequency.value=kind==='shot'?2200:kind==='growl'?180:kind==='step'?650:400;source.connect(filter);filter.connect(gain);source.start();source.stop(c.currentTime+duration);
    } else {
      const oscillator=c.createOscillator();oscillator.type='triangle';oscillator.frequency.setValueAtTime(kind==='reload'?700:kind==='hit'?180:95,c.currentTime);oscillator.frequency.exponentialRampToValueAtTime(50,c.currentTime+duration);oscillator.connect(gain);oscillator.start();oscillator.stop(c.currentTime+duration);
    }
  }
}
