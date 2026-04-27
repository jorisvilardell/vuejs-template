<script setup>
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'

const status = ref('idle') // idle | running | done | error
const phase = ref('')
const jobId = ref(null)
const result = ref(null)
const error = ref(null)
const startedAt = ref(0)
const elapsed = ref(0)

let es = null
let timer = null

const apiBase = import.meta.env.VITE_API_BASE || ''

const imgSrc = computed(() => result.value?.png_url ?? null)

function connectSse() {
  if (es) es.close()
  es = new EventSource(`${apiBase}/api/events`)
  es.addEventListener('world.started', (e) => {
    const data = JSON.parse(e.data)
    if (data.jobId !== jobId.value) return
    status.value = 'running'
    startedAt.value = data.at
    phase.value = 'Initialisation…'
  })
  es.addEventListener('world.progress', (e) => {
    const data = JSON.parse(e.data)
    if (data.jobId !== jobId.value) return
    phase.value = data.phase + '…'
  })
  es.addEventListener('world.done', (e) => {
    const data = JSON.parse(e.data)
    if (data.jobId !== jobId.value) return
    result.value = data.payload
    status.value = 'done'
    phase.value = ''
  })
  es.addEventListener('world.error', (e) => {
    const data = JSON.parse(e.data)
    if (data.jobId !== jobId.value) return
    error.value = data.error
    status.value = 'error'
    phase.value = ''
  })
  es.onerror = () => { /* auto reconnect via retry */ }
}

async function generate() {
  result.value = null
  error.value = null
  phase.value = 'Connexion…'
  status.value = 'running'

  const r = await fetch(`${apiBase}/api/generate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ size: 128, scale: 6 }),
  })
  if (!r.ok) {
    error.value = `HTTP ${r.status}`
    status.value = 'error'
    return
  }
  const j = await r.json()
  jobId.value = j.jobId
  startedAt.value = j.startedAt
}

function exportPng() {
  if (!result.value) return
  const a = document.createElement('a')
  a.href = result.value.png_url
  a.download = `world-${result.value.seed}.png`
  a.target = '_blank'
  a.rel = 'noopener'
  a.click()
}

function exportJson() {
  if (!result.value) return
  const a = document.createElement('a')
  a.href = result.value.json_url
  a.download = `world-${result.value.seed}.json`
  a.target = '_blank'
  a.rel = 'noopener'
  a.click()
}

onMounted(() => {
  connectSse()
  timer = setInterval(() => {
    elapsed.value = startedAt.value ? Date.now() - startedAt.value : 0
  }, 100)
})

onBeforeUnmount(() => {
  if (es) es.close()
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="wrapper">
    <h1>Pixel World Forge</h1>
    <p class="subtitle">Bruit de Perlin · Pixel art · Génération à la demande</p>

    <button
      class="big-button"
      :class="{ loading: status === 'running' }"
      :disabled="status === 'running'"
      @click="generate"
    >
      <span v-if="status !== 'running'">🌍 Générer un nouveau monde</span>
      <span v-else>⛰️ {{ phase || 'Forgeage…' }}</span>
    </button>

    <transition name="fade">
      <div v-if="result" class="result">
        <img :src="imgSrc" :alt="`Monde seed ${result.seed}`" class="map" />
        <div class="meta">
          <span>seed <code>{{ result.seed }}</code></span>
          <span>{{ result.size }}×{{ result.size }} ×{{ result.scale }}</span>
          <span>{{ result.duration_ms }} ms</span>
        </div>
        <div class="exports">
          <button @click="exportPng">⬇ Exporter PNG</button>
          <button @click="exportJson">⬇ Exporter JSON</button>
        </div>
      </div>
    </transition>

    <p v-if="status === 'error'" class="error">Erreur: {{ error }}</p>
  </div>
</template>

<style scoped>
.wrapper {
  min-height: 100dvh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1.5rem;
  background: radial-gradient(ellipse at center, #1a1f2e 0%, #0a0d14 100%);
  color: #e8e6df;
  font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
  padding: 2rem;
  text-align: center;
}

h1 {
  font-size: clamp(2rem, 5vw, 3.5rem);
  letter-spacing: -0.02em;
  margin: 0;
  background: linear-gradient(90deg, #6fb34e 0%, #3e82c4 50%, #f7f7f7 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle {
  margin: 0;
  opacity: 0.6;
  font-size: 0.95rem;
}

.big-button {
  font-size: 1.25rem;
  padding: 1.1rem 2.4rem;
  border: none;
  border-radius: 14px;
  background: linear-gradient(135deg, #6fb34e 0%, #3e82c4 100%);
  color: #0a0d14;
  font-weight: 700;
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease, filter 0.2s ease;
  box-shadow: 0 10px 30px -10px rgba(110, 178, 78, 0.5);
  min-width: 320px;
}
.big-button:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 16px 40px -12px rgba(110, 178, 78, 0.7);
}
.big-button:disabled {
  cursor: progress;
  filter: brightness(0.85);
}
.big-button.loading {
  animation: pulse 1.4s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { filter: brightness(0.85); }
  50% { filter: brightness(1.05); }
}

.result {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.8rem;
}
.map {
  image-rendering: pixelated;
  image-rendering: crisp-edges;
  width: min(70vmin, 720px);
  height: auto;
  border-radius: 8px;
  border: 1px solid #2a2f3e;
  box-shadow: 0 25px 60px -20px rgba(0, 0, 0, 0.7);
}
.meta {
  display: flex;
  gap: 1.2rem;
  font-size: 0.85rem;
  opacity: 0.7;
}
.meta code {
  background: rgba(255, 255, 255, 0.06);
  padding: 1px 6px;
  border-radius: 4px;
}
.exports {
  display: flex;
  gap: 0.8rem;
}
.exports button {
  padding: 0.5rem 1rem;
  border: 1px solid #2a2f3e;
  background: rgba(255, 255, 255, 0.04);
  color: inherit;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.9rem;
}
.exports button:hover {
  background: rgba(255, 255, 255, 0.08);
}

.error {
  color: #ff7d7d;
  font-size: 0.9rem;
}

.fade-enter-active, .fade-leave-active {
  transition: opacity 0.4s ease, transform 0.4s ease;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
