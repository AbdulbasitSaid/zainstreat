"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";

type Action = "publish" | "unpublish" | "archive" | "unarchive";

// Publish, unpublish and archive an event (Phase 31 Decision 12).
export function AdminEventActions({ id, published, archived }: { id: number; published: boolean; archived: boolean }) {
  const router = useRouter();
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);

  async function run(action: Action) {
    setBusy(true);
    setFailed(null);
    const response = await fetch(`/api/admin/events/${id}/${action}`, { method: "POST" });
    setBusy(false);
    if (!response.ok) {
      // Publishing needs the private address (Phase 32 Decision 12) and a
      // plate (Phase 35 Decision 6).
      const body: unknown = await response.json().catch(() => null);
      const fields = response.status === 400 ? JSON.stringify(body) : "";
      const missing = [
        fields.includes('"address"') ? "the address" : null,
        fields.includes('"plates"') ? "a plate" : null,
      ].filter((item): item is string => item !== null);
      setFailed(missing.length > 0 ? `Add ${missing.join(" and ")} on the details tab before publishing.` : "That didn't work. Try again.");
      return;
    }
    router.refresh();
  }

  return (
    <div className="flex flex-wrap items-center gap-3">
      {!archived &&
        (published ? (
          <Button type="button" variant="secondary" disabled={busy} onClick={() => void run("unpublish")}>
            Unpublish
          </Button>
        ) : (
          <Button type="button" disabled={busy} onClick={() => void run("publish")}>
            Publish
          </Button>
        ))}
      <Button type="button" variant="outline" disabled={busy} onClick={() => void run(archived ? "unarchive" : "archive")}>
        {archived ? "Unarchive" : "Archive"}
      </Button>
      {failed && (
        <span role="alert" className="text-sm font-semibold text-primary">
          {failed}
        </span>
      )}
    </div>
  );
}
