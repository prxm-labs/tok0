use anyhow::{Context, Result};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct StageTime {
    pub name: &'static str,
    pub duration: Duration,
    pub output_len: usize,
}

#[derive(Debug)]
pub struct ProfileResult {
    pub command: String,
    pub exec_time: Duration,
    pub compression_time: Duration,
    pub stages: Vec<StageTime>,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl ProfileResult {
    pub fn savings_pct(&self) -> f64 {
        if self.input_tokens == 0 {
            return 0.0;
        }
        (1.0 - self.output_tokens as f64 / self.input_tokens as f64) * 100.0
    }
}

/// Run a command and profile the compression pipeline stages.
pub fn profile_command(command: &str, args: &[&str]) -> Result<ProfileResult> {
    // Stage 0: Execute the command
    let t_exec = Instant::now();
    let output = crate::engine::shell::execute_command(command, args)
        .with_context(|| format!("Failed to execute: {} {:?}", command, args))?;
    let exec_time = t_exec.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let input_tokens = crate::engine::shell::count_tokens(&stdout);

    let mut stages = Vec::new();
    let t_compress = Instant::now();

    // Stage 1: strip_ansi
    let t1 = Instant::now();
    let s1 = crate::engine::shell::strip_ansi(&stdout);
    stages.push(StageTime {
        name: "strip_ansi",
        duration: t1.elapsed(),
        output_len: s1.len(),
    });

    // Stage 2: head_tail (simulated with typical config)
    let t2 = Instant::now();
    let s2 = crate::engine::shell::head_tail(&s1, 50, 20);
    stages.push(StageTime {
        name: "head_tail",
        duration: t2.elapsed(),
        output_len: s2.len(),
    });

    // Stage 3: hard_cap
    let t3 = Instant::now();
    let s3 = crate::engine::shell::hard_cap(&s2, 50_000);
    stages.push(StageTime {
        name: "hard_cap",
        duration: t3.elapsed(),
        output_len: s3.len(),
    });

    let compression_time = t_compress.elapsed();
    let output_tokens = crate::engine::shell::count_tokens(&s3);

    Ok(ProfileResult {
        command: format!("{} {}", command, args.join(" ")),
        exec_time,
        compression_time,
        stages,
        input_tokens,
        output_tokens,
    })
}

pub fn run(command: &str, args: &[&str]) -> Result<()> {
    let result = profile_command(command, args)?;
    println!("Command     : {}", result.command);
    println!("Exec time   : {:?}", result.exec_time);
    println!("Compress    : {:?}", result.compression_time);
    println!(
        "Tokens      : {} -> {} ({:.1}% savings)",
        result.input_tokens,
        result.output_tokens,
        result.savings_pct()
    );
    println!();
    println!("Stage breakdown:");
    for stage in &result.stages {
        println!(
            "  {:<20} {:>10?}  {:>8} chars",
            stage.name, stage.duration, stage.output_len
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_echo() {
        let result = profile_command("echo", &["hello", "world"]).expect("profile should succeed");
        assert!(result.exec_time.as_secs() < 5);
        assert!(result.input_tokens > 0);
        assert!(!result.stages.is_empty());
        assert_eq!(result.stages[0].name, "strip_ansi");
    }

    #[test]
    fn test_profile_result_savings() {
        let r = ProfileResult {
            command: "test".to_string(),
            exec_time: Duration::from_millis(1),
            compression_time: Duration::from_millis(0),
            stages: vec![],
            input_tokens: 100,
            output_tokens: 25,
        };
        assert!((r.savings_pct() - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_profile_result_zero_input() {
        let r = ProfileResult {
            command: "test".to_string(),
            exec_time: Duration::from_millis(0),
            compression_time: Duration::from_millis(0),
            stages: vec![],
            input_tokens: 0,
            output_tokens: 0,
        };
        assert!((r.savings_pct() - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_profile_stages_present() {
        let result = profile_command("echo", &["test"]).expect("profile should succeed");
        let stage_names: Vec<&str> = result.stages.iter().map(|s| s.name).collect();
        assert!(stage_names.contains(&"strip_ansi"));
        assert!(stage_names.contains(&"head_tail"));
        assert!(stage_names.contains(&"hard_cap"));
    }
}
