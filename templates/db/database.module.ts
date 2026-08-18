import { Global, Module } from "@nestjs/common";
import { databaseProvider } from "./database.provider";
import { DATABASE_TOKEN } from "./database-type";
{%- if db_orm == "drizzle" %}
import { postgresClientProvider } from "./postgres-client.provider";
{%- endif %}

@Global()
@Module({
  providers: [
{%- if db_orm == "drizzle" %}
    postgresClientProvider,
{%- endif %}
    databaseProvider,
  ],
  exports: [DATABASE_TOKEN],
})
export class DatabaseModule {}
