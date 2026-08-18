import type { Provider } from "@nestjs/common";
{%- if db_orm == "prisma" %}
import { PRISMA } from "./database.constants";
import { PrismaClient } from "@prisma/client";

export const databaseProvider: Provider = {
  provide: PRISMA,
  useFactory: () => new PrismaClient(),
};
{%- elif db_orm == "typeorm" %}
import { TYPEORM } from "./database.constants";
import { ConfigService } from "@nestjs/config";
import { DataSource } from "typeorm";

export const databaseProvider: Provider = {
  provide: TYPEORM,
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
{%- elif db_driver == "postgres-js" %}
import { drizzle } from "drizzle-orm/postgres-js";
import * as schema from "./schema";
import { DRIZZLE, DB_CLIENT } from "./database.constants";
import type { PostgresClient } from "./database-client.provider";

export const databaseProvider: Provider = {
  provide: DRIZZLE,
  inject: [DB_CLIENT],
  useFactory: (postgresClient: PostgresClient) => drizzle(postgresClient.sql, { schema }),
};
{%- else %}
import { drizzle } from "drizzle-orm/node-postgres";
import * as schema from "./schema";
import { DRIZZLE, DB_CLIENT } from "./database.constants";
import type { PostgresClient } from "./database-client.provider";

export const databaseProvider: Provider = {
  provide: DRIZZLE,
  inject: [DB_CLIENT],
  useFactory: (postgresClient: PostgresClient) => drizzle(postgresClient.pool, { schema }),
};
{%- endif %}
