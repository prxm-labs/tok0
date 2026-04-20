/**
 * OG image SVG builder — spec-sheet aesthetic mirrored from the live site.
 *
 * Output: a 1200×630 SVG string. Sharp converts the SVG to PNG inside the
 * `/og/[slug].png` endpoint. All typography uses font-family stacks that
 * fall back to system fonts so the image renders consistently in any build
 * environment, with or without the project's web fonts installed.
 */

const PAPER = "#f4f1ea";
const INK = "#0a0a0a";
const INK_2 = "#1a1a1a";
const INK_3 = "#3a3a38";
const INK_4 = "#6a6a66";
const ACCENT = "#ff4a1c";

const FONT_MONO = "'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, Consolas, monospace";
const FONT_SERIF = "'Instrument Serif', 'Times New Roman', Georgia, serif";
const FONT_SANS = "Inter, ui-sans-serif, system-ui, sans-serif";

const W = 1200;
const H = 630;

/** Inset of the page-frame border (matches `.page-frame::before` at 14px). */
const FRAME_INSET = 14;

/** Inner padding from the page-frame to the content gutter. */
const GUTTER = 56;

/** Strip vertical inset from the frame border and strip height. */
const STRIP_PAD = 24;
const STRIP_H = 40;

export interface OgOpts {
  /** Display title — rendered in serif at large size. Wraps to ≤2 lines. */
  title: string;
  /** Short blurb below the title, mono. Wraps to ≤3 lines, ellipsised if longer. */
  description: string;
  /** Optional accent fragment rendered on its own italic-accent line under the title. */
  titleAccent?: string;
  /** Top-strip labels. */
  stripLeft?: string;
  stripCenter?: string;
  stripRight?: string;
  /** Section ID line above the title — e.g. "04/" + "CONCEPTS · ARCHITECTURE". */
  sectionNum?: string;
  sectionLabel?: string;
  /** Bottom-strip labels. */
  bottomLeft?: string;
  bottomCenter?: string;
  bottomRight?: string;
}

