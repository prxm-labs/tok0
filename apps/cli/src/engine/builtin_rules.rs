use std::sync::OnceLock;

use super::rules::{FilterConfig, FilterRule};

/// All built-in TOML rules embedded at compile time.
/// Each entry is (filename, toml_content).
const BUILTIN_TOML: &[(&str, &str)] = &[
    (
        "ansible-playbook",
        include_str!("../../src/rules/ansible-playbook.toml"),
    ),
    ("argocd", include_str!("../../src/rules/argocd.toml")),
    ("7z", include_str!("../../src/rules/sevenzip.toml")),
    ("act", include_str!("../../src/rules/act.toml")),
    ("ava", include_str!("../../src/rules/ava.toml")),
    ("awk", include_str!("../../src/rules/awk.toml")),
    ("bandit", include_str!("../../src/rules/bandit.toml")),
    (
        "basedpyright",
        include_str!("../../src/rules/basedpyright.toml"),
    ),
    ("biome", include_str!("../../src/rules/biome.toml")),
    (
        "brew-install",
        include_str!("../../src/rules/brew-install.toml"),
    ),
    ("buildah", include_str!("../../src/rules/buildah.toml")),
    ("checkov", include_str!("../../src/rules/checkov.toml")),
    (
        "bundle-install",
        include_str!("../../src/rules/bundle-install.toml"),
    ),
    ("bun", include_str!("../../src/rules/bun.toml")),
    ("bunx", include_str!("../../src/rules/bunx.toml")),
    (
        "cargo-bench",
        include_str!("../../src/rules/cargo-bench.toml"),
    ),
    ("cargo-fmt", include_str!("../../src/rules/cargo-fmt.toml")),
    (
        "composer-install",
        include_str!("../../src/rules/composer-install.toml"),
    ),
    ("cosign", include_str!("../../src/rules/cosign.toml")),
    ("crane", include_str!("../../src/rules/crane.toml")),
    ("cypress", include_str!("../../src/rules/cypress.toml")),
    ("dasel", include_str!("../../src/rules/dasel.toml")),
    ("deno", include_str!("../../src/rules/deno.toml")),
    ("df", include_str!("../../src/rules/df.toml")),
    ("dmesg", include_str!("../../src/rules/dmesg.toml")),
    (
        "dotnet-build",
        include_str!("../../src/rules/dotnet-build.toml"),
    ),
    ("du", include_str!("../../src/rules/du.toml")),
    ("esbuild", include_str!("../../src/rules/esbuild.toml")),
    ("eza", include_str!("../../src/rules/eza.toml")),
    (
        "fail2ban-client",
        include_str!("../../src/rules/fail2ban-client.toml"),
    ),
    ("flux", include_str!("../../src/rules/flux.toml")),
    ("format", include_str!("../../src/rules/format.toml")),
    ("gcc", include_str!("../../src/rules/gcc.toml")),
    ("gcloud", include_str!("../../src/rules/gcloud.toml")),
    ("gitleaks", include_str!("../../src/rules/gitleaks.toml")),
    ("glab", include_str!("../../src/rules/glab.toml")),
    ("gpg", include_str!("../../src/rules/gpg.toml")),
    ("gradle", include_str!("../../src/rules/gradle.toml")),
    ("grpcurl", include_str!("../../src/rules/grpcurl.toml")),
    ("grype", include_str!("../../src/rules/grype.toml")),
    ("hadolint", include_str!("../../src/rules/hadolint.toml")),
    ("helm", include_str!("../../src/rules/helm.toml")),
    ("htop", include_str!("../../src/rules/htop.toml")),
    ("httpie", include_str!("../../src/rules/httpie.toml")),
    ("ip", include_str!("../../src/rules/ip.toml")),
    ("iptables", include_str!("../../src/rules/iptables.toml")),
    ("jest", include_str!("../../src/rules/jest.toml")),
    ("jira", include_str!("../../src/rules/jira.toml")),
    ("jj", include_str!("../../src/rules/jj.toml")),
    (
        "journalctl",
        include_str!("../../src/rules/journalctl.toml"),
    ),
    ("jq", include_str!("../../src/rules/jq.toml")),
    ("just", include_str!("../../src/rules/just.toml")),
    ("keytool", include_str!("../../src/rules/keytool.toml")),
    ("kustomize", include_str!("../../src/rules/kustomize.toml")),
    ("make", include_str!("../../src/rules/make.toml")),
    (
        "markdownlint",
        include_str!("../../src/rules/markdownlint.toml"),
    ),
    ("mise", include_str!("../../src/rules/mise.toml")),
    (
        "mix-compile",
        include_str!("../../src/rules/mix-compile.toml"),
    ),
    (
        "mix-format",
        include_str!("../../src/rules/mix-format.toml"),
    ),
    ("mocha", include_str!("../../src/rules/mocha.toml")),
    ("mtr", include_str!("../../src/rules/mtr.toml")),
    ("mvn-build", include_str!("../../src/rules/mvn-build.toml")),
    ("npx", include_str!("../../src/rules/npx.toml")),
    ("nslookup", include_str!("../../src/rules/nslookup.toml")),
    ("nx", include_str!("../../src/rules/nx.toml")),
    ("ollama", include_str!("../../src/rules/ollama.toml")),
    ("oxlint", include_str!("../../src/rules/oxlint.toml")),
    ("parcel", include_str!("../../src/rules/parcel.toml")),
    ("ping", include_str!("../../src/rules/ping.toml")),
    ("pio-run", include_str!("../../src/rules/pio-run.toml")),
    (
        "playwright",
        include_str!("../../src/rules/playwright.toml"),
    ),
    ("pnpm", include_str!("../../src/rules/pnpm.toml")),
    ("podman", include_str!("../../src/rules/podman.toml")),
    (
        "poetry-install",
        include_str!("../../src/rules/poetry-install.toml"),
    ),
    (
        "pre-commit",
        include_str!("../../src/rules/pre-commit.toml"),
    ),
    ("prettier", include_str!("../../src/rules/prettier.toml")),
    ("ps", include_str!("../../src/rules/ps.toml")),
    (
        "quarto-render",
        include_str!("../../src/rules/quarto-render.toml"),
    ),
    ("rollup", include_str!("../../src/rules/rollup.toml")),
    ("rsync", include_str!("../../src/rules/rsync.toml")),
    ("sed", include_str!("../../src/rules/sed.toml")),
    ("semgrep", include_str!("../../src/rules/semgrep.toml")),
    (
        "shellcheck",
        include_str!("../../src/rules/shellcheck.toml"),
    ),
    (
        "shopify-theme",
        include_str!("../../src/rules/shopify-theme.toml"),
    ),
    ("skaffold", include_str!("../../src/rules/skaffold.toml")),
    ("skopeo", include_str!("../../src/rules/skopeo.toml")),
    ("snyk", include_str!("../../src/rules/snyk.toml")),
    ("sops", include_str!("../../src/rules/sops.toml")),
    ("sort", include_str!("../../src/rules/sort.toml")),
    (
        "spring-boot",
        include_str!("../../src/rules/spring-boot.toml"),
    ),
    ("ssh", include_str!("../../src/rules/ssh.toml")),
    ("stat", include_str!("../../src/rules/stat.toml")),
    ("step", include_str!("../../src/rules/step.toml")),
    ("swc", include_str!("../../src/rules/swc.toml")),
    ("sysctl", include_str!("../../src/rules/sysctl.toml")),
    (
        "swift-build",
        include_str!("../../src/rules/swift-build.toml"),
    ),
    (
        "systemctl-status",
        include_str!("../../src/rules/systemctl-status.toml"),
    ),
    ("task", include_str!("../../src/rules/task.toml")),
    (
        "terraform-plan",
        include_str!("../../src/rules/terraform-plan.toml"),
    ),
    ("tfsec", include_str!("../../src/rules/tfsec.toml")),
    ("tilt", include_str!("../../src/rules/tilt.toml")),
    ("top", include_str!("../../src/rules/top.toml")),
    (
        "traceroute",
        include_str!("../../src/rules/traceroute.toml"),
    ),
    ("trivy", include_str!("../../src/rules/trivy.toml")),
    ("tofu-fmt", include_str!("../../src/rules/tofu-fmt.toml")),
    ("tofu-init", include_str!("../../src/rules/tofu-init.toml")),
    ("tofu-plan", include_str!("../../src/rules/tofu-plan.toml")),
    (
        "tofu-validate",
        include_str!("../../src/rules/tofu-validate.toml"),
    ),
    ("tree", include_str!("../../src/rules/tree.toml")),
    (
        "trunk-build",
        include_str!("../../src/rules/trunk-build.toml"),
    ),
    ("tsup", include_str!("../../src/rules/tsup.toml")),
    ("turbo", include_str!("../../src/rules/turbo.toml")),
    ("ty", include_str!("../../src/rules/ty.toml")),
    ("uniq", include_str!("../../src/rules/uniq.toml")),
    ("unzip", include_str!("../../src/rules/unzip.toml")),
    ("vault", include_str!("../../src/rules/vault.toml")),
    ("uv-sync", include_str!("../../src/rules/uv-sync.toml")),
    ("webpack", include_str!("../../src/rules/webpack.toml")),
    ("wget", include_str!("../../src/rules/wget.toml")),
    (
        "xcodebuild",
        include_str!("../../src/rules/xcodebuild.toml"),
    ),
    ("yadm", include_str!("../../src/rules/yadm.toml")),
    ("yamllint", include_str!("../../src/rules/yamllint.toml")),
    ("yarn", include_str!("../../src/rules/yarn.toml")),
    ("yq", include_str!("../../src/rules/yq.toml")),
    ("zip", include_str!("../../src/rules/zip.toml")),
];

