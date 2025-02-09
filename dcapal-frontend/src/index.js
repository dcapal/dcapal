import React, { Suspense } from "react";
import { createRoot } from "react-dom/client";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";

import "./style.css";
import { persistor, store } from "@app/store";
import { Router } from "@routes/router";
import { BrowserRouter } from "react-router-dom";
import App from './app/index.js';

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

const root = createRoot(document.getElementById("app"));
root.render(
  <React.StrictMode>
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <BrowserRouter>
          <Suspense fallback={<div>Loading...</div>}>
            <Router />
          </Suspense>
        </BrowserRouter>
      </PersistGate>
    </Provider>
  </React.StrictMode>
);
