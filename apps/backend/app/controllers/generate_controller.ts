import { randomUUID } from 'node:crypto'
import type { HttpContext } from '@adonisjs/core/http'
import logger from '@adonisjs/core/services/logger'
import worldBus from '#services/world_bus'
import { publishGenerate } from '#services/artemis_client'

export default class GenerateController {
  async store({ request, response }: HttpContext) {
    const body = request.only(['seed', 'size', 'scale']) as {
      seed?: number
      size?: number
      scale?: number
    }

    const jobId = randomUUID()
    const startedAt = Date.now()

    worldBus.publish({ type: 'world.started', jobId, at: startedAt })

    try {
      await publishGenerate({ jobId, ...body })
    } catch (err) {
      logger.error({ err, jobId }, 'publish failed')
      worldBus.publish({
        type: 'world.error',
        jobId,
        at: Date.now(),
        error: err instanceof Error ? err.message : String(err),
      })
      return response.internalServerError({ jobId, error: 'queue publish failed' })
    }

    return response.accepted({ jobId, startedAt })
  }
}
