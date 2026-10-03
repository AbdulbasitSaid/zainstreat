import Image from "next/image";
import { Link } from "@/i18n/navigation";
import logoImage from "@/public/logo.png";

export function Logo() {
  return (
    <Link href="/" className="inline-flex items-center" aria-label="Zain's Treat n More — Home">
      <Image src={logoImage} alt="Zain's Treat n More" width={53} height={48} priority />
    </Link>
  );
}
