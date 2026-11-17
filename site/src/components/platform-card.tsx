"use client";

import type { Asset, AssetKey } from "@/lib/platforms";
import { cn } from "@/lib/cn";
import { Icon, type IconName } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";

const notes: Record<AssetKey, string> = {
  "windows-x86_64":
    "SmartScreen pedirá confirmación la primera vez. Más información → Ejecutar.",
  "macos-aarch64":
    "Gatekeeper: clic derecho sobre la app → Abrir. El binario no está notarizado.",
  "linux-x86_64": "Marca el binario como ejecutable con chmod +x usdx-desktop.",
};

const icons: Record<AssetKey, IconName> = {
  "windows-x86_64": "windows",
  "macos-aarch64": "command",
  "linux-x86_64": "terminal",
};

type Props = {
  asset: Asset;
  downloadUrl: string | null;
  size: string;
  selected: boolean;
  recommended: boolean;
  onSelect: (key: AssetKey) => void;
  fallbackUrl: string;
};

export function PlatformCard({
  asset,
  downloadUrl,
  size,
  selected,
  recommended,
  onSelect,
  fallbackUrl,
}: Props) {
  const id = `platform-${asset.key}`;

  return (
    <div
      className={cn(
        "flex flex-col border transition-colors duration-150",
        selected
          ? "border-accent/70 bg-accent/[0.045]"
          : "border-border bg-surface/40 hover:border-border-hover",
      )}
    >
      <input
        type="radio"
        id={id}
        name="platform"
        value={asset.key}
        checked={selected}
        onChange={() => onSelect(asset.key)}
        className="peer sr-only platform-radio"
      />

      <label
        htmlFor={id}
        className="flex flex-1 cursor-pointer flex-col p-5"
      >
        <div className="flex items-start justify-between gap-3">
          <span className="grid size-9 place-items-center border border-border bg-surface-2 text-text-2">
            <Icon name={icons[asset.key]} size={16} />
          </span>
          {recommended ? (
            <span className="border border-accent/40 bg-accent/10 px-1.5 py-1 font-mono text-[8px] tracking-[0.12em] text-accent-text uppercase">
              Tu sistema
            </span>
          ) : null}
        </div>

        <h3 className="mt-4 text-[19px] font-semibold tracking-[-0.02em]">
          {asset.label}
        </h3>
        <p className="mt-1 font-mono text-[10px] tracking-[0.04em] text-text-3">
          {asset.platform} · {asset.arch}
        </p>

        <p className="mt-3 flex-1 text-caption leading-[1.6] text-text-2">
          {notes[asset.key]}
        </p>

        <p className="mt-4 truncate font-mono text-[9px] text-text-3">
          {asset.file}
        </p>
      </label>

      <div
        className={cn(
          "flex items-center justify-between gap-3 border-t px-5 py-3",
          selected ? "border-accent/30" : "border-border",
        )}
      >
        {downloadUrl ? (
          <a
            href={downloadUrl}
            className="inline-flex items-center gap-1.5 text-caption font-semibold text-text transition-colors duration-150 hover:text-accent-text"
          >
            Descargar ZIP
            <Icon name="download" size={12} />
          </a>
        ) : (
          <a
            href={fallbackUrl}
            target="_blank"
            rel="noreferrer noopener"
            className="inline-flex items-center gap-1.5 text-caption font-semibold text-text-2 transition-colors duration-150 hover:text-text"
          >
            Ver releases
            <Icon name="external" size={11} />
          </a>
        )}
        <MonoLabel className="text-[9px] tracking-[0.08em]">{size}</MonoLabel>
      </div>
    </div>
  );
}
