'use strict';

(function () {
  const session = document.getElementById('session');
  const announcer = document.getElementById('announcer');
  const statusEl = document.getElementById('connection-status');

  // The currently open <details> element receiving output
  let currentGroup = null;

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
    // Convert ANSI SGR sequences to styled spans
    const spans = parseAnsi(text);
    if (spans.length === 0) {
      p.textContent = text;
    } else {
      for (const { text: t, style } of spans) {
        if (!t) continue;
        if (style) {
          const span = document.createElement('span');
          span.style.cssText = style;
          span.textContent = t;
          p.appendChild(span);
        } else {
          p.appendChild(document.createTextNode(t));
        }
      }
    }
    currentGroup.appendChild(p);
    scrollToBottom();
    announceOutput(text);
  }

  function scrollToBottom() {
    session.scrollTop = session.scrollHeight;
  }

  function announceOutput(text) {
    // Trim announcer to last 50 children to prevent unbounded growth
    while (announcer.childElementCount >= 50) {
      announcer.removeChild(announcer.firstChild);
    }
    const p = document.createElement('p');
    // Plain text only for announcements — no ANSI codes
    p.textContent = text.replace(/\x1b\[[^m]*m/g, '');
    announcer.appendChild(p);
  }

  function handleInput(payload) {
    openNewGroup(payload.text);
  }

  function handleOutput(payload) {
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

  // SSE connection
  const es = new EventSource('/events');

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
    setStatus('Disconnected — reload to reconnect', 'disconnected');
  };
})();
