'use strict';

const FG_PALETTE = {
  30: '#b0b0b0', 31: '#ff8585', 32: '#7dff7d', 33: '#ffff85',
  34: '#b0b0ff', 35: '#ff85ff', 36: '#85ffff', 37: '#e8e8e8',
  90: '#c8c8c8', 91: '#ffaaaa', 92: '#aaffaa', 93: '#ffffaa',
  94: '#c8c8ff', 95: '#ffaaff', 96: '#aaffff', 97: '#ffffff',
};

/**
 * Apply a list of SGR numeric codes to a mutable state object.
 * state: { color: string|null, bold: boolean }
 */
function applyCodes(codes, state) {
  let i = 0;
  while (i < codes.length) {
    const c = codes[i];
    if (c === 0 || c === '') {
      state.color = null;
      state.bold = false;
    } else if (c === 1) {
      state.bold = true;
    } else if (c === 22) {
      state.bold = false;
    } else if (FG_PALETTE[c] !== undefined) {
      state.color = FG_PALETTE[c];
    } else if (c === 39) {
      state.color = null;
    } else if (c === 38 && codes[i + 1] === 2) {
      const r = codes[i + 2], g = codes[i + 3], b = codes[i + 4];
      state.color = `rgb(${r},${g},${b})`;
      i += 4;
    }
    // all other codes (background, 256-colour, etc.) silently ignored
    i++;
  }
}

function stateToStyle(state) {
  const parts = [];
  if (state.color) parts.push(`color:${state.color}`);
  if (state.bold) parts.push('font-weight:bold');
  return parts.join(';');
}

/**
 * Parse a string containing ANSI SGR escape sequences.
 * Returns an array of { text: string, style: string }.
 * Only SGR sequences (already stripped of cursor-movement server-side) are handled.
 */
function parseAnsi(text) {
  const result = [];
  const state = { color: null, bold: false };
  const re = /\x1b\[([\d;]*)m|([^\x1b]+)/g;
  let match;
  while ((match = re.exec(text)) !== null) {
    if (match[2] !== undefined) {
      // Plain text
      const style = stateToStyle(state);
      result.push({ text: match[2], style });
    } else {
      // SGR sequence — update state
      const raw = match[1];
      const codes = raw === '' ? [0] : raw.split(';').map(Number);
      applyCodes(codes, state);
    }
  }
  return result;
}

if (typeof module !== 'undefined') {
  module.exports = { parseAnsi };
}
