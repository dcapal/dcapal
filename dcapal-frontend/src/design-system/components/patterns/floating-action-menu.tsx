import * as React from "react";
import { Plus, X } from "lucide-react";
import { Button } from "../ui";
import { cn } from "../../lib/cn";

export interface FloatingActionMenuProps {
  actions: readonly {
    label: string;
    icon?: React.ReactNode;
    onClick: () => void;
  }[];
  label?: string;
  /** Layout-only hook for responsive compositions. */
  className?: string;
}
export function FloatingActionMenu({
  actions,
  label = "Open portfolio actions",
  className,
}: FloatingActionMenuProps) {
  const [open, setOpen] = React.useState(false);
  return (
    <div className={cn("ds-fab", className)}>
      <Button
        className="ds-fab__trigger"
        variant="primary"
        size="icon"
        aria-label={open ? "Close portfolio actions" : label}
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        {open ? <X aria-hidden strokeWidth={1.8} /> : <Plus aria-hidden />}
      </Button>
      <div className="ds-fab__actions" hidden={!open}>
        {actions.map((action) => (
          <Button
            key={action.label}
            variant="outline"
            onClick={() => {
              action.onClick();
              setOpen(false);
            }}
          >
            <span>{action.label}</span>
            {action.icon ? (
              <span aria-hidden className="ds-fab__action-icon">
                {action.icon}
              </span>
            ) : null}
          </Button>
        ))}
      </div>
    </div>
  );
}
