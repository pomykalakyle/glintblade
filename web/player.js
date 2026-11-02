/* Uses the globals exported by the pinned Macroquad bundle, without modifying it. */
(() => {
  'use strict';
  const gameCanvas = document.getElementById('glcanvas');
  const overlay = document.getElementById('start');
  const play = document.getElementById('play');
  const status = document.getElementById('status');
  const progress = document.getElementById('progress');
  const touch = document.getElementById('touch');
  const layoutSelect = document.getElementById('layout');
  const hint = document.getElementById('hint');
  const instructions = document.getElementById('instructions');
  const controlsRevision = 13;
  const layoutPreference = 'glintblade.keyLayout';
  let selectedLayout = 0;
  try {
    if (localStorage.getItem(layoutPreference) === '1') selectedLayout = 1;
  } catch (_) { /* Storage may be unavailable in an embedded player. */ }
  showLayout(selectedLayout);
  let state = 'idle';
  let started = false;
  let readyScheduled = false;
  let timeout;
  let download;
  const pointers = new Map();
  const physicalCodes = new Map();
  const physicalKeys = new Map();
  const touchKeys = new Map();
  const keyNames = { Semicolon: ';', Space: ' ' };

  function showLayout(value) {
    layoutSelect.value = String(value);
    const bindings = value === 1 ? 'WARS move · N slash · E dash' : 'WASD move · J slash · K dash';
    hint.textContent = bindings;
    instructions.textContent = value === 1
      ? 'WARS / arrows to move · N to slash\nE / Space to dash · Enter to start'
      : 'WASD / arrows to move · J to slash\nK / Space to dash · Enter to start';
    gameCanvas.setAttribute('aria-label', `GLINTBLADE. ${bindings}. Arrows also move; Space dashes.`);
  }
  function rememberLayout(value) {
    selectedLayout = value;
    showLayout(value);
    try { localStorage.setItem(layoutPreference, String(value)); } catch (_) { /* Optional preference. */ }
  }
  function syncLayout() {
    const actual = wasm_exports.glint_key_layout();
    if (actual !== selectedLayout) rememberLayout(actual);
  }

  function armStartupWatchdog() {
    clearTimeout(timeout);
    if (!document.hidden && started && state !== 'ready' && state !== 'error') {
      timeout = setTimeout(() => fail(new Error('Startup stalled')), 120000);
    }
  }
  function setState(next, message) {
    state = next;
    document.body.dataset.state = next;
    status.textContent = message;
    window.dispatchEvent(new CustomEvent('playerstate', { detail: { state: next, message } }));
    armStartupWatchdog();
  }
  function fail(error) {
    if (state === 'error' || !started) return;
    setState('error', 'Could not start the game. Try again or open the full page.');
    clearTimeout(timeout);
    download?.abort();
    if (typeof blocking_event_loop !== 'undefined') blocking_event_loop = true;
    releaseAll();
    if (typeof animation_frame_timeout !== 'undefined') cancelAnimationFrame(animation_frame_timeout);
    overlay.hidden = false;
    touch.hidden = true;
    progress.hidden = true;
    play.disabled = false;
    play.textContent = 'Try again';
    console.error('GLINTBLADE player:', error);
  }
  function ready() {
    if (state === 'error' || state === 'ready') return;
    clearTimeout(timeout);
    overlay.hidden = true;
    touch.hidden = false;
    play.disabled = false;
    // Apply only an explicit saved choice; a new visitor starts with WASD.
    wasm_exports.glint_set_key_layout(selectedLayout);
    showLayout(wasm_exports.glint_key_layout());
    setState('ready', 'Ready');
    // The canvas can change size without a window resize (touch dock, footer,
    // embedding layout). Keep Macroquad's backing buffer in sync with CSS.
    if (typeof ResizeObserver !== 'undefined') {
      new ResizeObserver(() => {
        if (state === 'ready') resize(gameCanvas, wasm_exports.resize);
      }).observe(gameCanvas);
    }
    gameCanvas.focus({ preventScroll: true });
  }
  function watchFirstDraw() {
    const frame = animation;
    animation = function (...args) {
      if (state !== 'error') {
        frame(...args);
        if (state === 'ready') syncLayout();
      }
    };
    // Fullscreen can be refused by an embedding site's permissions policy.
    // A refused optional view change must not abort the game.
    importObject.env.sapp_set_fullscreen = enabled => {
      try {
        const change = enabled ? gameCanvas.requestFullscreen?.() : document.exitFullscreen?.();
        change?.catch(() => {});
      } catch (_) { /* Remain playable in the current frame. */ }
    };
    for (const name of ['glDrawArrays', 'glDrawElements', 'glDrawArraysInstanced', 'glDrawElementsInstanced']) {
      const draw = importObject.env[name];
      if (typeof draw !== 'function') continue;
      importObject.env[name] = function (...args) {
        const result = draw(...args);
        if (!readyScheduled && state !== 'error') {
          readyScheduled = true;
          requestAnimationFrame(ready);
        }
        return result;
      };
    }
  }
  async function fetchGame() {
    download = new AbortController();
    const release = await fetch('build-info.json?v=controls-r13', { cache: 'no-store', signal: download.signal });
    if (!release.ok) throw new Error(`Build manifest unavailable: ${release.status}`);
    const build = await release.json();
    if (build.schema !== 1 || build.controls_revision !== controlsRevision
        || !/^[a-f0-9]{64}$/.test(build.sha256)
        || build.wasm !== `glintblade.${build.sha256.slice(0, 16)}.wasm`) {
      throw new Error('Game build manifest mismatch');
    }
    const response = await fetch(build.wasm, { signal: download.signal });
    if (!response.ok) throw new Error(`Download failed: ${response.status}`);
    const total = Number(response.headers.get('content-length')) || 0;
    let bytes;
    if (!response.body) bytes = new Uint8Array(await response.arrayBuffer());
    else {
      const reader = response.body.getReader();
      const chunks = [];
      let received = 0;
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
        received += value.byteLength;
        armStartupWatchdog();
        if (total) { progress.max = total; progress.value = received; }
        status.textContent = `Loading… ${(received / 1048576).toFixed(1)}${total ? ' / ' + (total / 1048576).toFixed(1) : ''} MB`;
      }
      bytes = new Uint8Array(received);
      let offset = 0;
      for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
    }
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
    if (Array.from(digest, byte => byte.toString(16).padStart(2, '0')).join('') !== build.sha256) {
      throw new Error('Game download does not match this release');
    }
    document.body.dataset.build = build.build;
    document.body.dataset.controlsRevision = String(build.controls_revision);
    return bytes;
  }
  async function start() {
    if (state === 'error') { window.location.reload(); return; }
    if (started) return;
    started = true;
    play.disabled = true;
    play.textContent = 'Loading…';
    progress.hidden = false;
    progress.removeAttribute('value');
    setState('loading', 'Loading the game…');
    try {
      if (typeof register_plugins !== 'function' || typeof WebAssembly === 'undefined') throw new Error('Game runtime unavailable');
      register_plugins(plugins);
      // Create the game's own audio context inside this click's user activation.
      if (typeof importObject.env.audio_init === 'function') importObject.env.audio_init();
      watchFirstDraw();
      const bytes = await fetchGame();
      if (state === 'error') return;
      progress.removeAttribute('value');
      setState('compiling', 'Starting the game…');
      const module = await WebAssembly.compile(bytes);
      if (state === 'error') return;
      add_missing_functions_stabs(module);
      const instance = await WebAssembly.instantiate(module, importObject);
      if (state === 'error') return;
      wasm_memory = instance.exports.memory;
      wasm_exports = instance.exports;
      if (wasm_exports.crate_version() !== version) throw new Error('Game runtime version mismatch');
      if (typeof wasm_exports.glint_key_layout !== 'function'
          || typeof wasm_exports.glint_set_key_layout !== 'function'
          || wasm_exports.glint_controls_revision?.() !== controlsRevision) {
        throw new Error('Game controls version mismatch');
      }
      init_plugins(plugins);
      setState('starting', 'Preparing the world and sound…');
      instance.exports.main();
    } catch (error) { fail(error); }
  }

  function logicalCode(event) {
    // Macroquad reads code, which follows physical positions under a software
    // layout. Follow the displayed letters for both software and firmware
    // Colemak-DH, while leaving arrows, Tab and other non-letter keys intact.
    if (/^[a-z]$/i.test(event.key)) return `Key${event.key.toUpperCase()}`;
    if (event.key === ';' || event.key === ':') return 'Semicolon';
    return event.code;
  }
  function sendKey(type, code, source = {}, repeat = source.repeat || false) {
    return gameCanvas.dispatchEvent(new KeyboardEvent(type, {
      code,
      key: source.key || keyNames[code] || (/^Key[A-Z]$/.test(code) ? code.slice(3).toLowerCase() : code),
      repeat,
      ctrlKey: source.ctrlKey || false,
      shiftKey: source.shiftKey || false,
      altKey: source.altKey || false,
      metaKey: source.metaKey || false,
      location: source.location || 0,
      isComposing: source.isComposing || false,
      bubbles: true,
      cancelable: true,
    }));
  }
  function releasePointer(id, immediate = false) {
    const entry = pointers.get(id);
    if (!entry) return;
    // The game samples held keys on its next fixed tick. Keep even a very fast
    // tap alive long enough to be sampled, including when a frame is slow.
    const remaining = 80 - (performance.now() - entry.pressedAt);
    if (!immediate && remaining > 0) {
      if (!entry.releaseTimer) entry.releaseTimer = setTimeout(() => releasePointer(id, true), remaining);
      return;
    }
    clearTimeout(entry.releaseTimer);
    pointers.delete(id);
    const count = (touchKeys.get(entry.code) || 1) - 1;
    if (count) touchKeys.set(entry.code, count);
    else {
      touchKeys.delete(entry.code);
      entry.button.classList.remove('pressed');
      if (!physicalKeys.has(entry.code)) sendKey('keyup', entry.code);
    }
  }
  function releaseAll() {
    for (const id of Array.from(pointers.keys())) releasePointer(id, true);
    for (const code of physicalKeys.keys()) sendKey('keyup', code);
    physicalCodes.clear();
    physicalKeys.clear();
  }
  play.addEventListener('click', start);
  layoutSelect.addEventListener('change', () => {
    releaseAll();
    rememberLayout(Number(layoutSelect.value));
    if (state === 'ready') {
      wasm_exports.glint_set_key_layout(selectedLayout);
      syncLayout();
      gameCanvas.focus({ preventScroll: true });
    }
  });
  for (const button of touch.querySelectorAll('button')) {
    button.tabIndex = -1;
    button.addEventListener('pointerdown', event => {
      event.preventDefault();
      if (state !== 'ready') return;
      gameCanvas.focus({ preventScroll: true });
      button.setPointerCapture(event.pointerId);
      releasePointer(event.pointerId, true);
      const code = button.dataset.code;
      pointers.set(event.pointerId, { code, button, pressedAt: performance.now() });
      const count = touchKeys.get(code) || 0;
      touchKeys.set(code, count + 1);
      button.classList.add('pressed');
      if (!count && !physicalKeys.has(code)) sendKey('keydown', code);
    });
    for (const type of ['pointerup', 'pointercancel', 'lostpointercapture']) button.addEventListener(type, event => {
      event.preventDefault();
      releasePointer(event.pointerId);
    });
    button.addEventListener('contextmenu', event => event.preventDefault());
  }
  gameCanvas.addEventListener('keydown', event => {
    if (!event.isTrusted) return;
    const source = event.code || event.key;
    // Repeats and release keep the original logical key even if Shift or the
    // operating-system layout changes while the physical key is still held.
    const held = physicalCodes.has(source);
    const code = held ? physicalCodes.get(source) : logicalCode(event);
    const alreadyDown = physicalKeys.has(code) || touchKeys.has(code);
    if (!held) {
      physicalCodes.set(source, code);
      physicalKeys.set(code, (physicalKeys.get(code) || 0) + 1);
    }
    event.stopImmediatePropagation();
    // An overlapping keyboard/touch press is a repeat, not a second action.
    // Propagate only the runtime's own default cancellation to the real event,
    // so unrelated browser shortcuts retain their normal behavior.
    if (!sendKey('keydown', code, event, event.repeat || held || alreadyDown)) event.preventDefault();
  }, true);
  gameCanvas.addEventListener('keyup', event => {
    if (!event.isTrusted) return;
    event.stopImmediatePropagation();
    const source = event.code || event.key;
    const code = physicalCodes.get(source);
    if (code === undefined) return;
    physicalCodes.delete(source);
    const count = (physicalKeys.get(code) || 1) - 1;
    if (count) physicalKeys.set(code, count);
    else physicalKeys.delete(code);
    // A second physical key or a touch can still be holding the logical key.
    if (!count && !touchKeys.has(code) && !sendKey('keyup', code, event)) event.preventDefault();
  }, true);
  gameCanvas.addEventListener('blur', releaseAll);
  window.addEventListener('blur', releaseAll);
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) releaseAll();
    armStartupWatchdog();
  });
  window.addEventListener('error', event => { if (started) fail(event.error || event.message); });
  window.addEventListener('unhandledrejection', event => { if (started) fail(event.reason); });
  gameCanvas.addEventListener('webglcontextlost', event => { event.preventDefault(); fail(new Error('Graphics context lost')); });
  // Query strings (including X's autoplay flag) never bypass the play gesture.
})();
