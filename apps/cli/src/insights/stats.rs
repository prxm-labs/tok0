use anyhow::{Context, Result};
use serde::Serialize;

use crate::engine::config;
use crate::engine::meter::Tracker;

#[derive(Debug, Serialize)]
pub struct SavingsSummary {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_commands: u64,
    pub top_commands: Vec<(String, u64)>,
}

impl SavingsSummary {
    pub fn savings_pct(&self) -> f64 {
        if self.total_input_tokens == 0 {
            return 0.0;
        }
        let saved = self
            .total_input_tokens
            .saturating_sub(self.total_output_tokens);
        (saved as f64 / self.total_input_tokens as f64) * 100.0
    }
}

pub fn format_summary(s: &SavingsSummary, format: &str) -> String {
    match format {
        "json" => serde_json::to_string_pretty(s).unwrap_or_else(|_| "{}".to_string()),
        "csv" => {
            let mut out = String::from("command,saved_tokens\n");
            for (cmd, saved) in &s.top_commands {
                out.push_str(&format!("{},{}\n", cmd, saved));
            }
            out
        }
        _ => {
            // text format (default)
            let mut out = String::new();
            out.push_str(&format!("Total commands:  {}\n", s.total_commands));
            out.push_str(&format!("Input tokens:    {}\n", s.total_input_tokens));
            out.push_str(&format!("Output tokens:   {}\n", s.total_output_tokens));
            out.push_str(&format!(
                "Tokens saved:    {} ({:.1}%)\n",
                s.total_input_tokens.saturating_sub(s.total_output_tokens),
                s.savings_pct()
            ));
            if !s.top_commands.is_empty() {
                out.push_str("\nTop commands by savings:\n");
                for (cmd, saved) in &s.top_commands {
                    out.push_str(&format!("  {:<20} {:>8} tokens\n", cmd, saved));
                }
            }
            out
        }
    }
}

pub fn render_ascii_graph(daily: &[(String, u64)], width: usize) -> String {
    if daily.is_empty() {
        return String::from("No daily data available.\n");
    }

    let max_saved = daily.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1);
    let mut out = String::new();

    for (date, saved) in daily {
        let bar_len = (*saved as f64 / max_saved as f64 * width as f64).round() as usize;
        let bar: String = "\u{2588}".repeat(bar_len);
        out.push_str(&format!("{} {:>6} {}\n", date, saved, bar));
    }

    out
}

pub fn run(graph: bool, history: bool, format: &str) -> Result<()> {
    let db = config::db_path();
    if !db.exists() {
        eprintln!("tok0: no tracking data yet (run some commands first)");
        return Ok(());
    }

    let tracker = Tracker::new(&db).context("Failed to open tracking database")?;

    if history {
        let recent = tracker
            .get_recent(20)
            .context("Failed to query recent commands")?;
        if recent.is_empty() {
            println!("No command history yet.");
            return Ok(());
        }
        println!(
            "{:<20} {:<20} {:>8} {:>8} {:>7}",
            "Timestamp", "Command", "Input", "Output", "Saved%"
        );
        for r in &recent {
            println!(
                "{:<20} {:<20} {:>8} {:>8} {:>6.1}%",
                r.timestamp, r.command, r.input_tokens, r.output_tokens, r.savings_pct
            );
        }
        return Ok(());
    }

    let summary = tracker
        .get_summary()
        .context("Failed to query savings summary")?;

    let top = tracker
        .get_top_commands(10)
        .context("Failed to query top commands")?;

    let s = SavingsSummary {
        total_input_tokens: summary.total_input_tokens,
        total_output_tokens: summary.total_output_tokens,
        total_commands: summary.total_commands,
        top_commands: top,
    };

    let formatted = format_summary(&s, format);
    print!("{}", formatted);

    if graph {
        let daily = tracker
            .get_daily(14)
            .context("Failed to query daily stats")?;
        let daily_pairs: Vec<(String, u64)> = daily
            .iter()
            .map(|d| (d.date.clone(), d.saved_tokens))
            .collect();
        println!("\nDaily savings (last 14 days):");
        print!("{}", render_ascii_graph(&daily_pairs, 40));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary(
        input: u64,
        output: u64,
        commands: u64,
        top: Vec<(String, u64)>,
    ) -> SavingsSummary {
        SavingsSummary {
            total_input_tokens: input,
            total_output_tokens: output,
            total_commands: commands,
            top_commands: top,
        }
    }

    #[test]
    fn test_format_summary_text() {
        let s = make_summary(
            10000,
            2000,
            50,
            vec![
                ("git status".to_string(), 5000),
                ("cargo build".to_string(), 3000),
            ],
        );
        let text = format_summary(&s, "text");
        assert!(text.contains("10000"), "should contain input tokens");
        assert!(text.contains("2000"), "should contain output tokens");
        assert!(text.contains("80.0%"), "should contain savings pct");
        assert!(text.contains("git status"), "should contain command name");
        assert!(text.contains("cargo build"), "should contain command name");
        assert!(text.contains("50"), "should contain command count");
    }

    #[test]
    fn test_format_summary_json() {
        let s = make_summary(5000, 1000, 25, vec![("ls".to_string(), 3000)]);
        let json_str = format_summary(&s, "json");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("should parse as valid JSON");
        assert_eq!(parsed["total_input_tokens"], 5000);
        assert_eq!(parsed["total_output_tokens"], 1000);
        assert_eq!(parsed["total_commands"], 25);
        assert!(parsed["top_commands"].is_array());
    }

    #[test]
    fn test_format_summary_csv() {
        let s = make_summary(
            8000,
            2000,
            10,
            vec![
                ("git log".to_string(), 4000),
                ("npm test".to_string(), 2000),
            ],
        );
        let csv = format_summary(&s, "csv");
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "command,saved_tokens", "CSV header");
        assert_eq!(lines[1], "git log,4000");
        assert_eq!(lines[2], "npm test,2000");
    }

    #[test]
    fn test_ascii_graph_renders() {
        let daily = vec![
            ("2026-04-01".to_string(), 1000_u64),
            ("2026-04-02".to_string(), 2000),
            ("2026-04-03".to_string(), 500),
        ];
        let graph = render_ascii_graph(&daily, 20);
        assert!(graph.contains("2026-04-01"), "should contain date");
        assert!(graph.contains("2026-04-02"), "should contain date");
        assert!(graph.contains("\u{2588}"), "should contain bar chars");
    }

    #[test]
    fn test_savings_pct_zero_input() {
        let s = make_summary(0, 0, 0, vec![]);
        assert!((s.savings_pct() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_format_summary_empty_commands() {
        let s = make_summary(1000, 500, 5, vec![]);
        let text = format_summary(&s, "text");
        assert!(text.contains("1000"), "should contain input tokens");
        assert!(text.contains("500"), "should contain output tokens");
        assert!(text.contains("50.0%"), "should contain savings pct");
        assert!(
            !text.contains("Top commands"),
            "should not show top commands section when empty"
        );
    }
}
