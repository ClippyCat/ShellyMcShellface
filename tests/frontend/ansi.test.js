'use strict';
const assert = require('assert');
const { parseAnsi } = require('../../src/frontend/ansi.js');

function run(name, fn) {
  try { fn(); console.log(`  PASS: ${name}`); }
  catch (e) { console.error(`  FAIL: ${name}\n       ${e.message}`); process.exitCode = 1; }
}

run('plain text returns single span with no style', () => {
  const spans = parseAnsi('hello');
  assert.equal(spans.length, 1);
  assert.equal(spans[0].text, 'hello');
  assert.equal(spans[0].style, '');
});

run('empty string returns empty array', () => {
  const spans = parseAnsi('');
  assert.equal(spans.length, 0);
});

run('reset code \x1b[0m ends current style', () => {
  const spans = parseAnsi('\x1b[1mhi\x1b[0m world');
  const boldSpan = spans.find(s => s.text === 'hi');
  const plainSpan = spans.find(s => s.text === ' world');
  assert.ok(boldSpan, 'bold span not found');
  assert.ok(plainSpan, 'plain span not found');
  assert.ok(boldSpan.style.includes('font-weight:bold'));
  assert.equal(plainSpan.style, '');
});

run('ANSI red (31) maps to #ff8585', () => {
  const spans = parseAnsi('\x1b[31mred\x1b[0m');
  const span = spans.find(s => s.text === 'red');
  assert.ok(span, 'red span not found');
  assert.ok(span.style.includes('color:#ff8585'), `got: ${span.style}`);
});

run('ANSI green (32) maps to #7dff7d', () => {
  const spans = parseAnsi('\x1b[32mgreen\x1b[0m');
  const span = spans.find(s => s.text === 'green');
  assert.ok(span.style.includes('color:#7dff7d'), `got: ${span.style}`);
});

run('bright blue (94) maps to #c8c8ff', () => {
  const spans = parseAnsi('\x1b[94mblue\x1b[0m');
  const span = spans.find(s => s.text === 'blue');
  assert.ok(span.style.includes('color:#c8c8ff'), `got: ${span.style}`);
});

run('bold (1) sets font-weight:bold', () => {
  const spans = parseAnsi('\x1b[1mbold\x1b[0m');
  const span = spans.find(s => s.text === 'bold');
  assert.ok(span.style.includes('font-weight:bold'));
});

run('compound code 1;32 applies bold and green', () => {
  const spans = parseAnsi('\x1b[1;32mtext\x1b[0m');
  const span = spans.find(s => s.text === 'text');
  assert.ok(span.style.includes('font-weight:bold'), `missing bold: ${span.style}`);
  assert.ok(span.style.includes('color:#7dff7d'), `missing color: ${span.style}`);
});

run('true color 38;2;255;128;0 passes through as rgb()', () => {
  const spans = parseAnsi('\x1b[38;2;255;128;0morange\x1b[0m');
  const span = spans.find(s => s.text === 'orange');
  assert.ok(span.style.includes('color:rgb(255,128,0)'), `got: ${span.style}`);
});

run('unrecognised codes silently stripped', () => {
  const spans = parseAnsi('\x1b[999mtext\x1b[0m');
  const span = spans.find(s => s.text === 'text');
  assert.ok(span, 'text span not found');
});

run('default fg (39) resets colour', () => {
  const spans = parseAnsi('\x1b[31mred\x1b[39mplain\x1b[0m');
  const plain = spans.find(s => s.text === 'plain');
  assert.ok(plain, 'plain span not found');
  assert.ok(!plain.style.includes('color'), `color not reset: ${plain.style}`);
});
