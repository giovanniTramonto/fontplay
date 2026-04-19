import { makeBlendPrompt } from '#shared/prompts/blendPrompt'
import { SYSTEM_PROMPT } from '#shared/prompts/systemPrompt'
import type { ColrConfig, Transform } from '#shared/types'
import { extractBlendResult, extractResult } from '#shared/utils/extractSvg'

interface OllamaResponse {
  message: { content: string }
}

export interface MoodResult {
  transforms: Transform[]
  colr: ColrConfig
}

export interface BlendResult {
  blendFactor: number
}

function getSessionId(): string {
  const key = 'sessionId'
  const existing = localStorage.getItem(key)
  if (existing) return existing
  const id = crypto.randomUUID()
  localStorage.setItem(key, id)
  return id
}

export async function askLLM(property: string): Promise<MoodResult> {
  if (import.meta.env.VITE_OLLAMA_URL) return askOllama(property)
  return askNetlify(property)
}

export async function askBlendLLM(
  font1Name: string,
  font2Name: string,
  font1Paths: Record<string, string>,
  font2Paths: Record<string, string>,
): Promise<BlendResult> {
  if (import.meta.env.VITE_OLLAMA_URL) return askBlendOllama(font1Name, font2Name, font1Paths, font2Paths)
  return askBlendNetlify(font1Name, font2Name, font1Paths, font2Paths)
}

async function askNetlify(property: string): Promise<MoodResult> {
  const response = await fetch('/api/ask', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Session-Id': getSessionId(),
    },
    body: JSON.stringify({ property }),
  })

  if (!response.ok) {
    const data = await response.json().catch(() => null)
    throw new Error(data?.error ?? `API error: ${response.status} ${response.statusText}`)
  }

  const data = await response.json()
  if (data.error) throw new Error(data.error)
  return {
    transforms: data.transforms,
    colr: { ...data.colr, effects: new Set(data.colr.active ?? []) },
  }
}

async function askOllama(property: string): Promise<MoodResult> {
  const response = await fetch(import.meta.env.VITE_OLLAMA_URL as string, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: import.meta.env.VITE_OLLAMA_MODEL ?? 'qwen2.5-coder',
      stream: false,
      options: { num_predict: 512 },
      messages: [
        { role: 'system', content: SYSTEM_PROMPT },
        { role: 'user', content: property },
      ],
    }),
  })

  if (!response.ok) throw new Error(`Ollama error: ${response.status} ${response.statusText}`)

  const data: OllamaResponse = await response.json()
  const result = extractResult(data.message.content)
  if (result !== null) return result

  throw new Error(`No valid result received. Response: "${data.message.content.slice(0, 120)}…"`)
}

async function askBlendNetlify(
  font1Name: string,
  font2Name: string,
  font1Paths: Record<string, string>,
  font2Paths: Record<string, string>,
): Promise<BlendResult> {
  const response = await fetch('/api/blend', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Session-Id': getSessionId(),
    },
    body: JSON.stringify({ font1Name, font2Name, font1Paths, font2Paths }),
  })

  if (!response.ok) {
    const data = await response.json().catch(() => null)
    throw new Error(data?.error ?? `API error: ${response.status} ${response.statusText}`)
  }

  const data = await response.json()
  if (data.error) throw new Error(data.error)
  return { blendFactor: data.blendFactor ?? 0.5 }
}

async function askBlendOllama(
  font1Name: string,
  font2Name: string,
  font1Paths: Record<string, string>,
  font2Paths: Record<string, string>,
): Promise<BlendResult> {
  const prompt = makeBlendPrompt(font1Name, font2Name, font1Paths, font2Paths)
  const response = await fetch(import.meta.env.VITE_OLLAMA_URL as string, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: import.meta.env.VITE_OLLAMA_MODEL ?? 'qwen2.5-coder',
      stream: false,
      options: { num_predict: 512 },
      messages: [{ role: 'user', content: prompt }],
    }),
  })

  if (!response.ok) throw new Error(`Ollama error: ${response.status} ${response.statusText}`)

  const data: OllamaResponse = await response.json()
  const result = extractBlendResult(data.message.content)
  if (result !== null) return result

  throw new Error(`No valid result received. Response: "${data.message.content.slice(0, 120)}…"`)
}
