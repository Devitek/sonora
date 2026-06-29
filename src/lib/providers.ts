// Provider catalogues + default-model placeholders, shared by the bar and the
// dedicated Settings window.

export interface ProviderOption {
  id: string;
  label: string;
}

export const PROVIDERS: ProviderOption[] = [
  { id: "gemini", label: "Gemini Live (streaming)" },
  { id: "mistral", label: "Mistral (Voxtral)" },
  { id: "openai", label: "OpenAI Whisper" },
  { id: "groq", label: "Groq Whisper (rapide)" },
  { id: "openai-compatible", label: "OpenAI-compatible" },
  { id: "whisper-local", label: "Whisper local (offline)" },
];

export const CLEANUP_ENGINES: ProviderOption[] = [
  { id: "gemini", label: "Gemini" },
  { id: "mistral", label: "Mistral" },
  { id: "groq", label: "Groq (rapide)" },
  { id: "openai", label: "OpenAI" },
  { id: "openai-compatible", label: "OpenAI-compatible (local…)" },
];

export const CLEANUP_MODEL_PLACEHOLDER: Record<string, string> = {
  gemini: "gemini-2.5-flash",
  mistral: "mistral-small-latest",
  groq: "llama-3.3-70b-versatile",
  openai: "gpt-4o-mini",
  "openai-compatible": "nom du modèle",
};

export const MODEL_PLACEHOLDER: Record<string, string> = {
  gemini: "gemini-2.5-flash-native-audio-latest",
  mistral: "voxtral-mini-latest",
  openai: "whisper-1",
  groq: "whisper-large-v3",
  "openai-compatible": "whisper-1",
  "whisper-local": "",
};

/** Providers that need an API key (i.e. not local Whisper / not key-optional). */
export const KEYED_PROVIDERS = ["gemini", "mistral", "openai", "groq", "openai-compatible"];

/** Tauri event emitted (from the backend) when persisted settings change. */
export const SETTINGS_CHANGED_EVENT = "sonora://settings-changed";
