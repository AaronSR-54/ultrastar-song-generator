import { cn } from "@/lib/cn";

type BrandMarkProps = {
  size?: number;
  /**
   * The app paints two detail levels: the full mark (grille, collar and filled
   * feathers) at 22 px or more, and a compact one below that so the fine
   * lines do not turn into a blot. Keep the same threshold here.
   */
  variant?: "full" | "compact";
  className?: string;
};

const feathers = [
  "M14.27 12.62 20.32 7.36 20.08 7.04 13.33 11.38Z",
  "M10.67 11.38 3.92 7.04 3.68 7.36 9.73 12.62Z",
  "M14.51 14.32 21.18 10.78 21.02 10.42 13.89 12.88Z",
  "M10.11 12.88 2.98 10.42 2.82 10.78 9.49 14.32Z",
  "M13.96 15.98 19.92 14.9 19.88 14.5 13.84 14.42Z",
  "M10.16 14.42 4.12 14.5 4.08 14.9 10.04 15.98Z",
];

/**
 * The brand mark: an original winged microphone drawn on the app's 24 × 24
 * icon grid. It is the same symbol the desktop app paints on its home screen
 * and in the title bar, not a separate decorative logo.
 */
export function BrandMark({
  size = 18,
  variant = size >= 22 ? "full" : "compact",
  className,
}: BrandMarkProps) {
  const full = variant === "full";

  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      aria-hidden="true"
      className={cn("shrink-0", className)}
    >
      {full ? (
        <g fill="currentColor" fillOpacity="0.55">
          {feathers.map((path) => (
            <path key={path} d={path} />
          ))}
        </g>
      ) : (
        <g
          stroke="currentColor"
          strokeWidth="1.55"
          strokeLinecap="round"
        >
          <path d="M13.4 11.4 20 7.6M13.4 13.4l7-2M10.6 11.4 4 7.6M10.6 13.4l-7-2" />
        </g>
      )}

      <circle
        cx="12"
        cy="6.6"
        r="2.9"
        stroke="currentColor"
        strokeWidth="1.7"
      />

      {full ? (
        <>
          <g stroke="currentColor" strokeWidth="0.85" strokeLinecap="round">
            <path d="M10.7 5.2h2.6M10.4 6.6h3.2M10.7 8h2.6" />
          </g>
          <path d="M9.9 9.3h4.2v1.2H9.9z" fill="currentColor" />
        </>
      ) : (
        <path
          d="M10.2 9.9h3.6"
          stroke="currentColor"
          strokeWidth="1.15"
          strokeLinecap="round"
        />
      )}

      <path
        d="M10.7 10.5h2.6l-.3 8.5h-2z"
        fill="currentColor"
        stroke="currentColor"
        strokeWidth="0.6"
        strokeLinejoin="round"
      />
    </svg>
  );
}

/** The 48 px square tile the app paints on its home screen, at landing scale. */
export function BrandTile({
  size = 32,
  markSize = 19,
  className,
}: {
  size?: number;
  markSize?: number;
  className?: string;
}) {
  return (
    <span
      style={{ width: size, height: size }}
      className={cn(
        "grid shrink-0 place-items-center border border-accent/30 bg-accent/15 text-accent-text",
        className,
      )}
    >
      <BrandMark size={markSize} variant="compact" />
    </span>
  );
}
