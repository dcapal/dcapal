import localforage from "localforage";
import axios from "axios";
import { setupCache } from "axios-cache-adapter";

const configure = () => {
  // Create `localforage` instance
  const forageStore = localforage.createInstance({
    // List of drivers used
    driver: [localforage.INDEXEDDB, localforage.LOCALSTORAGE],
    // Prefix all storage keys to prevent conflicts
    name: "dca-pal",
  });

  // Create `axios` instance with pre-configured `axios-cache-adapter` using a `localforage` store
  return setupCache({
    readHeaders: false,
    exclude: {
      query: false,
    },
    maxAge: 15 * 60 * 1000,
    store: forageStore, // Pass `localforage` store to `axios-cache-adapter`
    ignoreCache: true,
  });
};

const cache = configure();

// Create `axios` instance passing the newly created `cache.adapter`
export const api = axios.create({ adapter: cache.adapter });
