import { makeBlendPrompt } from '#shared/prompts/blendPrompt'
import { SPLICE_SYSTEM_PROMPT } from '#shared/prompts/splicePrompt'
import { SYSTEM_PROMPT } from '#shared/prompts/systemPrompt'
import type { ColrConfig, Transform } from '#shared/types'
import { extractBlendResult, extractResult, extractSpliceResult } from '#shared/utils/parseLLMResult'
import type { SpliceLLMResult } from '#shared/utils/parseLLMResult'

interface OllamaResponse {
  message: { content: string }
}

export interface StyleResult {
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

export async function askLLM(property: string): Promise<StyleResult> {
  if (import.meta.env.VITE_OLLAMA_URL) return askOllama(property)
  return askNetlify(property)
}

export async function askSpliceLLM(intensity: 'low' | 'medium' | 'high'): Promise<SpliceLLMResult> {
  if (import.meta.env.VITE_OLLAMA_URL) return askSpliceOllama(intensity)
  return askSpliceNetlify(intensity)
}

async function askSpliceNetlify(intensity: 'low' | 'medium' | 'high'): Promise<SpliceLLMResult> {
  const res = await fetch('/api/splice', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Session-Id': getSessionId() },
    body: JSON.stringify({ intensity }),
  })
  if (!res.ok) {
    const data = await res.json().catch(() => null)
    throw new Error(data?.error ?? `API error: ${res.status} ${res.statusText}`)
  }
  return res.json()
}

async function askSpliceOllama(intensity: 'low' | 'medium' | 'high'): Promise<SpliceLLMResult> {
  const res = await fetch(import.meta.env.VITE_OLLAMA_URL as string, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: import.meta.env.VITE_OLLAMA_MODEL,
      stream: false,
      messages: [
        { role: 'system', content: SPLICE_SYSTEM_PROMPT },
        { role: 'user', content: `intensity: ${intensity}` },
      ],
    }),
  })
  if (!res.ok) throw new Error(`Ollama error: ${res.status}`)
  const data = (await res.json()) as OllamaResponse
  return extractSpliceResult(data.message.content)
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

async function askNetlify(property: string): Promise<StyleResult> {
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

async function askOllama(property: string): Promise<StyleResult> {
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
