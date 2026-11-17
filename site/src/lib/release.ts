import type { AssetKey } from "@/lib/platforms";

const REPO = "AaronSR-54/ultrastar-song-generator";

export const RELEASES_URL = `https://github.com/${REPO}/releases`;
export const ACTIONS_URL = `https://github.com/${REPO}/actions/workflows/release.yml`;
export const SOURCE_URL = `https://github.com/${REPO}`;
export const LATEST_DOWNLOAD = `${RELEASES_URL}/latest/download`;

export type ReleaseAsset = {
  key: AssetKey;
  url: string;
  size: number;
};

export type Release = {
  version: string;
  publishedAt: string;
  assets: ReleaseAsset[];
  installers: ReleaseAsset[];
  checksumsUrl: string | null;
  installerChecksumsUrl: string | null;
};

export type ReleaseState =
  | { status: "available"; release: Release }
  | { status: "missing-assets"; version: string | null }
  | { status: "unavailable" };

export function formatSize(bytes: number, empty = "—"): string {
  if (!bytes) return empty;
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(1).replace(".", ",")} MB`;
}
