'use strict';

(function () {
  const session = document.getElementById('session');
  const announcer = document.getElementById('announcer');
  const statusEl = document.getElementById('connection-status');
  const clearBtn = document.getElementById('clear-btn');
  const reconnectBtn = document.getElementById('reconnect-btn');

  // The currently open <details> element receiving output
  let currentGroup = null;
  // True after each input event: skip the first output line (PTY echo of the command)
  let skipNextOutput = false;

  // Strip ANSI SGR colour codes, carriage returns, and Unicode box-drawing characters.
  function cleanText(text) {
    return text
      .replace(/\x1b\[[^m]*m/g, '')   // ANSI SGR colour/style sequences
      .replace(/\r/g, '')              // carriage returns
      .replace(/[\u2500-\u259F]/g, ''); // Unicode box-drawing and block-element characters
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
    p.textContent = cleanText(text);
    currentGroup.appendChild(p);
    scrollToBottom();
    announcer.textContent = cleanText(text);
  }

  function scrollToBottom() {
    session.scrollTop = session.scrollHeight;
  }

  function handleInput(payload) {
    openNewGroup(payload.text);
    skipNextOutput = true; // the PTY will echo this command as the first output line
  }

  function handleOutput(payload) {
    if (skipNextOutput) {
      skipNextOutput = false;
      return; // skip the echo — it is already the group label
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
      setStatus('Disconnected', 'disconnected');
      reconnectBtn.hidden = false;
      announcer.textContent = 'Connection lost. Reconnect button is now available in the footer.';
    };
  }

  connect();
})();
