"use client";

import { useState } from "react";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { AdminConfirmDialog } from "@/components/admin-confirm-dialog";
import { DRAFT_ACTION_MESSAGES, type DraftActionResult } from "@/lib/admin-drafts";
import type { AutosaveStatus } from "@/lib/use-autosave-draft";

const STATUS_TEXT: Record<AutosaveStatus, string> = {
  idle: "",
  saving: "Saving…",
  saved: "Draft saved",
  invalid: "Not saved — fix the highlighted fields",
  error: "Couldn't save — retrying on your next change",
  conflict: "Conflict — reload",
};

/** The autosave status, shown next to an editor's title. */
export function AdminDraftStatus({ status }: { status: AutosaveStatus }) {
  if (status === "idle") return null;
  return (
    <span
      role="status"
      className={`text-sm font-semibold ${
        status === "conflict" || status === "invalid" || status === "error" ? "text-primary-dark" : "text-text-muted"
      }`}
    >
      {STATUS_TEXT[status]}
    </span>
  );
}

/**
 * "Publish this change" and "Discard draft" for one editor (Phase 25
 * Decision 20), each behind an in-page confirmation.
 */
export function AdminDraftBar({
  pending,
  isNew,
  disabled = false,
  onPublish,
  onDiscard,
}: {
  /** Whether there's anything to publish or discard. */
  pending: boolean;
  /** An unpublished row: discarding archives it. */
  isNew: boolean;
  disabled?: boolean;
  onPublish: () => Promise<DraftActionResult>;
  onDiscard: () => Promise<DraftActionResult>;
}) {
  const [confirming, setConfirming] = useState<"publish" | "discard" | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function run(action: "publish" | "discard") {
    setBusy(true);
    setMessage(null);
    const result = await (action === "publish" ? onPublish() : onDiscard());
    setBusy(false);
    setConfirming(null);
    if (result === "ok") {
      setMessage(action === "publish" ? "Published — the live site now shows this." : null);
    } else {
      setMessage(DRAFT_ACTION_MESSAGES[result]);
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap gap-3">
        <Button type="button" onClick={() => setConfirming("publish")} disabled={disabled || busy || !pending}>
          Publish this change
        </Button>
        <Button
          type="button"
          variant="secondary"
          onClick={() => setConfirming("discard")}
          disabled={disabled || busy || !pending}
        >
          {isNew ? "Discard item" : "Discard draft"}
        </Button>
      </div>
      {message && <Notice>{message}</Notice>}

      <AdminConfirmDialog
        open={confirming === "publish"}
        title="Publish this change?"
        confirmLabel="Publish"
        busy={busy}
        onConfirm={() => run("publish")}
        onCancel={() => setConfirming(null)}
      >
        Visitors will see it straight away.
      </AdminConfirmDialog>
      <AdminConfirmDialog
        open={confirming === "discard"}
        title={isNew ? "Discard this new item?" : "Discard this draft?"}
        confirmLabel="Discard"
        busy={busy}
        onConfirm={() => run("discard")}
        onCancel={() => setConfirming(null)}
      >
        {isNew
          ? "It has never been published, so it will be archived."
          : "Your unpublished edits will be lost and the live version stays as it is."}
      </AdminConfirmDialog>
    </div>
  );
}
