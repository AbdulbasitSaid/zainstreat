"use client";

import { useRef } from "react";
import Image, { type StaticImageData } from "next/image";
import { gsap, useGSAP } from "@/lib/gsap";

export function SiteImage({
  src,
  alt,
  className = "",
  priority = false,
  float = false,
}: {
  // A bundled default, or an admin-uploaded URL (Phase 25 Decision 13).
  src: StaticImageData | string;
  alt: string;
  className?: string;
  priority?: boolean;
  float?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      if (!float) return;
      const mm = gsap.matchMedia();
      mm.add("(prefers-reduced-motion: no-preference)", () => {
        gsap.to(ref.current, { y: 10, duration: 3, ease: "sine.inOut", repeat: -1, yoyo: true });
      });
      return () => mm.revert();
    },
    { scope: ref, dependencies: [float] },
  );

  return (
    <div ref={ref} className={`relative min-h-[200px] overflow-hidden rounded-card ${className}`.trim()}>
      <Image
        src={src}
        alt={alt}
        fill
        sizes="(max-width: 768px) 100vw, 50vw"
        className="object-cover"
        // Remote uploads have no build-time blur data. Dev's http://
        // localhost:8080 API isn't reachable from the web container's
        // optimizer, as in MenuItemCard.
        placeholder={typeof src === "string" ? "empty" : "blur"}
        unoptimized={typeof src === "string" && src.startsWith("http://")}
        priority={priority}
      />
    </div>
  );
}
