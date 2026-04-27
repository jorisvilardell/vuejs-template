import logger from '@adonisjs/core/services/logger'
import app from '@adonisjs/core/services/app'
import { start, shutdown } from '#services/artemis_client'

start().catch((err) => logger.error({ err }, 'artemis start error'))
app.terminating(async () => {
  await shutdown()
})
