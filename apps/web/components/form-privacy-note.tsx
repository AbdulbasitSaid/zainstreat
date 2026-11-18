import { useTranslations } from "next-intl";
import { Link } from "@/i18n/navigation";

// Phase 33 Decision 13: a short notice under each form's submit button. No
// consent checkbox: the lawful basis is the order or enquiry itself.
export function FormPrivacyNote({
  form,
  className = "",
}: {
  form: "order" | "catering" | "contact" | "booking";
  className?: string;
}) {
  const t = useTranslations("FormPrivacy");
  return (
    <p className={`m-0 text-sm text-text-muted ${className}`.trim()}>
      {t.rich(form, {
        link: (chunks) => (
          <Link href="/privacy" className="underline">
            {chunks}
          </Link>
        ),
      })}
    </p>
  );
}
