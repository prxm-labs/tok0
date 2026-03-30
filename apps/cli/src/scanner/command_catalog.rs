use lazy_static::lazy_static;

lazy_static! {
    static ref COMPRESSIBLE_PREFIXES: Vec<&'static str> = vec![
        // ── Git ──────────────────────────────────────────────────
        "git ", "gh ", "jj ", "yadm ",
        // ── Rust ─────────────────────────────────────────────────
        "cargo ",
        // ── JS/TS ────────────────────────────────────────────────
        "npm ", "pnpm ", "bun ", "yarn ", "deno ",
        "vitest", "jest", "tsc", "tsgo",
        "eslint", "oxlint", "biome",
        "turbo ", "nx ",
        // ── Python ───────────────────────────────────────────────
        "pytest", "ruff ", "mypy ", "pip ", "pip3 ", "uv ",
        "poetry ", "basedpyright", "ty ",
        // ── Go ───────────────────────────────────────────────────
        "go ", "golangci-lint",
        // ── Ruby ─────────────────────────────────────────────────
        "rspec", "bundle ", "rake",
        // ── Dotnet ───────────────────────────────────────────────
        "dotnet ",
        // ── Java ─────────────────────────────────────────────────
        "gradle", "gradlew", "./gradlew",
        "mvn ",
        // ── Elixir ───────────────────────────────────────────────
        "mix ",
        // ── Swift ────────────────────────────────────────────────
        "swift build", "xcodebuild",
        // ── System ───────────────────────────────────────────────
        "ls", "find ", "grep ", "rg ", "diff ", "cat ", "head ", "tail ",
        "wc ", "env", "du ", "df ", "ps ", "stat ", "ping ",
        "touch ", "mkdir ", "cp ", "mv ", "rm ", "chmod ", "chown ", "ln ",
        "ssh ", "rsync ",
        // ── Cloud ────────────────────────────────────────────────
        "docker ", "podman ", "kubectl ", "k ", "terraform ", "tf ",
        "tofu ", "helm ", "aws ", "gcloud ", "az ",
        "skopeo ",
        // ── Build ────────────────────────────────────────────────
        "make ", "gmake ", "cmake ", "bazel ",
        "just ", "task ",
        // ── Package managers ─────────────────────────────────────
        "brew ", "apt ", "apt-get ", "dnf ",
        "composer ", "pio ",
        // ── Lint ─────────────────────────────────────────────────
        "shellcheck", "hadolint", "markdownlint", "yamllint",
        // ── CI/CD ────────────────────────────────────────────────
        "pre-commit",
        // ── Version managers ─────────────────────────────────────
        "mise ",
        // ── Misc tools ───────────────────────────────────────────
        "ansible-playbook", "jira ", "jq ",
        "ollama ", "quarto ", "sops ",
        "shopify ", "systemctl ",
        "fail2ban-client", "iptables",
        "spring-boot",
    ];
}

pub fn is_compressible(command: &str) -> bool {
    let cmd = command.trim();
    COMPRESSIBLE_PREFIXES
        .iter()
        .any(|prefix| cmd.starts_with(prefix) || cmd == prefix.trim())
}

