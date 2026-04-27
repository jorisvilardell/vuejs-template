/*
|--------------------------------------------------------------------------
| Environment variables service
|--------------------------------------------------------------------------
|
| The `Env.create` method creates an instance of the Env service. The
| service validates the environment variables and also cast values
| to JavaScript data types.
|
*/

import { Env } from '@adonisjs/core/env'

export default await Env.create(new URL('../', import.meta.url), {
  NODE_ENV: Env.schema.enum(['development', 'production', 'test'] as const),
  PORT: Env.schema.number(),
  APP_KEY: Env.schema.string(),
  HOST: Env.schema.string({ format: 'host' }),
  LOG_LEVEL: Env.schema.string(),

  ARTEMIS_HOST: Env.schema.string(),
  ARTEMIS_PORT: Env.schema.number.optional(),
  ARTEMIS_USER: Env.schema.string(),
  ARTEMIS_PASSWORD: Env.schema.string(),
  QUEUE_IN: Env.schema.string.optional(),
  QUEUE_OUT: Env.schema.string.optional(),
})
