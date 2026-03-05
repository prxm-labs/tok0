//! `prisma` (Prisma CLI) compressor — dispatches between subcommand
//! filters.
//!
//! - `prisma migrate …` → [`filter_migrate`]: keeps lines mentioning
//!   migrations, the `Your database is now in sync` summary, applied/
//!   warning notices, and any error lines; drops env-variable banners,
//!   the schema-loaded preamble, the tail `Update available` ASCII box,
//!   and Prisma Studio cursor-control noise.
//! - `prisma generate` → [`filter_generate`]: keeps the
//!   `✔ Generated Prisma Client` summary and any error lines; drops the
//!   schema-loaded banner and tip suffix.
//! - `prisma validate` → [`filter_validate`]: keeps the `is valid` line
//!   and any error lines; drops everything else.
//! - Any other subcommand (`db`, `studio`, `format`, `init`) →
//!   [`filter_default`]: passes through with the env-variable and
//!   Studio banner lines stripped, hard-capped at 20 lines.
//!
//! Wiring into `Commands` enum + dispatcher happens in a follow-up
//! task; this module exposes [`filter_prisma`] as the public
//! entry-point used by snapshot, property, and criterion-bench tests.

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;

use crate::engine::shell;

/// Args for the `prisma` subcommand. `argv` is the full argument list
/// passed through from the user (excluding the leading `prisma` /
/// `tok0 prisma` literal).
#[derive(Debug, Clone)]
pub struct PrismaArgs {
    pub argv: Vec<String>,
}

impl PrismaArgs {
    pub fn from_argv(argv: Vec<String>) -> Self {
        Self { argv }
    }

    /// Returns the first recognized Prisma subcommand from `argv`, or
    /// `"other"` if none matches.
    pub fn subcmd(&self) -> &str {
        for a in &self.argv {
            match a.as_str() {
                "migrate" | "generate" | "validate" | "studio" | "format" | "init" | "db" => {
                    return a.as_str();
                }
                _ => continue,
            }
        }
        "other"
    }
}

lazy_static! {
    static ref STUDIO_BANNER: Regex = Regex::new(r"^Prisma Studio is up").unwrap();
    static ref ENV_LINE: Regex = Regex::new(r"^Environment variables loaded").unwrap();
    static ref SCHEMA_LOADED: Regex = Regex::new(r"^Prisma schema loaded").unwrap();
    static ref MIGRATE_OK: Regex = Regex::new(r"^Your database is now in sync").unwrap();
    static ref GENERATE_OK: Regex = Regex::new(r"^✔ Generated Prisma Client").unwrap();
    static ref VALIDATE_OK: Regex = Regex::new(r"is valid").unwrap();
    static ref ERROR_LINE: Regex = Regex::new(r"(?i)\b(error|Error)\b").unwrap();
}

/// Public entry point. Strips ANSI from `raw`, then dispatches to the
/// per-subcommand filter based on `args.subcmd()`.
pub fn filter_prisma(args: &PrismaArgs, raw: &str) -> Result<String> {
    let stripped = shell::strip_ansi(raw);
    let out = match args.subcmd() {
        "migrate" => filter_migrate(&stripped),
        "generate" => filter_generate(&stripped),
        "validate" => filter_validate(&stripped),
        _ => filter_default(&stripped),
    };
    Ok(out)
}

fn filter_migrate(input: &str) -> String {
    input
        .lines()
        .filter(|l| {
            let lo = l.to_lowercase();
            lo.contains("migration")
                || MIGRATE_OK.is_match(l)
                || ERROR_LINE.is_match(l)
                || lo.contains("applied")
                || lo.contains("warning")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_generate(input: &str) -> String {
    input
        .lines()
        .filter(|l| GENERATE_OK.is_match(l) || ERROR_LINE.is_match(l) || l.contains(" in "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_validate(input: &str) -> String {
    input
        .lines()
        .filter(|l| VALIDATE_OK.is_match(l) || ERROR_LINE.is_match(l))
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_default(input: &str) -> String {
    input
        .lines()
        .filter(|l| {
            !ENV_LINE.is_match(l) && !STUDIO_BANNER.is_match(l) && !SCHEMA_LOADED.is_match(l)
        })
        .take(20)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    macro_rules! roundtrip {
        ($name:ident, $sub:literal, $fixture:literal, $floor:expr) => {
            #[test]
            fn $name() {
                let raw = include_str!(concat!("../../../tests/fixtures/js/", $fixture));
                let args = PrismaArgs::from_argv(vec!["prisma".into(), $sub.into()]);
                let out = filter_prisma(&args, raw).expect("filter_prisma must not fail");
                insta::assert_snapshot!(stringify!($name), out);
                let raw_tokens = count_tokens(raw).max(1);
                let out_tokens = count_tokens(&out);
                let pct = 100usize.saturating_sub(out_tokens * 100 / raw_tokens);
                assert!(
                    pct >= $floor,
                    "{} savings {}% below floor {}%",
                    stringify!($name),
                    pct,
                    $floor
                );
            }
        };
    }

    // migrate fixture has lots of noise (env banner, ASCII update box,
    // ANSI cursor sequences) — comfortably hits 60%+.
    roundtrip!(prisma_migrate, "migrate", "prisma_migrate_raw.txt", 60);
    // generate output is intrinsically only 8 lines; floor relaxed to
    // 30% (see fixtures/js/prisma_generate_meta.txt).
    roundtrip!(prisma_generate, "generate", "prisma_generate_raw.txt", 30);
    // validate output is intrinsically 2 lines / ~11 tokens; floor
    // relaxed to 30% (see fixtures/js/prisma_validate_meta.txt).
    roundtrip!(prisma_validate, "validate", "prisma_validate_raw.txt", 30);

    #[test]
    fn subcmd_recognizes_known_verbs() {
        let cases = [
            ("migrate", "migrate"),
            ("generate", "generate"),
            ("validate", "validate"),
            ("studio", "studio"),
            ("format", "format"),
            ("init", "init"),
            ("db", "db"),
            ("introspect", "other"),
            ("", "other"),
        ];
        for (input, expected) in cases {
            let argv = if input.is_empty() {
                vec![]
            } else {
                vec![input.to_string()]
            };
            let args = PrismaArgs::from_argv(argv);
            assert_eq!(args.subcmd(), expected, "case {:?}", input);
        }
    }

    #[test]
    fn default_filter_strips_studio_and_env_banners() {
        let raw = "Environment variables loaded from .env\n\
                   Prisma schema loaded from schema.prisma\n\
                   Prisma Studio is up on http://localhost:5555\n\
                   Some other line\n";
        let args = PrismaArgs::from_argv(vec!["studio".into()]);
        let out = filter_prisma(&args, raw).unwrap();
        assert!(!out.contains("Environment variables"));
        assert!(!out.contains("Prisma Studio is up"));
        assert!(!out.contains("Prisma schema loaded"));
        assert!(out.contains("Some other line"));
    }

    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]
        #[test]
        fn prop_prisma_never_strips_error_lines(s in "\\PC{0,4096}") {
            let args = PrismaArgs::from_argv(vec!["migrate".into()]);
            let out = filter_prisma(&args, &s).unwrap_or_default();
            for e in s.lines().filter(|l| l.contains("error") || l.contains("Error")) {
                if !e.is_empty() {
                    prop_assert!(
                        out.contains(e),
                        "stripped error line: {:?}",
                        e
                    );
                }
            }
        }
    }
}
