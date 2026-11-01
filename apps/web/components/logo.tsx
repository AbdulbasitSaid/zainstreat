import Image from "next/image";
import { Link } from "@/i18n/navigation";
import logoImage from "@/public/logo.png";

// `src` is the admin-uploaded logo (Phase 25), if any; its shape is
// unknown, so it's fitted into the bundled logo's box.
export function Logo({ src }: { src?: string | null }) {
  return (
    <Link href="/" className="inline-flex items-center" aria-label="Zain's Treat n More — Home">
      {src ? (
        <Image
          src={src}
          alt="Zain's Treat n More"
          width={53}
          height={48}
          priority
          unoptimized={src.startsWith("http://")}
          className="h-12 w-auto max-w-[8rem] object-contain"
        />
      ) : (
        <Image src={logoImage} alt="Zain's Treat n More" width={53} height={48} priority />
      )}
    </Link>
  );
}
