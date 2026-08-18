import type { Provider } from "@nestjs/common";
import { DATABASE_TOKEN } from "./database-type";
{%- if db_orm == "prisma" %}
import { PrismaClient } from "@prisma/client";

export const databaseProvider: Provider = {
  provide: DATABASE_TOKEN,
  useFactory: () => new PrismaClient(),
};
{%- elif db_orm == "typeorm" %}
import { ConfigService } from "@nestjs/config";
import { DataSource } from "typeorm";

export const databaseProvider: Provider = {
  provide: DATABASE_TOKEN,
  inject: [ConfigService],
  useFactory: (config: ConfigService) => {
    const dataSource = new DataSource({
      type: "postgres",
      url: config.getOrThrow<string>("DATABASE_URL"),
      entities: [],
      synchronize: false,
    });
    return dataSource.initialize();
  },
};
{%- else %}
import { ConfigService } from "@nestjs/config";
import { Pool } from "pg";
import { drizzle } from "drizzle-orm/node-postgres";
import * as schema from "./schema";

export const databaseProvider: Provider = {
  provide: DATABASE_TOKEN,
  inject: [ConfigService],
  useFactory: (config: ConfigService) => {
    const connectionString = config.getOrThrow<string>("DATABASE_URL");
    const pool = new Pool({ connectionString });
    return drizzle(pool, { schema });
  },
};
{%- endif %}
