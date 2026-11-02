/* Uses the globals exported by the pinned Macroquad bundle, without modifying it. */
(() => {
  'use strict';
  const gameCanvas = document.getElementById('glcanvas');
  const overlay = document.getElementById('start');
  const play = document.getElementById('play');
  const status = document.getElementById('status');
  const progress = document.getElementById('progress');
  const touch = document.getElementById('touch');
  let state = 'idle';
  let started = false;
  let readyScheduled = false;
  let timeout;
  let download;
  const pointers = new Map();
  const physicalKeys = new Set();
  const touchKeys = new Map();
  const keyNames = { KeyX: 'x', KeyC: 'c' };

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
    setState('ready', 'Ready');
    gameCanvas.focus({ preventScroll: true });
  }
  function watchFirstDraw() {
    const frame = animation;
    animation = function (...args) {
      if (state !== 'error') frame(...args);
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
    const response = await fetch('glintblade.wasm?v=pixellab-r9', { signal: download.signal });
    if (!response.ok) throw new Error(`Download failed: ${response.status}`);
    const total = Number(response.headers.get('content-length')) || 0;
    if (!response.body) return response.arrayBuffer();
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
    const bytes = new Uint8Array(received);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
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
      init_plugins(plugins);
      setState('starting', 'Preparing the world and sound…');
      instance.exports.main();
    } catch (error) { fail(error); }
  }

  function sendKey(type, code) {
    gameCanvas.dispatchEvent(new KeyboardEvent(type, { code, key: keyNames[code] || code, bubbles: true, cancelable: true }));
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
    for (const code of physicalKeys) sendKey('keyup', code);
    physicalKeys.clear();
  }
  play.addEventListener('click', start);
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
    if (event.isTrusted) physicalKeys.add(event.code);
  });
  gameCanvas.addEventListener('keyup', event => {
    if (event.isTrusted) {
      physicalKeys.delete(event.code);
      // The touch is still holding this key. Do not let Macroquad release it
      // when the physical keyboard is released first.
      if (touchKeys.has(event.code)) event.stopImmediatePropagation();
    }
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
