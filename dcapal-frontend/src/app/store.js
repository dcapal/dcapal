import { combineReducers, configureStore } from "@reduxjs/toolkit";
import appReducer, { Step } from "@app/appSlice";
import storage from "redux-persist/lib/storage";
import {
  persistReducer,
  persistStore,
  FLUSH,
  REHYDRATE,
  PAUSE,
  PERSIST,
  PURGE,
  REGISTER,
} from "redux-persist";
import createMigrate from "redux-persist/es/createMigrate";
import { bindAppStoreToRedux } from "@/state/appStore";

const migrations = {
  0: (state) => state,
  2: (state) => state,
  3: (state) => state,
  4: (state) => {
    const pfolio = state?.pfolio;
    const hasPfolio = typeof pfolio === "object" && pfolio !== null;
    const preferredCurrency =
      typeof pfolio?.quoteCcy === "string" ? pfolio.quoteCcy : "";

    return {
      ...state,
      app: {
        ...state?.app,
        ...(!hasPfolio && { allocationFlowStep: Step.PORTFOLIOS }),
        preferredCurrency,
      },
    };
  },
  5: (state) => state,
};

const rootConfig = {
  key: "root",
  version: 5,
  storage,
  migrate: createMigrate(migrations, { debug: false }),
};

const rootReducer = combineReducers({
  app: appReducer,
});

const persistedReducer = persistReducer(rootConfig, rootReducer);

export const store = configureStore({
  reducer: persistedReducer,
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware({
      serializableCheck: {
        ignoredActions: [FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER],
      },
    }),
});

bindAppStoreToRedux(store);

export const persistor = persistStore(store);
