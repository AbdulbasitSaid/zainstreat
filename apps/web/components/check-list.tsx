function CheckIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className="h-full w-full"
    >
      <path d="M5 12.5 9.5 17 19 7" />
    </svg>
  );
}

export function CheckList({ items }: { items: string[] }) {
  return (
    <ul className="grid list-none grid-cols-1 gap-3 p-0 sm:grid-cols-2">
      {items.map((item) => (
        <li
          key={item}
          className="flex items-center gap-3 rounded-card border border-text/10 bg-card px-4 py-3.5"
        >
          <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary p-1 text-white">
            <CheckIcon />
          </span>
          {item}
        </li>
      ))}
    </ul>
  );
}
