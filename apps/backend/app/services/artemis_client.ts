import stompit from 'stompit'
import logger from '@adonisjs/core/services/logger'
import worldBus, { type WorldEvent, type WorldPayload } from '#services/world_bus'

interface Config {
  host: string
  port: number
  user: string
  pass: string
  queueIn: string
  queueOut: string
}

let producerClient: stompit.Client | null = null
let consumerClient: stompit.Client | null = null
let started = false
let configCached: Config | null = null

function readConfig(): Config {
  if (configCached) return configCached
  const cfg: Config = {
    host: process.env.ARTEMIS_HOST || '127.0.0.1',
    port: Number(process.env.ARTEMIS_PORT ?? 61613),
    user: process.env.ARTEMIS_USER || 'artemis',
    pass: process.env.ARTEMIS_PASSWORD || 'artemis',
    queueIn: process.env.QUEUE_IN || 'world-gen',
    queueOut: process.env.QUEUE_OUT || 'world-done',
  }
  configCached = cfg
  return cfg
}

function connectOpts(cfg: Config): any {
  return {
    host: cfg.host,
    port: cfg.port,
    connectHeaders: {
      host: cfg.host,
      login: cfg.user,
      passcode: cfg.pass,
      'accept-version': '1.2',
      'heart-beat': '10000,10000',
    },
  }
}

async function connect(cfg: Config): Promise<stompit.Client> {
  return new Promise((resolve, reject) => {
    stompit.connect(connectOpts(cfg), (err, client) => {
      if (err) return reject(err)
      client.on('error', (e) => logger.error({ err: e }, 'stomp client error'))
      resolve(client)
    })
  })
}

async function ensureProducer(cfg: Config): Promise<stompit.Client> {
  if (producerClient) return producerClient
  producerClient = await connect(cfg)
  logger.info({ host: cfg.host, port: cfg.port }, 'stomp producer connected')
  return producerClient
}

async function startConsumer(cfg: Config): Promise<void> {
  consumerClient = await connect(cfg)
  logger.info({ host: cfg.host, queue: cfg.queueOut }, 'stomp consumer connected')

  consumerClient.subscribe(
    {
      destination: cfg.queueOut,
      ack: 'client-individual',
      'activemq.prefetchSize': '32',
    } as any,
    (err: Error | null, message: any) => {
      if (err) {
        logger.error({ err }, 'subscribe error')
        return
      }
      message.readString('utf-8', (rerr: Error | null, body: string | undefined) => {
        if (rerr) {
          logger.error({ err: rerr }, 'read message error')
          return
        }
        try {
          const payload = JSON.parse(body!) as WorldPayload & { job_id?: string }
          const jobId = payload.job_id || payload.id
          const event: WorldEvent = {
            type: 'world.done',
            jobId,
            at: Date.now(),
            payload,
          }
          worldBus.publish(event)
          consumerClient!.ack(message)
        } catch (e) {
          logger.error({ err: e, body }, 'invalid done message')
          consumerClient!.nack(message)
        }
      })
    }
  )
}

export async function start(): Promise<void> {
  if (started) return
  started = true
  const cfg = readConfig()
  try {
    await startConsumer(cfg)
  } catch (e) {
    logger.error({ err: e }, 'consumer start failed (will retry on first publish)')
    started = false
  }
}

export async function publishGenerate(payload: {
  jobId: string
  seed?: number
  size?: number
  scale?: number
}): Promise<void> {
  const cfg = readConfig()
  const client = await ensureProducer(cfg)
  const body = JSON.stringify({
    job_id: payload.jobId,
    seed: payload.seed,
    size: payload.size,
    scale: payload.scale,
  })

  return new Promise<void>((resolve, reject) => {
    const frame = client.send({
      destination: cfg.queueIn,
      'content-type': 'application/json',
      persistent: 'true',
    } as any)
    frame.write(body)
    frame.end((err: Error | undefined) => {
      if (err) return reject(err)
      resolve()
    })
  })
}

export async function shutdown(): Promise<void> {
  if (producerClient) {
    try {
      producerClient.disconnect()
    } catch {}
    producerClient = null
  }
  if (consumerClient) {
    try {
      consumerClient.disconnect()
    } catch {}
    consumerClient = null
  }
  started = false
}
