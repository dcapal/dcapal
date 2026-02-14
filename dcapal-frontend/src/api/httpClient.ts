import axios, { type AxiosInstance } from "axios";
import { DCAPAL_API } from "@app/config";

export const api: AxiosInstance = axios.create({
  baseURL: DCAPAL_API,
});
