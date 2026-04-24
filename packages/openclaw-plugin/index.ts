/**
 * tok0 Rewrite Plugin for OpenClaw.
 *
 * Transparently rewrites `exec` tool calls to their tok0 equivalents
 * before execution, achieving 60-90% LLM token savings.
 *
 * All rewrite logic lives in `tok0 rewrite` (src/bridge/rewriter.rs in
 * the tok0 source tree). This plugin is a thin delegate — to add or
 * change rules, edit the Rust dispatcher, not this file.
 *
 * Configuration (openclaw.plugin.json -> configSchema):
 *   enabled    — turn the whole plugin off without uninstalling it
 *   verbose    — log every command/rewrite pair to stderr
 *   timeout_ms — give up on a rewrite after N ms; fall back to the
 *                untransformed command rather than blocking the agent
 */

import { execFileSync } from "node:child_process";

let tok0Available: boolean | null = null;

function checkTok0(): boolean {
  if (tok0Available !== null) return tok0Available;
  try {
    execFileSync("tok0", ["--version"], { stdio: "ignore", timeout: 1000 });
    tok0Available = true;
  } catch {
    tok0Available = false;
  }
  return tok0Available;
}

function tryRewrite(command: string, timeoutMs: number): string | null {
  // Argv-style invocation. We rely on tok0's own argv splitting via the
  // existing `Commands::Rewrite` handler, which takes trailing args.
  // Pass the user's command as a single string arg so heredocs and
  // quoting survive intact — tok0 handles them with its own rules.
  try {
    const result = execFileSync("tok0", ["rewrite", command], {
      encoding: "utf-8",
      timeout: timeoutMs,
    }).trim();
    return result && result !== command ? result : null;
  } catch {
    return null;
  }
}

export default function register(api: any) {
  const pluginConfig = api.config ?? {};
  const enabled = pluginConfig.enabled !== false;
  const verbose = pluginConfig.verbose === true;
  const timeoutMs =
    typeof pluginConfig.timeout_ms === "number" ? pluginConfig.timeout_ms : 2000;

  if (!enabled) return;

  if (!checkTok0()) {
    console.warn(
      "[tok0] tok0 binary not found in PATH — plugin disabled. " +
        "Install with `curl -fsSL https://raw.githubusercontent.com/prxm-labs/tok0/main/install.sh | sh`."
    );
    return;
  }

  api.on(
    "before_tool_call",
    (event: { toolName: string; params: Record<string, unknown> }) => {
      if (event.toolName !== "exec") return;

      const command = event.params?.command;
      if (typeof command !== "string") return;

      const rewritten = tryRewrite(command, timeoutMs);
      if (!rewritten) return;

      if (verbose) {
        console.log(`[tok0] ${command} -> ${rewritten}`);
      }

      return { params: { ...event.params, command: rewritten } };
    },
    { priority: 10 }
  );

  if (verbose) {
    console.log("[tok0] OpenClaw plugin registered");
  }
}
