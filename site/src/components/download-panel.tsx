"use client";

import { useEffect, useMemo, useState } from "react";

import { PlatformCard } from "@/components/platform-card";
import { ButtonLink } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { MonoLabel } from "@/components/ui/mono-label";
import {
  ASSET_KEYS,
  ASSETS,
  detectedLabel,
  detectPlatform,
  type AssetKey,
  type Detection,
} from "@/lib/platforms";
import {
  ACTIONS_URL,
  RELEASES_URL,
  formatSize,
  type ReleaseState,
} from "@/lib/release";
import { cn } from "@/lib/cn";

type UaData = {
  platform?: string;
  architecture?: string;
  bitness?: string;
};

type NavigatorWithHints = Navigator & {
  userAgentData?: UaData & {
    getHighEntropyValues?: (hints: string[]) => Promise<UaData>;
  };
};

function detect(): Promise<Detection> {
  const nav = navigator as NavigatorWithHints;
  const hints = nav.userAgentData;

  return (async () => {
    let uaData: UaData | null = null;

    if (hints?.getHighEntropyValues) {
      try {
        uaData = await Promise.race([
          hints.getHighEntropyValues([
            "architecture",
            "bitness",
            "platformVersion",
          ]),
          new Promise<never>((_, reject) => {
            window.setTimeout(reject, 400);
          }),
        ]);
      } catch {
        uaData = hints.platform ? { platform: hints.platform } : null;
      }
    } else if (hints?.platform) {
      uaData = { platform: hints.platform };
    }

    return detectPlatform({
      userAgent: navigator.userAgent,
      uaData,
      maxTouchPoints: navigator.maxTouchPoints,
    });
  })();
}

