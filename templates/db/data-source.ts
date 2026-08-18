import "dotenv/config";
import { DataSource } from "typeorm";

/** Standalone `DataSource` the TypeORM CLI points at (`migration:run`,
 *  `migration:generate`, `schema:drop`, ...) — kept separate from the
 *  Nest-managed instance in `database.provider.ts` because the CLI runs
 *  outside the Nest app context and needs its own entrypoint. Both share
 *  this config so they never drift apart. */
export const dataSource = new DataSource({
  type: "postgres",
  url: process.env.DATABASE_URL,
  entities: [],
  migrations: ["src/database/migrations/*.ts"],
  synchronize: false,
});

export default dataSource;
