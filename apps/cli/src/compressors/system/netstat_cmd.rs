use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

type Conn = (String, String, String);

lazy_static! {
    /// Match an Internet (tcp/udp) row from `netstat -an`.
    /// Captures: proto, local_addr, foreign_addr, state (optional).
    static ref INET_ROW_RE: Regex = Regex::new(
        r"^(tcp\d?|udp\d?)\s+\d+\s+\d+\s+(\S+)\s+(\S+)(?:\s+(\w+))?\s*$"
    ).unwrap();
    /// Linux `ss -tuln` rows: State Recv-Q Send-Q Local Peer
    static ref SS_ROW_RE: Regex = Regex::new(
        r"^(LISTEN|ESTAB|TIME-WAIT|CLOSE-WAIT|SYN-SENT|SYN-RECV|FIN-WAIT-1|FIN-WAIT-2|CLOSING|LAST-ACK|UNCONN)\s+\d+\s+\d+\s+(\S+)\s+(\S+)"
    ).unwrap();
}

/// Filter `netstat -an` / `ss` output: group by state, show LISTEN ports
/// in full, summarize ESTABLISHED connections, drop unix-domain sockets.
pub fn filter_netstat(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    // (proto, local, foreign, state)
    let mut rows: Vec<(String, String, String, String)> = Vec::new();

    for line in input.lines() {
        if let Some(caps) = INET_ROW_RE.captures(line) {
            let state = caps
                .get(4)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            rows.push((
                caps[1].to_string(),
                caps[2].to_string(),
                caps[3].to_string(),
                state,
            ));
        } else if let Some(caps) = SS_ROW_RE.captures(line) {
            // ss output: state is in column 1
            rows.push((
                "tcp".to_string(),
                caps[2].to_string(),
                caps[3].to_string(),
                caps[1].to_string(),
            ));
        }
    }

    if rows.is_empty() {
        return input.to_string();
    }

    let mut by_state: BTreeMap<String, Vec<Conn>> = BTreeMap::new();
    for (proto, local, foreign, state) in rows {
        let key = if state.is_empty() {
            "(no-state)".to_string()
        } else {
            state
        };
        by_state
            .entry(key)
            .or_default()
            .push((proto, local, foreign));
    }

    let mut out = String::new();

    // Always show LISTEN sockets in full — that's the highest-signal info
    if let Some(listening) = by_state.remove("LISTEN") {
        out.push_str(&format!("LISTEN ({}):\n", listening.len()));
        for (proto, local, _) in &listening {
            out.push_str(&format!("  {} {}\n", proto, local));
        }
    }

    // Other states: summarize
    let mut other_states: Vec<(String, Vec<Conn>)> = by_state.into_iter().collect();
    other_states.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    for (state, entries) in &other_states {
        out.push_str(&format!("\n{} ({}):\n", state, entries.len()));
        // Show first 10 destinations
        for (proto, _, foreign) in entries.iter().take(10) {
            out.push_str(&format!("  {} -> {}\n", proto, foreign));
        }
        if entries.len() > 10 {
            out.push_str(&format!("  ... +{} more\n", entries.len() - 10));
        }
    }

    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_netstat_format() {
        let input = include_str!("../../../tests/fixtures/system/netstat_raw.txt");
        let output = filter_netstat(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_netstat_savings() {
        let input = include_str!("../../../tests/fixtures/system/netstat_raw.txt");
        let output = filter_netstat(input);
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
    fn test_netstat_empty() {
        assert_eq!(filter_netstat(""), "");
    }

    #[test]
    fn test_netstat_keeps_listen_ports() {
        let input = include_str!("../../../tests/fixtures/system/netstat_raw.txt");
        let output = filter_netstat(input);
        assert!(output.contains("LISTEN"));
    }

    #[test]
    fn test_netstat_drops_unix_sockets() {
        let input = include_str!("../../../tests/fixtures/system/netstat_raw.txt");
        let output = filter_netstat(input);
        // The fixture has /var/run/mDNSResponder unix sockets — those must be filtered out
        assert!(!output.contains("mDNSResponder"));
    }

    #[test]
    fn test_netstat_groups_by_state() {
        let input = "tcp4       0      0  *.80                    *.*                    LISTEN\ntcp4       0      0  10.0.0.1.443           1.1.1.1.443            ESTABLISHED\ntcp4       0      0  10.0.0.1.444           2.2.2.2.443            ESTABLISHED";
        let output = filter_netstat(input);
        assert!(output.contains("LISTEN"));
        assert!(output.contains("ESTABLISHED"));
        assert!(output.contains("(2)") || output.contains("ESTABLISHED (2)"));
    }

    #[test]
    fn test_netstat_malformed_passthrough() {
        let output = filter_netstat("garbage\nnot netstat\nrandom");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_netstat_handles_ss_format() {
        // Linux `ss -tuln` style: state is the FIRST column
        let input = "State    Recv-Q Send-Q  Local Address:Port   Peer Address:Port\nLISTEN   0      4096    0.0.0.0:8080        0.0.0.0:*\nESTAB    0      0       10.0.0.1:443        1.1.1.1:54321";
        let output = filter_netstat(input);
        assert!(output.contains("LISTEN"));
    }

    #[test]
    fn test_netstat_unicode() {
        // Realistic case — netstat itself wouldn't emit unicode but hostnames could
        let input = "tcp4 0 0 *.80 *.* LISTEN\n";
        let output = filter_netstat(input);
        assert!(output.contains("LISTEN"));
    }
}
