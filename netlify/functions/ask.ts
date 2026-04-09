import Anthropic from '@anthropic-ai/sdk'
import { getStore } from '@netlify/blobs'
import { MAX_PROMPT_LENGTH, MAX_SESSION_REQUESTS } from '#shared/config'
import { SYSTEM_PROMPT } from '#shared/prompts/systemPrompt'
import { extractTransforms } from '#shared/utils/extractSvg'

const client = new Anthropic({
  defaultHeaders: { 'anthropic-beta': 'prompt-caching-2024-07-31' },
})

export default async (req: Request) => {
  if (req.method !== 'POST') {
    return new Response('Method not allowed', { status: 405 })
  }

  const { property } = await req.json()

  if (!property || typeof property !== 'string' || property.trim().length === 0) {
    return new Response(JSON.stringify({ error: 'Property is required.' }), {
      status: 400,
      headers: { 'Content-Type': 'application/json' },
    })
  }

  if (property.length > MAX_PROMPT_LENGTH) {
    return new Response(
      JSON.stringify({ error: `Property must not exceed ${MAX_PROMPT_LENGTH} characters.` }),
      { status: 400, headers: { 'Content-Type': 'application/json' } },
    )
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
    max_tokens: 256,
    system: [{ type: 'text', text: SYSTEM_PROMPT, cache_control: { type: 'ephemeral' } }],
    messages: [{ role: 'user', content: property }],
  })

  const raw = message.content[0].type === 'text' ? message.content[0].text : ''
  const transforms = extractTransforms(raw)

  if (transforms === null) {
    return new Response(
      JSON.stringify({ error: `No valid transforms received. Response: "${raw.slice(0, 120)}…"` }),
      { status: 422, headers: { 'Content-Type': 'application/json' } },
    )
  }

  return new Response(JSON.stringify({ transforms }), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  })
}

export const config = { path: '/api/ask' }
