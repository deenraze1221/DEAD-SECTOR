// Original synthesized effects: no external files or downloads required.
export class GameAudio {
  unlock() {
    try { this.context ??= new AudioContext(); this.context.resume().catch(()=>{}); } catch { /* Audio is optional. */ }
  }
  music(mood){
    const c=this.context;if(!c||c.state!=='running')return;
    if(!this.stems){this.stems={};for(const name of ['calm','tense']){const bus=c.createGain();bus.gain.value=0;bus.connect(c.destination);const voices=[];for(let n=0;n<3;n++){const oscillator=c.createOscillator(),gain=c.createGain(),filter=c.createBiquadFilter();oscillator.type=name==='calm'?'sine':'triangle';gain.gain.value=.012;filter.frequency.value=name==='calm'?650:430;oscillator.connect(filter);filter.connect(gain);gain.connect(bus);oscillator.start();voices.push(oscillator);}this.stems[name]={bus,voices};}}
    if(this.musicMood!==mood){this.musicMood=mood;for(const [name,stem]of Object.entries(this.stems)){const g=stem.bus.gain;g.cancelScheduledValues(c.currentTime);g.setValueAtTime(g.value,c.currentTime);g.linearRampToValueAtTime(name===mood?1:0,c.currentTime+2.5);}}
    const bar=Math.floor(c.currentTime/8),beat=Math.floor(c.currentTime*2);
    if(this.musicBar!==bar){this.musicBar=bar;const chords=[[130.81,164.81,196],[110,130.81,164.81],[146.83,174.61,220],[98,130.81,164.81]];this.stems.calm.voices.forEach((o,i)=>o.frequency.setTargetAtTime(chords[bar%4][i],c.currentTime,.5));}
    if(this.musicBeat!==beat){this.musicBeat=beat;this.stems.tense.voices.forEach((o,i)=>o.frequency.setTargetAtTime([73.42,77.78,110][i]*(beat%8===7?1.059:1),c.currentTime,.05));const g=this.stems.tense.bus.gain;if(mood==='tense'&&this.musicMood===mood){/* Melody changes without restarting the crossfade. */}}
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
    if(kind==='wave'){const now=c.currentTime;for(let n=0;n<3;n++){const o=c.createOscillator(),g=c.createGain();o.type='triangle';o.frequency.value=110+n*35;g.gain.setValueAtTime(.001,now+n*.16);g.gain.linearRampToValueAtTime(.09,now+n*.16+.04);g.gain.exponentialRampToValueAtTime(.001,now+n*.16+.3);o.connect(g);g.connect(c.destination);o.start(now+n*.16);o.stop(now+n*.16+.32);}return;}
    if(kind==='cash'||kind==='eye'){const now=c.currentTime;for(let n=0;n<(kind==='cash'?3:2);n++){const o=c.createOscillator(),g=c.createGain(),start=now+n*.07;o.type='sine';o.frequency.setValueAtTime(kind==='cash'?700+n*230:430-n*120,start);g.gain.setValueAtTime(.045*Math.max(.05,volume),start);g.gain.exponentialRampToValueAtTime(.001,start+.14);o.connect(g);g.connect(c.destination);o.start(start);o.stop(start+.15);}return;}
    const duration=kind==='stomp'?.3:kind==='shot'?.16:kind==='hurt'?.22:kind==='growl'?.35:kind==='step'?.07:.09;
    const gain=c.createGain();gain.connect(c.destination);gain.gain.setValueAtTime((kind==='shot'?.15:.07)*Math.max(.01,Math.min(1,volume)),c.currentTime);gain.gain.exponentialRampToValueAtTime(.001,c.currentTime+duration);
    if(kind==='brutegrowl'||kind==='snarl'||kind==='stomp'||kind==='shot'||kind==='hurt'||kind==='growl'||kind==='step') {
      const buffer=c.createBuffer(1,Math.ceil(c.sampleRate*duration),c.sampleRate),data=buffer.getChannelData(0);
      for(let i=0;i<data.length;i++)data[i]=Math.random()*2-1;
      const source=c.createBufferSource();source.buffer=buffer;const filter=c.createBiquadFilter();filter.type='lowpass';filter.frequency.value=kind==='brutegrowl'?95:kind==='snarl'?350:kind==='stomp'?85:kind==='shot'?2200:kind==='growl'?180:kind==='step'?650:400;source.connect(filter);filter.connect(gain);source.start();source.stop(c.currentTime+duration);
    } else {
      const oscillator=c.createOscillator();oscillator.type='triangle';oscillator.frequency.setValueAtTime(kind==='reload'?700:kind==='hit'?180:95,c.currentTime);oscillator.frequency.exponentialRampToValueAtTime(50,c.currentTime+duration);oscillator.connect(gain);oscillator.start();oscillator.stop(c.currentTime+duration);
    }
  }
}
