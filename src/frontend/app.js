'use strict';

(function () {
  const session = document.getElementById('session');
  const announcer = document.getElementById('announcer');
  const statusEl = document.getElementById('connection-status');
  const clearBtn = document.getElementById('clear-btn');
  const reconnectBtn = document.getElementById('reconnect-btn');
  const ctrlModeEl = document.getElementById('ctrl-mode');

  // C0 control code lookup tables (excludes \x09 tab, \x0A newline, \x0D CR — handled elsewhere).
  const CTRL_NAMES = {
    '\x01':'SOH', '\x02':'STX', '\x03':'ETX', '\x04':'EOT', '\x05':'ENQ', '\x06':'ACK',
    '\x07':'BEL', '\x08':'BS',  '\x0B':'VT',  '\x0C':'FF',  '\x0E':'SO',  '\x0F':'SI',
    '\x10':'DLE', '\x11':'DC1', '\x12':'DC2', '\x13':'DC3', '\x14':'DC4', '\x15':'NAK',
    '\x16':'SYN', '\x17':'ETB', '\x18':'CAN', '\x19':'EM',  '\x1A':'SUB', '\x1B':'ESC',
    '\x1C':'FS',  '\x1D':'GS',  '\x1E':'RS',  '\x1F':'US',  '\x7F':'DEL',
  };
  const CTRL_MEANINGS = {
    '\x01':'start of heading', '\x02':'start of text',    '\x03':'interrupt',
    '\x04':'end of input',     '\x05':'enquiry',          '\x06':'acknowledge',
    '\x07':'bell',             '\x08':'backspace',        '\x0B':'vertical tab',
    '\x0C':'form feed',        '\x0E':'shift out',        '\x0F':'shift in',
    '\x10':'data link escape', '\x11':'resume',           '\x12':'device control 2',
    '\x13':'pause',            '\x14':'device control 4', '\x15':'not acknowledged',
    '\x16':'sync idle',        '\x17':'end of block',     '\x18':'cancel',
    '\x19':'end of medium',    '\x1A':'suspend',          '\x1B':'escape',
    '\x1C':'file separator',   '\x1D':'group separator',  '\x1E':'record separator',
    '\x1F':'unit separator',   '\x7F':'delete',
  };

  function renderCtrlChar(c) {
    const mode = ctrlModeEl.value;
    if (mode === 'name')    return '[' + CTRL_NAMES[c] + ']';
    if (mode === 'meaning') return '[' + CTRL_MEANINGS[c] + ']';
    // symbol: caret notation. DEL is conventionally ^? not ^\x7F.
    if (c === '\x7F') return '^?';
    return '^' + String.fromCharCode(c.charCodeAt(0) + 64);
  }

  // The currently open <details> element receiving output
  let currentGroup = null;
  // True after each input event: skip the first output line (PTY echo of the command)
  let skipNextOutput = false;
  let userQuit = false;

  // Produce readable plain text from raw PTY output.
  //
  // Rust's strip_non_sgr replaces non-SGR cursor-movement sequences with U+E000 (private use)
  // so that JS can collapse runs of those placeholders to a single space gap, preserving the
  // spacing between words that the TUI intended, while keeping real spaces (indentation, etc.)
  // intact.  SGR colour sequences that Rust passes through are deleted here without a gap
  // (they are style markers, not positional).  Backspace bytes (0x08) are simulated so that
  // readline edit-and-retype echoes collapse to the final typed text.
  function cleanText(text) {
    // Simulate \x08 (BS): each occurrence removes the preceding character from the buffer,
    // which correctly collapses readline's "overwrite" echo pattern into the final command.
    const bsBuf = [];
    for (const ch of text) {
      if (ch === '\x08') bsBuf.pop();
      else bsBuf.push(ch);
    }
    return bsBuf.join('')
      .replace(/\x1b\[[^a-zA-Z]*m/g, '')               // delete SGR colour/style (no gap)
      .replace(/\x1b\[[^a-zA-Z]*[a-zA-Z]/g, '\uE000')  // any remaining CSI → placeholder
      .replace(/\uE000+/g, ' ')                         // collapse placeholder runs → single space
      .replace(/\r\n/g, '\n')                           // Windows line endings → Unix
      .replace(/[^\n]*\r/g, '')                         // bare \r: discard text before cursor-return
      .replace(/[\u00A0\u3000\u2000-\u200A\u202F\u205F]/g, ' ') // Unicode spaces → ASCII space
      .replace(/[\u200B-\u200D\uFEFF]/g, '')            // zero-width chars → discard
      .replace(/[\x01-\x07\x0B\x0C\x0E-\x1F\x7F]/g, renderCtrlChar) // control chars → selected style
      .replace(/[^\x09\x0A\x20-\x7E\u3040-\u9FFF\uAC00-\uD7AF\uF900-\uFAFF]/g, ''); // keep ASCII + CJK
  }

  function createGroup(labelText) {
    const details = document.createElement('details');
    details.setAttribute('open', '');

    const summary = document.createElement('summary');
    const h2 = document.createElement('h2');
    h2.textContent = labelText;
    summary.appendChild(h2);
    details.appendChild(summary);

    return details;
  }

  function closeCurrentGroup() {
    if (currentGroup) {
      currentGroup.removeAttribute('open');
    }
  }

  function openNewGroup(labelText) {
    closeCurrentGroup();
    currentGroup = createGroup(labelText);
    session.appendChild(currentGroup);
  }

  function appendOutputLine(text) {
    if (!currentGroup) {
      openNewGroup('Session start');
    }
    const p = document.createElement('p');
    p.dataset.raw = text;
    p.textContent = cleanText(text);
    currentGroup.appendChild(p);
    scrollToBottom();
    announcer.textContent = cleanText(text);
  }

  function scrollToBottom() {
    session.scrollTop = session.scrollHeight;
  }

  function handleInput(payload) {
    // Discard a spurious empty-command event that fires at startup because the \r keystroke
    // used to launch ShellyMcShellface is still in the stdin buffer when raw mode begins.
    // Only suppress it when no output has arrived yet (currentGroup is null); after that,
    // real empty-Enter commands should still open a new group normally.
    if (payload.text === '(empty command)' && currentGroup === null) {
      skipNextOutput = true; // still skip the PTY echo so it doesn't appear as output
      return;
    }
    openNewGroup(cleanText(payload.text)); // raw keystrokes as placeholder
    skipNextOutput = true; // the PTY will echo the expanded command as the first output line
  }

  function handleOutput(payload) {
    if (skipNextOutput) {
      skipNextOutput = false;
      // Replace the placeholder label with the PTY echo, which reflects tab-completion and
      // other shell expansion — this is the actual command the shell received.
      const h2 = currentGroup && currentGroup.querySelector('summary h2');
      if (h2) { h2.dataset.raw = payload.text; h2.textContent = cleanText(payload.text); }
      return;
    }
    appendOutputLine(payload.text);
  }

  function handleStatus(payload) {
    if (payload.state === 'connected') {
      setStatus('Connected', 'connected');
    } else if (payload.state === 'pty_exited') {
      if (payload.code && payload.code !== 0) {
        setStatus(`Process exited with error (code ${payload.code})`, 'exited');
      } else {
        setStatus('Process exited', 'exited');
      }
    } else if (payload.state === 'pty_error') {
      setStatus('Process failed to start', 'error');
    } else if (payload.state === 'user_quit') {
      userQuit = true;
      setStatus('Session ended', 'exited');
    }
  }

  function setStatus(text, state) {
    statusEl.textContent = text;
    statusEl.dataset.state = state;
  }

  function clearSession() {
    while (session.firstChild) session.removeChild(session.firstChild);
    currentGroup = null;
    skipNextOutput = false;
    announcer.textContent = '';
  }

  // Clear button
  clearBtn.addEventListener('click', () => {
    clearSession();
    announcer.textContent = 'Session cleared';
  });

  // Re-render all stored output when the control-character display mode changes.
  ctrlModeEl.addEventListener('change', () => {
    document.querySelectorAll('[data-raw]').forEach(el => {
      el.textContent = cleanText(el.dataset.raw);
    });
  });

  // Reconnect button — shown when the SSE connection drops
  reconnectBtn.addEventListener('click', () => {
    reconnectBtn.hidden = true;
    clearSession();
    setStatus('Connecting…', 'connecting');
    connect();
  });

  // SSE connection
  let es;

  function connect() {
    es = new EventSource('/events');

    es.addEventListener('input', (e) => {
      try { handleInput(JSON.parse(e.data)); } catch (_) {}
    });

    es.addEventListener('output', (e) => {
      try { handleOutput(JSON.parse(e.data)); } catch (_) {}
    });

    es.addEventListener('status', (e) => {
      try { handleStatus(JSON.parse(e.data)); } catch (_) {}
    });

    es.onerror = () => {
      es.close(); // suppress automatic retry — would duplicate all DOM content
      if (userQuit) return; // process was intentionally ended
      setStatus('Disconnected', 'disconnected');
      reconnectBtn.hidden = false;
      announcer.textContent = 'Connection lost. Reconnect button is now available in the footer.';
    };
  }

  connect();
})();
