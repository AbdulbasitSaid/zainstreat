import { readFile } from "node:fs/promises";
import path from "node:path";
import { Document, Font, Image, Page, StyleSheet, Text, View, renderToBuffer } from "@react-pdf/renderer";
import type { AdminAttendees, Attendee, AttendeePlateCount } from "@/lib/admin-events";
import { PDF_COLORS } from "@/lib/pdf-theme";

// Phase 38 Decisions 27, 28: the attendee list as a real PDF file, in the
// site's fonts and colours. Static TTFs: react-pdf can't use next/font or
// variable fonts.
const FONT_DIR = path.join(process.cwd(), "assets/fonts");

Font.register({
  family: "Fraunces",
  fonts: [
    { src: path.join(FONT_DIR, "Fraunces-SemiBold.ttf"), fontWeight: 600 },
    { src: path.join(FONT_DIR, "Fraunces-Bold.ttf"), fontWeight: 700 },
  ],
});
Font.register({
  family: "Work Sans",
  fonts: [
    { src: path.join(FONT_DIR, "WorkSans-Regular.ttf"), fontWeight: 400 },
    { src: path.join(FONT_DIR, "WorkSans-SemiBold.ttf"), fontWeight: 600 },
  ],
});
// Names and notes wrap whole words, never hyphenated.
Font.registerHyphenationCallback((word) => [word]);

const COLUMNS = { tick: 26, name: 120, guest: 46, plate: 150, phone: 82 } as const;

const styles = StyleSheet.create({
  page: {
    paddingTop: 36,
    paddingBottom: 48,
    paddingHorizontal: 36,
    fontFamily: "Work Sans",
    fontSize: 9.5,
    color: PDF_COLORS.text,
  },
  header: {
    flexDirection: "row",
    alignItems: "center",
    gap: 16,
    backgroundColor: PDF_COLORS.primary,
    borderRadius: 14,
    paddingVertical: 16,
    paddingHorizontal: 20,
    color: PDF_COLORS.white,
  },
  logo: { width: 52, height: 52, borderRadius: 26, backgroundColor: PDF_COLORS.white, padding: 3, objectFit: "contain" },
  eyebrow: { fontSize: 8, fontWeight: 600, letterSpacing: 1.6, textTransform: "uppercase", opacity: 0.8 },
  title: { fontFamily: "Fraunces", fontWeight: 700, fontSize: 22, marginTop: 2 },
  subtitle: { fontSize: 10, marginTop: 3, opacity: 0.9 },
  plates: {
    marginTop: 14,
    backgroundColor: PDF_COLORS.cream,
    borderRadius: 12,
    paddingVertical: 10,
    paddingHorizontal: 14,
  },
  platesTitle: { fontFamily: "Fraunces", fontWeight: 600, fontSize: 12, marginBottom: 6 },
  pills: { flexDirection: "row", flexWrap: "wrap", gap: 6 },
  pill: {
    backgroundColor: PDF_COLORS.accentLight,
    color: PDF_COLORS.primaryDark,
    borderRadius: 999,
    paddingVertical: 3,
    paddingHorizontal: 9,
    fontSize: 9,
  },
  pillCount: { fontWeight: 600 },
  tableHead: {
    flexDirection: "row",
    borderBottomWidth: 1.5,
    borderBottomColor: PDF_COLORS.primary,
    paddingVertical: 6,
    color: PDF_COLORS.primary,
    fontSize: 8,
    fontWeight: 600,
    letterSpacing: 0.8,
    textTransform: "uppercase",
  },
  row: { flexDirection: "row", paddingVertical: 6, borderBottomWidth: 0.5, borderBottomColor: "#E5D9DB" },
  rowSoft: { backgroundColor: PDF_COLORS.backgroundSoft },
  partyStart: { borderTopWidth: 0.75, borderTopColor: "#C9B3B8" },
  cell: { paddingRight: 8 },
  tick: { width: 12, height: 12, borderWidth: 1.5, borderColor: PDF_COLORS.primary, borderRadius: 3, marginLeft: 4 },
  name: { fontWeight: 600 },
  muted: { color: PDF_COLORS.textMuted },
  empty: { marginTop: 20, color: PDF_COLORS.textMuted },
  footer: {
    position: "absolute",
    bottom: 22,
    left: 36,
    right: 36,
    flexDirection: "row",
    justifyContent: "space-between",
    borderTopWidth: 0.5,
    borderTopColor: "#D8C8CB",
    paddingTop: 6,
    fontSize: 8,
    color: PDF_COLORS.textMuted,
  },
});

