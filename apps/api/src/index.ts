import { Hono } from 'hono';
import { healthRoute } from '@/routes/health';
import { telemetryRoute } from '@/routes/telemetry';
import { statsRoute } from '@/routes/stats';
import type { Bindings } from '@/types';

export const app = new Hono<{ Bindings: Bindings }>();

app.route('/healthz', healthRoute);
app.route('/telemetry', telemetryRoute);
app.route('/stats', statsRoute);

app.onError((err, c) => {
  // Anonymization rule: log only error message + class. NEVER request body
  // or headers (which could contain Authorization tokens or geo data).
  console.error(`tok0-api error: ${err.name}: ${err.message}`);
  return c.json({ error: 'internal' }, 500);
});

app.notFound((c) => c.json({ error: 'not found' }, 404));

export default app;
