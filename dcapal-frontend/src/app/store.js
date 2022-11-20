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

const rootConfig = { key: "root", storage };

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
