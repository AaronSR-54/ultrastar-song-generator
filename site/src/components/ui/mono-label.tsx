import { cn } from "@/lib/cn";

export function MonoLabel({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "font-mono text-micro uppercase leading-none tracking-[0.12em] text-text-3",
        className,
      )}
    >
      {children}
    </span>
  );
}
