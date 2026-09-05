import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { AppErrorBoundary } from './AppErrorBoundary';
import App from './App';
import { I18nProvider } from './i18n';
import { applyTheme } from './theme';
// Same Inter cuts Quota Switcher ships, so the two apps read as one family
// even on a machine without Inter installed.
import '@fontsource/inter/latin-400.css';
import '@fontsource/inter/latin-500.css';
import '@fontsource/inter/latin-600.css';
import '@fontsource/inter/latin-700.css';
import '@fontsource/inter/latin-800.css';
import './styles.css';
import './theme-quota.css';

applyTheme();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <I18nProvider>
      <AppErrorBoundary>
        <App />
      </AppErrorBoundary>
    </I18nProvider>
  </StrictMode>
);
