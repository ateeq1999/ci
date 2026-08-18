{%- if db_orm == "prisma" -%}
export const PRISMA = Symbol("PRISMA");
{%- elif db_orm == "typeorm" -%}
export const TYPEORM = Symbol("TYPEORM");
{%- else -%}
export const DRIZZLE = Symbol("DRIZZLE");
export const DB_CLIENT = Symbol("DB_CLIENT");
{%- endif %}
