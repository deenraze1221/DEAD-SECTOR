import {SHOP_ITEMS,totalAmmo,AMMO_LIMIT} from './economy.js';
import {WEAPONS} from './weapon-config.js';
import {weaponSilhouette} from './weapon-icons.js';
import {t} from './i18n.js';
export class ShopUI{
 constructor(onAction,onClose){this.root=document.getElementById('shop-panel');this.status=document.getElementById('shop-status');this.pending=false;this.tab='weapon';document.getElementById('shop-tabs').addEventListener('click',e=>{const b=e.target.closest('[data-shop-tab]');if(b){this.tab=b.dataset.shopTab;this.render(this.account);}});this.root.addEventListener('click',e=>{const b=e.target.closest('[data-shop-action]');if(b&&!b.disabled)onAction(b.dataset.shopAction,b.dataset.item);});document.getElementById('close-shop').addEventListener('click',onClose);}
 show(account){this.root.hidden=false;this.render(account);document.getElementById('close-shop').focus();}
 hide(){this.root.hidden=true;this.pending=false;}
 message(text){this.status.textContent=t(text);}
 render(a){
  this.account=a;for(const b of document.querySelectorAll('[data-shop-tab]')){b.setAttribute('aria-selected',String(b.dataset.shopTab===this.tab));b.textContent=t(b.dataset.shopTab==='weapon'?'ОРУЖИЕ':'СНАРЯЖЕНИЕ');}
  document.getElementById('shop-title').textContent=t('ТЕРМИНАЛ · SECTOR MART');
  document.getElementById('shop-balance').textContent=`$${a.money} · ${t('ПАТРОНЫ')}: ${totalAmmo(a)}/${AMMO_LIMIT}`;
  document.getElementById('shop-owned').replaceChildren(...WEAPONS.filter(w=>a.weapons[w.id]).sort((a,b)=>Number(a.id==='Pistol')-Number(b.id==='Pistol')).map(w=>{const b=document.createElement('button');b.type='button';b.dataset.shopAction='equip';b.dataset.item=w.id;b.textContent=`${w.id==='Pistol'?'2':'1'} · ${t(w.label)} · ${a.weapons[w.id].durability}%`;b.disabled=this.pending||a.equipped===w.id;b.setAttribute('aria-pressed',String(a.equipped===w.id));return b;}));
  document.getElementById('shop-products').replaceChildren(...SHOP_ITEMS.filter(i=>this.tab==='weapon'?i.kind==='weapon':i.kind!=='weapon').sort((a,b)=>a.price-b.price).map(item=>{
   const card=document.createElement('article'),heading=document.createElement('h3'),detail=document.createElement('p'),button=document.createElement('button');
   const spec=WEAPONS.find(w=>w.id===item.id);heading.textContent=t(item.name);detail.textContent=spec?`${spec.capacity} ${t('патронов')} · ${spec.damage} ${t('урон')} · ${Math.round(60/spec.interval)} ${t('выстр/мин')} · ${spec.reload}s ${t('перезарядка')}`:item.id==='ammo'?t('Общий запас до 500, включая магазины'):item.id==='repair'?`${t('Прочность')}: ${a.weapons[a.equipped].durability}/100`:t('При ранении лечит сразу; при полном HP — в запас (H/J)');
   button.type='button';button.dataset.shopAction='buy';button.dataset.item=item.id;button.textContent=a.weapons[item.id]?t('КУПЛЕНО'):`${t('КУПИТЬ')} · $${item.price}`;
   button.disabled=this.pending||a.money<item.price||Boolean(a.weapons[item.id])||(item.id==='ammo'&&totalAmmo(a)+30>AMMO_LIMIT)||(item.id==='repair'&&a.weapons[a.equipped].durability>=100);
   if(spec){card.className='weapon-card';card.tabIndex=0;detail.className='weapon-stats';card.setAttribute('aria-label',t(item.name)+': '+detail.textContent);const icon=document.createElement('div');icon.innerHTML=weaponSilhouette(spec.id);card.append(heading,icon,detail,button);}else{const icon=document.createElement('div');icon.className='supply-silhouette';icon.innerHTML=supplyIcon(item.id);card.append(heading,icon,detail,button);}return card;
  }));
  document.getElementById('shop-medkits').textContent=`${t('АПТЕЧКИ')}: H · ${a.smallMedkit} (+25 HP) / J · ${a.largeMedkit} (+75 HP)`;
 }
}

function supplyIcon(id){const drawing=id==='ammo'?'<path d="M27 20l8-12 8 12v47H27zm23 0l8-12 8 12v47H50zm23 0l8-12 8 12v47H73z"/><path d="M24 56h68v15H24z"/>':id==='repair'?'<path d="M78 8a19 19 0 0 0-22 25L25 60a7 7 0 0 0 10 10l30-31A19 19 0 0 0 90 17L76 29l-10-9z"/>':'<path d="M25 22h66v48H25zM43 11h30v12H63v-5H53v5H43z"/><path class="medical-cross" d="M53 31h11v9h10v11H64v10H53V51H43V40h10z"/>';return `<svg viewBox="0 0 116 80" role="img" aria-label="${t(id==='ammo'?'ПАТРОНЫ':id==='repair'?'Прочность':'АПТЕЧКИ')}">${drawing}</svg>`;}