export function DownloadPanel({ release }: { release: ReleaseState }) {
  const [detection, setDetection] = useState<Detection | null>(null);
  const [selected, setSelected] = useState<AssetKey | null>(null);
  const [announcement, setAnnouncement] = useState("");

  useEffect(() => {
    let cancelled = false;

    detect().then((result) => {
      if (cancelled) return;
      setDetection(result);
      if (result.asset) {
        setSelected(result.asset);
        setAnnouncement(
          `Descarga recomendada para ${detectedLabel(result)}.`,
        );
      } else if (result.reason) {
        setAnnouncement(result.reason);
      }
    });

    return () => {
      cancelled = true;
    };
  }, []);

  const availableRelease =
    release.status === "available" ? release.release : null;

  const assetMap = useMemo(() => {
    const map = new Map<AssetKey, { url: string; size: number }>();
    for (const asset of availableRelease?.assets ?? []) {
      map.set(asset.key, { url: asset.url, size: asset.size });
    }
    return map;
  }, [availableRelease]);

  const selectedAsset = selected ? assetMap.get(selected) : undefined;
  const primaryHref = selectedAsset?.url ?? RELEASES_URL;
  const primaryLabel = selectedAsset
    ? `Descargar para ${selected ? ASSETS[selected].label : ""}`
    : availableRelease
      ? "Elegir descarga"
      : "Ver releases";

  const version =
    release.status === "available"
      ? release.release.version
      : release.status === "missing-assets"
        ? (release.version ?? "sin binarios")
        : "—";

  const statusNote = detection?.reason
    ? detection.reason
    : selectedAsset && selected
      ? `ZIP · ${formatSize(selectedAsset.size)} · recomendado para ${ASSETS[selected].short}.`
      : availableRelease
        ? "Tres plataformas. La detección solo cambia la recomendación."
        : release.status === "missing-assets"
          ? "Todavía no hay binarios publicados para esta versión."
          : "No hemos podido comprobar la última versión.";

  function select(key: AssetKey) {
    setSelected(key);
    setAnnouncement(`Plataforma seleccionada: ${ASSETS[key].short}.`);
  }

  return (
    <div>
      <div className="flex flex-col gap-6 border-b border-border pb-8 md:flex-row md:items-end md:justify-between">
        <div className="min-w-0">
          <div className="flex h-6 items-center gap-2.5">
            <MonoLabel className="text-[9px] tracking-[0.16em]">
              Tu sistema
            </MonoLabel>
            <span
              className={cn(
                "border px-1.5 py-0.5 font-mono text-[9px] tracking-[0.06em]",
                detection?.asset
                  ? "border-accent/40 bg-accent/10 text-accent-text"
                  : "border-border text-text-3",
              )}
            >
              {detection ? detectedLabel(detection) : "detectando…"}
            </span>
          </div>

          <div className="mt-4">
            <ButtonLink
              href={primaryHref}
              size="lg"
              icon={selectedAsset ? "download" : "external"}
              className="w-full sm:w-auto"
              {...(selectedAsset ? {} : { target: "_blank", rel: "noreferrer noopener" })}
            >
              {primaryLabel}
            </ButtonLink>
          </div>

          <p
            className={cn(
              "mt-3 flex min-h-[1.5em] max-w-[64ch] items-start gap-1.5 text-caption leading-[1.6]",
              detection?.reason ? "text-warning" : "text-text-3",
            )}
          >
            {detection?.reason ? (
              <Icon name="alert" size={11} className="mt-1 shrink-0" />
            ) : null}
            {statusNote}
          </p>

          {release.status === "missing-assets" ? (
            <a
              href={ACTIONS_URL}
              target="_blank"
              rel="noreferrer noopener"
              className="mt-2 inline-flex items-center gap-1.5 text-caption text-text-2 underline decoration-border underline-offset-4 transition-colors duration-150 hover:text-text"
            >
              Ver compilaciones en Actions
              <Icon name="external" size={11} />
            </a>
          ) : null}
        </div>

        <div className="shrink-0 md:text-right">
          <MonoLabel className="text-[9px] tracking-[0.16em]">
            Última versión
          </MonoLabel>
          <p className="mt-2 font-mono text-[13px] tracking-[0.04em]">
            {version}
          </p>
          <p className="mt-1 font-mono text-[9px] text-text-3">
            {availableRelease
              ? "GitHub Releases · URL estable"
              : "releases/latest"}
          </p>
        </div>
      </div>

      <fieldset className="mt-8 min-w-0">
        <legend className="sr-only">Elegir plataforma</legend>
        <div className="mb-4 flex items-center justify-between gap-4">
          <MonoLabel className="text-[9px] tracking-[0.16em]">
            Descargas
          </MonoLabel>
          <span className="font-mono text-[9px] tracking-[0.04em] text-text-3">
            ZIP · sin instalador
          </span>
        </div>

        <div className="grid gap-3 lg:grid-cols-3">
          {ASSET_KEYS.map((key) => {
            const data = assetMap.get(key);
            return (
              <PlatformCard
                key={key}
                asset={ASSETS[key]}
                downloadUrl={data?.url ?? null}
                size={data ? formatSize(data.size) : "—"}
                selected={selected === key}
                recommended={detection?.asset === key}
                onSelect={select}
                fallbackUrl={RELEASES_URL}
              />
            );
          })}
        </div>
      </fieldset>

      <div className="mt-8 grid gap-6 border-t border-border pt-6 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] lg:gap-10">
        <div>
          <MonoLabel className="text-[9px] tracking-[0.16em]">
            Integridad
          </MonoLabel>
          <p className="mt-2 max-w-[52ch] text-caption leading-[1.65] text-text-2">
            Cada ZIP se publica con su SHA-256. Descarga el archivo de sumas y
            compara antes de abrir el binario.
          </p>
          {availableRelease?.checksumsUrl ? (
            <a
              href={availableRelease.checksumsUrl}
              className="mt-3 inline-flex items-center gap-1.5 text-body font-medium text-text underline decoration-border underline-offset-4 transition-colors duration-150 hover:text-accent-text"
            >
              SHA256SUMS.txt
              <Icon name="download" size={13} />
            </a>
          ) : (
            <p className="mt-3 font-mono text-[10px] text-text-3">
              SHA256SUMS.txt se publica con cada release.
            </p>
          )}
          <p className="mt-4 max-w-[52ch] text-caption leading-[1.65] text-text-3">
            ¿Prefieres instalador nativo? La misma release adjunta MSI, DMG y
            DEB en{" "}
            <a
              href={RELEASES_URL}
              target="_blank"
              rel="noreferrer noopener"
              className="text-text-2 underline decoration-border underline-offset-4 transition-colors duration-150 hover:text-text"
            >
              GitHub Releases
            </a>
            .
          </p>
        </div>

        <div className="border border-border bg-field p-4">
          <MonoLabel className="text-[9px] tracking-[0.16em]">
            Verificar
          </MonoLabel>
          <dl className="mt-3 space-y-2.5 font-mono text-[10px] leading-[1.7]">
            <div className="grid grid-cols-[58px_minmax(0,1fr)] gap-2">
              <dt className="text-text-3">Linux</dt>
              <dd className="break-all text-text-2 select-all">
                sha256sum -c SHA256SUMS.txt
              </dd>
            </div>
            <div className="grid grid-cols-[58px_minmax(0,1fr)] gap-2">
              <dt className="text-text-3">macOS</dt>
              <dd className="break-all text-text-2 select-all">
                shasum -a 256 -c SHA256SUMS.txt
              </dd>
            </div>
            <div className="grid grid-cols-[58px_minmax(0,1fr)] gap-2">
              <dt className="text-text-3">Windows</dt>
              <dd className="break-all text-text-2 select-all">
                Get-FileHash .\usdx-desktop-windows-x86_64.zip -Algorithm SHA256
              </dd>
            </div>
          </dl>
        </div>
      </div>

      <p aria-live="polite" className="sr-only">
        {announcement}
      </p>
    </div>
  );
}
