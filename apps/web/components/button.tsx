import type { ComponentProps, ComponentPropsWithoutRef } from "react";
import { Link } from "@/i18n/navigation";

export type ButtonVariant = "primary" | "secondary" | "outline" | "invert";
export type ButtonColor = "brand" | "whatsapp";

const BASE_CLASSES =
  "inline-flex items-center justify-center gap-2 rounded-full px-7 py-3.5 " +
  "text-sm font-semibold uppercase tracking-wider transition-[transform,box-shadow,background-color,color] " +
  "duration-150 ease-out-expo hover:-translate-y-0.5 hover:scale-[1.02] hover:shadow-lg " +
  "active:translate-y-0 active:scale-[0.98] focus-visible:outline focus-visible:outline-[3px] " +
  "focus-visible:outline-accent focus-visible:outline-offset-2 " +
  "disabled:pointer-events-none disabled:opacity-50 disabled:hover:translate-y-0 disabled:hover:scale-100 disabled:hover:shadow-none";

const VARIANT_CLASSES: Record<ButtonVariant, (color: ButtonColor) => string> = {
  primary: () => "bg-primary text-white hover:bg-primary-light",
  secondary: () => "bg-transparent text-text border-[1.5px] border-text hover:bg-text hover:text-white",
  outline: (color) =>
    color === "whatsapp"
      ? "bg-transparent border-2 border-whatsapp-dark text-whatsapp-dark hover:bg-whatsapp-dark/10"
      : "bg-transparent border-2 border-primary text-primary hover:bg-primary/10",
  invert: () => "bg-white text-primary-dark hover:bg-accent-light",
};

function hasArrow(variant: ButtonVariant) {
  return variant === "primary" || variant === "invert";
}

export function buttonClasses({
  variant = "primary",
  color = "brand",
  className = "",
}: {
  variant?: ButtonVariant;
  color?: ButtonColor;
  className?: string;
}) {
  return `${BASE_CLASSES} ${VARIANT_CLASSES[variant](color)} ${className}`.trim();
}

function ArrowGlyph() {
  return (
    <span
      aria-hidden="true"
      className="inline-flex h-[1.5em] w-[1.5em] items-center justify-center rounded-full bg-[color-mix(in_srgb,currentColor_20%,transparent)] text-[0.9em]"
    >
      →
    </span>
  );
}

type ButtonOwnProps = {
  variant?: ButtonVariant;
  color?: ButtonColor;
};

export function Button({
  variant = "primary",
  color = "brand",
  className,
  children,
  disabled,
  ...rest
}: ButtonOwnProps & ComponentPropsWithoutRef<"button">) {
  return (
    <button className={buttonClasses({ variant, color, className })} disabled={disabled} {...rest}>
      {children}
      {hasArrow(variant) && !disabled && <ArrowGlyph />}
    </button>
  );
}

export function ButtonLink({
  variant = "primary",
  color = "brand",
  className,
  children,
  ...rest
}: ButtonOwnProps & ComponentProps<typeof Link>) {
  return (
    <Link role="button" className={buttonClasses({ variant, color, className })} {...rest}>
      {children}
      {hasArrow(variant) && <ArrowGlyph />}
    </Link>
  );
}
