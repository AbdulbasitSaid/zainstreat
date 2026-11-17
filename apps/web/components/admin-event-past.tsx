"use client";

import { useState, type ChangeEvent } from "react";
import Image from "next/image";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";
import { EVENT_LOCALES, toInput, type AdminEvent, type EventLocale } from "@/lib/admin-events";

const MAX_PHOTOS = 30;
const LOCALE_NAMES: Record<EventLocale, string> = { en: "English", nl: "Dutch", fr: "French" };

type Photo = { image_url: string; caption: string };

/**
 * After the event (Phase 31 Decision 13): the recap per language and an
 * ordered photo gallery, shown on the public event page once it's past.
 */
export function AdminEventPast({ event }: { event: AdminEvent }) {
  const router = useRouter();
  const [recap, setRecap] = useState<Record<EventLocale, string>>({
    en: event.content.en.recap,
    nl: event.content.nl.recap,
    fr: event.content.fr.recap,
  });
  const [photos, setPhotos] = useState<Photo[]>(event.photos.map((p) => ({ image_url: p.image_url, caption: p.caption ?? "" })));
  const [uploading, setUploading] = useState(0);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<"saved" | "failed" | null>(null);

  async function upload(changeEvent: ChangeEvent<HTMLInputElement>) {
    const files = [...(changeEvent.target.files ?? [])].slice(0, MAX_PHOTOS - photos.length);
    changeEvent.target.value = "";
    setUploading(files.length);
    for (const file of files) {
      const form = new FormData();
      form.append("file", file, file.name);
      const response = await fetch("/api/admin/media", { method: "POST", body: form });
      const body: unknown = await response.json().catch(() => null);
      if (response.ok && typeof body === "object" && body !== null && "url" in body && typeof body.url === "string") {
        const url = body.url;
        setPhotos((current) => [...current, { image_url: url, caption: "" }]);
      } else {
        setStatus("failed");
      }
      setUploading((count) => count - 1);
    }
  }

  const move = (index: number, delta: number) =>
    setPhotos((current) => {
      const next = [...current];
      const [photo] = next.splice(index, 1);
      next.splice(index + delta, 0, photo);
      return next;
    });

  async function save() {
    setSaving(true);
    setStatus(null);
    const input = toInput(event);
    for (const code of EVENT_LOCALES) input.content[code] = { ...input.content[code], recap: recap[code] };
    const [details, gallery] = await Promise.all([
      fetch(`/api/admin/events/${event.id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      }),
      fetch(`/api/admin/events/${event.id}/photos`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(photos.map((p) => ({ image_url: p.image_url, caption: p.caption || null }))),
      }),
    ]);
    setSaving(false);
    setStatus(details.ok && gallery.ok ? "saved" : "failed");
    router.refresh();
  }

  return (
    <div className="flex flex-col gap-8">
      {!event.is_past && (
        <Notice>The recap and photos show on the public page once the event is over. You can prepare them now.</Notice>
      )}

      <section className="flex flex-col gap-4">
        <h2 className="m-0 text-xl">Recap</h2>
        {EVENT_LOCALES.map((code) => (
          <div key={code}>
            <label htmlFor={`recap-${code}`} className="mb-1.5 block text-sm font-semibold">
              {LOCALE_NAMES[code]}
              {code !== "en" && <span className="font-normal text-text-muted"> (blank shows the English)</span>}
            </label>
            <textarea
              id={`recap-${code}`}
              rows={4}
              className="field"
              value={recap[code]}
              onChange={(e) => setRecap((current) => ({ ...current, [code]: e.target.value }))}
            />
          </div>
        ))}
      </section>

      <section className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="m-0 text-xl">
            Photos <span className="text-base text-text-muted">({photos.length} / {MAX_PHOTOS})</span>
          </h2>
          <label className="cursor-pointer rounded-full border-2 border-primary px-5 py-2 text-sm font-semibold text-primary hover:bg-primary/10">
            {uploading > 0 ? `Uploading ${uploading}…` : "Add photos"}
            <input type="file" accept="image/jpeg,image/png,image/webp" multiple className="sr-only" onChange={(e) => void upload(e)} disabled={photos.length >= MAX_PHOTOS} />
          </label>
        </div>
        <p className="m-0 text-sm text-text-muted">The first photo is shown largest. JPEG, PNG or WebP, up to 5 MB each.</p>
        <ol className="m-0 grid list-none gap-4 p-0 sm:grid-cols-2 lg:grid-cols-3">
          {photos.map((photo, index) => (
            <li key={`${photo.image_url}-${index}`} className="flex flex-col gap-2 rounded-card border border-text/10 p-3">
              <div className="relative aspect-square overflow-hidden rounded-xl">
                <Image src={photo.image_url} alt="" fill sizes="300px" unoptimized className="object-cover" />
              </div>
              <label htmlFor={`photo-caption-${index}`} className="sr-only">
                Caption for photo {index + 1}
              </label>
              <input
                id={`photo-caption-${index}`}
                className="field"
                placeholder="Caption (optional)"
                value={photo.caption}
                onChange={(e) => setPhotos((current) => current.map((p, i) => (i === index ? { ...p, caption: e.target.value } : p)))}
              />
              <div className="flex gap-2">
                <Button type="button" variant="secondary" className="px-3 py-1.5" disabled={index === 0} onClick={() => move(index, -1)} aria-label={`Move photo ${index + 1} earlier`}>
                  ←
                </Button>
                <Button
                  type="button"
                  variant="secondary"
                  className="px-3 py-1.5"
                  disabled={index === photos.length - 1}
                  onClick={() => move(index, 1)}
                  aria-label={`Move photo ${index + 1} later`}
                >
                  →
                </Button>
                <span className="flex-1" />
                <Button type="button" variant="outline" className="px-3 py-1.5" onClick={() => setPhotos((current) => current.filter((_, i) => i !== index))}>
                  Remove
                </Button>
              </div>
            </li>
          ))}
        </ol>
      </section>

      <div className="flex items-center gap-4">
        <Button type="button" onClick={() => void save()} disabled={saving || uploading > 0}>
          {saving ? "Saving…" : "Save recap and photos"}
        </Button>
        {status === "saved" && <span className="text-sm font-semibold text-whatsapp-dark">Saved</span>}
        {status === "failed" && <span className="text-sm font-semibold text-primary">Something didn&apos;t save. Try again.</span>}
      </div>
    </div>
  );
}
