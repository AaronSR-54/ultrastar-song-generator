import { DownloadPanel } from "@/components/download-panel";
import { EditorPreview } from "@/components/editor-preview";
import { ExportPreview } from "@/components/export-preview";
import { Faq } from "@/components/faq";
import { Hero } from "@/components/hero";
import { Pipeline } from "@/components/pipeline";
import { Privacy } from "@/components/privacy";
import { Requirements } from "@/components/requirements";
import { Section } from "@/components/ui/section";
import { getLatestRelease } from "@/lib/downloads";
import { RELEASES_URL, SOURCE_URL } from "@/lib/release";
import { siteUrl, SITE_DESCRIPTION } from "@/lib/site";

const description = SITE_DESCRIPTION;

export default async function Page() {
  const release = await getLatestRelease();

  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: "UltraStar Song Generator",
    applicationCategory: "MultimediaApplication",
    operatingSystem: "Windows 10, Windows 11, macOS 12+, Linux x86_64",
    description,
    url: siteUrl(),
    image: `${siteUrl()}/og.png`,
    downloadUrl:
      release.status === "available"
        ? (release.release.assets[0]?.url ?? RELEASES_URL)
        : RELEASES_URL,
    softwareVersion:
      release.status === "available"
        ? release.release.version.replace(/^v/, "")
        : undefined,
    isAccessibleForFree: true,
    offers: {
      "@type": "Offer",
      price: "0",
      priceCurrency: "EUR",
    },
    sameAs: [SOURCE_URL],
  };

  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <Hero />

      <Section
        id="como-funciona"
        index="01"
        label="Cómo funciona"
        title="Seis etapas, de la URL a la carpeta."
        lead="Cada etapa escribe su resultado en disco. Si algo falla, la app explica el motivo y continúa desde ahí, sin empezar de cero."
        meta={
          <>
            6 etapas
            <br />
            reintentables
          </>
        }
      >
        <Pipeline />
      </Section>

      <Section
        id="privacidad"
        index="02"
        label="Privacidad"
        title="Todo ocurre en tu equipo."
        lead="El audio no se sube a ningún servicio. Solo viajan el título, el artista y la duración para buscar la letra."
        meta={
          <>
            0 subidas
            <br />
            de audio
          </>
        }
      >
        <Privacy />
      </Section>

      <Section
        id="editor"
        index="03"
        label="La letra, medida"
        title="Cada sílaba cae donde se canta."
        lead="pYIN mide el pitch de la voz separada y el alineador fonético decide dónde empieza cada sílaba. Lo que no cuadre se corrige a mano, sin reprocesar."
        meta={
          <>
            pYIN
            <br />
            + fonemas
          </>
        }
      >
        <EditorPreview />
      </Section>

      <Section
        id="exportacion"
        index="04"
        label="Formato"
        title="Lista para UltraStar."
        lead="Un .txt validado, el audio, la carátula y el vídeo en una carpeta que el juego reconoce sola."
        meta={
          <>
            UltraStar
            <br />
            .txt
          </>
        }
      >
        <ExportPreview />
      </Section>

      <Section
        id="requisitos"
        index="05"
        label="Requisitos"
        title="Tres binarios, las mismas herramientas."
        lead="No hace falta Rust si usas una release. yt-dlp, ffmpeg y Node o Deno sí: son las piezas que hablan con YouTube y convierten el audio."
        meta={
          <>
            Windows · macOS
            <br />
            Linux
          </>
        }
      >
        <Requirements />
      </Section>

      <Section
        id="descargas"
        index="06"
        label="Descargas"
        title="Elige tu sistema."
        lead="Windows, macOS Apple Silicon y Linux x86_64. La detección cambia la recomendación, no esconde las otras descargas."
        meta={
          <>
            ZIP
            <br />
            sin instalador
          </>
        }
      >
        <DownloadPanel release={release} />
      </Section>

      <Section
        id="faq"
        index="07"
        label="Preguntas"
        title="Lo que suele frenar la primera canción."
        lead="Avisos del sistema, herramientas que faltan y modelos que se descargan una sola vez."
        meta={
          <>
            6 respuestas
            <br />
            concretas
          </>
        }
      >
        <Faq />
      </Section>
    </>
  );
}
