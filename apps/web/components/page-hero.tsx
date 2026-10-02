import type { ReactNode } from "react";
import { Reveal } from "@/components/reveal";

export function PageHero({ children }: { children: ReactNode }) {
  return (
    <section>
      <Reveal>
        <div className="mx-auto max-w-[60ch] text-center">{children}</div>
      </Reveal>
    </section>
  );
}
