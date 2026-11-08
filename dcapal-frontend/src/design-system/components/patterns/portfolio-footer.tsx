import * as React from "react";
import { cn } from "../../lib/cn";

export interface PortfolioFooterGroup {
  label: string;
  links: ReadonlyArray<{ label: string; href: string }>;
}

export type PortfolioFooterTone = "dark" | "light" | "warm";

export interface PortfolioFooterProps {
  groups?: ReadonlyArray<PortfolioFooterGroup>;
  tagline?: string;
  copyright?: string;
  /** Selects the semantic surface tone used by the shared sitemap footer. */
  tone?: PortfolioFooterTone;
  className?: string;
}

const defaultGroups: PortfolioFooterGroup[] = [
  {
    label: "Product",
    links: [
      { label: "My portfolios", href: "/allocate" },
      { label: "Recurring investments", href: "/recurring-investments" },
      { label: "Portfolio settings", href: "/portfolio-settings" },
    ],
  },
  {
    label: "Learn",
    links: [
      { label: "Investing basics", href: "/docs/investing-basics" },
      { label: "DCA strategy", href: "/docs/dca-strategy" },
      { label: "Guides", href: "/docs/guides" },
      { label: "Help centre", href: "/docs/help" },
    ],
  },
  {
    label: "Company",
    links: [
      { label: "About", href: "/about" },
      { label: "Careers", href: "/careers" },
      { label: "Contact", href: "/contact" },
    ],
  },
  {
    label: "Legal",
    links: [
      { label: "Terms of service", href: "/terms" },
      { label: "Privacy policy", href: "/privacy" },
      { label: "Cookie preferences", href: "/cookies" },
    ],
  },
];

/** Sitemap-style footer shared by the global index and portfolio workspace. */
export function PortfolioFooter({
  groups = defaultGroups,
  tagline = "Invest regularly. A calmer tomorrow.",
  copyright = "© 2026 DcaPal. All rights reserved.",
  tone = "light",
  className,
}: PortfolioFooterProps) {
  return (
    <footer
      className={cn(
        "ds-portfolio-footer",
        `ds-portfolio-footer--${tone}`,
        className
      )}
    >
      <div className="ds-portfolio-footer__brand">
        <strong>DcaPal</strong>
        <p>{tagline}</p>
      </div>
      <div className="ds-portfolio-footer__groups">
        {groups.map((group) => (
          <section key={group.label} aria-labelledby={`footer-${group.label}`}>
            <h2 id={`footer-${group.label}`}>{group.label}</h2>
            <ul>
              {group.links.map((link) => (
                <li key={link.label}>
                  <a href={link.href}>{link.label}</a>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
      <p className="ds-portfolio-footer__copyright">{copyright}</p>
    </footer>
  );
}
