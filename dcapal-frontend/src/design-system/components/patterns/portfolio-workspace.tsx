import * as React from "react";
import {
  ArrowLeft,
  Compass,
  Home,
  Menu,
  Repeat2,
  Settings2,
  X,
} from "lucide-react";
import { IconButton, Sheet, SheetClose, SheetContent, SheetTitle } from "../ui";
import { TopNavigation } from "./app-shell";
import { PortfolioFooter, type PortfolioFooterTone } from "./portfolio-footer";
import {
  PortfolioWorkspaceSidebar,
  type PortfolioWorkspaceNavGroup,
  type PortfolioWorkspaceNavItem,
} from "./portfolio-workspace-sidebar";

export type {
  PortfolioWorkspaceNavGroup,
  PortfolioWorkspaceNavItem,
} from "./portfolio-workspace-sidebar";

export interface PortfolioWorkspaceProps {
  portfolioName: string;
  asOf?: string;
  backHref?: string;
  onBack?: () => void;
  portfolioOptions?: ReadonlyArray<{ label: string; value: string }>;
  onPortfolioChange?: (value: string) => void;
  navGroups?: ReadonlyArray<PortfolioWorkspaceNavGroup>;
  selectedItem?: string;
  /** Return false when the link should follow its href instead of being handled locally. */
  onSelectItem?: (item: PortfolioWorkspaceNavItem) => boolean | void;
  mobileNavigationOpen?: boolean;
  onMobileNavigationOpenChange?: (open: boolean) => void;
  children: React.ReactNode;
  footer?: boolean;
  footerTone?: PortfolioFooterTone;
}

const defaultGroups: PortfolioWorkspaceNavGroup[] = [
  {
    label: "Portfolio",
    items: [
      {
        id: "overview",
        label: "Overview",
        icon: <Home aria-hidden />,
        href: "/allocate?section=overview",
      },
    ],
  },
  {
    label: "Actions",
    items: [
      {
        id: "allocate",
        label: "Allocate",
        icon: <Compass aria-hidden />,
        href: "/allocate?section=allocate",
      },
      {
        id: "rebalance",
        label: "Rebalance",
        icon: <Repeat2 aria-hidden />,
        href: "/allocate?section=rebalance",
      },
    ],
  },
  {
    label: "Planning",
    items: [
      {
        id: "guidance",
        label: "Strategic guidance",
        icon: <Compass aria-hidden />,
        href: "/allocate?section=guidance",
      },
    ],
  },
  {
    label: "Settings",
    items: [
      {
        id: "settings",
        label: "Portfolio settings",
        icon: <Settings2 aria-hidden />,
        href: "/allocate?section=settings",
      },
    ],
  },
];

/** Portfolio workspace shell with desktop rail and a mobile navigation sheet. */
export function PortfolioWorkspace({
  portfolioName,
  asOf = "As of 2 Aug 2026",
  backHref = "/allocate",
  onBack,
  portfolioOptions = [],
  onPortfolioChange,
  navGroups = defaultGroups,
  selectedItem = "overview",
  onSelectItem,
  mobileNavigationOpen,
  onMobileNavigationOpenChange,
  children,
  footer = true,
  footerTone = "light",
}: PortfolioWorkspaceProps) {
  const [uncontrolledNavigationOpen, setUncontrolledNavigationOpen] =
    React.useState(false);
  const navigationOpen = mobileNavigationOpen ?? uncontrolledNavigationOpen;
  const setNavigationOpen = (open: boolean) => {
    setUncontrolledNavigationOpen(open);
    onMobileNavigationOpenChange?.(open);
  };

  return (
    <div className="dcapal-theme ds-portfolio-workspace">
      <TopNavigation
        showPortfolioSelector={portfolioOptions.length > 0}
        portfolioName={portfolioName}
        portfolioOptions={portfolioOptions}
        onPortfolioChange={onPortfolioChange}
        showMenu={false}
      />
      <div className="ds-portfolio-workspace__body">
        <PortfolioWorkspaceSidebar
          portfolioName={portfolioName}
          asOf={asOf}
          groups={navGroups}
          selectedItem={selectedItem}
          onSelectItem={onSelectItem}
        />
        <main className="ds-portfolio-workspace__main">
          <a
            className="ds-portfolio-workspace__back"
            href={backHref}
            onClick={(event) => {
              if (onBack) {
                event.preventDefault();
                onBack();
              }
            }}
          >
            <ArrowLeft aria-hidden />
            My portfolios
          </a>
          {children}
        </main>
      </div>
      {footer ? <PortfolioFooter tone={footerTone} /> : null}
      <IconButton
        className="ds-portfolio-workspace__navigation-fab"
        label="Open portfolio navigation"
        variant="ghost"
        aria-expanded={navigationOpen}
        onClick={() => setNavigationOpen(true)}
      >
        <Menu aria-hidden />
      </IconButton>
      <Sheet open={navigationOpen} onOpenChange={setNavigationOpen}>
        <SheetContent className="ds-workspace-nav-sheet">
          <div className="ds-workspace-nav-sheet__header">
            <SheetTitle>Portfolio navigation</SheetTitle>
            <SheetClose
              render={
                <IconButton label="Close portfolio navigation" variant="ghost">
                  <X aria-hidden />
                </IconButton>
              }
            />
          </div>
          <PortfolioWorkspaceSidebar
            portfolioName={portfolioName}
            asOf={asOf}
            groups={navGroups}
            selectedItem={selectedItem}
            onSelectItem={onSelectItem}
            onNavigate={() => setNavigationOpen(false)}
            mode="drawer"
          />
        </SheetContent>
      </Sheet>
    </div>
  );
}
