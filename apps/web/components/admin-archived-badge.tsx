// Phase 25: a pending draft, or a row that has never been published.
export function AdminDraftBadge({ unpublished = false }: { unpublished?: boolean }) {
  return (
    <span className="inline-block rounded-full bg-accent-light px-3 py-1 text-xs font-semibold text-text">
      {unpublished ? "Unpublished" : "Draft"}
    </span>
  );
}

export function AdminArchivedBadge() {
  return (
    <span className="inline-block rounded-full bg-text/10 px-3 py-1 text-xs font-semibold capitalize text-text-muted">
      Archived
    </span>
  );
}

// Phase 38: Dutch or French missing, out of date, or DeepL gave up.
export function AdminTranslationBadge() {
  return (
    <span className="inline-block rounded-full border border-accent px-3 py-1 text-xs font-semibold text-text">
      Needs translation
    </span>
  );
}
