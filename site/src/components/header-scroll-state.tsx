"use client";

import { useEffect } from "react";

const SCROLL_THRESHOLD = 8;

/**
 * Sincroniza el atributo `data-scrolled` del header con la posición de la
 * página. La decisión visual vive en CSS; aquí solo se evita el trabajo por
 * frame con un listener pasivo y una única escritura al cruzar el umbral.
 */
export function HeaderScrollState() {
  useEffect(() => {
    const header = document.querySelector<HTMLElement>("[data-site-header]");
    if (!header) return;

    let frame = 0;

    const apply = () => {
      frame = 0;
      const next = window.scrollY > SCROLL_THRESHOLD ? "true" : "false";
      if (header.dataset.scrolled !== next) {
        header.dataset.scrolled = next;
      }
    };

    const schedule = () => {
      if (frame === 0) {
        frame = window.requestAnimationFrame(apply);
      }
    };

    // Cubre recargas con scroll restaurado y entradas con ancla.
    apply();
    window.addEventListener("scroll", schedule, { passive: true });

    return () => {
      window.removeEventListener("scroll", schedule);
      if (frame !== 0) {
        window.cancelAnimationFrame(frame);
      }
    };
  }, []);

  return null;
}
