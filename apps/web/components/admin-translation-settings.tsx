"use client";

import { useState, type FormEvent } from "react";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import type { TranslationSettingsStatus } from "@/lib/admin-translations";

const LABEL = "mb-1.5 block text-sm font-semibold";
const HINT = "m-0 mt-1 text-sm text-text-muted";

const NUMBER = new Intl.NumberFormat("en-NL");

function parseWords(text: string): string[] {
  return text
    .split("\n")
    .map((word) => word.trim())
    .filter((word) => word !== "");
}

function sameWords(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((word, index) => word === b[index]);
}

function UsageMeter({ status }: { status: TranslationSettingsStatus }) {
  if (!status.configured) {
    return (
      <Notice>
        Automatic translation isn&apos;t set up (no DeepL key). Type Dutch and French in each item&apos;s
        language tabs; English shows wherever they&apos;re blank.
      </Notice>
    );
  }
  if (!status.usage) {
    return <Notice>DeepL can&apos;t be reached right now, so nothing is being translated. It retries every five minutes.</Notice>;
  }
  const { count, limit } = status.usage;
  const used = limit > 0 ? Math.min(100, (count / limit) * 100) : 100;
  const stop = status.stop_line !== null && limit > 0 ? (status.stop_line / limit) * 100 : 100;
  const paused = status.stop_line !== null && count >= status.stop_line;
  return (
    <div className="flex flex-col gap-2">
      <p className="m-0 text-sm">
        <span className="font-semibold">DeepL: {NUMBER.format(count)}</span> of {NUMBER.format(limit)} characters used (
        {used.toFixed(1)}%)
      </p>
      <div
        role="meter"
        aria-label="DeepL characters used"
        aria-valuemin={0}
        aria-valuemax={limit}
        aria-valuenow={count}
        className="relative h-3 overflow-hidden rounded-full bg-background-soft"
      >
        <div className={`h-full ${paused ? "bg-primary-dark" : "bg-primary"}`} style={{ width: `${used}%` }} />
        <div aria-hidden className="absolute inset-y-0 w-0.5 bg-text" style={{ left: `${stop}%` }} />
      </div>
      <p className="m-0 text-sm text-text-muted">
        The line marks where translating stops ({status.stop_at_percent}%).
        {paused && " Automatic translation is paused: type Dutch and French yourself, or raise the limit below."}
      </p>
    </div>
  );
}

/**
 * Phase 38 Decisions 6, 20: DeepL's usage and the owner's controls. Saves
 * are live, guarded by `updated_at` like the other settings.
 */
export function AdminTranslationSettingsForm({ initialValue }: { initialValue: TranslationSettingsStatus }) {
  const [status, setStatus] = useState(initialValue);
  const [autoTranslate, setAutoTranslate] = useState(initialValue.auto_translate);
  const [stopAt, setStopAt] = useState(String(initialValue.stop_at_percent));
  const [wordsText, setWordsText] = useState(initialValue.keep_words.join("\n"));
  const [confirmingWords, setConfirmingWords] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  const words = parseWords(wordsText);
  const wordsChanged = !sameWords(words, status.keep_words);
  const stopAtNumber = Number.parseInt(stopAt, 10);
  const stopAtValid = Number.isInteger(stopAtNumber) && stopAtNumber >= 10 && stopAtNumber <= 100;
  const keepChangeShare =
    status.usage && status.usage.limit > 0 ? (status.keep_change_chars / status.usage.limit) * 100 : null;

  async function save() {
    setSubmitting(true);
    setNotice(null);
    setSaved(false);
    const response = await fetch("/api/admin/translations/settings", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        auto_translate: autoTranslate,
        stop_at_percent: stopAtNumber,
        keep_words: words,
        updated_at: status.updated_at,
      }),
    });
    setSubmitting(false);
    setConfirmingWords(false);
    if (response.ok) {
      const next = (await response.json()) as TranslationSettingsStatus;
      setStatus(next);
      setWordsText(next.keep_words.join("\n"));
      setSaved(true);
      return;
    }
    setNotice(
      response.status === 409
        ? "These settings were changed elsewhere. Reload the page to see the latest."
        : response.status === 400
          ? "Check the fields: up to 100 words of at most 40 characters, and a limit from 10 to 100%."
          : "Could not save. Please try again.",
    );
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!stopAtValid) {
      setNotice("Enter a limit from 10 to 100%.");
      return;
    }
    // A new keep list redoes every automatic translation: confirm first.
    if (wordsChanged && !confirmingWords && status.keep_change_chars > 0) {
      setConfirmingWords(true);
      return;
    }
    void save();
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-5">
      <UsageMeter status={status} />
      {status.configured && (
        <p className="m-0 text-sm text-text-muted">
          {status.pending_rows === 0
            ? "Every published item, category and rental is translated."
            : `${status.pending_rows} published ${status.pending_rows === 1 ? "row is" : "rows are"} waiting for DeepL (about ${NUMBER.format(status.pending_chars)} characters). The background job picks them up every five minutes.`}
        </p>
      )}
      {notice && <Notice>{notice}</Notice>}
      {saved && <p role="status" className="m-0 font-semibold text-primary">Saved.</p>}

      <label className="flex items-center gap-2 text-sm font-semibold">
        <input type="checkbox" checked={autoTranslate} onChange={(event) => setAutoTranslate(event.target.checked)} />
        Translate automatically with DeepL
      </label>
      <p className="m-0 -mt-3 text-sm text-text-muted">
        Off: nothing is sent to DeepL. Existing translations stay, and you type new ones yourself.
      </p>

      <div>
        <label htmlFor="translation-stop-at" className={LABEL}>
          Stop translating at (% of the allowance)
        </label>
        <input
          id="translation-stop-at"
          type="number"
          inputMode="numeric"
          min={10}
          max={100}
          step={1}
          className="field max-w-32"
          value={stopAt}
          aria-invalid={stopAtValid ? undefined : true}
          onChange={(event) => setStopAt(event.target.value)}
        />
        <p className={HINT}>Keeps the rest of the allowance in reserve. 80% is a safe default.</p>
      </div>

      <div>
        <label htmlFor="translation-keep-words" className={LABEL}>
          Words to keep
        </label>
        <textarea
          id="translation-keep-words"
          className="field"
          rows={6}
          value={wordsText}
          onChange={(event) => {
            setWordsText(event.target.value);
            setConfirmingWords(false);
          }}
        />
        <p className={HINT}>One per line. Dish names DeepL should leave exactly as they are, like Jollof or Egusi.</p>
      </div>

      {confirmingWords ? (
        <div className="flex flex-col gap-3 rounded-card border border-accent p-4">
          <p className="m-0 text-sm">
            Changing this list translates every automatic translation again: about{" "}
            {NUMBER.format(status.keep_change_chars)} characters
            {keepChangeShare !== null && ` (${keepChangeShare.toFixed(1)}% of your allowance)`}. Your own wording
            isn&apos;t touched.
          </p>
          <div className="flex flex-wrap gap-3">
            <Button type="submit" disabled={submitting}>
              {submitting ? "Saving…" : "Save and re-translate"}
            </Button>
            <Button type="button" variant="secondary" onClick={() => setConfirmingWords(false)}>
              Cancel
            </Button>
          </div>
        </div>
      ) : (
        <Button type="submit" disabled={submitting} className="self-start">
          {submitting ? "Saving…" : "Save"}
        </Button>
      )}
    </form>
  );
}
