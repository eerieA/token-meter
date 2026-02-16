// Repaint helpers for Linux transparent-window compositor issues.
// Exported functions can be called from index.html or other code.

export function forceMousemoveRepaint() {
  try {
    const x = Math.floor(window.innerWidth / 2);
    const y = Math.floor(window.innerHeight / 2);
    console.log('forceMousemoveRepaint called', x, y);
    const ev = new MouseEvent('mousemove', {
      bubbles: true,
      cancelable: true,
      clientX: x,
      clientY: y,
      screenX: x,
      screenY: y,
    });
    // dispatch on window and document.body to maximize chance of effect
    window.dispatchEvent(ev);
    document.body.dispatchEvent(ev);
  } catch (e) {
    console.warn('mousemove repaint failed', e);
  }
}

export function forceElementMousemove() {
  // Find the element at the center (or cursor location) and dispatch a mousemove on it.
  try {
    const x = Math.floor(window.innerWidth / 2);
    const y = Math.floor(window.innerHeight / 2);
    console.log('forceElementMousemove called', x, y);
    const el = document.elementFromPoint(x, y) || document.body;
    const ev = new MouseEvent('mousemove', {
      bubbles: true,
      cancelable: true,
      clientX: x,
      clientY: y,
      screenX: x,
      screenY: y,
    });
    el.dispatchEvent(ev);
    // also briefly toggle a non-layout CSS property on that element
    el.style.transform = 'translateZ(0)';
    requestAnimationFrame(() => {
      el.style.transform = '';
    });
  } catch (e) {
    console.warn('element mousemove repaint failed', e);
  }
}

export function forcePointerRepaint() {
  // PointerEvent (some engines respond better to PointerEvent)
  try {
    const x = Math.floor(window.innerWidth / 2);
    const y = Math.floor(window.innerHeight / 2);
    console.log('forcePointerRepaint called', x, y);
    const ev = new PointerEvent('pointermove', {
      bubbles: true,
      cancelable: true,
      pointerId: 1,
      pointerType: 'mouse',
      clientX: x,
      clientY: y,
      isPrimary: true,
    });
    window.dispatchEvent(ev);
    document.body.dispatchEvent(ev);
  } catch (e) {
    console.warn('pointer repaint failed', e);
  }
}

export function forceCssRepaint() {
  try {
    console.log('forceCssRepaint called');
    // Add a class to documentElement that applies a tiny filter/opacity change forcing composite
    const cls = 'tm-force-repaint';
    const styleId = 'tm-force-repaint-style';
    if (!document.getElementById(styleId)) {
      const s = document.createElement('style');
      s.id = styleId;
      // Tiny non-zero blur or nearly-1 opacity can force a composite stage
      s.textContent = `.${cls} { filter: opacity(0.999); -webkit-backdrop-filter: blur(0.0001px); }`;
      document.head.appendChild(s);
    }
    document.documentElement.classList.add(cls);
    // remove after a couple of frames
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        document.documentElement.classList.remove(cls);
      });
    });
  } catch (e) {
    console.warn('css repaint failed', e);
  }
}

// Tauri opacity toggle (may require Tauri API support)
export async function toggleWindowOpacityRepaint() {
  try {
    console.log('toggleWindowOpacityRepaint called');
    if (window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow) {
      const current = window.__TAURI__.window.getCurrentWindow();
      if (current && typeof current.setOpacity === 'function') {
        // set to 0.999 then back to 1.0 with a brief delay
        await current.setOpacity(0.999);
        await new Promise((r) => setTimeout(r, 30));
        await current.setOpacity(1.0);
        return;
      }
    }
    console.warn('Tauri window.setOpacity not available in this environment');
  } catch (e) {
    console.warn('toggleWindowOpacityRepaint failed', e);
  }
}

// forceFullWindowPaintNudge
export function forceFullWindowPaintNudge({ color = 'rgba(0,0,0,0.001)', holdMs = 40 } = {}) {
  try {
    console.log('forceFullWindowPaintNudge called', color, holdMs);
    let el = document.getElementById('tm-repaint-nudge');
    if (!el) {
      el = document.createElement('div');
      el.id = 'tm-repaint-nudge';
      Object.assign(el.style, {
        position: 'fixed',
        inset: '0',
        pointerEvents: 'none',
        zIndex: 9999999,
        transition: 'none',
      });
      document.documentElement.appendChild(el);
    }
    // Force color change (non-zero alpha) then revert after a short delay
    el.style.backgroundColor = color;
    // Wait a tick so compositor sees it, then remove
    setTimeout(() => {
      el.style.backgroundColor = 'transparent';
      // also remove from DOM after another tick to be thorough
      setTimeout(() => {
        if (el.parentElement) el.parentElement.removeChild(el);
      }, 30);
    }, holdMs);
  } catch (e) {
    console.warn('forceFullWindowPaintNudge failed', e);
  }
}