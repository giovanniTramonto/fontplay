import type { Transform } from '#shared/types'
import { SYSTEM_PROMPT } from '#shared/prompts/systemPrompt'
import { extractTransforms } from '#shared/utils/extractSvg'

interface OllamaResponse {
  message: { content: string }
}

function getSessionId(): string {
  const key = 'sessionId'
  const existing = localStorage.getItem(key)
  if (existing) return existing
  const id = crypto.randomUUID()
  localStorage.setItem(key, id)
  return id
}

export async function askLLM(property: string): Promise<Transform[]> {
  if (import.meta.env.VITE_OLLAMA_URL) return askOllama(property)
  return askNetlify(property)
}

async function askNetlify(property: string): Promise<Transform[]> {
  const response = await fetch('/api/ask', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Session-Id': getSessionId(),
    },
    body: JSON.stringify({ property }),
  })

  if (!response.ok) throw new Error(`API error: ${response.status} ${response.statusText}`)

  const data = await response.json()
  if (data.error) throw new Error(data.error)
  return data.transforms as Transform[]
}

async function askOllama(property: string): Promise<Transform[]> {
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
  const transforms = extractTransforms(data.message.content)
  if (transforms !== null) return transforms

  throw new Error(`No valid transforms received. Response: "${data.message.content.slice(0, 120)}…"`)
}
