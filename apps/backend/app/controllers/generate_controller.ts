import { randomUUID } from 'node:crypto'
import type { HttpContext } from '@adonisjs/core/http'
import logger from '@adonisjs/core/services/logger'
import env from '#start/env'
import worldBus, { type WorldPayload } from '#services/world_bus'

const FUNNY_PHASES = [
  'Soulèvement des montagnes',
  'Remplissage des océans',
  'Plantation des forêts',
  'Sculptage des côtes',
  'Saupoudrage de neige',
]

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

    const url = new URL('/generate', env.get('WORKER_URL'))
    if (body.seed !== undefined) url.searchParams.set('seed', String(body.seed))
    if (body.size !== undefined) url.searchParams.set('size', String(body.size))
    if (body.scale !== undefined) url.searchParams.set('scale', String(body.scale))

    queueMicrotask(async () => {
      const phaseTimer = setInterval(() => {
        const phase = FUNNY_PHASES[Math.floor(Math.random() * FUNNY_PHASES.length)]
        worldBus.publish({ type: 'world.progress', jobId, phase, at: Date.now() })
      }, 250)
      try {
        const r = await fetch(url, { method: 'POST' })
        if (!r.ok) {
          throw new Error(`worker ${r.status}: ${await r.text()}`)
        }
        const payload = (await r.json()) as WorldPayload
        worldBus.publish({ type: 'world.done', jobId, at: Date.now(), payload })
      } catch (err) {
        logger.error({ err, jobId }, 'worker call failed')
        worldBus.publish({
          type: 'world.error',
          jobId,
          at: Date.now(),
          error: err instanceof Error ? err.message : String(err),
        })
      } finally {
        clearInterval(phaseTimer)
      }
    })

    return response.accepted({ jobId, startedAt })
  }
}
