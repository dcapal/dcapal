import axios from "axios";

// Create `axios` instance passing the newly created `cache.adapter`
export const api = axios.create();

export const auth_api = axios.create();
