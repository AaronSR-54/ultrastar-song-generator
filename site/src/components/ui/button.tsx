import type {
  AnchorHTMLAttributes,
  ButtonHTMLAttributes,
  ReactNode,
} from "react";

import { cn } from "@/lib/cn";
import { Icon, type IconName } from "@/components/ui/icon";

type Variant = "primary" | "secondary" | "ghost";
type Size = "sm" | "md" | "lg";

type SharedProps = {
  variant?: Variant;
  size?: Size;
  icon?: IconName;
  iconPosition?: "leading" | "trailing";
  children: ReactNode;
};

const base =
  "group inline-flex select-none items-center justify-center gap-2 whitespace-nowrap font-semibold leading-none transition-[background-color,border-color,color,transform] duration-150 ease-out active:scale-[0.99] active:duration-75";

const sizes: Record<Size, string> = {
  sm: "h-9 px-4 text-body",
  md: "h-11 px-5 text-body",
  lg: "h-[52px] px-6 text-title",
};

const variants: Record<Variant, string> = {
  primary:
    "bg-accent text-on-accent hover:bg-accent-hover active:bg-accent-pressed",
  secondary:
    "border border-border bg-surface-2 text-text hover:border-border-hover hover:bg-surface-3 active:bg-surface-4",
  ghost: "text-text-2 hover:bg-surface-2 hover:text-text active:bg-surface-3",
};

function classes(variant: Variant, size: Size, className?: string) {
  return cn(base, sizes[size], variants[variant], className);
}

function ButtonIcon({
  icon,
  position,
}: {
  icon: IconName;
  position: "leading" | "trailing";
}) {
  return (
    <Icon
      name={icon}
      size={16}
      className={cn(
        "shrink-0 transition-transform duration-150 ease-out",
        position === "leading"
          ? "group-hover:-translate-x-[1.5px]"
          : "group-hover:translate-x-[1.5px]",
      )}
    />
  );
}

type ButtonLinkProps = SharedProps &
  Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "children">;

export function ButtonLink({
  variant = "primary",
  size = "md",
  icon,
  iconPosition = "trailing",
  className,
  children,
  ...rest
}: ButtonLinkProps) {
  return (
    <a className={classes(variant, size, className)} {...rest}>
      {icon && iconPosition === "leading" ? (
        <ButtonIcon icon={icon} position="leading" />
      ) : null}
      <span>{children}</span>
      {icon && iconPosition === "trailing" ? (
        <ButtonIcon icon={icon} position="trailing" />
      ) : null}
    </a>
  );
}

type ButtonProps = SharedProps &
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children">;

export function Button({
  variant = "primary",
  size = "md",
  icon,
  iconPosition = "trailing",
  className,
  children,
  ...rest
}: ButtonProps) {
  return (
    <button className={classes(variant, size, className)} {...rest}>
      {icon && iconPosition === "leading" ? (
        <ButtonIcon icon={icon} position="leading" />
      ) : null}
      <span>{children}</span>
      {icon && iconPosition === "trailing" ? (
        <ButtonIcon icon={icon} position="trailing" />
      ) : null}
    </button>
  );
}
