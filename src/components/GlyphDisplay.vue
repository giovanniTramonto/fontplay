<script setup lang="ts">
import { computed, onMounted, watch, useTemplateRef } from 'vue'
import { DEFAULT_TEXT } from '#shared/constants'

const props = defineProps<{
  fontFamily: string | null
  editable?: boolean
}>()

const text = defineModel<string>({ required: true })
const editableEl = useTemplateRef<HTMLElement>('editableEl')

const isCustom = computed(() => text.value !== DEFAULT_TEXT)

const fontSize = computed(() => {
  const n = text.value.length
  if (n <= 1) return 'clamp(4rem, 28vh, 16rem)'
  if (n <= 3) return 'clamp(3rem, 20vh, 12rem)'
  if (n <= 6) return 'clamp(2rem, 14vh, 8rem)'
  if (n <= 10) return 'clamp(1.5rem, 10vh, 6rem)'
  return 'clamp(1rem, 6vh, 4rem)'
})

function onInput(e: Event) {
  const el = e.currentTarget as HTMLElement
  const val = (el.textContent ?? '').trim()
  if (val) text.value = val
}

function onPaste(e: ClipboardEvent) {
  e.preventDefault()
  const plain = e.clipboardData?.getData('text/plain') ?? ''
  const sel = window.getSelection()
  if (!sel?.rangeCount) return
  sel.deleteFromDocument()
  sel.getRangeAt(0).insertNode(document.createTextNode(plain))
  sel.collapseToEnd()
}

function onReset() {
  text.value = DEFAULT_TEXT
}

onMounted(() => {
  if (editableEl.value) editableEl.value.textContent = text.value
})

watch(text, (val) => {
  const el = editableEl.value
  if (el && el.textContent?.trim() !== val) el.textContent = val
})
</script>

<template>
  <div class="display" :class="{ 'display--editable': editable }">
    <div v-if="editable"
      ref="editableEl"
      class="display-text"
      :class="{ 'display-text--plain': !fontFamily }"
      :style="{ fontFamily: fontFamily ? `'${fontFamily}', sans-serif` : 'inherit', fontSize }"
      contenteditable="plaintext-only"
      spellcheck="false"
      autocomplete="off"
      :data-placeholder="'Type here…'"
      @input="onInput"
      @paste="onPaste"
    ></div>
    <template v-else>
      <p v-if="text && fontFamily" class="display-text"
        :style="{ fontFamily: `'${fontFamily}', sans-serif`, fontSize }">
        {{ text }}
      </p>
      <p v-else-if="text" class="display-text display-text--plain" :style="{ fontSize }">
        {{ text }}
      </p>
      <p v-else class="placeholder">Upload a font and type some text</p>
    </template>
    <button v-if="editable && isCustom" type="button" class="btn reset-btn" @click="onReset">Reset</button>
  </div>
</template>

<style scoped>
.display {
  position: relative;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  display: flex;
  justify-content: center;
  padding: 2rem;
  overflow: hidden;
}

.display--editable {
  cursor: text;
}

.display-text {
  line-height: 1.1;
  margin: 0;
  color: var(--color-accent);
  word-break: break-all;
  outline: none;
  min-width: 1ch;
}

.display-text--plain {
  color: var(--color-text);
  opacity: 0.3;
}

[contenteditable]:empty::before {
  content: attr(data-placeholder);
  opacity: 0.4;
  pointer-events: none;
}

.placeholder {
  margin: 0;
  color: var(--color-text);
  opacity: 0.4;
}

.reset-btn {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  font-size: 0.75rem;
  padding: 0.2rem 0.5rem;
}
</style>
