import type { APIRoute, GetStaticPaths } from "astro";
import { getCollection } from "astro:content";
import sharp from "sharp";
import { buildOgSvg, type OgOpts } from "~/lib/og";
import { CLI_VERSION_TAG } from "~/lib/version";

interface OgPage {
  slug: string;
  opts: OgOpts;
}

const sectionOrder = [
  "Start",
  "Concepts",
  "Reference",
  "Integrations",
  "Operations",
  "Project",
];

async function collectPages(): Promise<OgPage[]> {
  const pages: OgPage[] = [];

  // Homepage — punchy serif headline + the same manifesto line as the hero.
  pages.push({
    slug: "home",
    opts: {
      title: "Strip the noise from every shell command",
      titleAccent: "before it hits the model.",
      description:
        "Open-source Rust CLI proxy. Compresses verbose shell output (git, cargo, npm, docker, kubectl) before it reaches an LLM. Typical savings 60–90%.",
      sectionNum: "00/",
      sectionLabel: "Open source · Pre-model token reduction",
      bottomCenter: `${CLI_VERSION_TAG} / MIT`,
    },
  });

  // Docs landing page.
  pages.push({
    slug: "docs",
    opts: {
      title: "tok0 documentation",
      description:
        "Install, configure, and extend tok0. The compression pipeline, TOML rule DSL, AI tool bridges, telemetry, and the full CLI reference.",
      sectionNum: "§",
      sectionLabel: "Documentation",
      stripCenter: "TOK0 · DOCS",
      stripRight: "FIELD MANUAL",
      bottomCenter: `${CLI_VERSION_TAG} / MIT`,
    },
  });

  // One image per MDX doc, with the section + position folded into the eyebrow.
  const docs = await getCollection("docs");
  // Numbering matches the sidebar's ordering: stable section order, then frontmatter `order`.
  const groups = new Map<string, typeof docs>();
  for (const entry of docs) {
    const arr = groups.get(entry.data.section) ?? [];
    arr.push(entry);
    groups.set(entry.data.section, arr);
  }
  const ordered = sectionOrder
    .filter((s) => groups.has(s))
    .map((s) => [s, groups.get(s)!] as const)
    .concat([...groups.entries()].filter(([s]) => !sectionOrder.includes(s)));

  let counter = 0;
  for (const [section, items] of ordered) {
    items.sort((a, b) => a.data.order - b.data.order);
    for (const entry of items) {
      counter += 1;
      const num = String(counter).padStart(2, "0");
      pages.push({
        slug: entry.id,
        opts: {
          title: entry.data.title,
          description: entry.data.description,
          sectionNum: `${num}/`,
          sectionLabel: `${section} · ${entry.data.title}`,
          stripCenter: "TOK0 · DOCS",
          stripRight: section.toUpperCase(),
          bottomCenter: `${CLI_VERSION_TAG} / MIT`,
        },
      });
    }
  }

  return pages;
}

export const getStaticPaths: GetStaticPaths = async () => {
  const pages = await collectPages();
  return pages.map(({ slug, opts }) => ({
    params: { slug },
    props: { opts },
  }));
};

export const GET: APIRoute = async ({ props }) => {
  const opts = props.opts as OgOpts;
  const svg = buildOgSvg(opts);
  const png = await sharp(Buffer.from(svg)).png({ compressionLevel: 9 }).toBuffer();
  return new Response(png, {
    headers: {
      "Content-Type": "image/png",
      "Cache-Control": "public, max-age=31536000, immutable",
    },
  });
};
