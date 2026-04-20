import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro:schema";

const docs = defineCollection({
  loader: glob({ pattern: "**/*.{md,mdx}", base: "./src/content/docs" }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    section: z.string(),
    order: z.number(),
    updated: z.string().optional(),
    hideLede: z.boolean().optional(),
  }),
});

export const collections = { docs };
