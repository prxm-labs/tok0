import { sqliteTable, text, integer, real, index, primaryKey } from 'drizzle-orm/sqlite-core';

export const instances = sqliteTable(
  'instances',
  {
    installationId: text('installation_id').primaryKey(),
    firstSeen: integer('first_seen').notNull(),
    lastSeen: integer('last_seen').notNull(),
    country: text('country'),
    os: text('os').notNull(),
    version: text('version').notNull(),
    totalCommands: integer('total_commands').notNull().default(0),
    totalSavedTokens: integer('total_saved_tokens').notNull().default(0),
    avgSavingsPct: real('avg_savings_pct').notNull().default(0),
  },
  (t) => ({
    lastSeenIdx: index('instances_last_seen_idx').on(t.lastSeen),
    countryIdx: index('instances_country_idx').on(t.country),
  })
);

export const commandStats = sqliteTable(
  'command_stats',
  {
    date: text('date').notNull(),
    command: text('command').notNull(),
    totalCount: integer('total_count').notNull().default(0),
    totalSaved: integer('total_saved').notNull().default(0),
  },
  (t) => ({
    pk: primaryKey({ columns: [t.date, t.command] }),
    dateIdx: index('command_stats_date_idx').on(t.date),
  })
);

export const instanceDaily = sqliteTable(
  'instance_daily',
  {
    installationId: text('installation_id').notNull(),
    date: text('date').notNull(),
    commands: integer('commands').notNull().default(0),
    inputTokens: integer('input_tokens').notNull().default(0),
    outputTokens: integer('output_tokens').notNull().default(0),
    savedTokens: integer('saved_tokens').notNull().default(0),
  },
  (t) => ({
    pk: primaryKey({ columns: [t.installationId, t.date] }),
    dateIdx: index('instance_daily_date_idx').on(t.date),
  })
);
