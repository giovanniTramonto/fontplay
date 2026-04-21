import Anthropic from '@anthropic-ai/sdk'
import { getStore } from '@netlify/blobs'
import { MAX_SESSION_REQUESTS } from '#shared/config'
import { SPLICE_SYSTEM_PROMPT } from '#shared/prompts/splicePrompt'
import { extractSpliceResult } from '#shared/utils/parseLLMResult'

const client = new Anthropic({
  defaultHeaders: { 'anthropic-beta': 'prompt-caching-2024-07-31' },
})

export default async (req: Request) => {
  if (req.method !== 'POST') {
    return new Response('Method not allowed', { status: 405 })
  }

  const { intensity } = await req.json()

  if (!intensity || !['low', 'medium', 'high'].includes(intensity)) {
    return new Response(JSON.stringify({ error: 'intensity must be low, medium, or high.' }), {
      status: 400,
      headers: { 'Content-Type': 'application/json' },
    })
  }

  const sessionId = req.headers.get('X-Session-Id')
  if (sessionId) {
    const store = getStore('sessions')
    const count = ((await store.get(sessionId, { type: 'json' })) as number | null) ?? 0
    if (count >= MAX_SESSION_REQUESTS) {
      return new Response(
        JSON.stringify({ error: `Session limit of ${MAX_SESSION_REQUESTS} requests reached.` }),
        { status: 429, headers: { 'Content-Type': 'application/json' } },
      )
    }
    await store.set(sessionId, String(count + 1))
  }

  const message = await client.messages.create({
    model: 'claude-sonnet-4-6',
    max_tokens: 2048,
    system: [{ type: 'text', text: SPLICE_SYSTEM_PROMPT, cache_control: { type: 'ephemeral' } }],
    messages: [{ role: 'user', content: `intensity: ${intensity}` }],
  })

  const raw = message.content[0].type === 'text' ? message.content[0].text : ''
  const result = extractSpliceResult(raw)

  return new Response(JSON.stringify(result), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  })
}

export const config = { path: '/api/splice' }
