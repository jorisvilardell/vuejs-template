import router from '@adonisjs/core/services/router'

const GenerateController = () => import('#controllers/generate_controller')
const EventsController = () => import('#controllers/events_controller')

router.get('/healthz', async ({ response }) => response.ok({ status: 'ok', service: 'backend' }))

router
  .group(() => {
    router.post('/generate', [GenerateController, 'store'])
    router.get('/events', [EventsController, 'stream'])
  })
  .prefix('/api')