pub fn classify(command: &str) -> Option<&'static str> {
    let cmd = command.trim();

    // Git
    if cmd.starts_with("git ")
        || cmd.starts_with("gh ")
        || cmd.starts_with("jj ")
        || cmd.starts_with("yadm ")
    {
        return Some("git");
    }

    // Rust
    if cmd.starts_with("cargo ") {
        return Some("rust");
    }

    // JS/TS
    if cmd.starts_with("npm ")
        || cmd.starts_with("pnpm ")
        || cmd.starts_with("yarn ")
        || cmd.starts_with("bun ")
        || cmd.starts_with("deno ")
        || cmd.starts_with("vitest")
        || cmd.starts_with("jest")
        || cmd.starts_with("tsc")
        || cmd.starts_with("tsgo")
        || cmd.starts_with("eslint")
        || cmd.starts_with("oxlint")
        || cmd.starts_with("biome")
        || cmd.starts_with("turbo ")
        || cmd.starts_with("nx ")
    {
        return Some("js");
    }

    // Python
    if cmd.starts_with("pytest")
        || cmd.starts_with("ruff")
        || cmd.starts_with("mypy")
        || cmd.starts_with("pip ")
        || cmd.starts_with("pip3 ")
        || cmd.starts_with("uv ")
        || cmd.starts_with("poetry ")
        || cmd.starts_with("basedpyright")
        || cmd.starts_with("ty ")
    {
        return Some("python");
    }

    // Go
    if cmd.starts_with("go ") || cmd.starts_with("golangci-lint") {
        return Some("go");
    }

    // Ruby
    if cmd.starts_with("rspec") || cmd.starts_with("bundle ") || cmd.starts_with("rake") {
        return Some("ruby");
    }

    // Dotnet
    if cmd.starts_with("dotnet ") {
        return Some("dotnet");
    }

    // Java
    if cmd.starts_with("gradle")
        || cmd.starts_with("gradlew")
        || cmd.starts_with("./gradlew")
        || cmd.starts_with("mvn ")
    {
        return Some("java");
    }

    // Elixir
    if cmd.starts_with("mix ") {
        return Some("elixir");
    }

    // Swift
    if cmd.starts_with("swift build") || cmd.starts_with("xcodebuild") {
        return Some("swift");
    }

    // Cloud
    if cmd.starts_with("docker ")
        || cmd.starts_with("podman ")
        || cmd.starts_with("kubectl ")
        || cmd.starts_with("k ")
        || cmd.starts_with("terraform ")
        || cmd.starts_with("tf ")
        || cmd.starts_with("tofu ")
        || cmd.starts_with("helm ")
        || cmd.starts_with("aws ")
        || cmd.starts_with("gcloud ")
        || cmd.starts_with("az ")
        || cmd.starts_with("skopeo ")
        || cmd.starts_with("ansible-playbook")
    {
        return Some("cloud");
    }

    // System
    if cmd.starts_with("ls")
        || cmd.starts_with("find ")
        || cmd.starts_with("grep ")
        || cmd.starts_with("rg ")
        || cmd.starts_with("diff ")
        || cmd.starts_with("cat ")
        || cmd.starts_with("head ")
        || cmd.starts_with("tail ")
        || cmd.starts_with("du ")
        || cmd.starts_with("df")
        || cmd.starts_with("wc ")
        || cmd.starts_with("env")
        || cmd.starts_with("ps ")
        || cmd.starts_with("stat ")
        || cmd.starts_with("ping ")
        || cmd.starts_with("touch ")
        || cmd.starts_with("mkdir ")
        || cmd.starts_with("cp ")
        || cmd.starts_with("mv ")
        || cmd.starts_with("rm ")
        || cmd.starts_with("chmod ")
        || cmd.starts_with("chown ")
        || cmd.starts_with("ln ")
        || cmd.starts_with("ssh ")
        || cmd.starts_with("rsync ")
        || cmd.starts_with("systemctl ")
        || cmd.starts_with("iptables")
        || cmd.starts_with("fail2ban-client")
    {
        return Some("system");
    }

    // Package managers
    if cmd.starts_with("brew ")
        || cmd.starts_with("apt ")
        || cmd.starts_with("apt-get ")
        || cmd.starts_with("dnf ")
        || cmd.starts_with("composer ")
    {
        return Some("pkg");
    }

    // Build
    if cmd.starts_with("make ")
        || cmd.starts_with("gmake ")
        || cmd.starts_with("cmake ")
        || cmd.starts_with("bazel ")
        || cmd.starts_with("just ")
        || cmd.starts_with("task ")
    {
        return Some("build");
    }

    // Lint
    if cmd.starts_with("shellcheck")
        || cmd.starts_with("hadolint")
        || cmd.starts_with("markdownlint")
        || cmd.starts_with("yamllint")
    {
        return Some("lint");
    }

    // CI/CD
    if cmd.starts_with("pre-commit") {
        return Some("cicd");
    }

    // Database
    if cmd.starts_with("psql")
        || cmd.starts_with("redis-cli")
        || cmd.starts_with("mysql")
        || cmd.starts_with("sqlite3")
        || cmd.starts_with("mongo")
    {
        return Some("db");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressible_commands() {
        assert!(is_compressible("git status"));
        assert!(is_compressible("cargo build --release"));
        assert!(is_compressible("npm install"));
        assert!(is_compressible("ls"));
        assert!(is_compressible("ls -la"));
        assert!(is_compressible("docker ps"));
    }

    #[test]
    fn test_compressible_new_commands() {
        // Silent commands
        assert!(is_compressible("touch file.txt"));
        assert!(is_compressible("mkdir -p foo/bar"));
        assert!(is_compressible("cp src dst"));
        assert!(is_compressible("mv old new"));
        assert!(is_compressible("rm file.txt"));
        assert!(is_compressible("chmod 755 script.sh"));
        assert!(is_compressible("chown user:group file"));
        assert!(is_compressible("ln -s target link"));

        // TOML-covered commands
        assert!(is_compressible("gcloud compute instances list"));
        assert!(is_compressible("jj status"));
        assert!(is_compressible("just build"));
        assert!(is_compressible("mix compile"));
        assert!(is_compressible("ansible-playbook site.yml"));
        assert!(is_compressible("tofu plan"));
        assert!(is_compressible("helm install my-release"));
        assert!(is_compressible("turbo run build"));
        assert!(is_compressible("xcodebuild -scheme MyApp"));
        assert!(is_compressible("basedpyright src/"));
        assert!(is_compressible("biome check ."));
        assert!(is_compressible("oxlint ."));

        // New native compressor commands
        assert!(is_compressible("ssh user@host"));
        assert!(is_compressible("rsync -avz src/ dst/"));
        assert!(is_compressible("ping google.com"));
        assert!(is_compressible("stat file.txt"));
    }

    #[test]
    fn test_non_compressible_commands() {
        assert!(!is_compressible("echo hello"));
        assert!(!is_compressible("curl https://example.com"));
        assert!(!is_compressible("vim file.txt"));
        assert!(!is_compressible("cd /tmp"));
    }

    #[test]
    fn test_classify_categories() {
        assert_eq!(classify("git log --oneline"), Some("git"));
        assert_eq!(classify("gh pr list"), Some("git"));
        assert_eq!(classify("cargo test"), Some("rust"));
        assert_eq!(classify("npm run build"), Some("js"));
        assert_eq!(classify("pnpm install"), Some("js"));
        assert_eq!(classify("pytest -v"), Some("python"));
        assert_eq!(classify("ruff check ."), Some("python"));
        assert_eq!(classify("go build ./..."), Some("go"));
        assert_eq!(classify("docker compose up"), Some("cloud"));
        assert_eq!(classify("ls -la"), Some("system"));
        assert_eq!(classify("find . -name '*.rs'"), Some("system"));
        assert_eq!(classify("brew install foo"), Some("pkg"));
        assert_eq!(classify("make all"), Some("build"));
        assert_eq!(classify("eslint src/"), Some("js"));
    }

    #[test]
    fn test_classify_new_categories() {
        assert_eq!(classify("rspec spec/"), Some("ruby"));
        assert_eq!(classify("bundle install"), Some("ruby"));
        assert_eq!(classify("dotnet build"), Some("dotnet"));
        assert_eq!(classify("gradle build"), Some("java"));
        assert_eq!(classify("mvn compile"), Some("java"));
        assert_eq!(classify("mix compile"), Some("elixir"));
        assert_eq!(classify("swift build"), Some("swift"));
        assert_eq!(classify("xcodebuild -scheme App"), Some("swift"));
        assert_eq!(classify("pre-commit run --all-files"), Some("cicd"));
    }

    #[test]
    fn test_classify_silent_commands() {
        assert_eq!(classify("touch foo.txt"), Some("system"));
        assert_eq!(classify("mkdir -p dir"), Some("system"));
        assert_eq!(classify("cp src dst"), Some("system"));
        assert_eq!(classify("mv old new"), Some("system"));
        assert_eq!(classify("rm file.txt"), Some("system"));
        assert_eq!(classify("chmod 755 script"), Some("system"));
        assert_eq!(classify("chown user file"), Some("system"));
        assert_eq!(classify("ln -s target link"), Some("system"));
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(classify("echo hello"), None);
        assert_eq!(classify("curl -s http://example.com"), None);
        assert_eq!(classify("vim"), None);
        assert_eq!(classify("cd /tmp"), None);
    }

    #[test]
    fn test_prefix_count() {
        assert!(
            COMPRESSIBLE_PREFIXES.len() >= 100,
            "Expected >=100 compressible prefixes, got {}",
            COMPRESSIBLE_PREFIXES.len()
        );
    }
}
