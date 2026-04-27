import type { HttpContext } from '@adonisjs/core/http'
import worldBus, { type WorldEvent } from '#services/world_bus'

export default class EventsController {
  async stream({ response, request }: HttpContext) {
    const stream = response.response
    stream.setHeader('Content-Type', 'text/event-stream')
    stream.setHeader('Cache-Control', 'no-cache, no-transform')
    stream.setHeader('Connection', 'keep-alive')
    stream.setHeader('X-Accel-Buffering', 'no')
    stream.statusCode = 200
    stream.flushHeaders()
    stream.write(`retry: 3000\n\n`)

    const send = (event: WorldEvent) => {
      stream.write(`event: ${event.type}\n`)
      stream.write(`data: ${JSON.stringify(event)}\n\n`)
    }

    const unsubscribe = worldBus.subscribe(send)

    const heartbeat = setInterval(() => {
      stream.write(`: ping ${Date.now()}\n\n`)
    }, 15_000)

    const close = () => {
      clearInterval(heartbeat)
      unsubscribe()
    }

    request.request.on('close', close)
    request.request.on('aborted', close)

    return new Promise<void>(() => {})
  }
}
