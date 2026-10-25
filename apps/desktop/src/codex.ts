import { invoke } from '@tauri-apps/api/core';
import type { Message, Session } from './data';

export type CodexSource = {
  root: string;
  loadedAt: string;
  readOnly: boolean;
};

export type CodexLibrary = {
  source: CodexSource;
  sessions: Session[];
};

export function hasCodexSource() {
  return '__TAURI_INTERNALS__' in window;
}

export function loadCodexLibrary() {
  return invoke<CodexLibrary>('list_codex_sessions');
}

export function syncCodexLibrary() {
  return invoke<CodexLibrary>('sync_codex');
}

export async function readCodexMessages(nativeId: string): Promise<Message[]> {
  const messages = await invoke<Message[]>('read_codex_messages', { nativeId });
  return messages.map(message => ({
    ...message,
    time: message.time ? new Date(message.time).toLocaleString() : '时间未知',
  }));
}

export function openCodexSession(nativeId: string) {
  return invoke<void>('open_codex_session', { nativeId });
}
