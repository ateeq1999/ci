import { Global, Module } from "@nestjs/common";
import { databaseProvider } from "./database.provider";
import { DATABASE_TOKEN } from "./database-type";

@Global()
@Module({
  providers: [databaseProvider],
  exports: [DATABASE_TOKEN],
})
export class DatabaseModule {}
