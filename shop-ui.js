import {SHOP_ITEMS,totalAmmo,AMMO_LIMIT} from './economy.js';
import {WEAPONS} from './weapon-config.js';
import {t} from './i18n.js';
export class ShopUI{
 constructor(onAction,onClose){this.root=document.getElementById('shop-panel');this.status=document.getElementById('shop-status');this.pending=false;this.root.addEventListener('click',e=>{const b=e.target.closest('[data-shop-action]');if(b&&!b.disabled)onAction(b.dataset.shopAction,b.dataset.item);});document.getElementById('close-shop').addEventListener('click',onClose);}
 show(account){this.root.hidden=false;this.render(account);document.getElementById('close-shop').focus();}
 hide(){this.root.hidden=true;this.pending=false;}
 message(text){this.status.textContent=t(text);}
 render(a){
  document.getElementById('shop-title').textContent=t('ТЕРМИНАЛ · SECTOR MART');
  document.getElementById('shop-balance').textContent=`$${a.money} · ${t('ПАТРОНЫ')}: ${totalAmmo(a)}/${AMMO_LIMIT}`;
  document.getElementById('shop-owned').replaceChildren(...WEAPONS.filter(w=>a.weapons[w.id]).sort((a,b)=>Number(a.id==='Pistol')-Number(b.id==='Pistol')).map(w=>{const b=document.createElement('button');b.type='button';b.dataset.shopAction='equip';b.dataset.item=w.id;b.textContent=`${w.id==='Pistol'?'2':'1'} · ${t(w.label)} · ${a.weapons[w.id].durability}%`;b.disabled=this.pending||a.equipped===w.id;b.setAttribute('aria-pressed',String(a.equipped===w.id));return b;}));
  document.getElementById('shop-products').replaceChildren(...SHOP_ITEMS.map(item=>{
   const card=document.createElement('article'),heading=document.createElement('h3'),detail=document.createElement('p'),button=document.createElement('button');
   const spec=WEAPONS.find(w=>w.id===item.id);heading.textContent=t(item.name);detail.textContent=spec?`${spec.capacity} ${t('патронов')} · ${spec.damage} ${t('урон')} · ${spec.reload}s ${t('перезарядка')}`:item.id==='ammo'?t('Общий запас до 500, включая магазины'):item.id==='repair'?`${t('Прочность')}: ${a.weapons[a.equipped].durability}/100`:t('Использовать: H — малая, J — большая');
   button.type='button';button.dataset.shopAction='buy';button.dataset.item=item.id;button.textContent=a.weapons[item.id]?t('КУПЛЕНО'):`${t('КУПИТЬ')} · $${item.price}`;
   button.disabled=this.pending||a.money<item.price||Boolean(a.weapons[item.id])||(item.id==='ammo'&&totalAmmo(a)+30>AMMO_LIMIT)||(item.id==='repair'&&a.weapons[a.equipped].durability>=100);
   card.append(heading,detail,button);return card;
  }));
  document.getElementById('shop-medkits').textContent=`${t('АПТЕЧКИ')}: H · ${a.smallMedkit} (+25 HP) / J · ${a.largeMedkit} (+75 HP)`;
 }
}
