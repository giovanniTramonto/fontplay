import Anthropic from '@anthropic-ai/sdk'
import { getStore } from '@netlify/blobs'
import { MAX_PROMPT_LENGTH, MAX_SESSION_REQUESTS } from '#shared/config'
import { makeBlendPrompt } from '#shared/prompts/blendPrompt'
import { extractBlendResult } from '#shared/utils/extractSvg'


const client = new Anthropic()

export default async (req: Request) => {
  if (req.method !== 'POST') {
    return new Response('Method not allowed', { status: 405 })
  }

  const { font1Name, font2Name, font1Paths, font2Paths } = await req.json()

  if (
    !font1Name ||
    !font2Name ||
    typeof font1Name !== 'string' ||
    typeof font2Name !== 'string'
  ) {
    return new Response(JSON.stringify({ error: 'font1Name and font2Name are required.' }), {
      status: 400,
      headers: { 'Content-Type': 'application/json' },
    })
  }

  if (font1Name.length > MAX_PROMPT_LENGTH || font2Name.length > MAX_PROMPT_LENGTH) {
    return new Response(
      JSON.stringify({ error: `Font names must not exceed ${MAX_PROMPT_LENGTH} characters.` }),
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

  const prompt = makeBlendPrompt(font1Name, font2Name, font1Paths ?? {}, font2Paths ?? {})
  const message = await client.messages.create({
    model: 'claude-sonnet-4-6',
    max_tokens: 512,
    messages: [{ role: 'user', content: prompt }],
  })

  const raw = message.content[0].type === 'text' ? message.content[0].text : ''
  const result = extractBlendResult(raw)

  if (result === null) {
    return new Response(
      JSON.stringify({ error: `No valid result received. Response: "${raw.slice(0, 120)}…"` }),
      { status: 422, headers: { 'Content-Type': 'application/json' } },
    )
  }

  return new Response(
    JSON.stringify({ blendFactor: result.blendFactor }),
    { status: 200, headers: { 'Content-Type': 'application/json' } },
  )
}

export const config = { path: '/api/blend' }
