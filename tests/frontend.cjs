const fs = require('fs');
const vm = require('vm');
const assert = require('assert');
class Element {
  constructor() { this.value = ''; this.children = []; this.classList = {add(){},remove(){}}; }
  append(...items) { this.children.push(...items); }
  replaceChildren() { this.children = []; }
  setAttribute() {}
  focus() {}
  addEventListener(name, handler) { this['event_' + name] = handler; }
  showModal() { this.open = true; }
  close() { this.open = false; this.event_close?.(); }
}
const ids = [...fs.readFileSync('ui/index.html','utf8').matchAll(/id="([^"]+)"/g)].map(m => m[1]);
const elements = Object.fromEntries(ids.map(id => [id,new Element()]));
const drafts = new Map();
const cfg = {theme_mode:'dark',color_preset:'violet',output_folder:'journal', blocks:[{id:'a',name:'First',start:'00:00',end:'12:00'}, {id:'b',name:'Second',start:'12:00',end:'23:59'}]};
let stored = [], fail = false, hidden = false, copied = '', dismissed = false;
const sandbox = {navigator:{clipboard:{writeText:async text => {copied=text;}}},document:{documentElement:{dataset:{}},getElementById:id => elements[id],createElement:() => new Element(), addEventListener(){}}, localStorage:{getItem:k => drafts.get(k) ?? null,setItem:(k,v) => drafts.set(k,v),removeItem:k => drafts.delete(k)}, window:{addEventListener(){},__TAURI__:{core:{invoke:async(cmd,args) => {
  if(cmd === 'get_config') return cfg;
  if(cmd === 'get_weekly_reminder') return null;
  if(cmd === 'weekly_review') return {path:'journal/week-2026-09-28.txt', text:'Weekly work journal: test entry', empty:false};
  if(cmd === 'dismiss_weekly_reminder') {dismissed=true; return;}
  if(cmd === 'get_default_config') return {theme_mode:'dark',color_preset:'blue',output_folder:'default-folder',start_with_windows:true,last_reminded:{},blocks:[{id:'morning',name:'Morning',start:'05:00',end:'12:00',reminder:true},{id:'afternoon',name:'Afternoon',start:'12:00',end:'18:00',reminder:true},{id:'evening',name:'Evening',start:'18:00',end:'22:00',reminder:true}]};
  if(cmd === 'save_config') { Object.assign(cfg, args.config); return; }
  if(cmd === 'get_today_str') return '2026-10-01';
  if(cmd === 'load_entries') return stored;
  if(cmd === 'save_entry') { if(fail) throw new Error('Disk full'); stored = [{date:args.date,block_id:args.blockId,work:args.work,notes:args.notes}]; return 'week.txt'; }
  if(cmd === 'hide_window') hidden = true;
}},event:{listen:async()=>{}}}},setTimeout:()=>0,clearTimeout(){},Date,crypto:require('crypto').webcrypto,structuredClone};
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync('ui/app.js','utf8'),sandbox);
const settle = () => new Promise(resolve => setImmediate(resolve));
(async()=>{
  await settle();
  vm.runInContext("selectBlock('a')",sandbox);
  elements.work.value = 'Unfinished work'; elements.work.oninput();
  elements.blocks.children[1].onclick();
  assert.equal(elements.work.value,'');
  elements.blocks.children[0].onclick();
  assert.equal(elements.work.value,'Unfinished work','Switching blocks must restore the draft');
  fail = true;
  await elements.entry.onsubmit({preventDefault(){}});
  assert.equal(hidden,false,'Failed saves must keep the window open');
  assert.equal(drafts.size > 0,true,'Failed saves must preserve drafts');
  assert.equal(elements.work.disabled,false);
  fail = false;
  await elements.entry.onsubmit({preventDefault(){}});
  assert.equal(hidden,true,'Successful saves should return to the tray');
  assert.equal(stored[0].work,'Unfinished work');
  assert.equal(drafts.has('daily-journal:journal:2026-10-01:a'),false);
  elements.settings.onclick();
  elements['theme-mode'].value = 'light'; elements['theme-mode'].onchange();
  elements['color-blue'].onchange();
  assert.equal(sandbox.document.documentElement.dataset.theme, 'light');
  assert.equal(sandbox.document.documentElement.dataset.preset, 'blue');
  elements['close-settings'].onclick();
  assert.equal(sandbox.document.documentElement.dataset.theme, 'dark', 'Cancel should restore the saved theme');
  assert.equal(sandbox.document.documentElement.dataset.preset, 'violet');
  elements.settings.onclick();
  elements['theme-mode'].value = 'light'; elements['theme-mode'].onchange();
  elements['color-sage'].onchange();
  await elements['settings-form'].onsubmit({preventDefault(){},submitter:new Element()});
  assert.equal(cfg.theme_mode, 'light');
  assert.equal(cfg.color_preset, 'sage');
  assert.equal(sandbox.document.documentElement.dataset.theme, 'light');
  assert.equal(JSON.parse(drafts.get('daily-journal-appearance')).color_preset, 'sage');
  vm.runInContext('applyAppearance(config)', sandbox);
  assert.equal(sandbox.document.documentElement.dataset.preset, 'sage');
  elements.settings.onclick();
  const savedWork = stored[0].work;
  const draftSnapshot = [...drafts.entries()];
  await elements['reset-settings'].onclick();
  assert.equal(sandbox.document.documentElement.dataset.theme, 'dark');
  assert.equal(sandbox.document.documentElement.dataset.preset, 'blue');
  assert.equal(elements['folder-path'].value, 'journal', 'Reset must retain the saved journal folder');
  assert.equal(cfg.color_preset, 'sage', 'Reset must wait for Save settings');
  assert.deepEqual([...drafts.entries()], draftSnapshot, 'Reset must preserve all drafts');
  assert.equal(stored[0].work, savedWork);
  elements['close-settings'].onclick();
  assert.equal(sandbox.document.documentElement.dataset.preset, 'sage', 'Cancel must undo reset preview');
  elements.settings.onclick();
  await elements['reset-settings'].onclick();
  await elements['settings-form'].onsubmit({preventDefault(){},submitter:new Element()});
  assert.equal(cfg.color_preset, 'blue');
  assert.equal(cfg.theme_mode, 'dark');
  assert.equal(cfg.output_folder, 'journal');
  assert.equal(cfg.blocks.length, 3);
  assert.equal(stored[0].work, savedWork);
  assert.equal(cfg.weekly_reminder.day, 5);
  assert.equal(cfg.weekly_reminder.time, '09:00');
  elements.settings.onclick();
  elements['weekly-day'].value = '6';
  elements['weekly-time'].value = '10:30';
  await elements['settings-form'].onsubmit({preventDefault(){},submitter:new Element()});
  assert.equal(cfg.weekly_reminder.day, 6);
  assert.equal(cfg.weekly_reminder.time, '10:30');
  await elements.review.onclick();
  assert.equal(elements['review-dialog'].open, true);
  await elements['review-copy-all'].onclick();
  assert(copied.includes('Summarize my progress') && copied.includes('test entry'));
  await elements['review-copy-prompt'].onclick();
  assert(!copied.includes('test entry'));
  await elements['review-done'].onclick();
  assert.equal(dismissed, true);
  assert.equal(elements['review-dialog'].open, false);
  elements.settings.onclick();
  vm.runInContext("settingsDraft.blocks.push({id:'early',name:'Before Morning',start:'04:00',end:'05:00',reminder:true}); renderSettings()",sandbox);
  assert.equal(elements['block-settings'].children[0].children[0].value, 'Before Morning', 'Settings must sort existing and added blocks by start time');
  const startInput = elements['block-settings'].children[0].children[1].children[0].children[0];
  startInput.value = '23:00'; startInput.onchange();
  assert.equal(elements['block-settings'].children.at(-1).children[0].value, 'Before Morning', 'Changing a start time must reorder settings');
  startInput.value = '04:00'; startInput.onchange();
  await elements['settings-form'].onsubmit({preventDefault(){},submitter:new Element()});
  assert.equal(cfg.blocks[0].id, 'early');
  assert.equal(elements.blocks.children[0].children[0].children[0].textContent, 'Before Morning', 'Main view must use chronological order after save');
  console.log('Frontend checks passed: chronological block ordering in main view and settings; weekly settings, review, copying and dismissal; reset preserves folder, entries and drafts; appearance preview, cancel and persistence; block drafts, failed save recovery, successful save and hide.');
})().catch(e => {console.error(e); process.exitCode=1;});
