export function siteUrl(): string {
  const raw =
    process.env.NEXT_PUBLIC_SITE_URL ??
    process.env.VERCEL_PROJECT_PRODUCTION_URL ??
    process.env.VERCEL_URL ??
    "http://localhost:3000";

  return raw.startsWith("http") ? raw.replace(/\/$/, "") : `https://${raw}`;
}

export const SITE_DESCRIPTION =
  "Convierte un enlace de YouTube en una canción jugable de UltraStar Deluxe. La letra, la voz y el pitch se procesan en tu equipo, sin subir nunca tu audio.";
