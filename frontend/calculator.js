'use strict';
// Calculator flow adapted from RepairCalc_demo.html, isolated from storefront state.

(()=>{
const root=document.getElementById('rc-full'),main=root.querySelector('#rc-main'),aside=root.querySelector('#rc-aside');
const fmt=n=>Number(n.toFixed(2)).toLocaleString('ru-RU'),rub=n=>n.toLocaleString('ru-RU',{maximumFractionDigits:2})+' ₽';
const esc=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
// Материалы и коэффициенты ниже — только демонстрационные данные.
const W={};
function w(id,title,name,unit,pack,price,rate=1){W[id]={id,title,name,unit,pack,price,rate};}
w('plastering','Оштукатуривание стен и потолков','Штукатурка «Ровная стена»','кг',25,650,8.5);
w('interior_painting','Подготовка и окрашивание стен и потолков','Краска «Интерьер»','л',5,2800,.2);
w('wallpapering','Подготовка и оклеивание обоями','Обои «Лён»','м²',5,1400,1.1);
w('wall_tiling','Облицовка стен плиткой','Плитка «Светлый камень»','м²',1.2,1680);
w('wall_waterproofing','Гидроизоляция стен влажных зон','Гидроизоляция «Мокрая зона»','кг',5,1900,1.4);
w('decorative_finishing','Декоративная штукатурная отделка','Штукатурка «Фактура»','кг',15,4500,1.5);
w('wall_paneling','Облицовка стен декоративными панелями','Панель «Дуб»','м²',2,3200,1.1);
w('drywall_partitions','Устройство перегородок из ГКЛ','Лист ГКЛ «Система»','м²',3,650,2.2);
w('drywall_wall_cladding','Обшивка существующих стен ГКЛ','Лист ГКЛ «Система»','м²',3,650,1.1);
w('drywall_ceiling','Устройство потолка из ГКЛ','Лист ГКЛ «Система»','м²',3,650,1.1);
w('suspended_ceiling','Другие подвесные потолки','Панели потолочные «Модуль»','м²',2.88,2600,1.1);
w('masonry_partitions','Перегородки из кирпича, блоков и плит','Пазогребневая плита «Стена»','м²',.333,450,1.05);
w('floor_screed','Устройство стяжки','Смесь «Стяжка»','кг',25,450,18);
w('floor_leveling','Выравнивание пола ровнителями','Ровнитель «Основа»','кг',20,760,1.6);
w('floor_waterproofing','Гидроизоляция пола','Гидроизоляция «Мокрая зона»','кг',5,1900,1.4);
w('floor_tiling','Укладка напольной плитки и керамогранита','Керамогранит «Песчаник»','м²',1.2,1980);
w('laminate_flooring','Укладка ламината','Ламинат «Светлый дуб»','м²',2.2,2400);
w('parquet_flooring','Укладка паркета и паркетной доски','Паркетная доска «Натур»','м²',2,6400);
w('roll_flooring','Настил линолеума и ковролина','Линолеум «Текстура»','м²',1,900);
w('vinyl_flooring','Укладка виниловых и кварцвиниловых покрытий','Кварцвинил «Дуб серый»','м²',2.2,3900);
w('floor_skirting','Монтаж напольных плинтусов','Плинтус «Профиль»','м',2.5,450);
w('electrical_wiring','Прокладка внутренней электропроводки','Кабель «Монтаж»','м',10,1400,1.1);
w('electrical_devices','Установка розеток и выключателей','Розетка «Линия»','шт.',1,650);
w('lighting_installation','Монтаж стационарного освещения','Светильник «Круг»','шт.',1,2400);
w('electrical_panel_installation','Монтаж квартирного электрощита','Корпус щита «Модуль»','шт.',1,4200);
w('water_supply_piping','Разводка внутреннего водоснабжения','Труба «Вода»','м',2,300,1.1);
w('sewer_piping','Разводка внутренней канализации','Труба «Слив»','м',2,650,1.1);
w('sanitary_installation','Установка сантехнических приборов','Раковина «Компакт»','шт.',1,5400);
w('faucet_installation','Установка смесителей и душевых систем','Смеситель «Поток»','шт.',1,3900);
w('interior_door_installation','Установка межкомнатных дверей','Дверной блок «Светлый»','шт.',1,12500);
const TYPES={bathroom:'Санузел',kitchen:'Кухня',living:'Комната',hallway:'Прихожая',balcony:'Лоджия'};
let serial=1;
function room(type='bathroom'){return{id:serial++,type,l:type==='bathroom'?3:4,b:type==='bathroom'?2:3,h:2.7,openings:2,wet:type==='bathroom',load:'normal',floor:'',wall:'',ceiling:'',partition:'',partitionArea:6,plaster:false,cladding:false,wallHydro:false,wetArea:8,floorHydro:false,screed:false,level:false,skirting:false,thickness:30,levelThickness:5,doors:0,wire:0,devices:0,lights:0,panel:0,water:0,sewer:0,fixtures:0,faucets:0};}
const state={step:0,object:'Квартира',repair:'Капитальный',rooms:[room()],ri:0,section:'Пол',reserve:.1,variants:{},modal:null,complete:false,filter:'all',error:''};
const current=()=>state.rooms[state.ri];
const geom=r=>({floor:r.l*r.b,wall:Math.max(0,2*(r.l+r.b)*r.h-r.openings),perim:2*(r.l+r.b)});
function selections(r){const g=geom(r),a=[];const add=(id,base,why)=>{if(id&&base>0)a.push({id,base,why,room:r.id,roomName:TYPES[r.type]});};
add(r.floor,g.floor,'Покрытие пола');add(r.wall,g.wall,'Отделка стен');add(r.ceiling,g.floor,'Потолок');
if(r.plaster)add('plastering',g.wall,'Выравнивание стен');if(r.cladding)add('drywall_wall_cladding',g.wall,'Обшивка стен');if(r.wallHydro)add('wall_waterproofing',Math.min(r.wetArea,g.wall),'Выделенная влажная зона стен');
if(r.floorHydro)add('floor_waterproofing',g.floor,'Новая гидроизоляция пола');if(r.screed)add('floor_screed',g.floor*r.thickness/10,'Стяжка '+r.thickness+' мм');if(r.level)add('floor_leveling',g.floor*r.levelThickness,'Ровнитель '+r.levelThickness+' мм');if(r.skirting)add('floor_skirting',Math.max(0,g.perim-.8),'Периметр за вычетом проёма 0,8 м');add(r.partition,r.partitionArea,'Новая перегородка');
[['doors','interior_door_installation'],['wire','electrical_wiring'],['devices','electrical_devices'],['lights','lighting_installation'],['panel','electrical_panel_installation'],['water','water_supply_piping'],['sewer','sewer_piping'],['fixtures','sanitary_installation'],['faucets','faucet_installation']].forEach(([k,id])=>add(id,r[k],k==='fixtures'?'Раковины в примере':'Количество задано пользователем'));return a;}
function allsel(){return state.rooms.flatMap(selections);}
const finishing=['wall_tiling','floor_tiling','laminate_flooring','parquet_flooring','roll_flooring','vinyl_flooring'];
function materials(){let map=new Map();function add(s,key,name,unit,pack,price,qty,why,kind='main'){
if(!qty)return;let x=map.get(key);if(!x){x={key,name,unit,pack,price,qty:0,works:new Set(),rooms:new Set(),parts:[],kind};map.set(key,x);}x.qty+=qty;x.works.add(s.id);x.rooms.add(s.room);x.parts.push(TYPES[state.rooms.find(r=>r.id===s.room).type]+': '+why+' → '+fmt(qty)+' '+unit);}
for(const s of allsel()){const t=W[s.id],factor=finishing.includes(s.id)?1+state.reserve:t.rate;const q=s.base*factor;
add(s,t.name,t.name,t.unit,t.pack,t.price,q,fmt(s.base)+' × '+fmt(factor));
if(['floor_tiling','wall_tiling'].includes(s.id)){add(s,'tile_glue','Плиточный клей «База»','кг',25,1100,s.base*4,'площадь × 4 кг/м²','aux');add(s,'grout','Затирка «Шов»','кг',2,790,s.base*.3,'площадь × 0,3 кг/м²','aux');}
if(s.id==='interior_painting')add(s,'primer_paint','Грунтовка под окраску','л',5,900,s.base*.15,'площадь × 0,15 л/м²','aux');
if(s.id==='wallpapering')add(s,'paper_glue','Клей для обоев','г',250,480,s.base/20*250,'250 г на 20 м²','aux');
if(['laminate_flooring','parquet_flooring'].includes(s.id))add(s,'underlay','Подложка для выбранного покрытия','м²',10,1200,s.base*1.05,'площадь × 1,05','aux');
}
return [...map.values()].map(x=>{const alt=state.variants[x.key]===1;const count=Math.ceil(Number((x.qty/x.pack).toPrecision(14))),price=alt?Math.round(x.price*1.3):x.price;return{...x,alt,count,price,total:count*price,name:x.name+(alt?' · вариант Плюс':'')};});}
const total=()=>materials().reduce((n,x)=>n+x.total,0);
function btn(label,act,extra='',cls=''){return `<button type="button" class="${cls}" data-action="${act}" ${extra}>${label}</button>`;}
function field(label,key,value,type='number',min=0,max=100,step=1){return `<label class="field"><span>${label}</span><input data-roomfield="${key}" aria-label="${label}" type="${type}" value="${esc(value)}" min="${min}" max="${max}" step="${step}" required></label>`;}
function select(label,key,val,options){return `<label class="field"><span>${label}</span><select data-roomfield="${key}" aria-label="${label}">${options.map(([v,l])=>`<option value="${v}" ${String(val)===String(v)?'selected':''}>${l}</option>`).join('')}</select></label>`;}
function check(label,key){return `<label class="check"><input type="checkbox" data-roomfield="${key}" ${current()[key]?'checked':''}>${label}</label>`;}
function choose(key,values){return `<div class="grid">${values.map(([v,desc])=>btn(`<strong>${v}</strong><span class="muted">${desc}</span>`,'project',`data-key="${key}" data-value="${v}" aria-pressed="${state[key]===v}"`,'choice')).join('')}</div>`;}
function steps(){root.querySelector('#rc-steps').innerHTML=['Объект','Комнаты','Работы','Смета'].map((x,i)=>btn(`${i+1}. ${x}`,'step',`data-value="${i}" ${i===state.step?'aria-current="step"':''}`,i===state.step?'current':'')).join('');root.querySelector('#rc-progress').style.width=((state.step+1)*25)+'%';}
function nav(){return `<div class="actions">${state.step?btn('← Назад','back'):''}${state.step<3?btn(state.step===2?'Подобрать материалы →':'Продолжить →','next','','primary'):''}</div>`;}
function roomtabs(){return `<div class="wrap">${state.rooms.map((r,i)=>btn(TYPES[r.type]+' · '+fmt(geom(r).floor)+' м²','room',`data-value="${i}" aria-pressed="${state.ri===i}"`)).join('')}</div>`;}
function project(){return `<h2>С чего начнём ремонт?</h2><p class="muted">Сначала объект, затем комнаты и решения для каждой из них.</p><section class="panel"><h3>Ваш объект</h3>${choose('object',[['Квартира','Ремонт помещений'],['Дом','Внутренний ремонт'],['Студия','Общее пространство']])}</section><section class="panel"><h3>Вариант ремонта</h3>${choose('repair',[['Косметический','Обновить отделку'],['Капитальный','Изменить отделку и системы'],['Дизайнерский','Индивидуальные решения']])}<p class="muted" style="margin:14px 0 0">Это стартовый ориентир. Конкретные работы вы выбираете сами.</p></section>${btn('Заполнить пример: санузел и комната','example')} ${nav()}`;}
function rooms(){const r=current(),g=geom(r);return `<h2>Помещения и размеры</h2><p class="muted">Площади считаются для прямоугольной комнаты.</p>${roomtabs()}<section class="panel">${select('Назначение комнаты','type',r.type,Object.entries(TYPES))}<div class="grid">${field('Длина, м','l',r.l,'number',.5,30,.1)}${field('Ширина, м','b',r.b,'number',.5,30,.1)}${field('Высота, м','h',r.h,'number',1.5,6,.1)}</div>${field('Площадь окон и дверей, м²','openings',r.openings,'number',0,200,.1)}<div class="notice">Пол ${fmt(g.floor)} м² · стены ${fmt(g.wall)} м² · потолок ${fmt(g.floor)} м²</div>${check('Есть зоны прямого попадания воды','wet')}${select('Нагрузка на пол','load',r.load,[['normal','Обычная'],['high','Высокая проходимость']])}<div class="muted">Условия предложены по назначению. Их можно исправить.</div></section><div class="wrap">${btn('+ Добавить комнату','add')}${state.rooms.length>1?btn('Удалить эту комнату','remove'):''}</div>${nav()}`;}
const sections=['Пол','Стены','Потолок','Перегородки и двери','Электрика и свет','Сантехника'];
function workform(){const r=current(),g=geom(r),keep=['','Оставить как есть'];let html='';
if(state.section==='Пол'){const forbidden=r.wet;let opts=[keep,['floor_tiling','Плитка или керамогранит'],['laminate_flooring','Ламинат'],['parquet_flooring','Паркетная доска'],['roll_flooring','Линолеум / ковролин'],['vinyl_flooring','Винил / кварцвинил']];html=select('Покрытие пола','floor',r.floor,opts)+`<p class="muted">Площадь ${fmt(g.floor)} м². ${forbidden?'Для мокрой зоны подбор ламината, паркета и рулонных покрытий в демо недоступен.':''}</p>`+check('Новая гидроизоляция пола','floorHydro')+check('Новая стяжка','screed')+(r.screed?field('Толщина стяжки, мм','thickness',r.thickness,'number',10,100,1):'')+check('Выравнивание ровнителем','level')+(r.level?field('Толщина ровнителя, мм','levelThickness',r.levelThickness,'number',1,30,1):'')+check('Добавить напольные плинтусы','skirting');}
if(state.section==='Стены'){html=select('Финишная отделка стен','wall',r.wall,[keep,['interior_painting','Покраска'],['wallpapering','Обои'],['wall_tiling','Плитка'],['decorative_finishing','Декоративная штукатурка'],['wall_paneling','Деревянные панели']])+`<p class="muted">В этом примере отделываем все стены: ${fmt(g.wall)} м².</p>`+check('Выровнять штукатуркой','plaster')+check('Обшить существующие стены ГКЛ','cladding')+check('Гидроизоляция выбранной зоны стен','wallHydro')+(r.wallHydro?field('Площадь гидроизоляции стен, м²','wetArea',r.wetArea,'number',.1,g.wall,.1):'');}
if(state.section==='Потолок')html=select('Решение для потолка','ceiling',r.ceiling,[keep,['interior_painting','Покраска'],['wallpapering','Обои'],['drywall_ceiling','Потолок из ГКЛ'],['suspended_ceiling','Модульный подвесной потолок']])+`<p class="muted">Площадь ${fmt(g.floor)} м². Натяжные потолки пока вне нашего набора.</p>`;
if(state.section==='Перегородки и двери')html=select('Новая перегородка','partition',r.partition,[['','Не нужна'],['drywall_partitions','Каркасная из ГКЛ'],['masonry_partitions','Из блоков или плит']])+(r.partition?field('Площадь перегородки с одной стороны, м²','partitionArea',r.partitionArea,'number',1,100,.5):'')+field('Межкомнатные двери, шт.','doors',r.doors,'number',0,15,1);
if(state.section==='Электрика и свет')html=`<div class="notice">Вводим известные объёмы. Калькулятор не проектирует электросеть по площади комнаты.</div><div class="two">${field('Длина проводки, м','wire',r.wire)}${field('Розетки / выключатели, шт.','devices',r.devices)}${field('Светильники, шт.','lights',r.lights)}${field('Корпус квартирного щита, шт.','panel',r.panel,'number',0,2)}</div><p class="muted">В демо считаются выбранные изделия. Защитная автоматика, сечения и схема требуют проекта.</p>`;
if(state.section==='Сантехника')html=`<div class="two">${field('Водопроводная труба, м','water',r.water)}${field('Канализационная труба, м','sewer',r.sewer)}${field('Раковины, шт.','fixtures',r.fixtures,'number',0,10)}${field('Смесители, шт.','faucets',r.faucets,'number',0,10)}</div><p class="muted">Пример класса сантехнических приборов — раковина. Фитинги и подключение уточняются по схеме.</p>`;
return html;}
function works(){return `<h2>Что меняем в каждой комнате?</h2><p class="muted">Откройте раздел и выберите нужные работы.</p>${roomtabs()}<div class="tabs">${sections.map(x=>btn(x,'section',`data-value="${x}" aria-pressed="${state.section===x}"`)).join('')}</div><section class="panel"><h3>${state.section}</h3>${workform()}</section><label class="field"><span>Запас финишного покрытия</span><select id="rc-reserve">${[.05,.1,.15].map(x=>`<option value="${x}" ${state.reserve===x?'selected':''}>${x*100}%</option>`).join('')}</select></label><details><summary>Что добавится к выбранным работам</summary><p class="muted">К плитке — клей и затирка; к окраске — грунтовка; к обоям — клей; к ламинату и паркету — подложка. Это упрощённые рецепты примера, без проверки реальной технологии.</p></details>${nav()}`;}
function productModal(){if(!state.modal)return '';const x=materials().find(x=>x.key===state.modal);if(!x)return '';return `<section class="modalzone"><div class="row" style="padding-top:0"><h3>Карточка материала</h3>${btn('Закрыть','close')}</div><h3 style="margin-top:16px">${esc(x.name)}</h3><p>${fmt(x.pack)} ${x.unit} в упаковке · ${rub(x.price)}</p><div class="two"><label class="field"><span>Нужное количество, ${x.unit}</span><input id="rc-product-qty" type="number" min="0" step="0.1" value="${Number(x.qty.toFixed(2))}"></label><div><div class="muted">К покупке</div><div id="rc-product-total" class="smalltotal">${x.count} уп. · ${rub(x.total)}</div></div></div><div class="muted">Отдельный расчёт на карточке, без изменения сметы.</div><details open><summary>Расчёт в проекте</summary>${x.parts.map(t=>`<div class="workline muted">${esc(t)}</div>`).join('')}<div class="workline">Итого ${fmt(x.qty)} ${x.unit} → ${x.count} уп.</div></details><details><summary>Почему этот материал</summary><p class="muted">Демонстрационная связь с работами: ${[...x.works].map(id=>W[id].title).join('; ')}. В рабочем сервисе нужны подтверждённые характеристики и проверка совместимости.</p></details></section>`;}
function estimate(){const items=materials(),display=state.filter==='all'?items:items.filter(x=>x.rooms.has(Number(state.filter)));return `<h2>${state.complete?'Список покупок готов':'Ваша смета материалов'}</h2><p class="muted">${state.rooms.length} помещ. · ${items.length} позиций · труд и доставка не включены</p>${state.complete?'<div class="success">✓ Список собран в этом примере. Ничего не заказано и не отправлено в магазин.</div>':''}<div class="notice">Для отделки показан упрощённый комплект. Конструкции и инженерные работы содержат только основные изделия; это не полная строительная смета.</div><div class="wrap resultnav">${btn('Все комнаты','filter','data-value="all" aria-pressed="'+(state.filter==='all')+'"')}${state.rooms.map(r=>btn(TYPES[r.type],'filter',`data-value="${r.id}" aria-pressed="${String(r.id)===state.filter}"`)).join('')}</div>${state.filter!=='all'?'<p class="muted">Показаны общие позиции проекта, используемые в этой комнате. Количество упаковок учитывает все комнаты.</p>':''}${productModal()}<section class="panel">${display.length?display.map((x)=>`<div class="sku"><div><span class="swatch">${x.kind==='aux'?'+':'▦'}</span><strong style="font-weight:500">${esc(x.name)}</strong><div class="muted">${x.count} уп. × ${fmt(x.pack)} ${x.unit} · нужно ${fmt(x.qty)} ${x.unit}</div>${btn('Карточка и расчёт','product',`data-key="${esc(x.key)}"`)} ${btn(x.alt?'Вернуть базовый':'Заменить на Плюс','variant',`data-key="${esc(x.key)}"`)}</div><div class="price">${rub(x.total)}<div class="muted">${rub(x.price)} / уп.</div></div></div>`).join(''):'Нет материалов: выберите работы на предыдущем шаге.'}</section><div class="actions">${btn('← Изменить работы','back')}${items.length?btn(state.complete?'Список обновлён ✓':'Собрать список покупок','finish','','primary'):''}</div>`;}
function sidebar(){const s=allsel(),n=new Set(s.map(x=>x.id)).size;aside.innerHTML=`<section class="panel"><div class="muted">ВАШ ПРОЕКТ</div><h3 style="margin-top:8px">${state.object}</h3><div class="muted">${state.repair} ремонт</div>${state.rooms.map(r=>`<div class="row"><span>${TYPES[r.type]}</span><span>${fmt(geom(r).floor)} м²</span></div>`).join('')}<div class="muted" style="margin-top:16px">${n?'Предварительная стоимость материалов':'Выберите работы для оценки'}</div><div class="total" aria-live="polite">${n?rub(total()):'—'}</div><div class="muted">Условные товары и цены.<br>Объём комплекта зависит от сценария.</div></section>`;}
function render(){steps();main.innerHTML=[project,rooms,works,estimate][state.step]();if(state.error)main.insertAdjacentHTML('afterbegin',`<p class="error" role="alert">${esc(state.error)}</p>`);sidebar();if(state.step===2&&current().wet){const select=main.querySelector('[data-roomfield="floor"]');if(select)for(const op of select.options)if(['laminate_flooring','parquet_flooring','roll_flooring'].includes(op.value)){op.disabled=true;op.textContent+=' — нет подтверждённого варианта';}}}
function valid(){const invalid=main.querySelector('input:invalid');if(invalid){state.error='Проверьте введённые значения и допустимые диапазоны.';invalid.reportValidity();return false;}for(const r of state.rooms){if(!(r.l>=.5&&r.l<=30&&r.b>=.5&&r.b<=30&&r.h>=1.5&&r.h<=6)){state.error='Проверьте размеры: длина и ширина 0,5–30 м, высота 1,5–6 м.';return false;}if(r.openings>2*(r.l+r.b)*r.h){state.error='Площадь проёмов больше площади стен. Исправьте размеры.';return false;}}state.error='';return true;}
function sanitize(r){if(r.wet&&['laminate_flooring','parquet_flooring','roll_flooring'].includes(r.floor)){r.floor='';state.error='Покрытие пола сброшено: для мокрой зоны в демо нет подтверждённого варианта. Выберите плитку или винил.';}r.wetArea=Math.min(r.wetArea,geom(r).wall);}
root.addEventListener('click',e=>{const b=e.target.closest('button[data-action]');if(!b||!root.contains(b))return;const a=b.dataset.action,v=b.dataset.value;state.error='';
if(a==='project')state[b.dataset.key]=v;
if(a==='example'){state.rooms=[room('bathroom'),room('living')];Object.assign(state.rooms[0],{floor:'floor_tiling',wall:'wall_tiling',floorHydro:true,wallHydro:true,wetArea:8,ceiling:'interior_painting',faucets:1});Object.assign(state.rooms[1],{floor:'laminate_flooring',wall:'wallpapering',skirting:true,ceiling:'interior_painting'});state.ri=0;state.step=1;state.complete=false;}
if(a==='next'||a==='step'){const dest=a==='next'?state.step+1:Number(v);if(dest>state.step&&!valid()){render();return;}state.step=Math.max(0,Math.min(3,dest));state.modal=null;}
if(a==='back'){state.step=Math.max(0,state.step-1);state.modal=null;}
if(a==='room'){state.ri=Number(v);}
if(a==='add'){state.rooms.push(room('living'));state.ri=state.rooms.length-1;}
if(a==='remove'&&state.rooms.length>1){state.rooms.splice(state.ri,1);state.ri=0;state.filter='all';state.complete=false;}
if(a==='section')state.section=v;
if(a==='product')state.modal=b.dataset.key;
if(a==='close')state.modal=null;
if(a==='variant'){state.variants[b.dataset.key]=state.variants[b.dataset.key]===1?0:1;state.complete=false;}
if(a==='finish')state.complete=true;
if(a==='filter')state.filter=v;
render();if(['next','back','step','example'].includes(a)){main.focus({preventScroll:true});root.scrollIntoView({behavior:'smooth',block:'start'});}});
root.addEventListener('change',e=>{const t=e.target;if(t.dataset.roomfield){const r=current(),k=t.dataset.roomfield;if(t.type==='number'&&!t.checkValidity()){state.error='Введите число в указанном диапазоне.';t.reportValidity();return;}r[k]=t.type==='checkbox'?t.checked:t.type==='number'?Number(t.value):t.value;if(k==='type')r.wet=r.type==='bathroom';if(k==='cladding'&&r.cladding)r.plaster=false;if(k==='plaster'&&r.plaster)r.cladding=false;state.error='';sanitize(r);state.complete=false;render();}if(t.id==='rc-reserve'){state.reserve=Number(t.value);state.complete=false;render();}});
root.addEventListener('input',e=>{if(e.target.id==='rc-product-qty'){const x=materials().find(x=>x.key===state.modal);const qty=Number(e.target.value);root.querySelector('#rc-product-total').textContent=Number.isFinite(qty)&&qty>=0?Math.ceil(qty/x.pack)+' уп. · '+rub(Math.ceil(qty/x.pack)*x.price):'Введите количество от 0';}});
render();
})();


