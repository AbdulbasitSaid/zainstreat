import Link from "next/link";

const TAG_CLASSES = "inline-block rounded-full bg-cream px-3 py-1 text-xs font-semibold text-text";

// Marks an order that came from an accepted catering quote (Phase 23
// Decision 8); with `enquiryId`, it links back to the enquiry.
export function AdminCateringTag({ enquiryId }: { enquiryId?: number }) {
  if (enquiryId === undefined) {
    return <span className={TAG_CLASSES}>Catering</span>;
  }
  return (
    <Link href={`/enquiries/catering/${enquiryId}`} className={`${TAG_CLASSES} underline`}>
      Catering enquiry #{enquiryId}
    </Link>
  );
}
