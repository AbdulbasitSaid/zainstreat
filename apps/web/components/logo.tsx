import Image from "next/image";
import { Link } from "@/i18n/navigation";
import logoImage from "@/public/logo.jpg";

export function Logo() {
  return (
    <Link href="/" className="inline-flex items-center" aria-label="Zain's Treat n More — Home">
      <Image
        src={logoImage}
        alt="Zain's Treat n More"
        width={48}
        height={48}
        priority
        className="rounded-full"
      />
    </Link>
  );
}
