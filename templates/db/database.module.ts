import { Global, Module } from "@nestjs/common";
import { databaseProvider } from "./database.provider";
{%- if db_orm == "prisma" %}
import { PRISMA } from "./database.constants";
{%- elif db_orm == "typeorm" %}
import { TYPEORM } from "./database.constants";
{%- else %}
import { DRIZZLE } from "./database.constants";
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
  exports: [{% if db_orm == "prisma" %}PRISMA{% elif db_orm == "typeorm" %}TYPEORM{% else %}DRIZZLE{% endif %}],
})
export class DatabaseModule {}
