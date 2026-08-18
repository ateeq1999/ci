import { z } from "zod";

const envSchema = z.object({
  NODE_ENV: z
    .enum(["development", "production", "test"])
    .default("development"),
  PORT: z.coerce.number().int().min(0).max(65535).default(3000),
  DATABASE_URL: z.string().url(),
});

export type EnvironmentVariables = z.infer<typeof envSchema>;

export function validate(config: Record<string, unknown>): EnvironmentVariables {
  const result = envSchema.safeParse(config);
  if (!result.success) {
    throw new Error(result.error.toString());
  }
  return result.data;
}
