"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";

type Action = "publish" | "unpublish" | "archive" | "unarchive";

// Publish, unpublish and archive an event (Phase 31 Decision 12).
export function AdminEventActions({ id, published, archived }: { id: number; published: boolean; archived: boolean }) {
  const router = useRouter();
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  async function run(action: Action) {
    setBusy(true);
    setFailed(false);
    const response = await fetch(`/api/admin/events/${id}/${action}`, { method: "POST" });
    setBusy(false);
    if (!response.ok) {
      setFailed(true);
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
      {failed && <span className="text-sm font-semibold text-primary">That didn&apos;t work. Try again.</span>}
    </div>
  );
}
