import type { ReactNode } from "react";
import { Reveal } from "@/components/reveal";

export function SplitRow({
  media,
  content,
  reverse = false,
  tinted = false,
  className = "",
}: {
  media: ReactNode;
  content: ReactNode;
  reverse?: boolean;
  tinted?: boolean;
  className?: string;
}) {
  const sectionClasses = tinted
    ? `rounded-card bg-background-soft px-6 py-section md:px-12 ${className}`.trim()
    : className;

  return (
    <section className={sectionClasses || undefined}>
      <Reveal>
        <div className="grid grid-cols-1 items-center gap-8 md:grid-cols-2 md:gap-14">
          {reverse ? (
            <>
              {media}
              {content}
            </>
          ) : (
            <>
              {content}
              {media}
            </>
          )}
        </div>
      </Reveal>
    </section>
  );
}
