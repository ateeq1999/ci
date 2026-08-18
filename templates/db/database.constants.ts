{%- if db_orm == "prisma" -%}
export const PRISMA = Symbol("PRISMA");
{%- elif db_orm == "typeorm" -%}
export const TYPEORM = Symbol("TYPEORM");
{%- else -%}
export const DRIZZLE = Symbol("DRIZZLE");
export const POSTGRES_CLIENT = Symbol("POSTGRES_CLIENT");
{%- endif %}
