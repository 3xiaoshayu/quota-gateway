import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';

// Single dark theme, same as Quota Switcher's window background.
export const WINDOW_BACKGROUND = '#131315';

async function applyNativeTheme(): Promise<void> {
  const currentWindow = getCurrentWindow();
  const currentWebview = getCurrentWebview();

  await Promise.allSettled([
    currentWindow.setTheme('dark'),
    currentWindow.setBackgroundColor(WINDOW_BACKGROUND),
    currentWebview.setBackgroundColor(WINDOW_BACKGROUND),
  ]);
}

export function applyTheme(): void {
  document.documentElement.dataset.theme = 'dark';
  document.documentElement.style.colorScheme = 'dark';
  document.documentElement.style.backgroundColor = WINDOW_BACKGROUND;
  if (document.body) {
    document.body.style.backgroundColor = WINDOW_BACKGROUND;
  }
  void applyNativeTheme();
}
