import { create } from "zustand";

export type ToastTone = "info" | "success" | "danger";

export interface Toast {
  id: string;
  message: string;
  tone: ToastTone;
}

interface ToastStore {
  toasts: Toast[];
  push: (input: { message: string; tone?: ToastTone }) => string;
  dismiss: (id: string) => void;
}

let nextId = 1;
const DEFAULT_MS = 4000;
const timers = new Map<string, ReturnType<typeof setTimeout>>();

export const useToastStore = create<ToastStore>((set, get) => ({
  toasts: [],
  push: ({ message, tone = "info" }) => {
    const id = `toast-${nextId}`;
    nextId += 1;
    set({ toasts: [...get().toasts, { id, message, tone }] });
    const timer = setTimeout(() => get().dismiss(id), DEFAULT_MS);
    timers.set(id, timer);
    return id;
  },
  dismiss: (id) => {
    const timer = timers.get(id);
    if (timer) {
      clearTimeout(timer);
      timers.delete(id);
    }
    set({ toasts: get().toasts.filter((toast) => toast.id !== id) });
  },
}));
