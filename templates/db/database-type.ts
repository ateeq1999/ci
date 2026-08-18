import { InjectionToken } from "@nestjs/common";
{%- if db_orm == "prisma" %}
import type { PrismaClient } from "@prisma/client";

export type Database = PrismaClient;
{%- elif db_orm == "typeorm" %}
import type { DataSource } from "typeorm";

export type Database = DataSource;
{%- else %}
import type { PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { Sql } from "postgres";
import type * as schema from "./schema";

export type Database = PostgresJsDatabase<typeof schema>;

/** Injection token for the underlying postgres.js client, kept separate from
 *  `DATABASE_TOKEN` so its lifecycle (closing the connection on shutdown)
 *  can be managed independently of the Drizzle wrapper. */
export const POSTGRES_CLIENT_TOKEN = new InjectionToken<Sql>("POSTGRES_CLIENT");
{%- endif %}

/** Injection token other modules use to access the database client. */
export const DATABASE_TOKEN = new InjectionToken<Database>("DATABASE_CLIENT");
