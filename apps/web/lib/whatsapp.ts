export const WHATSAPP_NUMBER = "31630545277";

export function buildWhatsAppLink(message: string): string {
  return buildWhatsAppLinkTo(WHATSAPP_NUMBER, message);
}

// A chat with someone else, e.g. admin sharing a quote with a customer.
export function buildWhatsAppLinkTo(number: string, message: string): string {
  return `https://wa.me/${number}?text=${encodeURIComponent(message)}`;
}

// A customer's phone as wa.me wants it: digits only, with country code.
// Dutch numbers usually come in as 06…, so a leading 0 becomes 31; null when
// what's left is too short to be a real number (Phase 23 Decision 14).
export function toWhatsAppNumber(phone: string): string | null {
  let digits = phone.replace(/\D/g, "");
  if (digits.startsWith("00")) {
    digits = digits.slice(2);
  } else if (digits.startsWith("0")) {
    digits = `31${digits.slice(1)}`;
  }
  return digits.length >= 8 ? digits : null;
}
