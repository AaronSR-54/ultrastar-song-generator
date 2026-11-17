import {
  ASSET_KEYS,
  ASSETS,
  CHECKSUMS_FILE,
  INSTALLER_CHECKSUMS_FILE,
  isInstallerFor,
} from "@/lib/platforms";
import {
  LATEST_DOWNLOAD,
  type ReleaseState,
} from "@/lib/release";

const REPO = "AaronSR-54/ultrastar-song-generator";

type GithubAsset = {
  name?: string;
  size?: number;
};

type GithubRelease = {
  tag_name?: string;
  published_at?: string;
  assets?: GithubAsset[];
};

function assetUrl(file: string): string {
  return `${LATEST_DOWNLOAD}/${file}`;
}

function parseRelease(data: GithubRelease): ReleaseState {
  const names = new Map(
    (data.assets ?? []).map((asset) => [asset.name, asset.size ?? 0]),
  );
  const assetNames = [...names.keys()].filter(
    (name): name is string => Boolean(name),
  );

  const missing = ASSET_KEYS.some((key) => !names.has(ASSETS[key].file));
  if (missing) {
    return { status: "missing-assets", version: data.tag_name ?? null };
  }

  const assets = ASSET_KEYS.map((key) => ({
    key,
    url: assetUrl(ASSETS[key].file),
    size: names.get(ASSETS[key].file) ?? 0,
  }));

  const installers = ASSET_KEYS.flatMap((key) => {
    const file = assetNames.find((name) => isInstallerFor(key, name));
    return file
      ? [{ key, url: assetUrl(file), size: names.get(file) ?? 0 }]
      : [];
  });

  return {
    status: "available",
    release: {
      version: data.tag_name ?? "latest",
      publishedAt: data.published_at ?? "",
      assets,
      installers,
      checksumsUrl: names.has(CHECKSUMS_FILE)
        ? assetUrl(CHECKSUMS_FILE)
        : null,
      installerChecksumsUrl: names.has(INSTALLER_CHECKSUMS_FILE)
        ? assetUrl(INSTALLER_CHECKSUMS_FILE)
        : null,
    },
  };
}

export async function getLatestRelease(): Promise<ReleaseState> {
  try {
    const response = await fetch(
      `https://api.github.com/repos/${REPO}/releases/latest`,
      {
        headers: {
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
          ...(process.env.GITHUB_TOKEN
            ? { Authorization: `Bearer ${process.env.GITHUB_TOKEN}` }
            : {}),
        },
        next: { revalidate: 300 },
      },
    );

    if (!response.ok) return { status: "unavailable" };

    const data = (await response.json()) as GithubRelease;
    return parseRelease(data);
  } catch {
    return { status: "unavailable" };
  }
}
