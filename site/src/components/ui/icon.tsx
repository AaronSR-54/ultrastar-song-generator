import type { SVGProps } from "react";

export type IconName =
  | "music"
  | "download"
  | "arrow-right"
  | "external"
  | "check"
  | "plus"
  | "minus"
  | "folder"
  | "play"
  | "waveform"
  | "monitor"
  | "windows"
  | "command"
  | "terminal"
  | "youtube"
  | "settings"
  | "disc"
  | "lock"
  | "cpu"
  | "file"
  | "alert"
  | "skip-back"
  | "skip-forward";

type IconProps = {
  name: IconName;
  size?: number;
  strokeWidth?: number;
} & Omit<SVGProps<SVGSVGElement>, "name" | "width" | "height">;

const paths: Record<IconName, React.ReactNode> = {
  music: (
    <>
      <path d="M9 18V5.6l10-2v12.8" />
      <circle cx="6.4" cy="18" r="2.6" />
      <circle cx="16.4" cy="16.4" r="2.6" />
    </>
  ),
  download: (
    <>
      <path d="M12 3v12" />
      <path d="m7 10 5 5 5-5" />
      <path d="M4 17v3h16v-3" />
    </>
  ),
  "arrow-right": (
    <>
      <path d="M3.5 12h17" />
      <path d="m13.5 5 7 7-7 7" />
    </>
  ),
  external: (
    <>
      <path d="M14 4h6v6" />
      <path d="M20 4 10.5 13.5" />
      <path d="M19 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5" />
    </>
  ),
  check: <path d="m4 12.5 5 5L20 6.5" />,
  plus: (
    <>
      <path d="M12 4v16" />
      <path d="M4 12h16" />
    </>
  ),
  minus: <path d="M4 12h16" />,
  folder: (
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
  ),
  play: <path d="M8 5.5v13l11-6.5Z" />,
  waveform: (
    <path d="M2 12h2l2-6 3 13 3-17 3 13 2-3h5" />
  ),
  monitor: (
    <>
      <rect x="3" y="4" width="18" height="13" />
      <path d="M9 21h6" />
      <path d="M12 17v4" />
    </>
  ),
  windows: (
    <>
      <path d="M4 5.4 11 4v7H4Z" />
      <path d="M13 3.7 20 2.4V11h-7Z" />
      <path d="M4 12.5h7v7.1L4 18.3Z" />
      <path d="M13 12.5h7v8.2l-7-1.3Z" />
    </>
  ),
  command: (
    <path d="M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3Z" />
  ),
  terminal: (
    <>
      <path d="m5 7 4 5-4 5" />
      <path d="M12 17h7" />
      <path d="M3 3h18v18H3Z" />
    </>
  ),
  youtube: (
    <>
      <rect x="2.5" y="5.5" width="19" height="13" />
      <path d="m10 9 5 3-5 3Z" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9 7 7M17 17l2.1 2.1M19.1 4.9 17 7M7 17l-2.1 2.1" />
    </>
  ),
  disc: (
    <>
      <circle cx="12" cy="12" r="9" />
      <circle cx="12" cy="12" r="2.5" />
    </>
  ),
  lock: (
    <>
      <rect x="4" y="10" width="16" height="11" />
      <path d="M8 10V7a4 4 0 0 1 8 0v3" />
    </>
  ),
  cpu: (
    <>
      <rect x="6" y="6" width="12" height="12" />
      <rect x="9.5" y="9.5" width="5" height="5" />
      <path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4" />
    </>
  ),
  file: (
    <>
      <path d="M5 3h9l5 5v13H5Z" />
      <path d="M14 3v5h5" />
    </>
  ),
  alert: (
    <>
      <path d="M12 3 2 21h20Z" />
      <path d="M12 9v6" />
      <path d="M12 18.5v.5" />
    </>
  ),
  "skip-back": (
    <>
      <path d="M19 5v14L9 12Z" />
      <path d="M5 5v14" />
    </>
  ),
  "skip-forward": (
    <>
      <path d="M5 5v14l10-7Z" />
      <path d="M19 5v14" />
    </>
  ),
};

export function Icon({
  name,
  size = 16,
  strokeWidth = 1.7,
  className,
  ...rest
}: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
      {...rest}
    >
      {paths[name]}
    </svg>
  );
}
