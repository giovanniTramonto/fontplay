import Anthropic from '@anthropic-ai/sdk'
import { getStore } from '@netlify/blobs'
import { MAX_SESSION_REQUESTS } from '#shared/config'
import { RECOMBINE_PROMPT } from '#shared/prompts/recombinePrompt'

const client = new Anthropic()

export default async (req: Request) => {
  if (req.method !== 'POST') {
    return new Response('Method not allowed', { status: 405 })
  }

  const { char, image1, image2 } = await req.json()

  if (!char || !image1 || !image2) {
    return new Response(JSON.stringify({ error: 'char, image1 and image2 are required.' }), {
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
    model: 'claude-opus-4-5',
    max_tokens: 2048,
    system: RECOMBINE_PROMPT,
    messages: [
      {
        role: 'user',
        content: [
          {
            type: 'image',
            source: { type: 'base64', media_type: 'image/png', data: image1 },
          },
          {
            type: 'image',
            source: { type: 'base64', media_type: 'image/png', data: image2 },
          },
          {
            type: 'text',
            text: `These are two renderings of the letter "${char}". Design a hybrid glyph combining elements from both.`,
          },
        ],
      },
    ],
  })

  const raw = message.content[0].type === 'text' ? message.content[0].text : ''

  const match = raw.match(/\{[\s\S]*\}/)
  if (!match) {
    return new Response(
      JSON.stringify({ error: `No valid JSON received. Response: "${raw.slice(0, 120)}…"` }),
      { status: 422, headers: { 'Content-Type': 'application/json' } },
    )
  }

  try {
    const parsed = JSON.parse(match[0]) as { path?: unknown; reasoning?: string }
    const rawPath = parsed.path
    const path = typeof rawPath === 'string'
      ? rawPath
      : Array.isArray(rawPath)
        ? (rawPath as string[]).join(' ')
        : null
    if (!path) throw new Error('missing path')
    return new Response(JSON.stringify({ path, reasoning: parsed.reasoning ?? '' }), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    })
  } catch {
    return new Response(
      JSON.stringify({ error: `Invalid JSON in response: "${raw.slice(0, 120)}…"` }),
      { status: 422, headers: { 'Content-Type': 'application/json' } },
    )
  }
}

export const config = { path: '/api/recombine' }
