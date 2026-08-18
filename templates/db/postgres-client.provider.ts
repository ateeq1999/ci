import { Logger, type OnApplicationShutdown, type Provider } from "@nestjs/common";
import { ConfigService } from "@nestjs/config";
import { POSTGRES_CLIENT } from "./database.constants";
{%- if db_driver == "postgres-js" %}
import postgres, { type Sql } from "postgres";

/** Wraps the postgres.js client so Nest can close it on shutdown — a plain
 *  `Sql` instance from the `postgres` package doesn't implement the
 *  lifecycle hook itself. */
export class PostgresClient implements OnApplicationShutdown {
  private readonly logger = new Logger(PostgresClient.name);

  constructor(readonly sql: Sql) {}

  async onApplicationShutdown(signal?: string) {
    this.logger.log(`Closing database connection (signal: ${signal})`);
    await this.sql.end();
  }
}

export const postgresClientProvider: Provider = {
  provide: POSTGRES_CLIENT,
  inject: [ConfigService],
  useFactory: (config: ConfigService) => {
    const connectionString = config.getOrThrow<string>("DATABASE_URL");
    return new PostgresClient(postgres(connectionString, { prepare: false }));
  },
};
{%- else %}
import { Pool } from "pg";

/** Wraps the pg `Pool` so Nest can close it on shutdown — a plain `Pool`
 *  instance doesn't implement the lifecycle hook itself. */
export class PostgresClient implements OnApplicationShutdown {
  private readonly logger = new Logger(PostgresClient.name);

  constructor(readonly pool: Pool) {}

  async onApplicationShutdown(signal?: string) {
    this.logger.log(`Closing database connection (signal: ${signal})`);
    await this.pool.end();
  }
}

export const postgresClientProvider: Provider = {
  provide: POSTGRES_CLIENT,
  inject: [ConfigService],
  useFactory: (config: ConfigService) => {
    const connectionString = config.getOrThrow<string>("DATABASE_URL");
    return new PostgresClient(new Pool({ connectionString }));
  },
};
{%- endif %}