static BUILTIN_CONFIGS: OnceLock<Vec<FilterConfig>> = OnceLock::new();

/// Returns all built-in TOML filter rules, parsed once and cached.
pub fn builtin_rules() -> &'static [FilterConfig] {
    BUILTIN_CONFIGS.get_or_init(|| {
        BUILTIN_TOML
            .iter()
            .filter_map(
                |(name, toml_str)| match toml::from_str::<FilterRule>(toml_str) {
                    Ok(rule) => Some(rule.filter),
                    Err(e) => {
                        eprintln!(
                            "tok0: warning: failed to parse built-in rule '{}': {}",
                            name, e
                        );
                        None
                    }
                },
            )
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_builtin_rules_parse() {
        let rules = builtin_rules();
        assert_eq!(
            rules.len(),
            BUILTIN_TOML.len(),
            "All {} built-in TOML rules should parse successfully",
            BUILTIN_TOML.len()
        );
    }

    #[test]
    fn test_builtin_rules_have_names() {
        for rule in builtin_rules() {
            assert!(!rule.name.is_empty(), "Built-in rule must have a name");
        }
    }

    #[test]
    fn test_builtin_rules_have_commands() {
        for rule in builtin_rules() {
            assert!(
                !rule.commands.is_empty(),
                "Built-in rule '{}' must have at least one command",
                rule.name
            );
        }
    }

    #[test]
    fn test_builtin_rule_names_unique() {
        let rules = builtin_rules();
        let mut seen = std::collections::HashSet::new();
        for rule in rules {
            assert!(
                seen.insert(&rule.name),
                "Duplicate built-in rule name: '{}'",
                rule.name
            );
        }
    }

    #[test]
    fn test_builtin_rules_cached() {
        let a = builtin_rules();
        let b = builtin_rules();
        assert!(std::ptr::eq(a, b), "OnceLock should return same slice");
    }

    #[test]
    fn test_specific_rules_exist() {
        let rules = builtin_rules();
        let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"gcloud"), "Should contain gcloud rule");
        assert!(
            names.contains(&"cargo-fmt"),
            "Should contain cargo-fmt rule"
        );
        assert!(
            names.contains(&"xcodebuild"),
            "Should contain xcodebuild rule"
        );
        assert!(names.contains(&"uv-sync"), "Should contain uv-sync rule");
    }
}
