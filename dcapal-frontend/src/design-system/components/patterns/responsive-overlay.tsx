import * as React from "react";
import { X } from "lucide-react";
import {
  Dialog,
  DialogCloseButton,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerTitle,
  IconButton,
} from "../ui";
import { cn } from "../../lib/cn";
import { useNarrowViewport } from "./use-narrow-viewport";

export interface ResponsiveOverlayProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Heading content; an adornment such as a class dot may precede the text. */
  title: React.ReactNode;
  description?: React.ReactNode;
  /** Small label above the title, such as the journey step. */
  eyebrow?: React.ReactNode;
  /** Names the close button: `Close {closeLabel}`. */
  closeLabel: string;
  /** Width class for the desktop dialog. */
  className?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  /** Forwarded to the popup so callers can restore focus on close. */
  finalFocus?: React.ComponentPropsWithoutRef<
    typeof DialogContent
  >["finalFocus"];
}

/**
 * A modal that is a Base UI dialog from 48rem and a Base UI drawer below it,
 * with the same title, description, body and footer in both.
 */
export function ResponsiveOverlay({
  open,
  onOpenChange,
  title,
  description,
  eyebrow,
  closeLabel,
  className,
  children,
  footer,
  finalFocus,
}: ResponsiveOverlayProps) {
  const isMobile = useNarrowViewport();
  if (isMobile) {
    return (
      <Drawer open={open} onOpenChange={onOpenChange}>
        <DrawerContent
          className={cn("ds-edit-asset-sheet ds-responsive-sheet", className)}
          finalFocus={finalFocus}
        >
          <div className="ds-edit-asset-sheet__header">
            <div>
              {eyebrow ? <span className="ds-eyebrow">{eyebrow}</span> : null}
              <DrawerTitle>{title}</DrawerTitle>
              {description ? (
                <DrawerDescription>{description}</DrawerDescription>
              ) : null}
            </div>
            <DrawerClose
              render={
                <IconButton label={`Close ${closeLabel}`} variant="ghost">
                  <X aria-hidden />
                </IconButton>
              }
            />
          </div>
          {children}
          {footer ? (
            <DrawerFooter className="ds-dialog-footer--stacked-mobile">
              {footer}
            </DrawerFooter>
          ) : null}
        </DrawerContent>
      </Drawer>
    );
  }
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className={cn("ds-responsive-dialog", className)}
        finalFocus={finalFocus}
      >
        <DialogHeader>
          <div>
            {eyebrow ? <span className="ds-eyebrow">{eyebrow}</span> : null}
            <DialogTitle>{title}</DialogTitle>
            {description ? (
              <DialogDescription>{description}</DialogDescription>
            ) : null}
          </div>
          <DialogCloseButton label={`Close ${closeLabel}`} />
        </DialogHeader>
        {children}
        {footer ? (
          <DialogFooter className="ds-dialog-footer--stacked-mobile">
            {footer}
          </DialogFooter>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
