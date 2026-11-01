// Client-side calls for Phase 25's Publish and Discard (requirement.md
// Decision 19), through the same-origin admin proxies.

export type DraftKind = "site_content" | "menu_item" | "category";

export interface PublishEntry {
  kind: DraftKind;
  id?: number;
  draft_updated_at: string | null;
}

export type DraftActionResult = "ok" | "conflict" | "invalid" | "error";

function toResult(status: number): DraftActionResult {
  if (status >= 200 && status < 300) return "ok";
  if (status === 409) return "conflict";
  if (status === 400) return "invalid";
  return "error";
}

export async function publishEntries(entries: PublishEntry[]): Promise<DraftActionResult> {
  try {
    const response = await fetch("/api/admin/publish", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entries }),
    });
    return toResult(response.status);
  } catch {
    return "error";
  }
}

export async function discardDraft(kind: DraftKind, id?: number): Promise<DraftActionResult> {
  try {
    const url = id === undefined ? `/api/admin/drafts/${kind}` : `/api/admin/drafts/${kind}/${id}`;
    const response = await fetch(url, { method: "DELETE" });
    return toResult(response.status);
  } catch {
    return "error";
  }
}

export const DRAFT_ACTION_MESSAGES: Record<Exclude<DraftActionResult, "ok">, string> = {
  conflict: "Something changed since you loaded this page — reload and try again.",
  invalid: "This change can't be published as it is — check the fields above.",
  error: "That didn't work. Please try again.",
};
