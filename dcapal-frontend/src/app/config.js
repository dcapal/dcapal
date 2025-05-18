import { createClient } from "@supabase/supabase-js";

export const DCAPAL_CANONICAL = "https://dcapal.com";

export const DCAPAL_API = "/api";

export const MEDIA_SMALL = "only screen and (min-width: 640px)";
export const MEDIA_MEDIUM = "only screen and (min-width: 768px)";

export const DEMO_PF_60_40 = "60-40";
export const DEMO_PF_ALL_SEASONS = "all-seasons";
export const DEMO_PF_MR_RIP = "mr-rip";
export const DEMO_PF_HODLX = "hodlx";

export const REFRESH_PRICE_INTERVAL_SEC = 5 * 60;

export const supabase = createClient(
  process.env.VITE_SUPABASE_URL,
  process.env.VITE_SUPABASE_ANON_KEY
);
