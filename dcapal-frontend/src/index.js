import React, { Suspense } from "react";
import { createRoot } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import {
  configureApiClientAuth,
  configureApiClientBaseUrl,
} from "@dcapal/api-client";

import "./style.css";
import { Router } from "@routes/router";
import { BrowserRouter } from "react-router-dom";
import { HelmetProvider } from "react-helmet-async";
import { DCAPAL_API, supabase } from "@app/config";
import { queryClient } from "@/api/queryClient";

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import translationEN from "../public/locales/en/translation.json";
import translationIT from "../public/locales/it/translation.json";

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: translationEN,
      it: translationIT,
    },
    fallbackLng: "en",
    interpolation: {
      escapeValue: false,
    },
  });

document.documentElement.lang = i18n.language;
i18n.on("languageChanged", (lang) => (document.documentElement.lang = lang));

// Configure the generated client before rendering so every initial query uses
// the same API base URL and authentication callbacks.
configureApiClientBaseUrl(DCAPAL_API);
configureApiClientAuth({
  getAccessToken: async () => {
    const {
      data: { session },
    } = await supabase.auth.getSession();
    return session?.access_token;
  },
  refreshAccessToken: async () => {
    const {
      data: { session },
      error,
    } = await supabase.auth.refreshSession();
    if (error) throw error;
    return session?.access_token;
  },
  onAuthFailure: async () => {
    await supabase.auth.signOut();
  },
});

// Start MSW only for browser journeys; production must use the real API.
const startMocks = async () => {
  if (process.env.REACT_APP_E2E_MSW !== "1") {
    return;
  }

  const { worker } = await import("./mocks/browser");
  await worker.start({
    quiet: true,
    onUnhandledRequest({ request }, print) {
      const url = new URL(request.url);
      if (url.pathname.startsWith("/api/")) {
        print.error();
        throw new Error(
          `[MSW] Unhandled API request: ${request.method} ${url.pathname}`
        );
      }
    },
  });
};

const renderApp = () => {
  const root = createRoot(document.getElementById("app"));
  root.render(
    <HelmetProvider>
      <React.StrictMode>
        <QueryClientProvider client={queryClient}>
          <BrowserRouter>
            <Suspense fallback={<div>Loading...</div>}>
              <Router />
            </Suspense>
          </BrowserRouter>
        </QueryClientProvider>
      </React.StrictMode>
    </HelmetProvider>
  );
};

startMocks().then(renderApp);
