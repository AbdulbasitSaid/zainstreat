export function ImageSlot({ label }: { label: string }) {
  return (
    <div className="image-slot" role="img" aria-label={label}>
      {label}
    </div>
  );
}
