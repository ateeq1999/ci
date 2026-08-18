import { InjectionToken } from "@nestjs/common";
{%- if db_orm == "prisma" %}
import type { PrismaClient } from "@prisma/client";

export type Database = PrismaClient;
{%- elif db_orm == "typeorm" %}
import type { DataSource } from "typeorm";

export type Database = DataSource;
{%- else %}
import type { NodePgDatabase } from "drizzle-orm/node-postgres";
import type * as schema from "./schema";

export type Database = NodePgDatabase<typeof schema>;
{%- endif %}

/** Injection token other modules use to access the database client. */
export const DATABASE_TOKEN = new InjectionToken<Database>("DATABASE_CLIENT");
