import React from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";

import "./style.css";
import { App } from "./app";
import { store, persistor } from "./app/store";
import { Root } from "./routes/root";
import ErrorPage from "./routes/errorPage";
import { Router } from "./routes/router";
import { BrowserRouter } from "react-router-dom";

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import translationEN from "../public/locales/en/translation.json";
import translationIT from "../public/locales/it/translation.json";

const router = createBrowserRouter([
  {
    path: "/",
    element: <Root />,
    errorElement: <ErrorPage />,
  },
  {
    path: "allocate",
    element: <App />,
  },
]);

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

const root = createRoot(document.getElementById("app"));
root.render(
  <React.StrictMode>
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <BrowserRouter>
          <Router />
        </BrowserRouter>
      </PersistGate>
    </Provider>
  </React.StrictMode>
);
