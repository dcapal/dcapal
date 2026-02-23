/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_E2E_MSW?: string;
  readonly VITE_ENABLE_COOKIE_BUTTON?: string;
  readonly VITE_SUPABASE_URL: string;
  readonly VITE_SUPABASE_ANON_KEY: string;
  readonly VITE_PROXY_API_TARGET?: string;
  readonly VITE_PROXY_YAHOO_CHART_TARGET?: string;
  readonly VITE_PROXY_YAHOO_SEARCH_TARGET?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
