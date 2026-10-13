import * as React from "react";
import { ChevronRight } from "lucide-react";
import { cn } from "../../lib/cn";

export interface PortfolioWorkspaceNavItem {
  id: string;
  label: string;
  icon?: React.ReactNode;
  href?: string;
}

export interface PortfolioWorkspaceNavGroup {
  label: string;
  items: ReadonlyArray<PortfolioWorkspaceNavItem>;
}

export interface PortfolioWorkspaceSidebarProps {
  portfolioName: string;
  asOf?: string;
  groups: ReadonlyArray<PortfolioWorkspaceNavGroup>;
  selectedItem: string;
  /** Return false when the link should follow its href instead of being handled locally. */
  onSelectItem?: (item: PortfolioWorkspaceNavItem) => boolean | void;
  onNavigate?: () => void;
  mode?: "rail" | "drawer";
  className?: string;
}

function WorkspaceNavigation({
  groups,
  selectedItem,
  onSelectItem,
  onNavigate,
  mobile,
}: {
  groups: ReadonlyArray<PortfolioWorkspaceNavGroup>;
  selectedItem: string;
  onSelectItem?: (item: PortfolioWorkspaceNavItem) => boolean | void;
  onNavigate?: () => void;
  mobile: boolean;
}) {
  return (
    <nav
      className={cn("ds-workspace-nav", mobile && "ds-workspace-nav--mobile")}
      aria-label="Portfolio navigation"
    >
      {groups.map((group, index) => (
        <div className="ds-workspace-nav__group" key={group.label}>
          <h2>{group.label}</h2>
          <div className="ds-workspace-nav__items">
            {group.items.map((item) => {
              const content = (
                <>
                  {item.icon}
                  <span>{item.label}</span>
                  {mobile ? <ChevronRight aria-hidden /> : null}
                </>
              );
              const activate = (event: React.MouseEvent<HTMLElement>) => {
                const handled = onSelectItem?.(item);
                if (onSelectItem && handled !== false) event.preventDefault();
                onNavigate?.();
              };

              return item.href ? (
                <a
                  className="ds-workspace-nav__item"
                  data-active={selectedItem === item.id ? "true" : undefined}
                  aria-current={selectedItem === item.id ? "page" : undefined}
                  href={item.href}
                  key={item.id}
                  onClick={activate}
                >
                  {content}
                </a>
              ) : (
                <button
                  type="button"
                  className="ds-workspace-nav__item"
                  data-active={selectedItem === item.id ? "true" : undefined}
                  aria-pressed={selectedItem === item.id}
                  key={item.id}
                  onClick={activate}
                >
                  {content}
                </button>
              );
            })}
          </div>
          {index < groups.length - 1 ? (
            <div className="ds-workspace-nav__separator" />
          ) : null}
        </div>
      ))}
    </nav>
  );
}

/** Canonical portfolio navigation for the desktop rail and mobile drawer. */
export function PortfolioWorkspaceSidebar({
  portfolioName,
  asOf = "As of 2 Aug 2026",
  groups,
  selectedItem,
  onSelectItem,
  onNavigate,
  mode = "rail",
  className,
}: PortfolioWorkspaceSidebarProps) {
  const mobile = mode === "drawer";

  return (
    <aside
      className={cn(
        "ds-portfolio-workspace-sidebar",
        `ds-portfolio-workspace-sidebar--${mode}`,
        className
      )}
    >
      <div className="ds-portfolio-workspace-sidebar__heading">
        <strong>{portfolioName}</strong>
        <span>{asOf}</span>
      </div>
      <WorkspaceNavigation
        groups={groups}
        selectedItem={selectedItem}
        onSelectItem={onSelectItem}
        onNavigate={onNavigate}
        mobile={mobile}
      />
    </aside>
  );
}
