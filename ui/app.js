const $ = id => document.getElementById(id);
const invoke = (command, args) => window.__TAURI__.core.invoke(command, args);
let config, entries = [], selected, settingsDraft, busy = false, pendingReminder;
let toastTimer;
let settingsBusy = false;
const colorPresets = ['violet', 'blue', 'sage', 'amber'];
function applyAppearance(appearance) {
  document.documentElement.dataset.theme = appearance.theme_mode === 'light' ? 'light' : 'dark';
  document.documentElement.dataset.preset = colorPresets.includes(appearance.color_preset) ? appearance.color_preset : 'blue';
}
function fillWeeklySettings() {
  const reminder = settingsDraft.weekly_reminder ?? {enabled:true, day:5, time:'09:00'};
  $('weekly-enabled').checked = reminder.enabled;
  $('weekly-day').value = String(reminder.day);
  $('weekly-time').value = reminder.time;
}
function fillAppearanceSettings() {
  $('theme-mode').value = settingsDraft.theme_mode ?? 'dark';
  for (const preset of colorPresets) $('color-' + preset).checked = (settingsDraft.color_preset ?? 'blue') === preset;
}
$('theme-mode').onchange = () => { settingsDraft.theme_mode = $('theme-mode').value; applyAppearance(settingsDraft); };
for (const preset of colorPresets) $('color-' + preset).onchange = () => { settingsDraft.color_preset = preset; applyAppearance(settingsDraft); };
$('settings-dialog').addEventListener('close', () => { if (config) applyAppearance(config); showPendingReview(); });
function toast(message) { $('toast').textContent = message; $('toast').classList.remove('hidden'); clearTimeout(toastTimer); toastTimer = setTimeout(() => $('toast').classList.add('hidden'), 5000); }
function draftKey() { return `daily-journal:${config.output_folder}:${$('date').value}:${selected}`; }
function persistDraft() {
  if (!selected) return;
  try { localStorage.setItem(draftKey(), JSON.stringify({work: $('work').value, notes: $('notes').value})); $('draft-status').textContent = 'Draft kept on this computer'; }
  catch { $('draft-status').textContent = 'Draft storage unavailable - save before closing.'; }
}
function sortBlocks(blocks) { return blocks.sort((a, b) => a.start.localeCompare(b.start)); }
function renderBlocks() {
  sortBlocks(config.blocks);
  $('blocks').replaceChildren();
  for (const block of config.blocks) {
    const button = document.createElement('button'); button.className = `block-item block-choice ${selected === block.id ? 'selected' : ''}`;
    button.setAttribute('aria-pressed', String(selected === block.id));
    const info = document.createElement('span'); info.className = 'block-info';
    const title = document.createElement('span'); title.className = 'block-name'; title.textContent = block.name;
    const time = document.createElement('span'); time.className = 'block-time'; time.textContent = `${block.start} \u2013 ${block.end}`;
    const status = document.createElement('span'); status.className = 'block-status'; status.textContent = entries.some(e => e.date === $('date').value && e.block_id === block.id) ? 'Saved' : 'Write';
    info.append(title, time); button.append(info, status); button.onclick = () => { if (!busy) { persistDraft(); selectBlock(block.id); } }; $('blocks').append(button);
  }
}
function selectBlock(id) {
  selected = id; const block = config.blocks.find(b => b.id === id); if (!block) return;
  const saved = entries.find(e => e.date === $('date').value && e.block_id === id);
  let draft; try { draft = JSON.parse(localStorage.getItem(draftKey())); } catch {}
  $('work').value = draft?.work ?? saved?.work ?? ''; $('notes').value = draft?.notes ?? saved?.notes ?? '';
  $('notes-details').open = !!$('notes').value;
  $('entry-title').textContent = block.name; $('entry-time').textContent = `${block.start} \u2013 ${block.end}`;
  $('draft-status').textContent = draft ? 'Restored your draft' : saved ? 'Saved entry - edit to update' : '';
  renderBlocks(); $('work').focus();
}
function currentBlock() {
  sortBlocks(config.blocks);
  const time = new Date().toTimeString().slice(0,5);
  return config.blocks.find(b => b.start < b.end ? time >= b.start && time < b.end : time >= b.start || time < b.end)?.id ?? config.blocks[0].id;
}
async function refresh(id) { entries = await invoke('load_entries', {date: $('date').value}); selectBlock(id ?? currentBlock()); }
$('work').oninput = persistDraft; $('notes').oninput = persistDraft;
let loadedDate, lastToday;
$('date').onchange = async () => {
  if (busy || !$('date').value) { $('date').value = loadedDate; return; }
  const targetDate = $('date').value; $('date').value = loadedDate; persistDraft(); $('date').value = targetDate;
  try { await refresh(selected); loadedDate = targetDate; } catch (e) { $('date').value = loadedDate; toast(String(e)); }
};
$('entry').onsubmit = async event => {
  event.preventDefault(); if (busy || !$('work').value.trim()) return;
  busy = true; $('save').disabled = true; $('date').disabled = true; $('work').disabled = true; $('notes').disabled = true;
  const key = draftKey();
  try {
    await invoke('save_entry', { date: $('date').value, blockId: selected, work: $('work').value, notes: $('notes').value });
    try { localStorage.removeItem(key); } catch {}
    entries = await invoke('load_entries', {date: $('date').value}); renderBlocks(); $('draft-status').textContent = 'Saved to your weekly text file'; toast('Saved. You can get back to work.');
    await invoke('hide_window');
  } catch (e) { toast(String(e)); }
  finally { busy = false; $('save').disabled = false; $('date').disabled = false; $('work').disabled = false; $('notes').disabled = false; if (pendingReminder) { selectBlock(pendingReminder); pendingReminder = null; } showPendingReview(); }
};
document.addEventListener('keydown', e => { if ((e.ctrlKey || e.metaKey) && e.key === 'Enter' && !$('settings-dialog').open && !$('review-dialog').open) { e.preventDefault(); $('entry').requestSubmit(); } });
$('week').onclick = async () => { try { await invoke('open_week', {date: $('date').value}); } catch (e) { toast(String(e)); } };
$('folder').onclick = async () => { try { await invoke('open_folder'); } catch (e) { toast(String(e)); } };
function renderSettings() {
  sortBlocks(settingsDraft.blocks);
  $('block-settings').replaceChildren();
  settingsDraft.blocks.forEach((block, index) => {
    const row = document.createElement('div'); row.className = 'settings-block';
    const name = document.createElement('input'); name.className = 'text-input'; name.value = block.name; name.placeholder = 'Block name'; name.required = true; name.setAttribute('aria-label', 'Block name'); name.oninput = () => block.name = name.value;
    row.append(name);
    const times = document.createElement('div'); times.className = 'schedule-row';
    for (const field of ['start','end']) { const label = document.createElement('label'); label.textContent = field === 'start' ? 'From ' : 'To '; const input = document.createElement('input'); input.type = 'time'; input.className = 'text-input'; input.value = block[field]; input.required = true; input.oninput = () => block[field] = input.value; input.onchange = () => { block[field] = input.value; if (field === 'start' && input.value) renderSettings(); }; label.append(input); times.append(label); }
    const label = document.createElement('label'); label.className = 'checkbox-label'; const check = document.createElement('input'); check.type = 'checkbox'; check.checked = block.reminder; check.onchange = () => block.reminder = check.checked; label.append(check, 'Reminder');
    const remove = document.createElement('button'); remove.type = 'button'; remove.className = 'btn btn-remove'; remove.textContent = 'Remove'; remove.disabled = settingsDraft.blocks.length === 1; remove.onclick = () => { settingsDraft.blocks.splice(index,1); renderSettings(); };
    times.append(label); row.append(times, remove); $('block-settings').append(row);
  });
}
$('settings').onclick = () => { if (busy || !config) return; persistDraft(); settingsDraft = structuredClone(config); $('autostart').checked = settingsDraft.start_with_windows; $('folder-path').value = settingsDraft.output_folder; $('settings-error').textContent = ''; $('settings-note').textContent = 'Save to apply changes'; fillAppearanceSettings(); fillWeeklySettings(); renderSettings(); $('settings-dialog').showModal(); };
$('reset-settings').onclick = async () => {
  if (settingsBusy) return;
  settingsBusy = true;
  const draftBeforeReset = settingsDraft;
  const button = $('reset-settings'); button.disabled = true; $('save-settings').disabled = true;
  try {
    const defaults = await invoke('get_default_config');
    if (!$('settings-dialog').open || settingsDraft !== draftBeforeReset) return;
    defaults.output_folder = config.output_folder;
    defaults.last_reminded = config.last_reminded;
    settingsDraft = defaults;
    $('autostart').checked = defaults.start_with_windows;
    $('folder-path').value = defaults.output_folder;
    $('settings-error').textContent = '';
    $('settings-note').textContent = 'Defaults restored. Save to apply.';
    fillAppearanceSettings(); fillWeeklySettings(); renderSettings(); applyAppearance(defaults);
  } catch (e) { $('settings-error').textContent = String(e); }
  finally { settingsBusy = false; button.disabled = false; $('save-settings').disabled = false; }
};
$('close-settings').onclick = () => $('settings-dialog').close();
$('add-block').onclick = () => { settingsDraft.blocks.push({id: crypto.randomUUID(), name: `Block ${settingsDraft.blocks.length + 1}`, start: '09:00', end: '12:00', reminder: true}); renderSettings(); };
$('browse').onclick = async () => { try { const folder = await invoke('pick_folder'); if (folder) { settingsDraft.output_folder = folder; $('folder-path').value = folder; } } catch (e) { $('settings-error').textContent = String(e); } };
$('settings-form').onsubmit = async event => {
  event.preventDefault(); if (settingsBusy) return; settingsBusy = true; $('reset-settings').disabled = true; settingsDraft.start_with_windows = $('autostart').checked;
  sortBlocks(settingsDraft.blocks);
  settingsDraft.weekly_reminder = {...settingsDraft.weekly_reminder, enabled:$('weekly-enabled').checked, day:Number($('weekly-day').value), time:$('weekly-time').value};
  const submit = event.submitter; submit.disabled = true;
  try {
    await invoke('save_config', {config: settingsDraft}); config = settingsDraft; applyAppearance(config); try { localStorage.setItem('daily-journal-appearance', JSON.stringify({theme_mode: config.theme_mode, color_preset: config.color_preset})); } catch {} $('settings-dialog').close(); await refresh(config.blocks.some(b => b.id === selected) ? selected : config.blocks[0].id); toast('Settings saved');
  } catch (e) { $('settings-error').textContent = String(e); }
  finally { settingsBusy = false; submit.disabled = false; $('reset-settings').disabled = false; }
};
let pendingReview, reviewDate, reviewText;
const feedbackPrompt = `Review my weekly work journal. Summarize my progress and wins, identify patterns in focus, distractions, and how I spend time. Use concrete examples from the journal. Do not assume missing entries mean I did no work, and note that this week may still be in progress. Suggest three practical improvements for next week and one small habit to try. End with two useful reflection questions. Keep the feedback concise and constructive.`;
async function showReview(date) {
  try {
    const review = await invoke('weekly_review', {date});
    reviewDate = date; reviewText = review.text;
    $('review-file').textContent = review.path;
    $('review-empty').textContent = review.empty ? 'No entries saved for this week yet. Add a check-in before reviewing.' : 'Your weekly file is ready to use in your AI chat.';
    $('review-open').disabled = review.empty;
    $('review-copy-all').disabled = review.empty;
    $('review-prompt').value = feedbackPrompt;
    $('review-status').textContent = '';
    if (!$('review-dialog').open) $('review-dialog').showModal();
  } catch (e) { toast(String(e)); }
}
async function showPendingReview() {
  if (!pendingReview || config?.weekly_reminder?.enabled === false || busy || $('settings-dialog').open || $('review-dialog').open) return;
  await showReview(pendingReview);
}
$('review').onclick = () => showReview($('date').value);
$('close-review').onclick = () => $('review-dialog').close();
$('review-folder').onclick = async () => { try { await invoke('open_folder'); } catch(e) { $('review-status').textContent = String(e); } };
$('review-open').onclick = async () => { try { await invoke('open_week', {date:reviewDate}); } catch(e) { $('review-status').textContent = String(e); } };
async function copyReview(includeJournal) {
  try {
    await navigator.clipboard.writeText(feedbackPrompt + (includeJournal ? '\n\n' + reviewText : ''));
    $('review-status').textContent = includeJournal ? 'Copied. Paste into your AI chat.' : 'Prompt copied. Attach the weekly file to your AI chat.';
  } catch { $('review-status').textContent = 'Could not copy. Select and copy the prompt above manually.'; }
}
$('review-copy-prompt').onclick = () => copyReview(false);
$('review-copy-all').onclick = () => copyReview(true);
$('review-done').onclick = async () => {
  try { await invoke('dismiss_weekly_reminder', {date:reviewDate}); if (pendingReview === reviewDate) pendingReview = null; $('review-dialog').close(); }
  catch(e) { $('review-status').textContent = String(e); }
};
async function init() {
  try {
    config = await invoke('get_config'); applyAppearance(config); $('date').value = await invoke('get_today_str'); loadedDate = $('date').value; lastToday = loadedDate;
    await window.__TAURI__.event.listen('reminder-triggered', async event => {
      if (busy) { pendingReminder = event.payload; return; }
      persistDraft();
      try { config = await invoke('get_config'); applyAppearance(config); $('date').value = await invoke('get_today_str'); loadedDate = $('date').value; lastToday = loadedDate; await refresh(event.payload); toast('A quick check-in for this block'); } catch (e) { toast(String(e)); }
    });
    await refresh();
    await window.__TAURI__.event.listen('weekly-review-triggered', event => { pendingReview = event.payload; showPendingReview(); });
    pendingReview = await invoke('get_weekly_reminder');
    await showPendingReview();
  } catch (e) { toast(`Could not load journal: ${e}`); }
}
init();

window.addEventListener('focus', async () => {
  if (!config || busy || $('settings-dialog').open) return;
  try {
    const today = await invoke('get_today_str');
    if (today !== lastToday && $('date').value === lastToday) {
      persistDraft(); $('date').value = today; loadedDate = today; await refresh();
    }
    lastToday = today;
  } catch (e) { toast(String(e)); }
});
