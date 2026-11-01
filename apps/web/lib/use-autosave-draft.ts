"use client";

import { useCallback, useEffect, useRef, useState } from "react";

export type AutosaveStatus = "idle" | "saving" | "saved" | "invalid" | "error" | "conflict";

export type SaveResult<R> =
  | { ok: true; data: R }
  | { ok: false; status: number; body: unknown };

/** PUTs a draft through a same-origin admin proxy, never throwing. */
export async function putDraft<R>(url: string, body: unknown): Promise<SaveResult<R>> {
  try {
    const response = await fetch(url, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const parsed: unknown = await response.json().catch(() => null);
    return response.ok ? { ok: true, data: parsed as R } : { ok: false, status: response.status, body: parsed };
  } catch {
    return { ok: false, status: 0, body: null };
  }
}

/**
 * Phase 25 Decision 22 / Risk 9: saves `value` as a draft 700 ms after the
 * last change. One save is in flight at a time; a newer value arriving
 * mid-save is saved once that returns. Every save sends and receives the
 * `draft_updated_at` concurrency token, and a `409` stops autosaving until
 * the page is reloaded. A `400` keeps the last valid draft and hands the
 * body to `onInvalid` for field errors.
 */
export function useAutosaveDraft<T, R extends { draft_updated_at: string | null }>(opts: {
  value: T;
  /** Client-side gate; a value that fails it is never sent. */
  isValid: (value: T) => boolean;
  save: (value: T, draftUpdatedAt: string | null) => Promise<SaveResult<R>>;
  initialDraftUpdatedAt: string | null;
  /** Runs after each successful save — refreshes the preview frames. */
  onSaved: (data: R) => void;
  onInvalid?: (body: unknown) => void;
  delayMs?: number;
}) {
  const { value, delayMs = 700 } = opts;
  const [status, setStatus] = useState<AutosaveStatus>("idle");

  const key = JSON.stringify(value);
  const tokenRef = useRef(opts.initialDraftUpdatedAt);
  const savedKeyRef = useRef(key);
  const inFlightRef = useRef<Promise<void> | null>(null);
  const queuedRef = useRef<T | null>(null);
  const stoppedRef = useRef(false);
  const latestRef = useRef({ ...opts, value });

  useEffect(() => {
    latestRef.current = { ...opts, value };
  });

  const runSave = useCallback(async function run(next: T): Promise<void> {
    if (stoppedRef.current) return;
    if (inFlightRef.current) {
      queuedRef.current = next;
      return inFlightRef.current;
    }
    const nextKey = JSON.stringify(next);
    if (nextKey === savedKeyRef.current) return;

    setStatus("saving");
    const attempt = (async () => {
      const result = await latestRef.current.save(next, tokenRef.current);
      if (result.ok) {
        tokenRef.current = result.data.draft_updated_at;
        savedKeyRef.current = nextKey;
        setStatus("saved");
        latestRef.current.onSaved(result.data);
      } else if (result.status === 409) {
        stoppedRef.current = true;
        setStatus("conflict");
      } else if (result.status === 400) {
        setStatus("invalid");
        latestRef.current.onInvalid?.(result.body);
      } else {
        setStatus("error");
      }
    })();

    inFlightRef.current = attempt;
    try {
      await attempt;
    } finally {
      inFlightRef.current = null;
    }

    const queued = queuedRef.current;
    queuedRef.current = null;
    if (queued !== null) await run(queued);
  }, []);

  useEffect(() => {
    if (key === savedKeyRef.current || stoppedRef.current) return;
    if (!latestRef.current.isValid(latestRef.current.value)) return;
    const timer = setTimeout(() => void runSave(latestRef.current.value), delayMs);
    return () => clearTimeout(timer);
  }, [key, delayMs, runSave]);

  /** Saves the current value now (e.g. before Publish), then waits. */
  const flush = useCallback(async () => {
    const current = latestRef.current.value;
    if (latestRef.current.isValid(current)) await runSave(current);
    while (inFlightRef.current) await inFlightRef.current;
  }, [runSave]);

  /** After a publish or discard: the server's token moved on. */
  const reset = useCallback((draftUpdatedAt: string | null) => {
    tokenRef.current = draftUpdatedAt;
    savedKeyRef.current = JSON.stringify(latestRef.current.value);
    setStatus("idle");
  }, []);

  const draftUpdatedAt = useCallback(() => tokenRef.current, []);

  return { status, flush, reset, draftUpdatedAt };
}