(() => {
  const store = document.getElementById('storefront');
  const page = document.getElementById('calculator-page');
  const heading = document.getElementById('calculator-title');
  const title = document.title;
  function display(active, focus = false) {
    store.hidden = active;
    page.hidden = !active;
    document.title = active ? 'Калькулятор ремонта — Максидом' : title;
    document.querySelectorAll('[data-open-calculator]').forEach(button => {
      if (active) button.setAttribute('aria-current', 'page');
      else button.removeAttribute('aria-current');
    });
    if (focus) {
      (active ? heading : store).focus({ preventScroll: true });
      window.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }
  function open() {
    closeCatalog();
    if (location.hash !== '#calculator') location.hash = 'calculator';
    display(true, true);
  }
  function close() {
    if (location.hash === '#calculator') history.replaceState(null, '', location.pathname + location.search);
    display(false);
  }
  window.RepairCalculator = { open, close };
  document.addEventListener('click', event => {
    if (event.target.closest('[data-open-calculator]')) open();
    if (event.target.closest('[data-close-calculator]')) { close(); store.focus({preventScroll:true}); window.scrollTo({top:0,behavior:'smooth'}); }
  });
  window.addEventListener('hashchange', () => display(location.hash === '#calculator', true));
  display(location.hash === '#calculator');
})();
