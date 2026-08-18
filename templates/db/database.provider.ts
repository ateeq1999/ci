import type { Provider } from "@nestjs/common";
{%- if db_orm == "prisma" %}
import { DATABASE_TOKEN } from "./database-type";
import { PrismaClient } from "@prisma/client";

export const databaseProvider: Provider = {
  provide: DATABASE_TOKEN,
  useFactory: () => new PrismaClient(),
};
{%- elif db_orm == "typeorm" %}
import { DATABASE_TOKEN } from "./database-type";
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
import { drizzle } from "drizzle-orm/postgres-js";
import * as schema from "./schema";
import { DATABASE_TOKEN, POSTGRES_CLIENT_TOKEN } from "./database-type";
import type { PostgresClient } from "./postgres-client.provider";

export const databaseProvider: Provider = {
  provide: DATABASE_TOKEN,
  inject: [POSTGRES_CLIENT_TOKEN],
  useFactory: (postgresClient: PostgresClient) => drizzle(postgresClient.sql, { schema }),
};
{%- endif %}
