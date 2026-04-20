import { useEffect, useState } from "react";
import {
  fetchStats,
  formatCount,
  formatTokens,
  FALLBACK_STATS,
  type StatsSnapshot,
} from "~/lib/stats";

type Tile = {
  label: string;
  value: string;
  suffix?: string;
  foot: string;
  accent?: boolean;
};

function tilesFor(s: StatsSnapshot): Tile[] {
  const totalCommands = s.daily.reduce((a, d) => a + (d.commands ?? 0), 0);
  return [
    {
      label: "Tokens compressed",
      value: formatTokens(s.total_saved_tokens),
      foot: "Cumulative · all tracked instances",
      accent: true,
    },
    {
      label: "Avg. savings",
      value: `${s.avg_savings_pct.toFixed(1)}`,
      suffix: "%",
      foot: "Across every wrapped command",
    },
    {
      label: "Active instances",
      value: formatCount(s.active_instances),
      foot: `Reporting in last ${s.inactive_threshold_days} days`,
    },
    {
      label: "Commands run · 30 d",
      value: formatCount(totalCommands),
      foot: `${s.daily.length} day rolling window`,
    },
  ];
}

type Props = {
  initial: StatsSnapshot;
  initialLive: boolean;
};

export default function StatsTiles({ initial, initialLive }: Props) {
  const [stats, setStats] = useState<StatsSnapshot>(initial ?? FALLBACK_STATS);
  const [live, setLive] = useState(initialLive);
  const [refreshedAt, setRefreshedAt] = useState<number>(initial?.generated_at ?? 0);

  useEffect(() => {
    const ctrl = new AbortController();
    fetchStats(ctrl.signal)
      .then((s) => { setStats(s); setLive(true); setRefreshedAt(s.generated_at); })
      .catch(() => { /* keep build-time snapshot */ });
    return () => ctrl.abort();
  }, []);

  const tiles = tilesFor(stats);
  const ts = refreshedAt
    ? new Date(refreshedAt).toISOString().replace("T", " ").slice(0, 19) + " UTC"
    : "—";

  return (
    <>
      <div className="stat-grid">
        {tiles.map((t, i) => (
          <div key={t.label} className={`stat-cell${t.accent ? " accent" : ""}`}>
            <div className="stat-label">
              {String(i + 1).padStart(2, "0")} · {t.label}
            </div>
            <div className="stat-value">
              {t.value}
              {t.suffix && <span className="stat-suffix">{t.suffix}</span>}
            </div>
            <div className="stat-foot">{t.foot}</div>
          </div>
        ))}
      </div>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: 16,
          marginTop: 14,
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          letterSpacing: "0.22em",
          textTransform: "uppercase",
          color: "var(--color-ink-4)",
        }}
      >
        <span>
          <span style={{ color: live ? "var(--color-accent)" : "var(--color-ink-4)" }}>●</span>{" "}
          {live ? "Live · api.tok0.dev/stats" : "Snapshot · last build"}
        </span>
        <span>GENERATED · {ts}</span>
      </div>
    </>
  );
}
