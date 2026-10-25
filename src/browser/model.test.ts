// ABOUTME: Verifies address normalization and bounded tab behavior.
// ABOUTME: Runs through a tiny TypeScript compile plus Node's built-in test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  MAX_TABS,
  normalizeAddress,
  reserveTabId,
  releaseTabId,
  isSleepEligible,
  tabCapacityAvailable,
} from "./model.js";

test("normalizes an explicit HTTPS URL", () => {
  assert.equal(normalizeAddress(" https://example.com/a?q=1 "), "https://example.com/a?q=1");
});

test("adds HTTPS to a hostname", () => {
  assert.equal(normalizeAddress("example.com/docs"), "https://example.com/docs");
});

test("uses DuckDuckGo for words instead of treating them as a host", () => {
  assert.equal(normalizeAddress("light browser"), "https://duckduckgo.com/?q=light%20browser");
});

test("rejects insecure, executable, local, and hostless schemes", () => {
  assert.throws(() => normalizeAddress("http://example.com"), /HTTPS/);
  assert.throws(() => normalizeAddress("javascript:alert(1)"), /HTTP/);
  assert.throws(() => normalizeAddress("file:///C:/secret.txt"), /HTTP/);
  assert.throws(() => normalizeAddress("https://"), /HTTP/);
});

test("rejects an empty address", () => {
  assert.throws(() => normalizeAddress("  "), /alamat/i);
});

test("caps tabs at the declared limit", () => {
  assert.equal(tabCapacityAvailable(MAX_TABS - 1), true);
  assert.equal(tabCapacityAvailable(MAX_TABS), false);
  assert.equal(tabCapacityAvailable(MAX_TABS + 1), false);
});

test("reserves unique tab ids without exceeding the limit", () => {
  const ids = new Set<number>();
  for (let id = 1; id <= MAX_TABS; id += 1) {
    assert.equal(reserveTabId(ids, id), true);
  }
  assert.equal(reserveTabId(ids, MAX_TABS + 1), false);
  assert.equal(reserveTabId(ids, 1), false);
});

test("releases a tab id for reuse", () => {
  const ids = new Set([1]);
  releaseTabId(ids, 1);
  assert.equal(reserveTabId(ids, 1), true);
});

test("only inactive tabs can sleep", () => {
  assert.equal(isSleepEligible(1, 2), true);
  assert.equal(isSleepEligible(1, 1), false);
});
