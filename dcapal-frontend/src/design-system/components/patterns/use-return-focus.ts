import * as React from "react";

/**
 * Remembers the element that opened an overlay so focus can return to it when
 * the overlay closes. Pass `ref` as the overlay's `finalFocus` and call
 * `remember` from the handler that opens it, while the opener still has focus.
 */
export function useReturnFocus() {
  const ref = React.useRef<HTMLElement | null>(null);
  const remember = React.useCallback(() => {
    const active = document.activeElement;
    ref.current =
      active instanceof HTMLElement && active !== document.body ? active : null;
  }, []);
  return { ref, remember };
}
