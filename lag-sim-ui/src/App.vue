<template>
  <div class="min-h-screen flex items-center justify-center px-6 py-10 font-glitch">
    <div class="w-full max-w-4xl border border-neon-cyan/60 shadow-glow bg-black/70 backdrop-blur rounded-xl p-8 relative overflow-hidden">
      <div class="absolute inset-0 opacity-20 bg-[radial-gradient(circle_at_50%_0%,#26fff2_0%,transparent_60%)]"></div>

      <div class="relative z-10">
        <div class="flex items-center justify-between mb-6">
          <h1 class="text-3xl md:text-4xl font-title text-neon-pink tracking-widest">LAGOCALYPSE PANEL</h1>
          <div class="flex items-center gap-3">
            <span class="w-3 h-3 rounded-full" :class="warningLightClass"></span>
            <span class="text-xs uppercase tracking-widest text-neon-cyan">warning light</span>
          </div>
        </div>

        <div class="mb-6">
          <div class="text-center font-title text-2xl md:text-3xl text-neon-lime" ref="statusText">
            {{ stats.goofy }}
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
          <div class="space-y-6">
            <div>
              <div class="flex justify-between text-sm mb-2">
                <span>Latency (ms)</span>
                <span class="text-neon-cyan">{{ config.latency }}</span>
              </div>
              <input type="range" min="0" max="3000" v-model="config.latency" class="w-full accent-neon-pink" @input="sendConfig" />
            </div>

            <div>
              <div class="flex justify-between text-sm mb-2">
                <span>Drop %</span>
                <span class="text-neon-cyan">{{ config.drop }}</span>
              </div>
              <input type="range" min="0" max="90" v-model="config.drop" class="w-full accent-neon-red" @input="sendConfig" />
            </div>

            <div>
              <div class="flex justify-between text-sm mb-2">
                <span>Bandwidth (kbps)</span>
                <span class="text-neon-cyan">{{ config.bandwidth }}</span>
              </div>
              <input type="range" min="16" max="2048" v-model="config.bandwidth" class="w-full accent-neon-blue" @input="sendConfig" />
            </div>

            <div class="flex items-center justify-between">
              <label class="flex items-center gap-3">
                <input type="checkbox" v-model="config.chaos" @change="sendConfig" class="scale-125 accent-neon-lime">
                <span class="uppercase tracking-widest">Chaos mode</span>
              </label>
              <div class="text-xs text-neon-red uppercase tracking-widest">{{ config.chaos ? "GLITCHING" : "stable-ish" }}</div>
            </div>

            <button
              ref="worseBtn"
              @click="makeItWorse"
              class="bg-neon-red text-black font-title py-3 px-6 rounded-lg shadow-glow uppercase tracking-widest"
            >
              MAKE IT WORSE
            </button>
          </div>

          <div class="space-y-6">
            <div class="border border-neon-purple/50 rounded-lg p-4 bg-black/50">
              <div class="text-sm uppercase tracking-widest text-neon-purple mb-2">Live Damage</div>
              <div class="flex justify-between">
                <span>Bytes delayed</span>
                <span class="text-neon-cyan">{{ stats.bytes_delayed }}</span>
              </div>
              <div class="flex justify-between">
                <span>Packets dropped</span>
                <span class="text-neon-cyan">{{ stats.packets_dropped }}</span>
              </div>
            </div>

            <div class="border border-neon-cyan/40 rounded-lg p-4 bg-black/40">
              <div class="text-sm uppercase tracking-widest text-neon-cyan mb-2">Damage Meter</div>
              <div class="h-4 w-full bg-black rounded overflow-hidden border border-neon-cyan/50">
                <div class="h-full bg-neon-pink" :style="{ width: damage + '%' }" ref="progressBar"></div>
              </div>
              <div class="text-xs mt-2 text-neon-lime">{{ damageText }}</div>
            </div>

            <div class="border border-neon-lime/40 rounded-lg p-4 bg-black/40">
              <div class="text-sm uppercase tracking-widest text-neon-lime mb-2">Proxy Status</div>
              <div class="text-xs">
                <div>GET /stats</div>
                <div>POST /config</div>
                <div class="text-neon-pink mt-2">Frontend and Rust are separated, like all good chaos</div>
              </div>
            </div>
          </div>
        </div>

        <div class="mt-8 text-xs text-neon-cyan/60">
          Tip: set your browser proxy to 127.0.0.1:9000 and watch the internet wobble.
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, watch } from "vue"
import { gsap } from "gsap"

