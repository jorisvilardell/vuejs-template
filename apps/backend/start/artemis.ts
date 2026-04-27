import logger from '@adonisjs/core/services/logger'
import app from '@adonisjs/core/services/app'
import env from '#start/env'
import { start, shutdown } from '#services/artemis_client'

const mode = (env.get('MODE') || 'local').toLowerCase()

if (mode === 'queue') {
  start().catch((err) => logger.error({ err }, 'artemis start error'))
  app.terminating(async () => {
    await shutdown()
  })
} else {
  logger.info({ mode }, 'artemis client disabled (MODE != queue)')
}
