import Image from "next/image";

export function PlaceholderImage({ label, className = "" }: { label: string; className?: string }) {
  return (
    <div className={`relative min-h-[200px] overflow-hidden rounded-card ${className}`.trim()}>
      <Image
        src={`https://placehold.co/800x800?text=${encodeURIComponent(label)}`}
        alt={label}
        fill
        sizes="(max-width: 768px) 100vw, 50vw"
        className="object-cover"
      />
    </div>
  );
}
