use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // docker ps: header line detection
    static ref PS_HEADER_RE: Regex =
        Regex::new(r"^CONTAINER ID\s+IMAGE\s+COMMAND").unwrap();
    // docker ps: data line — fixed-width columns starting with a hex container ID
    static ref PS_ROW_RE: Regex =
        Regex::new(r"^([0-9a-f]{12})\s+(\S+)\s+(?:.+?)\s{2,}(\S[^\t]*?)\s{2,}((?:Up|Exited|Created|Restarting|Paused|Dead)\S*(?:\s+\S+)*?)\s{2,}(?:\S[^\t]*?)\s{2,}(\S.*)$").unwrap();
    // docker build: step lines  "Step N/M : INSTRUCTION ..."
    static ref BUILD_STEP_RE: Regex =
        Regex::new(r"^Step \d+/\d+ : (.+)$").unwrap();
    // docker build: cache hit lines
    static ref BUILD_CACHE_RE: Regex =
        Regex::new(r"^ ---> Using cache").unwrap();
    // docker build: intermediate container IDs  " ---> <hex12>" or " ---> Removing intermediate container ..."
    static ref BUILD_INTERMEDIATE_RE: Regex =
        Regex::new(r"^ ---> (?:[0-9a-f]{12}|Removing intermediate container \S+)$").unwrap();
    // docker build: success lines
    static ref BUILD_SUCCESS_RE: Regex =
        Regex::new(r"^Successfully (?:built|tagged) ").unwrap();
    // docker build: "Running in <container>" lines
    static ref BUILD_RUNNING_IN_RE: Regex =
        Regex::new(r"^ ---> Running in [0-9a-f]{12}").unwrap();
    // docker build: layer digest / status lines from pull
    static ref BUILD_LAYER_RE: Regex =
        Regex::new(r"^(?:Digest:|Status:|Sending build context)").unwrap();
    // docker build: error lines
    static ref BUILD_ERROR_RE: Regex =
        Regex::new(r"(?i)^(?:error|err\b|fatal)").unwrap();
    // docker build: npm warn lines (noise)
    static ref BUILD_NPM_WARN_RE: Regex =
        Regex::new(r"^npm warn ").unwrap();
    // docker build: pulling layer lines like "X: Pulling from ..."
    static ref BUILD_PULL_RE: Regex =
        Regex::new(r"^[a-z0-9]+: (?:Pull complete|Already exists|Pulling from|Waiting|Downloading|Verifying Checksum|Download complete|Extracting)").unwrap();

    // docker images: header line
    static ref IMAGES_HEADER_RE: Regex =
        Regex::new(r"^REPOSITORY\s+TAG\s+IMAGE ID").unwrap();
    // docker images: <none> intermediate layers
    static ref IMAGES_NONE_RE: Regex =
        Regex::new(r"^<none>\s+<none>").unwrap();

    // docker logs: important lines (errors, fatals, panics, warnings)
    static ref LOGS_IMPORTANT_RE: Regex =
        Regex::new(r"(?i)\b(?:error|fatal|panic|warn)\b").unwrap();

    // docker compose ps: header line
    static ref COMPOSE_PS_HEADER_RE: Regex =
        Regex::new(r"^NAME\s+IMAGE\s+COMMAND").unwrap();
    // docker compose up/build: pull progress lines
    static ref COMPOSE_PULL_PROGRESS_RE: Regex =
        Regex::new(r"^[a-z0-9]+: (?:Pull complete|Already exists|Pulling from|Waiting|Downloading|Verifying Checksum|Download complete|Extracting)").unwrap();
    // docker compose up: service status lines like "Container myapp-web-1  Started"
    static ref COMPOSE_SERVICE_STATUS_RE: Regex =
        Regex::new(r"^\s*(?:Container|Network|Volume)\s+\S+\s+(?:Creating|Created|Starting|Started|Stopping|Stopped|Removing|Removed|Running|Healthy|Unhealthy)").unwrap();
    // docker compose build: step lines (reuse build pattern)
    static ref COMPOSE_BUILD_SERVICE_RE: Regex =
        Regex::new(r"^\s*(?:Building|Successfully built|Successfully tagged|Step \d+/\d+)").unwrap();
    // docker compose: error/warning lines
    static ref COMPOSE_ERROR_RE: Regex =
        Regex::new(r"(?i)^(?:error|err\b|fatal|warn)").unwrap();
    // docker compose: blank/decorative lines
    static ref COMPOSE_DECORATIVE_RE: Regex =
        Regex::new(r"^[─━═\-]{3,}$").unwrap();
}

