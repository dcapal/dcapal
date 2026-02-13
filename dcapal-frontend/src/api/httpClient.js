import axios from "axios";
import { DCAPAL_API } from "@app/config";

export const api = axios.create({
  baseURL: DCAPAL_API,
});
