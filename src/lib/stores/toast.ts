import { writable } from 'svelte/store';

export interface ToastState {
  visible: boolean;
  message: string;
  type?: 'success' | 'info' | 'error';
}

export const toastStore = writable<ToastState>({
  visible: false,
  message: '',
  type: 'success',
});

let toastTimeout: ReturnType<typeof setTimeout> | null = null;

export function showToast(message: string, type: 'success' | 'info' | 'error' = 'success') {
  if (toastTimeout) {
    clearTimeout(toastTimeout);
  }
  toastStore.set({ visible: true, message, type });
  toastTimeout = setTimeout(() => {
    toastStore.update((s) => ({ ...s, visible: false }));
  }, 3400);
}
