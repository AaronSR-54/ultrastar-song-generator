export type AssetKey =
  | "windows-x86_64"
  | "macos-aarch64"
  | "linux-x86_64";

export type Asset = {
  key: AssetKey;
  file: string;
  label: string;
  platform: string;
  arch: string;
  short: string;
};

export const ASSET_KEYS: AssetKey[] = [
  "windows-x86_64",
  "macos-aarch64",
  "linux-x86_64",
];

export const ASSETS: Record<AssetKey, Asset> = {
  "windows-x86_64": {
    key: "windows-x86_64",
    file: "usdx-desktop-windows-x86_64.zip",
    label: "Windows",
    platform: "10 / 11",
    arch: "x86_64",
    short: "Windows x86_64",
  },
  "macos-aarch64": {
    key: "macos-aarch64",
    file: "usdx-desktop-macos-aarch64.zip",
    label: "macOS",
    platform: "12+",
    arch: "Apple Silicon",
    short: "macOS Apple Silicon",
  },
  "linux-x86_64": {
    key: "linux-x86_64",
    file: "usdx-desktop-linux-x86_64.zip",
    label: "Linux",
    platform: "x86_64",
    arch: "glibc",
    short: "Linux x86_64",
  },
};

export const CHECKSUMS_FILE = "SHA256SUMS.txt";

export type DetectedOs =
  | "windows"
  | "macos"
  | "linux"
  | "ipados"
  | "ios"
  | "android"
  | "unknown";

export type DetectedArch = "x86_64" | "arm64" | "unknown";

export type Detection = {
  os: DetectedOs;
  arch: DetectedArch;
  confidence: "high" | "low";
  asset: AssetKey | null;
  reason: string | null;
};

export type DetectInput = {
  userAgent: string;
  uaData?: {
    platform?: string;
    architecture?: string;
    bitness?: string;
  } | null;
  maxTouchPoints?: number;
};

const OS_LABELS: Record<DetectedOs, string> = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
  ipados: "iPadOS",
  ios: "iOS",
  android: "Android",
  unknown: "Sistema desconocido",
};

export function detectedLabel(detection: Detection): string {
  if (detection.os === "unknown") return "Sistema desconocido";
  if (detection.arch === "unknown") return OS_LABELS[detection.os];
  return `${OS_LABELS[detection.os]} · ${detection.arch}`;
}

function archFromHint(architecture?: string): DetectedArch {
  if (!architecture) return "unknown";
  if (architecture === "arm") return "arm64";
  if (architecture === "x86") return "x86_64";
  return "unknown";
}

function windows(arch: DetectedArch, confidence: "high" | "low"): Detection {
  if (arch === "arm64") {
    return {
      os: "windows",
      arch,
      confidence,
      asset: null,
      reason:
        "Todavía no hay binario para Windows ARM. Puedes compilar desde el código.",
    };
  }
  return {
    os: "windows",
    arch: "x86_64",
    confidence,
    asset: "windows-x86_64",
    reason: null,
  };
}

function macos(arch: DetectedArch, confidence: "high" | "low"): Detection {
  if (arch === "x86_64") {
    return {
      os: "macos",
      arch,
      confidence,
      asset: null,
      reason:
        "No hay ZIP para Mac Intel. Compila la app con Rust o usa un equipo Apple Silicon.",
    };
  }
  return {
    os: "macos",
    arch: arch === "arm64" ? "arm64" : "unknown",
    confidence,
    asset: "macos-aarch64",
    reason:
      arch === "arm64"
        ? null
        : "El ZIP es para Apple Silicon. Si tu Mac es Intel, compila desde el código.",
  };
}

function linux(arch: DetectedArch, confidence: "high" | "low"): Detection {
  if (arch === "arm64") {
    return {
      os: "linux",
      arch,
      confidence,
      asset: null,
      reason:
        "No hay binario para Linux ARM. Compila la app con Rust desde el código.",
    };
  }
  return {
    os: "linux",
    arch: "x86_64",
    confidence,
    asset: "linux-x86_64",
    reason: null,
  };
}

export function detectPlatform(input: DetectInput): Detection {
  const ua = input.userAgent;
  const hint = input.uaData?.platform?.toLowerCase();
  const hasHints = Boolean(input.uaData?.platform);
  const confidence = hasHints ? "high" : "low";

  if (hint === "android" || (!hint && /android/i.test(ua))) {
    return {
      os: "android",
      arch: "unknown",
      confidence,
      asset: null,
      reason:
        "La app es de escritorio. En Android no hay binario; abre esta página en un ordenador.",
    };
  }

  if (hint === "ios" || (!hint && /iphone|ipod/i.test(ua))) {
    return {
      os: "ios",
      arch: "unknown",
      confidence,
      asset: null,
      reason:
        "La app es de escritorio. En iPhone no hay binario; abre esta página en un ordenador.",
    };
  }

  const isPhone = /iphone|ipod/i.test(ua);
  const touchMac =
    !isPhone &&
    /macintosh|mac os x/i.test(ua) &&
    (input.maxTouchPoints ?? 0) > 1;

  if (hint === "ipados" || (!hint && (touchMac || /ipad/i.test(ua)))) {
    return {
      os: "ipados",
      arch: "unknown",
      confidence,
      asset: null,
      reason:
        "La app es de escritorio. En iPad no hay binario; abre esta página en un ordenador.",
    };
  }

  if (hint === "windows" || (!hint && /windows/i.test(ua))) {
    const hintArch = archFromHint(input.uaData?.architecture);
    if (hintArch === "arm64" || (!hasHints && /arm64|aarch64/i.test(ua))) {
      return windows("arm64", confidence);
    }
    return windows("x86_64", confidence);
  }

  if (hint === "macos" || (!hint && /macintosh|mac os x/i.test(ua))) {
    const hintArch = archFromHint(input.uaData?.architecture);
    if (hintArch === "x86_64") return macos("x86_64", confidence);
    if (hintArch === "arm64") return macos("arm64", confidence);
    return macos("unknown", confidence);
  }

  if (hint === "chrome os" || (!hint && /cros/i.test(ua))) {
    return {
      os: "unknown",
      arch: "unknown",
      confidence,
      asset: null,
      reason:
        "Chrome OS no ejecuta la app de escritorio. Elige una plataforma en la lista.",
    };
  }

  if (hint === "linux" || (!hint && /linux/i.test(ua))) {
    if (archFromHint(input.uaData?.architecture) === "arm64") {
      return linux("arm64", confidence);
    }
    if (!hasHints && /aarch64|arm64/i.test(ua)) {
      return linux("arm64", confidence);
    }
    return linux("x86_64", confidence);
  }

  return {
    os: "unknown",
    arch: "unknown",
    confidence: "low",
    asset: null,
    reason:
      "No hemos podido identificar tu sistema. Elige una plataforma en la lista.",
  };
}
