import { combineReducers, configureStore } from "@reduxjs/toolkit";
import portfolioReducer from "../components/allocationFlow/portfolioStep/portfolioSlice";
import appReducer from "./appSlice";
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

const rootConfig = {
  key: "root",
  version: 1,
  storage,
  migrate: (state) => {
    const assets = state?.pfolio?.assets;
    if (!assets || Object.keys(assets).length === 0) {
      return Promise.resolve(state);
    }

    const anyAsset = Object.values(assets)[0];
    if (!anyAsset.aclass) {
      console.log(
        "Old 'pfolio' persisted state version, missing 'aclass' property. Purging state"
      );
      return Promise.resolve({});
    }

    return Promise.resolve(state);
  },
};

const rootReducer = combineReducers({
  app: appReducer,
  pfolio: portfolioReducer,
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

export const persistor = persistStore(store);