const PRINTED_AT = new Intl.DateTimeFormat("en-GB", {
  timeZone: "Europe/Amsterdam",
  day: "numeric",
  month: "short",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

function plateLabel(count: Pick<AttendeePlateCount, "plate_name" | "choices">): string {
  const name = count.plate_name ?? "No meal";
  return count.choices ? `${name} (${count.choices})` : name;
}

function GuestRow({ guest, index }: { guest: Attendee; index: number }) {
  const first = guest.guest_no === 1;
  return (
    <View
      wrap={false}
      style={[styles.row, ...(index % 2 === 1 ? [styles.rowSoft] : []), ...(first && index > 0 ? [styles.partyStart] : [])]}
    >
      <View style={[styles.cell, { width: COLUMNS.tick }]}>
        <View style={styles.tick} />
      </View>
      <Text style={[styles.cell, styles.name, { width: COLUMNS.name }]}>{first ? guest.booker_name : ""}</Text>
      <Text style={[styles.cell, styles.muted, { width: COLUMNS.guest }]}>
        {guest.guest_no} of {guest.party_size}
      </Text>
      <Text style={[styles.cell, { width: COLUMNS.plate }]}>{plateLabel(guest)}</Text>
      <Text style={[styles.cell, { width: COLUMNS.phone }]}>{first ? guest.phone : ""}</Text>
      <Text style={{ flex: 1 }}>{guest.note ?? ""}</Text>
    </View>
  );
}

export function AttendeePdf({ data, logo, printedAt }: { data: AdminAttendees; logo: Buffer | null; printedAt: Date }) {
  const guests = data.confirmed_guests === 1 ? "guest" : "guests";
  return (
    <Document title={`${data.event.slug}-attendees`} author="Zain's Treat n More" language="en">
      <Page size="A4" style={styles.page}>
        <View style={styles.header}>
          {/* react-pdf's Image has no alt; the PDF is for printing. */}
          {/* eslint-disable-next-line jsx-a11y/alt-text */}
          {logo && <Image src={logo} style={styles.logo} />}
          <View style={{ flex: 1 }}>
            <Text style={styles.eyebrow}>Check-in sheet</Text>
            <Text style={styles.title}>{data.event.title}</Text>
            <Text style={styles.subtitle}>
              {data.event.starts_at_label} · {data.event.area} · {data.confirmed_guests} {guests}
            </Text>
          </View>
        </View>

        {data.plate_counts.length > 0 && (
          <View style={styles.plates} wrap={false}>
            <Text style={styles.platesTitle}>Plates</Text>
            <View style={styles.pills}>
              {data.plate_counts.map((count) => (
                <Text key={`${count.plate_name ?? "none"}-${count.choices}`} style={styles.pill}>
                  {plateLabel(count)} <Text style={styles.pillCount}>× {count.count}</Text>
                </Text>
              ))}
            </View>
          </View>
        )}

        {data.guests.length === 0 ? (
          <Text style={styles.empty}>No confirmed guests yet.</Text>
        ) : (
          <View style={{ marginTop: 16 }}>
            {/* `fixed`: repeats at the top of every page the table runs onto. */}
            <View style={styles.tableHead} fixed>
              {/* The tick-box column; Work Sans has no ✓ glyph. */}
              <Text style={[styles.cell, { width: COLUMNS.tick }]} />
              <Text style={[styles.cell, { width: COLUMNS.name }]}>Name</Text>
              <Text style={[styles.cell, { width: COLUMNS.guest }]}>Guest</Text>
              <Text style={[styles.cell, { width: COLUMNS.plate }]}>Plate</Text>
              <Text style={[styles.cell, { width: COLUMNS.phone }]}>Phone</Text>
              <Text style={{ flex: 1 }}>Note</Text>
            </View>
            {data.guests.map((guest, index) => (
              <GuestRow key={`${guest.booker_name}-${guest.payment_reference ?? ""}-${guest.guest_no}-${index}`} guest={guest} index={index} />
            ))}
          </View>
        )}

        <View style={styles.footer} fixed>
          <Text>Zain&apos;s Treat n More · printed {PRINTED_AT.format(printedAt)}</Text>
          <Text render={({ pageNumber, totalPages }) => `page ${pageNumber} of ${totalPages}`} />
        </View>
      </Page>
    </Document>
  );
}

// react-pdf draws PNG and JPEG only, and embeds them at full size; anything
// else, or anything big enough to bloat the email attachment, falls back to
// the bundled 200px logo.
const MAX_LOGO_BYTES = 300_000;

function isPngOrJpeg(bytes: Buffer): boolean {
  const png = bytes.subarray(0, 4).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47]));
  const jpeg = bytes[0] === 0xff && bytes[1] === 0xd8;
  return png || jpeg;
}

/** The site's logo (Site content), else assets/pdf-logo.png, else none. */
export async function loadPdfLogo(url: string | null): Promise<Buffer | null> {
  if (url) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(5000), cache: "no-store" });
      if (response.ok) {
        const bytes = Buffer.from(await response.arrayBuffer());
        if (bytes.length <= MAX_LOGO_BYTES && isPngOrJpeg(bytes)) return bytes;
      }
    } catch {
      // Fall through to the bundled logo.
    }
  }
  try {
    return await readFile(path.join(process.cwd(), "assets/pdf-logo.png"));
  } catch {
    return null;
  }
}

export function renderAttendeePdf(data: AdminAttendees, logo: Buffer | null): Promise<Buffer> {
  return renderToBuffer(<AttendeePdf data={data} logo={logo} printedAt={new Date()} />);
}

export function attendeePdfFilename(data: AdminAttendees): string {
  return `${data.event.slug}-attendees.pdf`;
}
