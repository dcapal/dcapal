import * as React from "react";

const mobileViewportQuery = "(max-width: 47.99rem)";

function getSnapshot() {
  return (
    typeof window !== "undefined" &&
    window.matchMedia(mobileViewportQuery).matches
  );
}

function subscribe(callback: () => void) {
  if (typeof window === "undefined") return () => undefined;
  const mediaQuery = window.matchMedia(mobileViewportQuery);
  if (mediaQuery.addEventListener)
    mediaQuery.addEventListener("change", callback);
  else mediaQuery.addListener(callback);
  return () => {
    if (mediaQuery.removeEventListener)
      mediaQuery.removeEventListener("change", callback);
    else mediaQuery.removeListener(callback);
  };
}

/** Whether the viewport is phone-sized, where overlays render as drawers. */
export function useNarrowViewport(): boolean {
  return React.useSyncExternalStore(subscribe, getSnapshot, () => false);
}
