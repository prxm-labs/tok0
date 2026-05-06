/**
 * Single source of truth for the CLI version surfaced on the marketing site.
 *
 * The canonical version lives in `apps/cli/Cargo.toml`. We import that file
 * as raw text via Vite's `?raw` query, so the SemVer string is inlined into
 * the bundle at build time. This keeps the homepage in lock-step with
 * whatever release-please last promoted — no manual sync, no drift between
 * the binary the user installs and the version banner above the install
 * command.
 *
 * The parser throws on missing/unparseable Cargo.toml so the build fails
 * loudly rather than silently shipping an empty version.
 */
// `?raw` is a Vite primitive — the file contents are embedded at compile
// time. No fs access is performed at runtime, so this works under SSG,
// SSR, and the production preview server alike.
import CARGO_TOML_RAW from "../../../cli/Cargo.toml?raw";

function parseCliVersion(raw: string): string {
  // Anchor on the [package] table so we don't pick up a [bench] or
  // workspace-member version line.
  const pkgIdx = raw.indexOf("[package]");
  if (pkgIdx === -1) {
    throw new Error(
      "[lib/version] No [package] section in apps/cli/Cargo.toml — refusing to ship a website with no CLI version.",
    );
  }
  const tail = raw.slice(pkgIdx);
  const match = tail.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(
      '[lib/version] No version = "X.Y.Z" line under [package] in apps/cli/Cargo.toml.',
    );
  }
  const version = match[1].trim();
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(
      `[lib/version] Cargo.toml package.version "${version}" is not SemVer-shaped.`,
    );
  }
  return version;
}

/** SemVer string read from `apps/cli/Cargo.toml` at build time. */
export const CLI_VERSION: string = parseCliVersion(CARGO_TOML_RAW);

/** `v<CLI_VERSION>` — convenience for headers/banners. */
export const CLI_VERSION_TAG: string = `v${CLI_VERSION}`;
