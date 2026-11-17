"use client";

import { useRef, useState, useSyncExternalStore } from "react";
import { QRCodeCanvas, QRCodeSVG } from "qrcode.react";
import { Button, buttonClasses } from "@/components/button";
import { EVENT_LOCALES, publicOrigin, type EventLocale } from "@/lib/admin-events";

const LOCALE_NAMES: Record<EventLocale, string> = { en: "English", nl: "Dutch", fr: "French" };

function subscribeNoop() {
  return () => {};
}

/**
 * Sharing an event (Phase 31 Decision 18): its public link per language,
 * copy, WhatsApp, and a QR code with a print-size PNG download.
 */
export function AdminEventShare({ slug, title, published }: { slug: string; title: string; published: boolean }) {
  const [locale, setLocale] = useState<EventLocale>("en");
  const [copied, setCopied] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  // window.location only exists in the browser.
  const origin = useSyncExternalStore(subscribeNoop, publicOrigin, () => "");
  const url = `${origin}/${locale}/events/${slug}`;

  async function copy() {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopied(false);
    }
  }

  function download() {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const link = document.createElement("a");
    link.href = canvas.toDataURL("image/png");
    link.download = `${slug}-${locale}-qr.png`;
    link.click();
  }

  return (
    <div className="flex flex-col gap-6">
      {!published && (
        <p className="m-0 rounded-card bg-accent-light px-5 py-4 text-sm">
          This event isn&apos;t published yet, so the link shows a 404 to guests. Publish it before sharing.
        </p>
      )}

      <div role="radiogroup" aria-label="Link language" className="flex gap-1 self-start rounded-full bg-background-soft p-1">
        {EVENT_LOCALES.map((code) => (
          <button
            key={code}
            type="button"
            role="radio"
            aria-checked={locale === code}
            onClick={() => setLocale(code)}
            className={`rounded-full px-4 py-1.5 text-sm font-semibold ${locale === code ? "bg-primary text-white" : "text-text"}`}
          >
            {LOCALE_NAMES[code]}
          </button>
        ))}
      </div>

      <div className="grid items-start gap-8 md:grid-cols-[auto_1fr]">
        <div className="flex flex-col items-center gap-3 rounded-card bg-white p-5 shadow-sm">
          {origin && <QRCodeSVG value={url} size={256} level="M" marginSize={2} title={`QR code for ${title}`} />}
          {/* Hidden, print-size copy for the PNG download. */}
          {origin && <QRCodeCanvas ref={canvasRef} value={url} size={1024} level="M" marginSize={4} className="hidden" />}
          <Button type="button" onClick={download}>
            Download PNG
          </Button>
        </div>

        <div className="flex flex-col gap-4">
          <label htmlFor="event-share-url" className="text-sm font-semibold">
            Public link
          </label>
          <input id="event-share-url" readOnly className="field" value={url} onFocus={(e) => e.target.select()} />
          <div className="flex flex-wrap gap-3">
            <Button type="button" variant="secondary" onClick={() => void copy()}>
              {copied ? "Copied!" : "Copy link"}
            </Button>
            <a
              href={`https://wa.me/?text=${encodeURIComponent(`${title}\n${url}`)}`}
              target="_blank"
              rel="noopener noreferrer"
              className={buttonClasses({ variant: "outline", color: "whatsapp" })}
            >
              Share on WhatsApp
            </a>
            <a href={url} target="_blank" rel="noopener noreferrer" className={buttonClasses({ variant: "secondary" })}>
              Open page
            </a>
          </div>
          <p className="m-0 text-sm text-text-muted">
            Print the QR code on flyers, or share the link in group chats. Guests book and pay from the page.
          </p>
        </div>
      </div>
    </div>
  );
}
