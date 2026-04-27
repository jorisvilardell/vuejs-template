import { EventEmitter } from 'node:events'

export type WorldEvent =
  | { type: 'world.started'; jobId: string; at: number }
  | { type: 'world.progress'; jobId: string; phase: string; at: number }
  | { type: 'world.done'; jobId: string; at: number; payload: WorldPayload }
  | { type: 'world.error'; jobId: string; at: number; error: string }

export interface WorldPayload {
  id: string
  seed: number
  size: number
  scale: number
  png_url: string
  png_key: string
  json_url: string
  json_key: string
  duration_ms: number
}

class WorldBus extends EventEmitter {
  publish(event: WorldEvent) {
    this.emit('event', event)
  }

  subscribe(listener: (event: WorldEvent) => void): () => void {
    this.on('event', listener)
    return () => this.off('event', listener)
  }
}

const bus = new WorldBus()
bus.setMaxListeners(0)
export default bus
