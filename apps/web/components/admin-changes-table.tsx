"use client";

import Link from "next/link";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { AdminConfirmDialog } from "@/components/admin-confirm-dialog";
import { formatDateTime } from "@/lib/format";
import {
  KIND_LABELS,
  changePreviewPath,
  isImageField,
  type ChangeEntry,
  type ChangedField,
} from "@/lib/admin-changes";
import { DRAFT_ACTION_MESSAGES, discardDraft, publishEntries, type PublishEntry } from "@/lib/admin-drafts";

function toPublishEntry(entry: ChangeEntry): PublishEntry {
  return entry.id === null
    ? { kind: entry.kind, draft_updated_at: entry.draft_updated_at }
    : { kind: entry.kind, id: entry.id, draft_updated_at: entry.draft_updated_at };
}

function entryKey(entry: ChangeEntry): string {
  return `${entry.kind}:${entry.id ?? ""}`;
}

function Value({ field, value }: { field: string; value: ChangedField["before"] }) {
  if (value === null || value === "") return <span className="text-text-muted">—</span>;
  if (typeof value === "boolean") return <>{value ? "Yes" : "No"}</>;
  if (isImageField(field)) {
    // eslint-disable-next-line @next/next/no-img-element -- uploaded photos aren't always a configured next/image remote pattern.
    return <img src={value} alt="" className="h-14 w-14 rounded-card object-cover" />;
  }
  return <>{value}</>;
}

/**
 * Every pending draft and unpublished row, with field diffs and per-entry
 * Preview, Publish and Discard, plus Publish all in one atomic request
 * (Phase 25 Decisions 19–20).
 */
export function AdminChangesTable({ entries }: { entries: ChangeEntry[] }) {
  const router = useRouter();
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<{ action: "publish-all" } | { action: "discard"; entry: ChangeEntry } | null>(
    null,
  );

  async function run(key: string, action: () => Promise<Awaited<ReturnType<typeof publishEntries>>>) {
    setBusy(key);
    setMessage(null);
    const result = await action();
    setBusy(null);
    setConfirming(null);
    if (result === "ok") router.refresh();
    else setMessage(DRAFT_ACTION_MESSAGES[result]);
  }

  if (entries.length === 0) {
    return <p className="m-0 text-text-muted">Nothing waiting to be published.</p>;
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center gap-3">
        <Button type="button" onClick={() => setConfirming({ action: "publish-all" })} disabled={busy !== null}>
          Publish all ({entries.length})
        </Button>
        <Link href="/preview-all" className="text-sm font-semibold text-primary underline">
          Preview everything
        </Link>
      </div>
      {message && <Notice>{message}</Notice>}

      <ul className="m-0 flex list-none flex-col gap-4 p-0">
        {entries.map((entry) => {
          const key = entryKey(entry);
          return (
            <li key={key} className="flex flex-col gap-3 rounded-card border border-text/10 p-4">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="text-xs font-semibold uppercase tracking-wider text-text-muted">
                    {KIND_LABELS[entry.kind]}
                  </span>
                  <span className="font-semibold">{entry.label}</span>
                  {entry.is_new && (
                    <span className="rounded-full bg-accent-light px-2.5 py-0.5 text-xs font-semibold">New</span>
                  )}
                  {entry.draft_updated_at && (
                    <span className="text-xs text-text-muted">{formatDateTime(entry.draft_updated_at)}</span>
                  )}
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Link
                    href={`/preview-all?path=${encodeURIComponent(changePreviewPath(entry))}`}
                    className="text-sm font-semibold text-primary underline"
                  >
                    Preview
                  </Link>
                  <Button
                    type="button"
                    variant="secondary"
                    disabled={busy !== null}
                    onClick={() => run(key, () => publishEntries([toPublishEntry(entry)]))}
                  >
                    {busy === key ? "Publishing…" : "Publish"}
                  </Button>
                  <Button
                    type="button"
                    variant="secondary"
                    disabled={busy !== null}
                    onClick={() => setConfirming({ action: "discard", entry })}
                  >
                    Discard
                  </Button>
                </div>
              </div>

              {entry.changed_fields.length > 0 && (
                <table className="w-full border-collapse text-sm">
                  <tbody>
                    {entry.changed_fields.map((change) => (
                      <tr key={change.field} className="border-t border-text/10 align-top">
                        <th scope="row" className="w-48 py-2 pr-4 text-left font-semibold text-text-muted">
                          {change.field}
                        </th>
                        <td className="py-2 pr-4 text-text-muted line-through decoration-primary/60">
                          {!entry.is_new && <Value field={change.field} value={change.before} />}
                        </td>
                        <td className="py-2">
                          <Value field={change.field} value={change.after} />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </li>
          );
        })}
      </ul>

      <AdminConfirmDialog
        open={confirming?.action === "publish-all"}
        title={`Publish all ${entries.length} changes?`}
        confirmLabel="Publish all"
        busy={busy === "all"}
        onConfirm={() => run("all", () => publishEntries(entries.map(toPublishEntry)))}
        onCancel={() => setConfirming(null)}
      >
        Everything on this page goes live together. If anything changed since you loaded the page,
        nothing is published.
      </AdminConfirmDialog>
      <AdminConfirmDialog
        open={confirming?.action === "discard"}
        title="Discard this change?"
        confirmLabel="Discard"
        busy={busy !== null}
        onConfirm={() => {
          if (confirming?.action !== "discard") return;
          const { entry } = confirming;
          void run(entryKey(entry), () => discardDraft(entry.kind, entry.id ?? undefined));
        }}
        onCancel={() => setConfirming(null)}
      >
        {confirming?.action === "discard" && confirming.entry.is_new
          ? "It has never been published, so it will be archived."
          : "The unpublished edits will be lost and the live version stays as it is."}
      </AdminConfirmDialog>
    </div>
  );
}
