// Shared types mirrored between the Rust backend and the Svelte frontend.

export type RecordingState = "idle" | "starting" | "listening" | "error";

/** A finalized or in-progress transcript line. */
export interface TranscriptSegment {
  id: string;
  text: string;
  /** true while the provider may still revise this text (streaming partial). */
  partial: boolean;
  createdAt: number;
}

/** One saved dictation session in the history. */
export interface HistoryEntry {
  id: string;
  text: string;
  createdAt: number;
}

/** Events emitted by the Rust backend over the Tauri event bus. */
export type BackendEvent =
  | { kind: "state"; state: RecordingState }
  | { kind: "partial"; text: string }
  | { kind: "final"; text: string }
  | { kind: "level"; rms: number }
  | { kind: "error"; message: string };

export const EVENT_CHANNEL = "transcript://event";

/** Control actions pushed from the backend (global hotkey / CLI / tray). */
export type ControlAction = "toggle" | "start" | "stop" | "show";
export interface ControlEvent {
  action: ControlAction;
}
export const CONTROL_CHANNEL = "transcript://control";
