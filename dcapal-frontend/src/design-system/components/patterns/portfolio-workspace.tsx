import * as React from "react";
import { ArrowLeft, Compass, Home, Settings2, TrendingUp } from "lucide-react";
import { TopNavigation } from "./app-shell";
import {
  FloatingActionMenu,
  type FloatingActionMenuAction,
} from "./floating-action-menu";
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
  /**
   * The actions of the current page, offered on phones through the floating
   * "Open page actions" button. Leave it empty on pages that pin a sticky
   * action bar. Navigation never goes here.
   */
  pageActions?: ReadonlyArray<FloatingActionMenuAction>;
  /** Opens the page-actions menu on first render (Canvas stories). */
  pageActionsDefaultOpen?: boolean;
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
        id: "invest",
        label: "Invest",
        icon: <TrendingUp aria-hidden />,
        href: "/invest",
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

/**
 * Portfolio workspace shell. Desktop shows the navigation rail; phones open
 * the same navigation from the top bar's menu button and reach the current
 * page's actions through the floating page-actions menu.
 */
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
  pageActions = [],
  pageActionsDefaultOpen,
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
        showMenu
        menuOpen={navigationOpen}
        onMenuOpenChange={setNavigationOpen}
        menuTitle="Portfolio navigation"
        menuButtonClassName="ds-portfolio-workspace__menu"
        mobileMenuContent={
          <PortfolioWorkspaceSidebar
            portfolioName={portfolioName}
            asOf={asOf}
            groups={navGroups}
            selectedItem={selectedItem}
            onSelectItem={onSelectItem}
            onNavigate={() => setNavigationOpen(false)}
            mode="drawer"
          />
        }
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
      <FloatingActionMenu
        actions={pageActions}
        defaultOpen={pageActionsDefaultOpen}
      />
    </div>
  );
}
