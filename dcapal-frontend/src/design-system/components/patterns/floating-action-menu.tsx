import * as React from "react";
import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import { Plus, X } from "lucide-react";
import { cn } from "../../lib/cn";

/** One action of the current page, offered by the mobile page-actions menu. */
export interface FloatingActionMenuAction {
  /** Visible label, also the accessible name of the action. */
  label: string;
  /** Decorative icon shown in the circle after the label. */
  icon?: React.ReactNode;
  /** Emphasises the page's main action. Use it for at most one action. */
  primary?: boolean;
  /** Keeps the action visible but unavailable, for example while a form is invalid. */
  disabled?: boolean;
  onClick: () => void;
}

export interface FloatingActionMenuProps {
  /** The actions of the current page, in reading order. Navigation does not belong here. */
  actions: readonly FloatingActionMenuAction[];
  /** Accessible name of the closed button. */
  label?: string;
  /** Accessible name of the button while the menu is open. */
  closeLabel?: string;
  /** Accessible name of the open menu. */
  menuLabel?: string;
  /** Opens the menu on first render (Canvas stories). */
  defaultOpen?: boolean;
  /** Layout-only hook for responsive compositions. */
  className?: string;
}

/**
 * The page-actions menu of the phone layout: a floating "+" button that opens
 * a dimmed overlay with the actions of the current page as a stack of pills.
 * It never lists navigation, and it is hidden from 48rem up, where the page
 * header carries the same actions.
 *
 * It is a modal Base UI dialog, so Escape or a tap on the dimmed area closes
 * it, focus moves to the first action while it is open and returns to the
 * floating button afterwards. Choosing an action closes the menu first and
 * runs the action once it is gone, so an action that opens a dialog keeps the
 * focus that dialog moves. The open menu repeats the button as a close
 * button in the same spot, because the modal dialog makes the page, and with
 * it the original button, inert.
 *
 * Pages that pin a sticky action bar to the bottom supply no actions, so the
 * button and the bar never compete for the same corner.
 */
export function FloatingActionMenu({
  actions,
  label = "Open page actions",
  closeLabel = "Close page actions",
  menuLabel = "Page actions",
  defaultOpen = false,
  className,
}: FloatingActionMenuProps) {
  const [open, setOpen] = React.useState(defaultOpen);
  const triggerRef = React.useRef<HTMLButtonElement>(null);
  // The action to run after the menu has finished closing.
  const pendingAction = React.useRef<(() => void) | null>(null);

  if (actions.length === 0) return null;

  return (
    <BaseDialog.Root
      open={open}
      onOpenChange={setOpen}
      onOpenChangeComplete={(nextOpen) => {
        if (nextOpen) return;
        const action = pendingAction.current;
        pendingAction.current = null;
        action?.();
      }}
    >
      <BaseDialog.Trigger
        ref={triggerRef}
        className={cn("ds-fab", className)}
        aria-label={label}
      >
        <Plus aria-hidden />
      </BaseDialog.Trigger>
      <BaseDialog.Portal className="dcapal-theme">
        <BaseDialog.Backdrop className="ds-fab__scrim" />
        <BaseDialog.Popup
          className="ds-fab__popup"
          aria-label={menuLabel}
          finalFocus={() =>
            // A chosen action opens its own surface and owns the focus.
            pendingAction.current ? false : (triggerRef.current ?? true)
          }
        >
          <div className="ds-fab__actions">
            {actions.map((action) => (
              <button
                key={action.label}
                type="button"
                className="ds-fab__action"
                data-primary={action.primary || undefined}
                disabled={action.disabled}
                onClick={() => {
                  pendingAction.current = action.onClick;
                  setOpen(false);
                }}
              >
                <span>{action.label}</span>
                {action.icon ? (
                  <span aria-hidden className="ds-fab__action-icon">
                    {action.icon}
                  </span>
                ) : null}
              </button>
            ))}
          </div>
          <BaseDialog.Close className="ds-fab__close" aria-label={closeLabel}>
            <X aria-hidden strokeWidth={1.8} />
          </BaseDialog.Close>
        </BaseDialog.Popup>
      </BaseDialog.Portal>
    </BaseDialog.Root>
  );
}
