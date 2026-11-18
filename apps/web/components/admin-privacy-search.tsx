"use client";

import Link from "next/link";
import { useState, type FormEvent, type ReactNode } from "react";
import { AdminConfirmDialog } from "@/components/admin-confirm-dialog";
import { Button } from "@/components/button";
import { formatDateTime, formatPrice } from "@/lib/format";
import {
  anonymiseEmail,
  exportUrl,
  isEmpty,
  searchPrivacy,
  type PrivacyBlocking,
  type PrivacySearchResult,
} from "@/lib/admin-privacy";

const CELL = "py-3 pr-4";
const HEAD = "py-3 pr-4 font-semibold";

function Section({ title, count, children }: { title: string; count: number; children: ReactNode }) {
  if (count === 0) return null;
  return (
    <section className="flex flex-col gap-2">
      <h2 className="m-0 text-xl">
        {title} ({count})
      </h2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">{children}</table>
      </div>
    </section>
  );
}

function HeadRow({ columns }: { columns: string[] }) {
  return (
    <thead>
      <tr className="border-b border-text/15 text-left text-xs uppercase tracking-wider text-text-muted">
        {columns.map((column) => (
          <th key={column} scope="col" className={HEAD}>
            {column}
          </th>
        ))}
      </tr>
    </thead>
  );
}

function BlockingNotice({ blocking }: { blocking: PrivacyBlocking }) {
  return (
    <div role="alert" className="rounded-2xl border border-text/10 p-5 text-sm">
      <p className="m-0 mb-2 font-semibold">Finish or cancel these first:</p>
      <ul className="m-0 pl-5">
        {blocking.orders.map((id) => (
          <li key={`o${id}`}>
            <Link href={`/orders/${id}`} className="text-primary underline">
              Order #{id}
            </Link>{" "}
            is still open
          </li>
        ))}
        {blocking.bookings.map((id) => (
          <li key={`b${id}`}>Event booking #{id} is for an upcoming event</li>
        ))}
      </ul>
    </div>
  );
}

