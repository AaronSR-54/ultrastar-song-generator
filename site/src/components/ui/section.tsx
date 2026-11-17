import { cn } from "@/lib/cn";
import { MonoLabel } from "@/components/ui/mono-label";

type SectionProps = {
  id: string;
  index: string;
  label: string;
  title: React.ReactNode;
  lead?: React.ReactNode;
  meta?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
  contentClassName?: string;
};

export function Section({
  id,
  index,
  label,
  title,
  lead,
  meta,
  children,
  className,
  contentClassName,
}: SectionProps) {
  return (
    <section
      id={id}
      aria-labelledby={`${id}-title`}
      className={cn("border-t border-border", className)}
    >
      <div className="mx-auto w-full max-w-[1200px] px-6 py-20 md:px-10 md:py-28 lg:px-12">
        <header className="grid gap-5 md:grid-cols-[160px_minmax(0,1fr)_auto] md:gap-8 lg:grid-cols-[200px_minmax(0,1fr)_auto]">
          <div className="pt-1.5">
            <MonoLabel>
              {index} — {label}
            </MonoLabel>
          </div>
          <div>
            <h2
              id={`${id}-title`}
              className="max-w-[18ch] text-[clamp(2rem,4vw,3.25rem)] font-semibold leading-[1.02] tracking-[-0.03em]"
            >
              {title}
            </h2>
            {lead ? (
              <p className="mt-5 max-w-[64ch] text-[15px] leading-[1.7] text-text-2 md:text-base">
                {lead}
              </p>
            ) : null}
          </div>
          {meta ? (
            <div className="hidden max-w-[22ch] pt-1.5 text-right md:block">
              <MonoLabel className="text-[10px] leading-[1.7] tracking-[0.1em]">
                {meta}
              </MonoLabel>
            </div>
          ) : null}
        </header>
        <div className={cn("reveal mt-12 md:mt-16", contentClassName)}>
          {children}
        </div>
      </div>
    </section>
  );
}
