import { cookies, draftMode } from "next/headers";

/**
 * In draft mode (Phase 25 Decision 21), public fetchers read drafts from
 * the admin preview API with the visitor's admin session cookie. Draft
 * mode is only ever enabled on the cms host, after a session check.
 */
export async function previewFetchInit(): Promise<RequestInit | null> {
  if (!(await draftMode()).isEnabled) return null;
  const cookie = (await cookies()).toString();
  return { headers: { cookie }, cache: "no-store" };
}

export async function isPreview(): Promise<boolean> {
  return (await draftMode()).isEnabled;
}