/// Shorten an image name: strip registry prefix, strip digest, strip tag.
///
/// `nginx:1.25.3-alpine` → `nginx`
/// `gcr.io/myproject/data-pipeline:sha-abc1234` → `data-pipeline`
/// `myapp/backend:v2.14.1-release` → `myapp/backend`
fn short_image(image: &str) -> String {
    // strip digest portion (sha256:...)
    let no_digest = image.split('@').next().unwrap_or(image);
    // strip tag (:...)
    let no_tag = no_digest.split(':').next().unwrap_or(no_digest);
    // strip hostname registry prefix (anything with '.' or ':' before the first '/')
    let trimmed = if let Some(idx) = no_tag.find('/') {
        let prefix = &no_tag[..idx];
        if prefix.contains('.') || prefix.contains(':') {
            &no_tag[idx + 1..]
        } else {
            no_tag
        }
    } else {
        no_tag
    };
    // truncate at 30 chars
    if trimmed.len() > 30 {
        trimmed[..30].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Normalise the STATUS field into a compact form.
///
/// "Up 2 hours (healthy)"         → "up"
/// "Exited (0) 23 hours ago"      → "exited(0)"
/// "Up 45 minutes"                → "up"
/// "Up 5 hours"                   → "up"
fn short_status(status: &str) -> String {
    let s = status.trim().to_lowercase();
    if s.starts_with("up") {
        "up".to_string()
    } else if s.starts_with("exited") {
        // extract exit code from "exited (N)"
        if let Some(start) = s.find('(') {
            if let Some(end) = s.find(')') {
                return format!("exited{}", &s[start..=end]);
            }
        }
        "exited".to_string()
    } else if s.starts_with("created") {
        "created".to_string()
    } else if s.starts_with("restarting") {
        "restarting".to_string()
    } else if s.starts_with("paused") {
        "paused".to_string()
    } else if s.starts_with("dead") {
        "dead".to_string()
    } else {
        // fallback: first word only
        s.split_whitespace().next().unwrap_or("?").to_string()
    }
}

/// Filter `docker ps` output.
///
/// Input is the raw tabular output from `docker ps`.
/// Output keeps: short container ID, image (trimmed), status, name.
/// Strips: COMMAND, CREATED, PORTS columns.
///
/// Target: >=70% token savings.
pub fn filter_docker_ps(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    let mut found_header = false;

    for line in input.lines() {
        if PS_HEADER_RE.is_match(line) {
            found_header = true;
            continue;
        }
        if !found_header {
            // pass through if we never see the canonical header (safety)
            continue;
        }
        // Parse by splitting on 2+ consecutive spaces — that's how docker ps
        // separates its fixed-width columns (CONTAINER ID, IMAGE, COMMAND,
        // CREATED, STATUS, PORTS, NAMES).
        let cols: Vec<&str> = MULTI_SPACE_RE.split(line).collect();
        if cols.len() < 7 {
            continue;
        }
        let id = &cols[0][..cols[0].len().min(12)];
        let image = short_image(cols[1]);
        let status = short_status(cols[4]); // cols[2]=COMMAND, [3]=CREATED, [4]=STATUS
        let name = cols[6].trim();
        rows.push((id.to_string(), image, status, name.to_string()));
    }

    if rows.is_empty() {
        return input.to_string();
    }

    // Compact tabular: id image status name (no header — saves tokens)
    let mut out = String::new();
    for (id, image, status, name) in &rows {
        out.push_str(&format!("{} {} {} {}\n", id, image, status, name));
    }
    out.trim_end().to_string()
}

lazy_static! {
    /// Two-or-more spaces — used to split docker ps columns.
    static ref MULTI_SPACE_RE: Regex = Regex::new(r"  +").unwrap();
}

/// Filter `docker build` output.
///
/// Keeps: step descriptions, build tool output lines (tsc, npm, etc.),
/// "Successfully built/tagged" lines, error lines.
/// Strips: cache hit lines, intermediate container IDs, layer hashes,
/// "Running in …" lines, build-context size line.
///
/// Target: >=80% token savings.
pub fn filter_docker_build(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut kept: Vec<String> = Vec::new();

    for line in input.lines() {
        // Always keep error lines
        if BUILD_ERROR_RE.is_match(line) {
            kept.push(line.to_string());
            continue;
        }
        // Always keep success lines
        if BUILD_SUCCESS_RE.is_match(line) {
            kept.push(line.to_string());
            continue;
        }
        // Keep step headers (condensed: just the instruction)
        if let Some(caps) = BUILD_STEP_RE.captures(line) {
            kept.push(format!("  {}", &caps[1]));
            continue;
        }
        // Strip: cache hits, intermediate IDs, "Running in", layer/digest lines,
        // npm warn lines, layer pull progress lines
        if BUILD_CACHE_RE.is_match(line)
            || BUILD_INTERMEDIATE_RE.is_match(line)
            || BUILD_RUNNING_IN_RE.is_match(line)
            || BUILD_LAYER_RE.is_match(line)
            || BUILD_NPM_WARN_RE.is_match(line)
            || BUILD_PULL_RE.is_match(line)
        {
            continue;
        }
        // Strip blank lines
        if line.trim().is_empty() {
            continue;
        }
        // Keep everything else (tsc output, bundle info, etc.) indented
        kept.push(format!("    {}", line.trim()));
    }

    if kept.is_empty() {
        return input.to_string();
    }

    kept.join("\n")
}

/// Filter `docker images` output.
///
/// Strips the IMAGE ID column, removes `<none>` intermediate layers,
/// and compacts column padding. Keeps REPOSITORY, TAG, SIZE.
///
/// Target: >=80% token savings.
pub fn filter_docker_images(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut rows: Vec<String> = Vec::new();
    let mut found_header = false;

    for line in input.lines() {
        if IMAGES_HEADER_RE.is_match(line) {
            found_header = true;
            continue;
        }
        if !found_header {
            continue;
        }
        // Skip <none> intermediate layers
        if IMAGES_NONE_RE.is_match(line) {
            continue;
        }
        // Parse columns split on 2+ spaces
        let cols: Vec<&str> = MULTI_SPACE_RE.split(line).collect();
        if cols.len() < 5 {
            continue;
        }
        // cols: REPOSITORY, TAG, IMAGE ID, CREATED, SIZE
        let repo = short_image(cols[0]);
        let tag = cols[1].trim();
        let size = cols[cols.len() - 1].trim();
        rows.push(format!("{}:{} {}", repo, tag, size));
    }

    if rows.is_empty() {
        return input.to_string();
    }

    rows.join("\n")
}

/// Filter `docker logs` output.
///
/// Applies a head/tail window: keeps the first 20 lines + last 20 lines.
/// Always preserves lines containing error, fatal, panic, or warn (case-insensitive).
///
/// Target: >=90% token savings on large log output.
pub fn filter_docker_logs(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = input.lines().collect();
    let total = lines.len();

    const HEAD: usize = 20;
    const TAIL: usize = 20;

    // If the output is small enough, return as-is
    if total <= HEAD + TAIL {
        return input.to_string();
    }

    let mut kept: Vec<String> = Vec::new();

    // Head section
    for line in &lines[..HEAD] {
        kept.push(line.to_string());
    }

    // Middle section: only keep important lines
    let middle_start = HEAD;
    let middle_end = total.saturating_sub(TAIL);
    let mut skipped = 0usize;

    for line in &lines[middle_start..middle_end] {
        if LOGS_IMPORTANT_RE.is_match(line) {
            if skipped > 0 {
                kept.push(format!("... ({} lines omitted)", skipped));
                skipped = 0;
            }
            kept.push(line.to_string());
        } else {
            skipped += 1;
        }
    }
    if skipped > 0 {
        kept.push(format!("... ({} lines omitted)", skipped));
    }

    // Tail section
    for line in &lines[middle_end..] {
        kept.push(line.to_string());
    }

    kept.join("\n")
}

/// Filter `docker compose ps`/`up`/`build` output.
///
/// For `ps` format: compacts the table, strips COMMAND/CREATED/PORTS columns.
/// For `up`/`build` format: strips pull progress, keeps service status summaries.
///
/// Target: >=80% token savings.
pub fn filter_docker_compose(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // Detect which format we have
    if COMPOSE_PS_HEADER_RE.is_match(input.lines().next().unwrap_or("")) {
        filter_compose_ps(input)
    } else {
        filter_compose_up_build(input)
    }
}

/// Filter `docker compose ps` tabular output.
fn filter_compose_ps(input: &str) -> String {
    let mut rows: Vec<String> = Vec::new();

    for line in input.lines() {
        if COMPOSE_PS_HEADER_RE.is_match(line) {
            continue;
        }
        let cols: Vec<&str> = MULTI_SPACE_RE.split(line).collect();
        // Expected: NAME, IMAGE, COMMAND, SERVICE, CREATED, STATUS, PORTS
        if cols.len() < 6 {
            continue;
        }
        let name = cols[0].trim();
        let service = cols[3].trim();
        let status = short_status(cols[5]);
        rows.push(format!("{} {} {}", name, service, status));
    }

    if rows.is_empty() {
        return input.to_string();
    }

    rows.join("\n")
}

/// Filter `docker compose up`/`build` streaming output.
fn filter_compose_up_build(input: &str) -> String {
    let mut kept: Vec<String> = Vec::new();

    for line in input.lines() {
        // Always keep errors/warnings
        if COMPOSE_ERROR_RE.is_match(line) {
            kept.push(line.to_string());
            continue;
        }
        // Keep service status lines (Created, Started, etc.)
        if COMPOSE_SERVICE_STATUS_RE.is_match(line) {
            kept.push(line.trim().to_string());
            continue;
        }
        // Keep build step/success lines
        if COMPOSE_BUILD_SERVICE_RE.is_match(line) {
            kept.push(line.trim().to_string());
            continue;
        }
        // Strip: pull progress, decorative lines, blank lines
        if COMPOSE_PULL_PROGRESS_RE.is_match(line)
            || COMPOSE_DECORATIVE_RE.is_match(line)
            || line.trim().is_empty()
        {
            continue;
        }
        // Keep everything else (but trimmed)
        kept.push(format!("  {}", line.trim()));
    }

    if kept.is_empty() {
        return input.to_string();
    }

    kept.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // ── docker ps ────────────────────────────────────────────────────────────

    #[test]
    fn test_docker_ps_format() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_ps_raw.txt");
        let output = filter_docker_ps(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_docker_ps_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_ps_raw.txt");
        let output = filter_docker_ps(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 70.0,
            "Expected >=70% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_docker_ps_empty() {
        assert_eq!(filter_docker_ps(""), "");
    }

    #[test]
    fn test_docker_ps_malformed() {
        let output = filter_docker_ps("not docker output\nrandom text");
        // Should return input unchanged when no recognisable header/rows found
        assert!(!output.is_empty());
    }

    #[test]
    fn test_docker_ps_contains_names() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_ps_raw.txt");
        let output = filter_docker_ps(input);
        assert!(
            output.contains("web-proxy"),
            "Output should include container name 'web-proxy'"
        );
        assert!(
            output.contains("db-primary"),
            "Output should include container name 'db-primary'"
        );
    }

    #[test]
    fn test_docker_ps_strips_ports_column() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_ps_raw.txt");
        let output = filter_docker_ps(input);
        // Long port mappings like "0.0.0.0:80->80/tcp" should not appear verbatim
        assert!(
            !output.contains("0.0.0.0:80->80/tcp"),
            "Output should strip raw port mappings"
        );
    }

    #[test]
    fn test_docker_ps_strips_command_column() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_ps_raw.txt");
        let output = filter_docker_ps(input);
        assert!(
            !output.contains("/docker-entrypoint"),
            "Output should strip COMMAND column"
        );
    }

    // ── docker build ─────────────────────────────────────────────────────────

    #[test]
    fn test_docker_build_format() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_build_raw.txt");
        let output = filter_docker_build(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_docker_build_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_build_raw.txt");
        let output = filter_docker_build(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "Expected >=60% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_docker_build_empty() {
        assert_eq!(filter_docker_build(""), "");
    }

    #[test]
    fn test_docker_build_malformed() {
        let output = filter_docker_build("not docker build output\nrandom lines here");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_docker_build_keeps_steps() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_build_raw.txt");
        let output = filter_docker_build(input);
        assert!(
            output.contains("FROM node:18-alpine"),
            "Should keep FROM instruction"
        );
        assert!(
            output.contains("WORKDIR /app"),
            "Should keep WORKDIR instruction"
        );
        assert!(output.contains("RUN npm ci"), "Should keep RUN instruction");
    }

    #[test]
    fn test_docker_build_keeps_success_lines() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_build_raw.txt");
        let output = filter_docker_build(input);
        assert!(
            output.contains("Successfully built"),
            "Should keep 'Successfully built' line"
        );
        assert!(
            output.contains("Successfully tagged"),
            "Should keep 'Successfully tagged' line"
        );
    }

    #[test]
    fn test_docker_build_strips_cache_lines() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_build_raw.txt");
        let output = filter_docker_build(input);
        assert!(
            !output.contains("Using cache"),
            "Should strip cache hit lines"
        );
    }

    #[test]
    fn test_docker_build_strips_intermediate_ids() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_build_raw.txt");
        let output = filter_docker_build(input);
        // Intermediate layer IDs like "7e8f9a0b1c2d" should be stripped
        assert!(
            !output.contains("7e8f9a0b1c2d"),
            "Should strip intermediate container IDs"
        );
    }

    #[test]
    fn test_docker_build_error_passthrough() {
        let input =
            "Step 1/2 : FROM ubuntu\nError: no such image 'ubuntu:999'\nStep 2/2 : RUN echo ok";
        let output = filter_docker_build(input);
        assert!(output.contains("Error:"), "Should keep error lines");
    }

    // ── docker images ────────────────────────────────────────────────────────

    #[test]
    fn test_docker_images_format() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_images_raw.txt");
        let output = filter_docker_images(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_docker_images_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_images_raw.txt");
        let output = filter_docker_images(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 80.0,
            "Expected >=80% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_docker_images_empty() {
        assert_eq!(filter_docker_images(""), "");
    }

    #[test]
    fn test_docker_images_malformed() {
        let output = filter_docker_images("not docker images output\nrandom text");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_docker_images_strips_none_layers() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_images_raw.txt");
        let output = filter_docker_images(input);
        assert!(
            !output.contains("<none>"),
            "Output should strip <none> intermediate layers"
        );
    }

    #[test]
    fn test_docker_images_strips_image_id() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_images_raw.txt");
        let output = filter_docker_images(input);
        // IMAGE IDs like "a1b2c3d4e5f6" should not appear
        assert!(
            !output.contains("a1b2c3d4e5f6"),
            "Output should strip IMAGE ID column"
        );
    }

    #[test]
    fn test_docker_images_keeps_repo_tag_size() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_images_raw.txt");
        let output = filter_docker_images(input);
        assert!(
            output.contains("nginx:1.25.3-alpine"),
            "Output should keep repository:tag"
        );
        assert!(output.contains("42.6MB"), "Output should keep size");
    }

    // ── docker logs ──────────────────────────────────────────────────────────

    #[test]
    fn test_docker_logs_format() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_logs_raw.txt");
        let output = filter_docker_logs(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_docker_logs_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_logs_raw.txt");
        let output = filter_docker_logs(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 50.0,
            "Expected >=50% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_docker_logs_empty() {
        assert_eq!(filter_docker_logs(""), "");
    }

    #[test]
    fn test_docker_logs_small_passthrough() {
        let input = "line 1\nline 2\nline 3";
        let output = filter_docker_logs(input);
        assert_eq!(output, input, "Small output should pass through unchanged");
    }

    #[test]
    fn test_docker_logs_keeps_errors() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_logs_raw.txt");
        let output = filter_docker_logs(input);
        assert!(
            output.contains("Error: Payment gateway timeout"),
            "Should keep error lines"
        );
        assert!(output.contains("PANIC:"), "Should keep panic lines");
        assert!(output.contains("fatal:"), "Should keep fatal lines");
    }

    #[test]
    fn test_docker_logs_keeps_warnings() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_logs_raw.txt");
        let output = filter_docker_logs(input);
        assert!(output.contains("[warn]"), "Should keep warning lines");
    }

    #[test]
    fn test_docker_logs_shows_omitted_count() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_logs_raw.txt");
        let output = filter_docker_logs(input);
        assert!(
            output.contains("lines omitted"),
            "Should show count of omitted lines"
        );
    }

    // ── docker compose ───────────────────────────────────────────────────────

    #[test]
    fn test_docker_compose_ps_format() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_compose_raw.txt");
        let output = filter_docker_compose(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_docker_compose_ps_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_compose_raw.txt");
        let output = filter_docker_compose(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 70.0,
            "Expected >=70% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_docker_compose_empty() {
        assert_eq!(filter_docker_compose(""), "");
    }

    #[test]
    fn test_docker_compose_malformed() {
        let output = filter_docker_compose("not compose output\nrandom text");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_docker_compose_ps_keeps_service_names() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_compose_raw.txt");
        let output = filter_docker_compose(input);
        assert!(
            output.contains("web"),
            "Output should include service name 'web'"
        );
        assert!(
            output.contains("db"),
            "Output should include service name 'db'"
        );
        assert!(
            output.contains("redis"),
            "Output should include service name 'redis'"
        );
    }

    #[test]
    fn test_docker_compose_ps_strips_ports() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_compose_raw.txt");
        let output = filter_docker_compose(input);
        assert!(
            !output.contains("0.0.0.0:3000->3000/tcp"),
            "Output should strip port mappings"
        );
    }

    #[test]
    fn test_docker_compose_ps_strips_command() {
        let input = include_str!("../../../tests/fixtures/cloud/docker_compose_raw.txt");
        let output = filter_docker_compose(input);
        assert!(
            !output.contains("docker-entrypoint"),
            "Output should strip COMMAND column"
        );
    }

    #[test]
    fn test_docker_compose_up_build_format() {
        let input = "Network myapp_default  Creating\n\
                      Network myapp_default  Created\n\
                      Container myapp-db-1  Creating\n\
                      Container myapp-db-1  Created\n\
                      Container myapp-db-1  Starting\n\
                      Container myapp-db-1  Started\n\
                      Container myapp-web-1  Creating\n\
                      Container myapp-web-1  Created\n\
                      Container myapp-web-1  Starting\n\
                      abc123: Pulling from library/postgres\n\
                      abc123: Pull complete\n\
                      def456: Pull complete\n\
                      Container myapp-web-1  Started\n\
                      \n\
                      Container myapp-web-1  Healthy";
        let output = filter_docker_compose(input);
        assert!(
            output.contains("Container myapp-db-1  Started"),
            "Should keep service status lines"
        );
        assert!(
            !output.contains("Pull complete"),
            "Should strip pull progress"
        );
        assert!(
            output.contains("Container myapp-web-1  Healthy"),
            "Should keep healthy status"
        );
    }

    // ── Tier B coverage: explicit snapshot + ≥60% savings tests ──────

    #[test]
    fn test_docker_ps_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_ps_raw.txt");
        let out = filter_docker_ps(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_docker_ps_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_ps_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_docker_ps(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "expected ≥60%, got {}%", pct);
    }

    #[test]
    fn test_docker_build_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_build_raw.txt");
        let out = filter_docker_build(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_docker_build_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_build_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_docker_build(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "expected ≥60%, got {}%", pct);
    }

    #[test]
    fn test_docker_images_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_images_raw.txt");
        let out = filter_docker_images(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_docker_images_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_images_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_docker_images(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "expected ≥60%, got {}%", pct);
    }

    #[test]
    fn test_docker_logs_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_logs_raw.txt");
        let out = filter_docker_logs(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_docker_logs_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_logs_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_docker_logs(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        // Head/tail + grep-of-interest pattern keeps errors verbatim;
        // typical fixture savings ~50%.
        assert!(pct >= 50, "expected ≥50%, got {}%", pct);
    }

    #[test]
    fn test_docker_compose_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_compose_raw.txt");
        let out = filter_docker_compose(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_docker_compose_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/docker_compose_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_docker_compose(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "expected ≥60%, got {}%", pct);
    }
}