const escapeXml = (s: string): string =>
  s.replace(/[<>&'"]/g, (ch) =>
    ({ "<": "&lt;", ">": "&gt;", "&": "&amp;", "'": "&apos;", '"': "&quot;" })[ch] ?? ch,
  );

/** Greedy word-wrap to a max number of lines; final line is ellipsised on overflow. */
function wrap(text: string, maxChars: number, maxLines: number): string[] {
  const words = text.split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let cur = "";
  for (const w of words) {
    const candidate = cur ? `${cur} ${w}` : w;
    if (candidate.length > maxChars && cur) {
      lines.push(cur);
      cur = w;
      if (lines.length === maxLines) {
        cur = "";
        break;
      }
    } else {
      cur = candidate;
    }
  }
  if (cur && lines.length < maxLines) lines.push(cur);
  const consumed = lines.join(" ").split(/\s+/).filter(Boolean).length;
  if (consumed < words.length && lines.length === maxLines) {
    let last = lines[maxLines - 1];
    while (last.length > maxChars - 1 && last.includes(" ")) {
      last = last.slice(0, last.lastIndexOf(" "));
    }
    lines[maxLines - 1] = `${last.replace(/[\s,.;:—–-]+$/, "")}…`;
  }
  return lines;
}

/** Picks a serif font size that comfortably fits the longest wrapped line. */
function pickTitleSize(maxLineChars: number): number {
  if (maxLineChars <= 14) return 110;
  if (maxLineChars <= 18) return 96;
  if (maxLineChars <= 22) return 84;
  if (maxLineChars <= 28) return 70;
  if (maxLineChars <= 36) return 58;
  return 48;
}

function corners(): string {
  // Each bracket is the same L-shape that ships in Marketing.astro's body.
  // Source path: "M22 1 H1 V22"; rotated for each corner around the offset point.
  const SIZE = 22;
  const OFFSET = 6;
  const stroke = `stroke="${INK}" stroke-width="1.2" fill="none"`;
  const tl = `<g transform="translate(${OFFSET},${OFFSET})" ${stroke}><path d="M${SIZE} 1 H1 V${SIZE}"/></g>`;
  const tr = `<g transform="translate(${W - OFFSET},${OFFSET}) rotate(90)" ${stroke}><path d="M${SIZE} 1 H1 V${SIZE}"/></g>`;
  const br = `<g transform="translate(${W - OFFSET},${H - OFFSET}) rotate(180)" ${stroke}><path d="M${SIZE} 1 H1 V${SIZE}"/></g>`;
  const bl = `<g transform="translate(${OFFSET},${H - OFFSET}) rotate(270)" ${stroke}><path d="M${SIZE} 1 H1 V${SIZE}"/></g>`;
  return tl + tr + br + bl;
}

/** Industrial dark strip — three columns: left/center/right. */
function strip(y: number, left: string, center: string, right: string, arrow: boolean): string {
  const x = FRAME_INSET + STRIP_PAD;
  const width = W - 2 * (FRAME_INSET + STRIP_PAD);
  const baseline = y + 26;
  const arrowPrefix = arrow ? `<tspan fill="${ACCENT}">&gt;&gt;&gt;</tspan> ` : "";
  const fontAttrs = `font-family="${FONT_MONO}" font-size="13" font-weight="500" letter-spacing="2"`;
  return `
    <rect x="${x}" y="${y}" width="${width}" height="${STRIP_H}" fill="${INK}"/>
    <text x="${x + 18}" y="${baseline}" fill="${PAPER}" ${fontAttrs}>${escapeXml(left)}</text>
    <text x="${W / 2}" y="${baseline}" text-anchor="middle" fill="${PAPER}" ${fontAttrs}>${escapeXml(center)}</text>
    <text x="${x + width - 18}" y="${baseline}" text-anchor="end" fill="${PAPER}" ${fontAttrs}>${arrowPrefix}${escapeXml(right)}</text>
  `;
}

export function buildOgSvg(opts: OgOpts): string {
  const {
    title,
    description,
    titleAccent,
    stripLeft = "OPEN SOURCE · COMPRESSION PROXY",
    stripCenter = "TOK0 · RUST CLI",
    stripRight = "PRE-MODEL TOKEN REDUCTION",
    sectionNum,
    sectionLabel,
    bottomLeft = "BUILT IN RUST · SINGLE STATIC BINARY",
    bottomCenter = "MIT",
    bottomRight = "GITHUB.COM/PRXM-LABS/TOK0",
  } = opts;

  // ── title sizing: wrap to ≤2 lines, then pick a size that fits ───────────
  // Try progressively narrower widths until the title fits in 2 lines.
  const wrapAttempts = [22, 28, 36, 44];
  let titleLines: string[] = [];
  let chosenWidth = wrapAttempts[wrapAttempts.length - 1];
  for (const w of wrapAttempts) {
    const tryLines = wrap(title, w, 2);
    if (tryLines.length <= 2 && tryLines.every((l) => l.length <= w)) {
      titleLines = tryLines;
      chosenWidth = w;
      break;
    }
  }
  if (titleLines.length === 0) titleLines = wrap(title, 44, 2);
  const longestLine = Math.max(...titleLines.map((l) => l.length));
  const titleSize = pickTitleSize(Math.max(longestLine, chosenWidth));
  const titleLineH = Math.round(titleSize * 0.96);

  const accentSize = titleAccent ? Math.round(titleSize * 0.62) : 0;
  const accentLineH = Math.round(accentSize * 1.05);

  const descLines = wrap(description, 60, 3);
  const descSize = 20;
  const descLineH = 30;

  // ── vertical layout (top → bottom) ───────────────────────────────────────
  const topStripY = FRAME_INSET + STRIP_PAD;
  const bottomStripY = H - FRAME_INSET - STRIP_PAD - STRIP_H;
  const sectionY = topStripY + STRIP_H + 60;

  const titleBlockY = sectionY + 28;
  const titleX = FRAME_INSET + STRIP_PAD + GUTTER;

  // Compose title text elements.
  const titleEls = titleLines
    .map(
      (line, i) =>
        `<text x="${titleX}" y="${titleBlockY + (i + 1) * titleLineH}" fill="${INK}" font-family="${FONT_SERIF}" font-size="${titleSize}" font-weight="400" letter-spacing="-1.2">${escapeXml(line)}</text>`,
    )
    .join("");

  const titleEndY = titleBlockY + titleLines.length * titleLineH;

  const accentEl = titleAccent
    ? `<text x="${titleX}" y="${titleEndY + accentLineH}" fill="${ACCENT}" font-family="${FONT_SERIF}" font-size="${accentSize}" font-style="italic" font-weight="400" letter-spacing="-0.6">${escapeXml(titleAccent)}</text>`
    : "";

  const accentEndY = titleAccent ? titleEndY + accentLineH : titleEndY;

  const descTopY = accentEndY + 36;
  const descEls = descLines
    .map(
      (line, i) =>
        `<text x="${titleX}" y="${descTopY + (i + 1) * descLineH}" fill="${INK_2}" font-family="${FONT_MONO}" font-size="${descSize}" font-weight="400" letter-spacing="0.2">${escapeXml(line)}</text>`,
    )
    .join("");

  // ── tok0 wordmark, top-right of content area ─────────────────────────────
  const wordmark = `
    <g transform="translate(${W - FRAME_INSET - STRIP_PAD - 24}, ${sectionY + 4})">
      <text text-anchor="end" font-family="${FONT_SANS}" font-size="34" font-weight="800" letter-spacing="-0.8" fill="${INK}">
        tok0<tspan font-size="14" dy="-14" fill="${INK_4}">®</tspan>
      </text>
    </g>`;

  // ── section ID strip above the title ─────────────────────────────────────
  const sectionEl =
    sectionNum && sectionLabel
      ? `
    <g transform="translate(${titleX}, ${sectionY})">
      <text font-family="${FONT_MONO}" font-size="13" font-weight="600" letter-spacing="2.4">
        <tspan fill="${ACCENT}" font-weight="700">${escapeXml(sectionNum)}</tspan>
        <tspan dx="14" fill="${INK_3}">${escapeXml(sectionLabel.toUpperCase())}</tspan>
      </text>
    </g>`
      : "";

  // ── hairline rule above the bottom strip, anchored from the description ──
  const ruleY = Math.min(descTopY + descLines.length * descLineH + 36, bottomStripY - 28);
  const ruleEl = `<line x1="${titleX}" y1="${ruleY}" x2="${W - titleX}" y2="${ruleY}" stroke="${INK}" stroke-opacity="0.18" stroke-width="1"/>`;

  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}">
  <defs>
    <pattern id="grain" width="9" height="9" patternUnits="userSpaceOnUse">
      <circle cx="0.5" cy="0.5" r="0.6" fill="${INK}" fill-opacity="0.04"/>
      <circle cx="4.5" cy="4.5" r="0.5" fill="${INK}" fill-opacity="0.025"/>
    </pattern>
  </defs>

  <rect width="${W}" height="${H}" fill="${PAPER}"/>
  <rect width="${W}" height="${H}" fill="url(#grain)"/>

  <rect x="${FRAME_INSET}" y="${FRAME_INSET}" width="${W - 2 * FRAME_INSET}" height="${H - 2 * FRAME_INSET}" fill="none" stroke="${INK}" stroke-width="1"/>

  ${corners()}

  ${strip(topStripY, stripLeft, stripCenter, stripRight, true)}

  ${sectionEl}
  ${wordmark}
  ${titleEls}
  ${accentEl}
  ${descEls}
  ${ruleEl}

  ${strip(bottomStripY, bottomLeft, bottomCenter, bottomRight, false)}
</svg>`;
}
