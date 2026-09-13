import * as React from "react";
import { Dialog as BaseDialog } from "@base-ui/react/dialog";
import { Drawer as BaseDrawer } from "@base-ui/react/drawer";
import { Menu as BaseMenu } from "@base-ui/react/menu";
import { X } from "lucide-react";
import { cn } from "../../lib/cn";
import { IconButton } from "./button";

export const Dialog = BaseDialog.Root;

export const DialogTrigger = BaseDialog.Trigger;

export interface DialogContentProps extends React.ComponentPropsWithoutRef<
  typeof BaseDialog.Popup
> {
  /** Compact dialogs use a tighter section rhythm for dense forms. */
  density?: "default" | "compact";
  /** Nested dialogs sit above the parent dialog while preserving its context. */
  layer?: "default" | "nested";
}

export const DialogContent = React.forwardRef<
  HTMLDivElement,
  DialogContentProps
>(function DialogContent(
  { className, children, density = "default", layer = "default", ...props },
  ref
) {
  return (
    <BaseDialog.Portal className="dcapal-theme">
      <BaseDialog.Backdrop
        className={cn("ds-dialog-backdrop", `ds-dialog-backdrop--${layer}`)}
      />
      <BaseDialog.Viewport
        className={cn("ds-dialog-viewport", `ds-dialog-viewport--${layer}`)}
      >
        <BaseDialog.Popup
          ref={ref}
          className={cn(
            "ds-dialog-popup",
            `ds-dialog-popup--${density}`,
            `ds-dialog-popup--${layer}`,
            className
          )}
          {...props}
        >
          {children}
        </BaseDialog.Popup>
      </BaseDialog.Viewport>
    </BaseDialog.Portal>
  );
});

export const DialogHeader = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div className={cn("ds-dialog-header", className)} {...props} />
);
export const DialogTitle = React.forwardRef<
  HTMLHeadingElement,
  React.ComponentPropsWithoutRef<typeof BaseDialog.Title>
>(function DialogTitle({ className, ...props }, ref) {
  return (
    <BaseDialog.Title
      ref={ref}
      className={cn("ds-dialog-title", className)}
      {...props}
    />
  );
});
export const DialogDescription = React.forwardRef<
  HTMLParagraphElement,
  React.ComponentPropsWithoutRef<typeof BaseDialog.Description>
>(function DialogDescription({ className, ...props }, ref) {
  return (
    <BaseDialog.Description
      ref={ref}
      className={cn("ds-dialog-description", className)}
      {...props}
    />
  );
});
export const DialogClose = BaseDialog.Close;

/** A consistent close affordance for dialog headers. */
export function DialogCloseButton({
  label = "Close dialog",
  className,
  ...props
}: Omit<React.ComponentPropsWithoutRef<typeof BaseDialog.Close>, "render"> & {
  label?: string;
  className?: string;
}) {
  return (
    <BaseDialog.Close
      {...props}
      render={
        <IconButton
          label={label}
          variant="ghost"
          className={cn("ds-dialog-header__close", className)}
        >
          <X aria-hidden strokeWidth={1.8} />
        </IconButton>
      }
    />
  );
}
export const DialogFooter = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div className={cn("ds-dialog-footer", className)} {...props} />
);

export const DropdownMenu = BaseMenu.Root;
export const DropdownMenuTrigger = BaseMenu.Trigger;
export const DropdownMenuContent = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseMenu.Popup>
>(function DropdownMenuContent({ className, ...props }, ref) {
  return (
    <BaseMenu.Portal className="dcapal-theme">
      <BaseMenu.Positioner
        className="ds-menu-positioner"
        side="bottom"
        sideOffset={4}
      >
        <BaseMenu.Popup
          ref={ref}
          className={cn("ds-menu-popup", className)}
          {...props}
        />
      </BaseMenu.Positioner>
    </BaseMenu.Portal>
  );
});
export const DropdownMenuItem = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseMenu.Item>
>(function DropdownMenuItem({ className, ...props }, ref) {
  return (
    <BaseMenu.Item
      ref={ref}
      className={cn("ds-select-item", className)}
      {...props}
    />
  );
});
export const DropdownMenuGroup = BaseMenu.Group;

export const Drawer = BaseDrawer.Root;
export const DrawerTrigger = BaseDrawer.Trigger;
export const DrawerClose = BaseDrawer.Close;
export const DrawerTitle = React.forwardRef<
  HTMLHeadingElement,
  React.ComponentPropsWithoutRef<typeof BaseDrawer.Title>
>(function DrawerTitle({ className, ...props }, ref) {
  return (
    <BaseDrawer.Title
      ref={ref}
      className={cn("ds-dialog-title", className)}
      {...props}
    />
  );
});
export const DrawerDescription = React.forwardRef<
  HTMLParagraphElement,
  React.ComponentPropsWithoutRef<typeof BaseDrawer.Description>
>(function DrawerDescription({ className, ...props }, ref) {
  return (
    <BaseDrawer.Description
      ref={ref}
      className={cn("ds-dialog-description", className)}
      {...props}
    />
  );
});
export const DrawerContent = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof BaseDrawer.Popup> & {
    /** Nested drawers sit above the parent dialog while preserving its context. */
    layer?: "default" | "nested";
  }
>(function DrawerContent(
  { className, children, layer = "default", ...props },
  ref
) {
  return (
    <BaseDrawer.Portal className="dcapal-theme">
      <BaseDrawer.Backdrop
        className={cn("ds-drawer-backdrop", `ds-drawer-backdrop--${layer}`)}
      />
      <BaseDrawer.Viewport
        className={cn("ds-drawer-viewport", `ds-drawer-viewport--${layer}`)}
      >
        <BaseDrawer.Popup
          ref={ref}
          className={cn(
            "ds-drawer-popup",
            `ds-drawer-popup--${layer}`,
            className
          )}
          {...props}
        >
          <BaseDrawer.Content>{children}</BaseDrawer.Content>
        </BaseDrawer.Popup>
      </BaseDrawer.Viewport>
    </BaseDrawer.Portal>
  );
});
export const DrawerFooter = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div className={cn("ds-dialog-footer", className)} {...props} />
);

/** Sheet is a named drawer composition so mobile overlays share one focus contract. */
export const Sheet = Drawer;
export const SheetTrigger = DrawerTrigger;
export const SheetContent = DrawerContent;
export const SheetClose = DrawerClose;
export const SheetTitle = DrawerTitle;
export const SheetDescription = DrawerDescription;
