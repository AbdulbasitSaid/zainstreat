import type { ComponentProps, ComponentPropsWithoutRef } from "react";
import { Link } from "@/i18n/navigation";
import { LiquidSurface } from "@/components/liquid-surface";

export type ButtonVariant = "primary" | "secondary" | "outline" | "invert";
export type ButtonColor = "brand" | "whatsapp";

const BASE_CLASSES =
  "group/liquid relative isolate inline-flex items-center justify-center gap-2 rounded-full px-7 py-3.5 " +
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

// Each variant's look redrawn as a <LiquidSurface /> (fill, border and
// hover), which takes over from VARIANT_CLASSES' background and border once
// it's running. Mirrors VARIANT_CLASSES; keep the two in step.
type LiquidLook = { pathClassName: string; splashClassName: string; strokeWidth?: number };

const FILL_TRANSITION = "transition-[fill] duration-150";

const LIQUID_LOOKS: Record<ButtonVariant, (color: ButtonColor) => LiquidLook> = {
  primary: () => ({
    pathClassName: `fill-primary group-hover/liquid:fill-primary-light ${FILL_TRANSITION}`,
    splashClassName: "fill-primary",
  }),
  secondary: () => ({
    pathClassName: `fill-text/0 stroke-text group-hover/liquid:fill-text ${FILL_TRANSITION}`,
    splashClassName: "fill-text",
    strokeWidth: 1.5,
  }),
  outline: (color) =>
    color === "whatsapp"
      ? {
          pathClassName: `fill-whatsapp-dark/0 stroke-whatsapp-dark group-hover/liquid:fill-whatsapp-dark/10 ${FILL_TRANSITION}`,
          splashClassName: "fill-whatsapp",
          strokeWidth: 2,
        }
      : {
          pathClassName: `fill-primary/0 stroke-primary group-hover/liquid:fill-primary/10 ${FILL_TRANSITION}`,
          splashClassName: "fill-primary",
          strokeWidth: 2,
        },
  invert: () => ({
    pathClassName: `fill-white group-hover/liquid:fill-accent-light ${FILL_TRANSITION}`,
    splashClassName: "fill-white",
  }),
};

// The liquid stand-in for the base classes' hover shadow.
function ButtonLiquid({ variant, color }: { variant: ButtonVariant; color: ButtonColor }) {
  return (
    <LiquidSurface
      {...LIQUID_LOOKS[variant](color)}
      className="transition-[filter] duration-150 group-hover/liquid:drop-shadow-lg"
    />
  );
}

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
      <ButtonLiquid variant={variant} color={color} />
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
      <ButtonLiquid variant={variant} color={color} />
    </Link>
  );
}

// A button-styled plain <a>, for external links (e.g. WhatsApp) that
// shouldn't go through the locale-aware Link.
export function ButtonAnchor({
  variant = "primary",
  color = "brand",
  className,
  children,
  ...rest
}: ButtonOwnProps & ComponentPropsWithoutRef<"a">) {
  return (
    <a role="button" className={buttonClasses({ variant, color, className })} {...rest}>
      {children}
      {hasArrow(variant) && <ArrowGlyph />}
      <ButtonLiquid variant={variant} color={color} />
    </a>
  );
}
