import { NestFactory } from '@nestjs/core';
import { FastifyAdapter, NestFastifyApplication } from '@nestjs/platform-fastify';
import fastifyCookie from '@fastify/cookie';
import { AppModule } from './app.module';
import { AppLogger } from './logger/logger.service';

async function bootstrap() {
  const app = await NestFactory.create<NestFastifyApplication>(AppModule, new FastifyAdapter());
  app.useLogger(app.get(AppLogger));
  await app.register(fastifyCookie);
  await app.listen(process.env.PORT ?? 3000);
}
bootstrap();
