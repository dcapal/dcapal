import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import type * as React from "react";

/** Combines conditional layout classes while keeping utility conflicts predictable. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** A render prop accepted by Base UI primitives for composition. */
export type RenderProp =
  React.ReactElement | ((props: Record<string, unknown>) => React.ReactElement);