const config = reactive({
  latency: 120,
  drop: 5,
  bandwidth: 256,
  chaos: false
})

const stats = reactive({
  bytes_delayed: 0,
  packets_dropped: 0,
  goofy: "NETWORK IS HAVING A BAD DAY"
})

const damage = ref(1)
const damageText = ref("minimal harm")
const statusText = ref(null)
const progressBar = ref(null)
const worseBtn = ref(null)

const warningLightClass = ref("bg-neon-red shadow-[0_0_8px_#ff2d2d]")

const sendConfig = async () => {
  const body = new URLSearchParams({
    latency: String(config.latency),
    drop: String(config.drop),
    bandwidth: String(config.bandwidth),
    chaos: String(config.chaos)
  })
  await fetch("/config", { method: "POST", body })
  recalcDamage()
}

const recalcDamage = () => {
  const latencyScore = (config.latency / 3000) * 35
  const dropScore = (config.drop / 90) * 40
  const bandwidthScore = ((2048 - config.bandwidth) / (2048 - 16)) * 20
  const chaosBonus = config.chaos ? 10 : 0
  const liveScore = Math.min(10, (stats.packets_dropped * 0.5) + (stats.bytes_delayed / 8000))

  const d = Math.min(100, latencyScore + dropScore + bandwidthScore + chaosBonus + liveScore)
  damage.value = Math.max(1, Math.round(d))

  damageText.value =
    d > 85 ? "total collapse" :
    d > 65 ? "severe misery" :
    d > 40 ? "moderate panic" :
    d > 20 ? "mild chaos" :
    "minimal harm"
}

const pullStats = async () => {
  const r = await fetch("/stats")
  const j = await r.json()
  stats.bytes_delayed = j.bytes_delayed
  stats.packets_dropped = j.packets_dropped
  stats.goofy = j.goofy
  recalcDamage()
}

const makeItWorse = async () => {
  config.latency = Math.min(3000, config.latency + 200)
  config.drop = Math.min(90, config.drop + 5)
  config.bandwidth = Math.max(16, config.bandwidth - 64)
  await sendConfig()
  gsap.fromTo(".min-h-screen", { x: -6, y: 4 }, { x: 6, y: -4, duration: 0.2, yoyo: true, repeat: 3 })
}

watch(() => config.chaos, (v) => {
  if (v) {
    gsap.to(statusText.value, { x: 2, y: -2, duration: 0.06, yoyo: true, repeat: -1 })
  } else {
    gsap.killTweensOf(statusText.value)
    gsap.to(statusText.value, { x: 0, y: 0, duration: 0.1 })
  }
})

onMounted(async () => {
  gsap.to(warningLightClass, { duration: 0 })
  setInterval(pullStats, 700)
  gsap.to(progressBar.value, { x: 2, duration: 0.08, yoyo: true, repeat: -1 })
  gsap.to(".shadow-glow", { boxShadow: "0 0 22px rgba(173,255,47,0.5)", duration: 1.6, yoyo: true, repeat: -1 })
  gsap.to(warningLightClass, { duration: 0 })
  setInterval(() => {
    warningLightClass.value = Math.random() > 0.5 ? "bg-neon-red shadow-[0_0_10px_#ff2d2d]" : "bg-neon-lime shadow-[0_0_10px_#adff2f]"
  }, 600)
})
</script>