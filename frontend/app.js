'use strict';

const categories = [
  { name: 'Стройматериалы', count: 'Для надёжной основы', image: 'materials' },
  { name: 'Инструменты', count: 'Для любых задач', image: 'drill' },
  { name: 'Сантехника', count: 'Комфорт в деталях', image: 'faucet' },
  { name: 'Краски', count: 'Добавьте цвет', image: 'paint' },
  { name: 'Освещение', count: 'В правильном свете', image: 'lamp' },
  { name: 'Для дома', count: 'Больше уюта', image: 'chair' },
];
const products = [
  { id: 1, name: 'Дрель-шуруповёрт аккумуляторная, 18 В, 2 АКБ', category: 'Инструменты', image: 'drill', price: 3990, old: 5490, reviews: 38, hit: true },
  { id: 2, name: 'Краска интерьерная для стен и потолков, белая, 9 л', category: 'Краски', image: 'paint', price: 1890, old: 2490, reviews: 64, hit: true },
  { id: 3, name: 'Смеситель для раковины, однорычажный, хром', category: 'Сантехника', image: 'faucet', price: 2490, old: 3190, reviews: 26, hit: false },
  { id: 4, name: 'Лампа настольная с деревянным основанием, E27', category: 'Освещение', image: 'lamp', price: 1290, old: 0, reviews: 42, hit: true },
  { id: 5, name: 'Ламинат Дуб натуральный, 33 класс, 8 мм', category: 'Стройматериалы', image: 'materials', price: 899, old: 1190, reviews: 17, hit: false, unit: 'м²' },
  { id: 6, name: 'Кресло интерьерное с мягким сиденьем, оливковое', category: 'Для дома', image: 'chair', price: 7490, old: 9990, reviews: 21, hit: false },
];
const $ = (selector) => document.querySelector(selector);
const money = (value) => new Intl.NumberFormat('ru-RU').format(value);
// All state belongs to this browser. No requests or backend integrations.
function restore(key, fallback) {
  try { return JSON.parse(localStorage.getItem(key)) ?? fallback; } catch { return fallback; }
}
const savedCart = restore('maxidom-demo-cart', {});
const savedFavorites = restore('maxidom-demo-favorites', []);
const state = {
  cart: Object.fromEntries(products.filter(p => Number.isInteger(savedCart?.[p.id]) && savedCart[p.id] > 0 && savedCart[p.id] <= 99).map(p => [p.id, savedCart[p.id]])),
  favorites: new Set(Array.isArray(savedFavorites) ? savedFavorites.filter(id => products.some(p => p.id === id)) : []),
  category: '', query: '', tab: 'all', favoritesOnly: false, slide: 0,
};
function persist() {
  try {
    localStorage.setItem('maxidom-demo-cart', JSON.stringify(state.cart));
    localStorage.setItem('maxidom-demo-favorites', JSON.stringify([...state.favorites]));
  } catch { /* Keep the demo usable when browser storage is disabled. */ }
  const total = Object.values(state.cart).reduce((sum, n) => sum + n, 0);
  $('#cart-count').textContent = total;
  $('#cart-count').hidden = !total;
  $('#favorites-count').textContent = state.favorites.size;
  $('#favorites-count').hidden = !state.favorites.size;
  $('#favorites-button').setAttribute('aria-pressed', String(state.favoritesOnly));
}
let toastTimer;
function toast(message) {
  clearTimeout(toastTimer);
  $('#toast').textContent = message;
  $('#toast').hidden = false;
  toastTimer = setTimeout(() => { $('#toast').hidden = true; }, 2800);
}
function renderProducts() {
  const visible = products.filter(p => (!state.category || p.category === state.category)
    && (!state.query || `${p.name} ${p.category}`.toLocaleLowerCase('ru').includes(state.query.toLocaleLowerCase('ru')))
    && (state.tab !== 'sale' || p.old) && (state.tab !== 'popular' || p.hit)
    && (!state.favoritesOnly || state.favorites.has(p.id)));
  $('#products-title').textContent = state.favoritesOnly ? 'Избранное' : state.query ? `Поиск: «${state.query}»` : state.category || 'Выгодные предложения';
  $('#results-count').textContent = `Найдено: ${visible.length}`;
  $('#empty-state').hidden = visible.length > 0;
  $('#products').innerHTML = visible.map(p => `<article class="product">
    ${p.old ? `<span class="product-label">−${Math.round((1 - p.price / p.old) * 100)}%</span>` : '<span class="product-label hit">Хит продаж</span>'}
    <button class="favorite ${state.favorites.has(p.id) ? 'selected' : ''}" data-favorite="${p.id}" aria-label="${state.favorites.has(p.id) ? 'Убрать из' : 'Добавить в'} избранное: ${p.name}" aria-pressed="${state.favorites.has(p.id)}">${state.favorites.has(p.id) ? '♥' : '♡'}</button>
    <img class="product-image" src="assets/${p.image}.svg" alt="${p.name}">
    <div class="product-meta"><span>Код: 1000${p.id}764</span><span class="stars" aria-label="Рейтинг 5 из 5">★★★★★ <span style="color:#999;letter-spacing:0">(${p.reviews})</span></span></div>
    <h3>${p.name}</h3><p class="availability">В наличии</p><div class="product-bottom"><div><span class="old-price">${p.old ? `${money(p.old)} ₽` : ''}</span><span class="price">${money(p.price)} ₽ <small>/ ${p.unit || 'шт.'}</small></span></div><button class="add-cart" data-add="${p.id}" aria-label="В корзину: ${p.name}">+</button></div></article>`).join('');
  document.querySelectorAll('[data-tab]').forEach(button => {
    const active = button.dataset.tab === state.tab;
    button.classList.toggle('active', active);
    button.setAttribute('aria-pressed', String(active));
  });
  persist();
}
function closeCatalog() { $('#catalog-menu').hidden = true; $('#catalog-button').setAttribute('aria-expanded', 'false'); }
function toggleCatalog() {
  $('#catalog-menu').hidden = !$('#catalog-menu').hidden;
  $('#catalog-button').setAttribute('aria-expanded', String(!$('#catalog-menu').hidden));
}
function selectCategory(name = '') {
  window.RepairCalculator?.close();
  state.category = name; state.query = ''; state.tab = 'all'; state.favoritesOnly = false;
  $('#search-input').value = ''; closeCatalog(); renderProducts();
  $('#products-section').scrollIntoView({ behavior: 'smooth', block: 'start' });
}
function openDialog(title, content) {
  closeCatalog(); $('#dialog-title').textContent = title;
  $('#dialog-content').innerHTML = content;
  if (!$('#dialog').open) $('#dialog').showModal();
}
function showCart() {
  const items = products.filter(p => state.cart[p.id]);
  const total = items.reduce((sum, p) => sum + p.price * state.cart[p.id], 0);
  openDialog('Корзина', items.length ? items.map(p => `<div class="cart-row"><img src="assets/${p.image}.svg" alt=""><div><h3>${p.name}</h3><span>${money(p.price)} ₽</span></div><div class="quantity"><button data-quantity="${p.id}" data-delta="-1" aria-label="Уменьшить количество: ${p.name}">−</button><span>${state.cart[p.id]}</span><button data-quantity="${p.id}" data-delta="1" aria-label="Увеличить количество: ${p.name}">+</button></div></div>`).join('') + `<div class="cart-total"><span>Итого</span><span>${money(total)} ₽</span></div><p class="dialog-note">Это демонстрационная корзина. Оформление и оплата заказа пока не подключены.</p>` : '<p>В корзине пока пусто.</p><p class="dialog-note">Добавьте товары из каталога — они появятся здесь.</p>');
}
$('#categories').innerHTML = categories.map(c => `<button class="category" data-category="${c.name}"><img src="assets/${c.image}.svg" alt=""><strong>${c.name}</strong><small>${c.count}</small></button>`).join('');
$('#catalog-links').innerHTML = categories.map(c => `<button data-category="${c.name}">${c.name} <span aria-hidden="true">→</span></button>`).join('') + '<button class="red" data-open-calculator>Калькулятор ремонта <span aria-hidden="true">→</span></button>';
document.addEventListener('click', event => {
  const button = event.target.closest('button');
  if (button?.dataset.category) selectCategory(button.dataset.category);
  if (button?.dataset.add) {
    const id = Number(button.dataset.add);
    if ((state.cart[id] || 0) >= 99) return toast('Максимальное количество — 99 шт.');
    state.cart[id] = (state.cart[id] || 0) + 1; persist(); toast('Товар добавлен в корзину');
  }
  if (button?.dataset.favorite) {
    const id = Number(button.dataset.favorite);
    state.favorites.has(id) ? state.favorites.delete(id) : state.favorites.add(id);
    renderProducts();
  }
  if (button?.dataset.tab) { state.tab = button.dataset.tab; renderProducts(); }
  if (button?.dataset.quantity) {
    const id = Number(button.dataset.quantity);
    state.cart[id] = Math.min(99, state.cart[id] + Number(button.dataset.delta));
    if (state.cart[id] <= 0) delete state.cart[id];
    persist(); showCart();
  }
  if (button?.dataset.city) {
    $('#current-city').textContent = button.dataset.city;
    try { localStorage.setItem('maxidom-demo-city', JSON.stringify(button.dataset.city)); } catch {}
    $('#dialog').close(); toast('Город изменён');
  }
  if (button?.dataset.info) openDialog(button.dataset.info, '<p>Этот раздел появится в полной версии сайта.</p><p class="dialog-note">Сейчас открыт локальный макет для хакатона. Сервисы магазина и сервер не подключены.</p>');
  if (button?.dataset.promo) showPromos();
  if (button?.dataset.slide !== undefined) changeSlide(Number(button.dataset.slide));
  if (!event.target.closest('.header')) closeCatalog();
});
$('#catalog-button').addEventListener('click', toggleCatalog);
$('#all-categories').addEventListener('click', () => { window.RepairCalculator?.close(); window.scrollTo({ top: 0, behavior: 'smooth' }); toggleCatalog(); });
$('#search-form').addEventListener('submit', event => {
  event.preventDefault(); window.RepairCalculator?.close(); state.query = $('#search-input').value.trim(); state.category = ''; state.favoritesOnly = false; state.tab = 'all';
  renderProducts(); $('#products-section').scrollIntoView({ behavior: 'smooth' });
});
$('#search-input').addEventListener('input', () => { if (!$('#search-input').value && state.query) { state.query = ''; renderProducts(); } });
$('#reset-products').addEventListener('click', () => selectCategory());
$('#clear-search').addEventListener('click', () => selectCategory());
$('#favorites-button').addEventListener('click', () => {
  window.RepairCalculator?.close();
  const next = !state.favoritesOnly;
  state.category = ''; state.query = ''; state.tab = 'all'; state.favoritesOnly = next; $('#search-input').value = '';
  renderProducts(); $('#products-section').scrollIntoView({ behavior: 'smooth' });
});
$('#cart-button').addEventListener('click', showCart);
$('#account-button').addEventListener('click', () => openDialog('Личный кабинет', '<p>Рады видеть вас в Максидоме!</p><p class="dialog-note">Вход и регистрация появятся после подключения сервера. В демонстрационной версии можно пользоваться избранным и корзиной без аккаунта.</p>'));
$('#city-button').addEventListener('click', () => openDialog('Выберите город', '<div class="city-options">' + ['Санкт-Петербург', 'Москва', 'Казань', 'Екатеринбург', 'Нижний Новгород', 'Самара'].map(city => `<button data-city="${city}">${city}</button>`).join('') + '</div>'));
const savedCity = restore('maxidom-demo-city', null);
if (typeof savedCity === 'string') $('#current-city').textContent = savedCity;
$('#close-dialog').addEventListener('click', () => $('#dialog').close());
$('#dialog').addEventListener('click', event => {
  const bounds = $('#dialog').getBoundingClientRect();
  if (event.target === $('#dialog') && (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom)) $('#dialog').close();
});
document.addEventListener('keydown', event => { if (event.key === 'Escape') closeCatalog(); });
function showPromos() { selectCategory(); state.tab = 'sale'; renderProducts(); }
$('#promo-button').addEventListener('click', showPromos);
$('#hero-cta').addEventListener('click', () => selectCategory());
$('#inspiration-button').addEventListener('click', () => selectCategory('Стройматериалы'));
const slides = [
  ['БОЛЬШЕ ИДЕЙ — БОЛЬШЕ ВОЗМОЖНОСТЕЙ', 'Дом начинается<br>с ваших идей', 'Всё для ремонта, уюта и вдохновения.<br>Создавайте пространство, в котором хорошо.', '#eee9df'],
  ['ВРЕМЯ ОБНОВЛЯТЬ', 'Новый цвет.<br>Новое настроение.', 'Краски и инструменты для ваших перемен.<br>Вдохните новую жизнь в любимый дом.', '#e5e9df'],
  ['МАЛЕНЬКИЕ ДЕТАЛИ — БОЛЬШОЙ УЮТ', 'Сделайте дом<br>местом силы', 'Тёплый свет, любимые вещи и немного заботы.<br>Всё для вашего личного пространства.', '#eee5df'],
];
function changeSlide(index) {
  state.slide = (index + slides.length) % slides.length;
  const [eyebrow, title, description, background] = slides[state.slide];
  $('#hero-eyebrow').textContent = eyebrow; $('#hero-title').innerHTML = title;
  $('#hero-description').innerHTML = description; $('#hero').style.background = background;
  document.querySelectorAll('[data-slide]').forEach(button => {
    const active = Number(button.dataset.slide) === state.slide;
    button.classList.toggle('active', active);
    button.setAttribute('aria-current', String(active));
  });
}
$('#slide-prev').addEventListener('click', () => changeSlide(state.slide - 1));
$('#slide-next').addEventListener('click', () => changeSlide(state.slide + 1));
renderProducts();
