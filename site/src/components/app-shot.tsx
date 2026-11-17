import Image from "next/image";

import { cn } from "@/lib/cn";

type AppShotProps = {
  src: string;
  alt: string;
  label: string;
  caption?: string;
  priority?: boolean;
  sizes?: string;
  className?: string;
  imageClassName?: string;
};

/**
 * A real capture of the desktop app. The frame is only a hairline around the
 * screenshot: everything inside it was rendered by the app itself.
 */
export function AppShot({
  src,
  alt,
  label,
  caption = "captura real de la app",
  priority = false,
  sizes = "(min-width: 1200px) 1104px, 100vw",
  className,
  imageClassName,
}: AppShotProps) {
  return (
    <figure className={cn("min-w-0", className)}>
      <div className="border border-border bg-surface p-1">
        <Image
          src={src}
          alt={alt}
          width={1040}
          height={700}
          priority={priority}
          sizes={sizes}
          className={cn("block h-auto w-full", imageClassName)}
        />
      </div>
      <figcaption className="mt-3 flex items-baseline justify-between gap-4 font-mono text-[9px] tracking-[0.1em] text-text-3 uppercase">
        <span>{label}</span>
        <span className="text-right">{caption}</span>
      </figcaption>
    </figure>
  );
}
