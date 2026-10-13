import * as React from "react";
import { ChevronDown, Menu, X } from "lucide-react";
import {
  Avatar,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  IconButton,
  Sheet,
  SheetClose,
  SheetContent,
  SheetTitle,
} from "../ui";
import { cn } from "../../lib/cn";

export interface TopNavigationProps {
  brand?: string;
  showPortfolioSelector?: boolean;
  portfolioName?: string;
  portfolioOptions?: ReadonlyArray<{ label: string; value: string }>;
  onPortfolioChange?: (value: string) => void;
  links?: ReadonlyArray<{ label: string; href: string; active?: boolean }>;
  userName?: string;
  onMenuClick?: () => void;
  mobileMenuContent?: React.ReactNode;
  showMenu?: boolean;
}

export function TopNavigation({
  brand = "DcaPal",
  showPortfolioSelector,
  portfolioName = "Core portfolio",
  portfolioOptions = [],
  onPortfolioChange,
  links = [],
  userName = "Leonardo Arcari",
  onMenuClick,
  mobileMenuContent,
  showMenu = true,
}: TopNavigationProps) {
  const [menuOpen, setMenuOpen] = React.useState(false);
  const shouldShowPortfolioSelector =
    showPortfolioSelector ?? portfolioOptions.length > 0;
  const initials = userName
    .split(" ")
    .map((part) => part[0])
    .join("");

  const portfolioTrigger = (
    <button
      type="button"
      className="ds-top-nav__portfolio"
      aria-label={`Select portfolio, current portfolio ${portfolioName}`}
    >
      <span>{portfolioName}</span>
      <ChevronDown aria-hidden strokeWidth={1.8} />
    </button>
  );

  const openMenu = () => {
    onMenuClick?.();
    setMenuOpen(true);
  };

  return (
    <header className="ds-top-nav">
      <a className="ds-top-nav__brand" href="/">
        {brand}
      </a>
      {shouldShowPortfolioSelector && portfolioOptions.length > 0 ? (
        <DropdownMenu>
          <DropdownMenuTrigger render={portfolioTrigger} />
          <DropdownMenuContent>
            {portfolioOptions.map((option) => (
              <DropdownMenuItem
                key={option.value}
                onClick={() => onPortfolioChange?.(option.value)}
              >
                {option.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      ) : shouldShowPortfolioSelector ? (
        portfolioTrigger
      ) : null}
      <nav aria-label="Primary navigation" className="ds-top-nav__links">
        {links.map((link) => (
          <a
            key={link.href}
            className="ds-top-nav__link"
            data-active={link.active || undefined}
            href={link.href}
          >
            {link.label}
          </a>
        ))}
      </nav>
      <span className="ds-top-nav__user">
        <Avatar fallback={initials} size="sm" />
        <span className="ds-top-nav__user-name">{userName}</span>
      </span>
      {showMenu ? (
        <>
          <IconButton
            label={menuOpen ? "Close navigation menu" : "Open navigation menu"}
            variant="ghost"
            className="ds-top-nav__menu"
            aria-expanded={menuOpen}
            onClick={() => (menuOpen ? setMenuOpen(false) : openMenu())}
          >
            {menuOpen ? (
              <X aria-hidden strokeWidth={1.8} />
            ) : (
              <Menu aria-hidden strokeWidth={1.8} />
            )}
          </IconButton>
          <Sheet open={menuOpen} onOpenChange={setMenuOpen}>
            <SheetContent className="ds-top-nav__drawer">
              <div className="ds-top-nav__drawer-header">
                <SheetTitle>Navigation</SheetTitle>
                <SheetClose
                  render={
                    <IconButton label="Close navigation menu" variant="ghost">
                      <X aria-hidden strokeWidth={1.8} />
                    </IconButton>
                  }
                />
              </div>
              <nav
                aria-label="Mobile navigation"
                className="ds-top-nav__drawer-links"
              >
                {mobileMenuContent ??
                  links.map((link) => (
                    <a
                      key={link.href}
                      className="ds-top-nav__drawer-link"
                      data-active={link.active || undefined}
                      href={link.href}
                      onClick={() => setMenuOpen(false)}
                    >
                      {link.label}
                    </a>
                  ))}
              </nav>
            </SheetContent>
          </Sheet>
        </>
      ) : null}
    </header>
  );
}

export interface AppShellProps extends React.HTMLAttributes<HTMLDivElement> {
  navigation?: React.ReactNode;
  footer?: React.ReactNode;
}

export function AppShell({
  children,
  navigation,
  footer,
  className,
  ...props
}: AppShellProps) {
  return (
    <div className={cn("dcapal-theme ds-app-shell", className)} {...props}>
      {navigation ?? <TopNavigation />}
      <main className="ds-app-shell__main">{children}</main>
      {footer}
    </div>
  );
}
