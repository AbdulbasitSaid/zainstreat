import Link from "next/link";
import type { Metadata } from "next";
import { AdminDeviceSimulator } from "@/components/admin-device-simulator";
import { normalizePreviewPath } from "@/lib/preview-paths";

export const metadata: Metadata = {
  title: "Preview | Zain's Admin",
};

// Full-screen simulator (Phase 25 plan 10.5), at /preview-all because
// proxy.ts claims /preview/ for the frames themselves.
export default async function AdminPreviewAllPage({
  searchParams,
}: {
  searchParams: Promise<{ path?: string | string[] }>;
}) {
  const { path } = await searchParams;
  const initialPath = normalizePreviewPath(Array.isArray(path) ? path[0] : (path ?? null));

  return (
    <div className="flex h-[calc(100dvh-5rem)] flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="m-0 text-2xl">Preview</h1>
        <Link href="/changes" className="text-sm font-semibold text-primary underline">
          ‹ Back to changes
        </Link>
      </div>
      <div className="min-h-0 flex-1">
        <AdminDeviceSimulator initialPath={path === undefined ? undefined : (initialPath ?? "")} />
      </div>
    </div>
  );
}
