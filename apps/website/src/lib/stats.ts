export const STATS_ENDPOINT =
  import.meta.env.PUBLIC_TOK0_API_URL ?? "https://api.tok0.dev";

export type DailyPoint = {
  date: string;
  active_instances: number;
  commands: number;
  input_tokens: number;
  output_tokens: number;
  saved_tokens: number;
  avg_savings_pct: number;
};

export type TopCommand = {
  command: string;
  count: number;
  saved_tokens: number;
};

export type StatsSnapshot = {
  active_instances: number;
  total_instances: number;
  inactive_threshold_days: number;
  total_saved_tokens: number;
  avg_savings_pct: number;
  generated_at: number;
  by_country: Record<string, number>;
  by_os: Record<string, number>;
  by_version: Record<string, number>;
  top_commands: TopCommand[];
  daily: DailyPoint[];
};

export const FALLBACK_STATS: StatsSnapshot = {
  active_instances: 0,
  total_instances: 0,
  inactive_threshold_days: 10,
  total_saved_tokens: 0,
  avg_savings_pct: 0,
  generated_at: 0,
  by_country: {},
  by_os: {},
  by_version: {},
  top_commands: [],
  daily: [],
};

export async function fetchStats(signal?: AbortSignal): Promise<StatsSnapshot> {
  const res = await fetch(`${STATS_ENDPOINT}/stats`, {
    headers: { accept: "application/json" },
    signal,
  });
  if (!res.ok) throw new Error(`stats request failed: ${res.status}`);
  const data = (await res.json()) as Partial<StatsSnapshot>;
  return {
    active_instances: data.active_instances ?? 0,
    total_instances: data.total_instances ?? 0,
    inactive_threshold_days: data.inactive_threshold_days ?? 10,
    total_saved_tokens: data.total_saved_tokens ?? 0,
    avg_savings_pct: data.avg_savings_pct ?? 0,
    generated_at: data.generated_at ?? Date.now(),
    by_country: data.by_country ?? {},
    by_os: data.by_os ?? {},
    by_version: data.by_version ?? {},
    top_commands: data.top_commands ?? [],
    daily: data.daily ?? [],
  };
}

/** Build-time fetch with graceful fallback so SSG never breaks. */
export async function fetchStatsSafe(): Promise<{ stats: StatsSnapshot; live: boolean }> {
  try {
    const ctrl = new AbortController();
    const t = setTimeout(() => ctrl.abort(), 5000);
    const stats = await fetchStats(ctrl.signal);
    clearTimeout(t);
    return { stats, live: true };
  } catch {
    return { stats: FALLBACK_STATS, live: false };
  }
}

export function formatCount(n: number): string {
  return new Intl.NumberFormat("en-US").format(Math.max(0, Math.round(n)));
}

export function formatTokens(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(2)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return formatCount(n);
}
