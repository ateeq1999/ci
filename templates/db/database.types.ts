{%- if db_orm == "prisma" -%}
import type { PrismaClient } from "@prisma/client";

export type Database = PrismaClient;
{%- elif db_orm == "typeorm" -%}
import type { DataSource } from "typeorm";

export type Database = DataSource;
{%- elif db_driver == "postgres-js" -%}
import type { PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type * as schema from "./schema";

export type Database = PostgresJsDatabase<typeof schema>;
{%- else -%}
import type { NodePgDatabase } from "drizzle-orm/node-postgres";
import type * as schema from "./schema";

export type Database = NodePgDatabase<typeof schema>;
{%- endif %}
