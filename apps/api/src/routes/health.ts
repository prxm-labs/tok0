import { Hono } from 'hono';
import type { Bindings } from '@/types';

export const healthRoute = new Hono<{ Bindings: Bindings }>();

healthRoute.get('/', (c) => c.json({ ok: true }));
