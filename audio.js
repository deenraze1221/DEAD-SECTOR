// Original synthesized effects: no external files or downloads required.
export class GameAudio {
  unlock() {
    try { this.context ??= new AudioContext(); this.context.resume().catch(()=>{}); } catch { /* Audio is optional. */ }
  }
  play(kind) {
    const c=this.context;if(!c||c.state!=='running')return;
    const duration=kind==='shot'?.16:kind==='hurt'?.22:.09;
    const gain=c.createGain();gain.connect(c.destination);gain.gain.setValueAtTime(kind==='shot'?.15:.07,c.currentTime);gain.gain.exponentialRampToValueAtTime(.001,c.currentTime+duration);
    if(kind==='shot'||kind==='hurt') {
      const buffer=c.createBuffer(1,Math.ceil(c.sampleRate*duration),c.sampleRate),data=buffer.getChannelData(0);
      for(let i=0;i<data.length;i++)data[i]=Math.random()*2-1;
      const source=c.createBufferSource();source.buffer=buffer;const filter=c.createBiquadFilter();filter.type='lowpass';filter.frequency.value=kind==='shot'?2200:400;source.connect(filter);filter.connect(gain);source.start();source.stop(c.currentTime+duration);
    } else {
      const oscillator=c.createOscillator();oscillator.type='triangle';oscillator.frequency.setValueAtTime(kind==='reload'?700:kind==='hit'?180:95,c.currentTime);oscillator.frequency.exponentialRampToValueAtTime(50,c.currentTime+duration);oscillator.connect(gain);oscillator.start();oscillator.stop(c.currentTime+duration);
    }
  }
}