// Phase 33 Decision 16: search one email, export or anonymise it.
export function AdminPrivacySearch() {
  const [email, setEmail] = useState("");
  const [searched, setSearched] = useState<string | null>(null);
  const [result, setResult] = useState<PrivacySearchResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [confirmEmail, setConfirmEmail] = useState("");
  const [blocking, setBlocking] = useState<PrivacyBlocking | null>(null);

  async function search(address: string) {
    setBusy(true);
    setMessage(null);
    const found = await searchPrivacy(address);
    setBusy(false);
    if (!found) {
      setResult(null);
      setSearched(null);
      setMessage("Enter a full email address.");
      return;
    }
    setResult(found);
    setSearched(address);
    setBlocking(null);
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void search(email.trim());
  }

  async function handleAnonymise() {
    if (!searched) return;
    setBusy(true);
    const outcome = await anonymiseEmail(searched, confirmEmail);
    setBusy(false);
    if (outcome.kind === "mismatch") {
      setMessage("The two email addresses don't match. Nothing was changed.");
      return;
    }
    setConfirming(false);
    setConfirmEmail("");
    if (outcome.kind === "blocked") {
      setBlocking(outcome.blocking);
      return;
    }
    if (outcome.kind === "failed") {
      setMessage("Something went wrong. Nothing was changed. Please try again.");
      return;
    }
    await search(searched);
    setMessage(`Done. Everything for ${searched} is anonymised.`);
  }

  const blocked =
    result !== null && (result.blocking.orders.length > 0 || result.blocking.bookings.length > 0);

  return (
    <div className="flex flex-col gap-8">
      <form onSubmit={handleSubmit} className="flex max-w-xl flex-wrap items-end gap-3">
        <div className="min-w-64 flex-1">
          <label htmlFor="privacy-email" className="mb-1.5 block text-sm font-semibold">
            Customer email
          </label>
          <input
            id="privacy-email"
            type="email"
            className="field"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            autoComplete="off"
            required
          />
        </div>
        <Button type="submit" disabled={busy || email.trim() === ""}>
          {busy ? "Searching…" : "Search"}
        </Button>
      </form>

      {message && (
        <p className="m-0 font-semibold" aria-live="polite">
          {message}
        </p>
      )}

      {result && searched && isEmpty(result) && (
        <p className="m-0 text-text-muted">Nothing is stored for {searched}.</p>
      )}

      {result && searched && !isEmpty(result) && (
        <>
          <div className="flex flex-wrap gap-3">
            <a href={exportUrl(searched)} download className="font-semibold text-primary underline">
              Export everything (JSON)
            </a>
            <Button type="button" variant="secondary" onClick={() => setConfirming(true)} disabled={busy}>
              Anonymise…
            </Button>
          </div>

          {(blocking ?? (blocked ? result.blocking : null)) && (
            <BlockingNotice blocking={blocking ?? result.blocking} />
          )}

          <Section title="Orders" count={result.orders.length}>
            <HeadRow columns={["Order", "Status", "Total", "Placed"]} />
            <tbody>
              {result.orders.map((o) => (
                <tr key={o.id} className="border-b border-text/10">
                  <td className={CELL}>
                    <Link href={`/orders/${o.id}`} className="font-semibold text-primary underline">
                      #{o.id}
                    </Link>
                  </td>
                  <td className={CELL}>{o.status}</td>
                  <td className={CELL}>{formatPrice(o.total)}</td>
                  <td className="py-3">{formatDateTime(o.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </Section>

          <Section title="Event bookings" count={result.bookings.length}>
            <HeadRow columns={["Booking", "Event", "Status", "Party", "Booked"]} />
            <tbody>
              {result.bookings.map((b) => (
                <tr key={b.id} className="border-b border-text/10">
                  <td className={CELL}>#{b.id}</td>
                  <td className={CELL}>
                    <Link href={`/events/${b.event_id}`} className="font-semibold text-primary underline">
                      {b.event_slug}
                    </Link>
                  </td>
                  <td className={CELL}>{b.status}</td>
                  <td className={CELL}>{b.party_size}</td>
                  <td className="py-3">{formatDateTime(b.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </Section>

          <Section title="Catering enquiries" count={result.enquiries.length}>
            <HeadRow columns={["Enquiry", "Event", "Quote", "Submitted"]} />
            <tbody>
              {result.enquiries.map((e) => (
                <tr key={e.id} className="border-b border-text/10">
                  <td className={CELL}>
                    <Link href={`/enquiries/catering/${e.id}`} className="font-semibold text-primary underline">
                      #{e.id}
                    </Link>
                  </td>
                  <td className={CELL}>{e.event_type}</td>
                  <td className={CELL}>{e.quote_status ?? "—"}</td>
                  <td className="py-3">{formatDateTime(e.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </Section>

          <Section title="Contact messages" count={result.contact_messages.length}>
            <HeadRow columns={["Message", "Subject", "Sent"]} />
            <tbody>
              {result.contact_messages.map((m) => (
                <tr key={m.id} className="border-b border-text/10">
                  <td className={CELL}>
                    <Link href={`/enquiries/contact/${m.id}`} className="font-semibold text-primary underline">
                      #{m.id}
                    </Link>
                  </td>
                  <td className={CELL}>{m.subject}</td>
                  <td className="py-3">{formatDateTime(m.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </Section>
        </>
      )}

      <AdminConfirmDialog
        open={confirming}
        title="Anonymise this customer?"
        confirmLabel="Anonymise"
        busy={busy}
        onConfirm={() => void handleAnonymise()}
        onCancel={() => {
          setConfirming(false);
          setConfirmEmail("");
        }}
      >
        <p className="m-0 mb-3">
          Names, emails, phone numbers, addresses and notes for {searched} are removed for good, and
          their tracking links stop working. Contact messages are deleted. Order amounts stay for
          your books. This can&apos;t be undone.
        </p>
        <label htmlFor="privacy-confirm" className="mb-1.5 block font-semibold text-text">
          Type the email address again to confirm
        </label>
        <input
          id="privacy-confirm"
          type="email"
          className="field"
          value={confirmEmail}
          onChange={(event) => setConfirmEmail(event.target.value)}
          autoComplete="off"
        />
      </AdminConfirmDialog>
    </div>
  );
}
