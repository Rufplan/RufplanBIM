// Plans to 3D (ADR-036): plan sheets as the images Claude reads. Images are scaled so the
// long side is at most 1568 px (what Claude sees, so its pixel coordinates are these); PDF
// pages are drawn with pdf.js, loaded on first use.

/** A plan sheet ready to send, with a preview URL. */
export interface PlanPage {
  name: string;
  url: string;
  mediaType: string;
  data: string;
  width: number;
  height: number;
}

const LONG_SIDE = 1568;

function fromCanvas(src: CanvasImageSource, w: number, h: number, name: string): PlanPage {
  const k = Math.min(1, LONG_SIDE / Math.max(w, h));
  const c = document.createElement("canvas");
  c.width = Math.round(w * k);
  c.height = Math.round(h * k);
  const g = c.getContext("2d")!;
  g.fillStyle = "#fff";
  g.fillRect(0, 0, c.width, c.height);
  g.drawImage(src, 0, 0, c.width, c.height);
  const url = c.toDataURL("image/jpeg", 0.9);
  return {
    name,
    url,
    mediaType: "image/jpeg",
    data: url.split(",")[1] ?? "",
    width: c.width,
    height: c.height,
  };
}

async function fromImage(file: File): Promise<PlanPage> {
  const url = URL.createObjectURL(file);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    return fromCanvas(img, img.width, img.height, file.name);
  } finally {
    URL.revokeObjectURL(url);
  }
}

// pdf.js runs its parser in a worker; the app's CSP allows workers from blob: only, so the
// worker's source is bundled as text and started from a Blob.
let pdfjs: Promise<typeof import("pdfjs-dist")> | null = null;
function loadPdfjs() {
  pdfjs ??= (async () => {
    const [lib, worker] = await Promise.all([
      import("pdfjs-dist"),
      import("pdfjs-dist/build/pdf.worker.min.mjs?raw"),
    ]);
    const url = URL.createObjectURL(new Blob([worker.default], { type: "text/javascript" }));
    lib.GlobalWorkerOptions.workerPort = new Worker(url, { type: "module" });
    return lib;
  })();
  return pdfjs;
}

async function fromPdf(file: File, maxPages: number): Promise<PlanPage[]> {
  const lib = await loadPdfjs();
  const pdf = await lib.getDocument({ data: new Uint8Array(await file.arrayBuffer()) }).promise;
  const out: PlanPage[] = [];
  for (let n = 1; n <= Math.min(pdf.numPages, maxPages); n++) {
    const page = await pdf.getPage(n);
    const base = page.getViewport({ scale: 1 });
    const vp = page.getViewport({ scale: LONG_SIDE / Math.max(base.width, base.height) });
    const c = document.createElement("canvas");
    c.width = Math.round(vp.width);
    c.height = Math.round(vp.height);
    await page.render({ canvasContext: c.getContext("2d")!, viewport: vp }).promise;
    out.push(fromCanvas(c, c.width, c.height, `${file.name} p${n}`));
  }
  await pdf.destroy();
  return out;
}

/** Plan sheets from dropped or picked files: images, and each page of a PDF (up to
 * `maxPages` in all). */
export async function readPlanFiles(files: File[], maxPages: number): Promise<PlanPage[]> {
  const out: PlanPage[] = [];
  for (const f of files) {
    if (out.length >= maxPages) break;
    const left = maxPages - out.length;
    if (f.type === "application/pdf" || /\.pdf$/i.test(f.name)) {
      out.push(...(await fromPdf(f, left)));
    } else if (f.type.startsWith("image/")) {
      out.push(await fromImage(f));
    }
  }
  return out;
}

/** A level name guessed from a sheet's file name ("fallingwater-second-floor.jpg"). */
export function guessLevel(name: string, index: number): string {
  const s = name.toLowerCase();
  const words: [RegExp, string][] = [
    [/basement|cellar|lower level/, "Basement"],
    [/ground|first|1st|level ?1\b|main/, "First Floor"],
    [/second|2nd|level ?2\b|upper/, "Second Floor"],
    [/third|3rd|level ?3\b/, "Third Floor"],
    [/fourth|4th|level ?4\b/, "Fourth Floor"],
  ];
  for (const [re, level] of words) if (re.test(s)) return level;
  return (
    ["First Floor", "Second Floor", "Third Floor", "Fourth Floor"][index] ?? `Level ${index + 1}`
  );
}
