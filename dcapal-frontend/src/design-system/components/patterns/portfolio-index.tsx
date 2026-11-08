import * as React from "react";
import { FileText, MoreHorizontal, Plus, Sprout, Upload } from "lucide-react";
import { Button, Card, CardContent, IconButton } from "../ui";
import { PageHeader } from "./page-header";
import { cn } from "../../lib/cn";
import { portfolioSetupSteps } from "./portfolio-setup";
import { PortfolioIcon, type PortfolioIconId } from "../ui/portfolio-icon";
import { PortfolioIconPicker } from "./portfolio-icon-picker";

export interface PortfolioSummary {
  id: string;
  href?: string;
  name: string;
  description: string;
  currentValue: string;
  dailyChange: string;
  dailyChangePercent: string;
  totalReturn: string;
  totalReturnPercent: string;
  allocationLabel: string;
  allocationDetail: string;
  lastUpdated: string;
  icon?: PortfolioIconId;
}

export interface PortfolioSummaryCardProps {
  portfolio: PortfolioSummary;
  onSelect?: (portfolio: PortfolioSummary) => void;
  onMenu?: (portfolio: PortfolioSummary) => void;
  onIconChange?: (portfolio: PortfolioSummary, icon: PortfolioIconId) => void;
}

/** A selectable portfolio summary that works as the global index entry point. */
export function PortfolioSummaryCard({
  portfolio,
  onSelect,
  onMenu,
  onIconChange,
}: PortfolioSummaryCardProps) {
  const activate = () => onSelect?.(portfolio);

  return (
    <Card className="ds-portfolio-summary-card">
      <CardContent className="ds-portfolio-summary-card__content">
        <a
          className="ds-portfolio-summary-card__surface"
          href={portfolio.href ?? `/allocate?portfolio=${portfolio.id}`}
          aria-label={`Open ${portfolio.name}`}
          onClick={activate}
        >
          <div className="ds-portfolio-summary-card__header">
            <div className="ds-portfolio-summary-card__identity">
              <span className="ds-portfolio-summary-card__icon" aria-hidden>
                <PortfolioIcon id={portfolio.icon} strokeWidth={1.8} />
              </span>
              <div>
                <h2>{portfolio.name}</h2>
                <p>{portfolio.description}</p>
              </div>
            </div>
          </div>
          <div className="ds-portfolio-summary-card__metrics">
            <div>
              <span>Current value</span>
              <strong>{portfolio.currentValue}</strong>
              <em>
                {portfolio.dailyChange} ({portfolio.dailyChangePercent})
              </em>
              <small>Since previous close</small>
            </div>
            <div>
              <span>Total return</span>
              <strong className="ds-portfolio-summary-card__positive">
                {portfolio.totalReturn}
              </strong>
              <em className="ds-portfolio-summary-card__positive">
                ({portfolio.totalReturnPercent})
              </em>
              <small>Since inception</small>
            </div>
          </div>
          <div className="ds-portfolio-summary-card__footer">
            <span>
              Last updated <strong>{portfolio.lastUpdated}</strong>
            </span>
            <span>
              Allocation <strong>{portfolio.allocationLabel}</strong>
              <small>{portfolio.allocationDetail}</small>
            </span>
          </div>
        </a>
      </CardContent>
      {onIconChange ? (
        <PortfolioIconPicker
          value={portfolio.icon}
          onValueChange={(icon) => onIconChange(portfolio, icon)}
          label={`Customize icon for ${portfolio.name}`}
          className="ds-portfolio-summary-card__menu"
        />
      ) : onMenu ? (
        <IconButton
          label={`More actions for ${portfolio.name}`}
          variant="ghost"
          size="sm"
          className="ds-portfolio-summary-card__menu"
          onClick={() => onMenu(portfolio)}
        >
          <MoreHorizontal aria-hidden />
        </IconButton>
      ) : null}
    </Card>
  );
}

export interface PortfolioIndexProps {
  portfolios?: ReadonlyArray<PortfolioSummary>;
  onSelectPortfolio?: (portfolio: PortfolioSummary) => void;
  onCreatePortfolio?: () => void;
  onImportPortfolio?: () => void;
  onPortfolioMenu?: (portfolio: PortfolioSummary) => void;
  onPortfolioIconChange?: (
    portfolio: PortfolioSummary,
    icon: PortfolioIconId
  ) => void;
}

/** Decorative plant-and-document illustration for the first portfolio state. */
function EmptyPortfolioIllustration() {
  return (
    <span
      aria-hidden="true"
      style={{
        position: "relative",
        display: "grid",
        width: "var(--dcapal-avatar-size-lg)",
        height: "var(--dcapal-avatar-size-lg)",
        placeItems: "center",
        borderRadius: "var(--dcapal-radius-round)",
        background: "var(--dcapal-primary-soft)",
        color: "var(--dcapal-primary)",
      }}
    >
      <FileText size={28} strokeWidth={1.6} />
      <span
        style={{
          position: "absolute",
          right: "var(--dcapal-space-1)",
          bottom: "var(--dcapal-space-1)",
          display: "grid",
          width: "var(--dcapal-avatar-size-sm)",
          height: "var(--dcapal-avatar-size-sm)",
          placeItems: "center",
          borderRadius: "var(--dcapal-radius-round)",
          background: "var(--dcapal-surface)",
        }}
      >
        <Sprout size={16} strokeWidth={1.8} />
      </span>
    </span>
  );
}

/** Global portfolio index with the calm first-use state and populated cards. */
export function PortfolioIndex({
  portfolios = [],
  onSelectPortfolio,
  onCreatePortfolio,
  onImportPortfolio,
  onPortfolioMenu,
  onPortfolioIconChange,
}: PortfolioIndexProps) {
  const isEmpty = portfolios.length === 0;
  const portfolioActions = (
    <div className="ds-portfolio-index__actions">
      <Button
        variant={isEmpty ? "primary" : "outline"}
        onClick={onCreatePortfolio}
      >
        <Plus aria-hidden />
        {isEmpty ? "Create your first portfolio" : "Create portfolio"}
      </Button>
      <Button variant="outline" onClick={onImportPortfolio}>
        <Upload aria-hidden />
        Import portfolio
      </Button>
    </div>
  );

  return (
    <div
      className={cn(
        "ds-portfolio-index",
        isEmpty && "ds-portfolio-index--empty"
      )}
    >
      <PageHeader
        title="My portfolios"
        description={
          isEmpty ? (
            <>
              <span className="ds-portfolio-index__empty-description ds-portfolio-index__empty-description--desktop">
                Start building a smarter, long-term investment plan.
              </span>
              <span className="ds-portfolio-index__empty-description ds-portfolio-index__empty-description--mobile">
                Create your first portfolio to get started.
              </span>
            </>
          ) : (
            `${portfolios.length} portfolio${portfolios.length === 1 ? "" : "s"}`
          )
        }
        actions={isEmpty ? undefined : portfolioActions}
      />

      {isEmpty ? (
        <section
          className="ds-portfolio-index__welcome"
          aria-labelledby="portfolio-welcome-title"
        >
          <Card variant="ghost">
            <CardContent>
              <EmptyPortfolioIllustration />
              <h2 id="portfolio-welcome-title">
                You don’t have any portfolios yet
              </h2>
              <p>
                Create a portfolio to start tracking your investments, set
                target weights and stay on track with your goals.
              </p>
              <div className="ds-portfolio-index__welcome-actions">
                <Button onClick={onCreatePortfolio}>
                  Create your first portfolio
                </Button>
                <Button variant="outline" onClick={onImportPortfolio}>
                  Import portfolio
                </Button>
              </div>
            </CardContent>
          </Card>
          <div className="ds-portfolio-index__how-it-works">
            <div>
              <h2>How it works</h2>
              <p>Set up your portfolio in a few simple steps.</p>
            </div>
            <div className="ds-portfolio-index__steps">
              {portfolioSetupSteps.map((step, index) => (
                <Card key={step.title} density="compact" variant="ghost">
                  <CardContent>
                    <span className="ds-portfolio-index__step-icon" aria-hidden>
                      {step.icon}
                    </span>
                    <span className="ds-portfolio-index__step-number">
                      {index + 1}
                    </span>
                    <div>
                      <h3>{step.title}</h3>
                      <p>{step.description}</p>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>
        </section>
      ) : (
        <section
          className="ds-portfolio-index__list"
          aria-label="Your portfolios"
        >
          {portfolios.map((portfolio) => (
            <PortfolioSummaryCard
              key={portfolio.id}
              portfolio={portfolio}
              onSelect={onSelectPortfolio}
              onMenu={onPortfolioMenu}
              onIconChange={onPortfolioIconChange}
            />
          ))}
        </section>
      )}
    </div>
  );
}

export const portfolioSummaryFixtures: PortfolioSummary[] = [
  {
    id: "core",
    href: "/allocate?portfolio=core",
    name: "Core portfolio",
    description: "A balanced, long-term portfolio",
    currentValue: "€124,680.75",
    dailyChange: "+€612.18",
    dailyChangePercent: "+0.49%",
    totalReturn: "+€12,480.75",
    totalReturnPercent: "+11.11%",
    allocationLabel: "On target",
    allocationDetail: "100% allocated",
    lastUpdated: "2 Aug 2026, 10:24",
    icon: "pie-chart",
  },
  {
    id: "growth",
    href: "/allocate?portfolio=growth",
    name: "Long-term growth",
    description: "Growth focused, higher risk",
    currentValue: "€52,340.12",
    dailyChange: "+€342.56",
    dailyChangePercent: "+0.69%",
    totalReturn: "+€8,912.40",
    totalReturnPercent: "+20.51%",
    allocationLabel: "On target",
    allocationDetail: "100% allocated",
    lastUpdated: "2 Aug 2026, 09:58",
    icon: "trending-up",
  },
  {
    id: "emergency",
    href: "/allocate?portfolio=emergency",
    name: "Emergency fund",
    description: "Safety first, short-term needs",
    currentValue: "€15,250.00",
    dailyChange: "+€12.50",
    dailyChangePercent: "+0.08%",
    totalReturn: "+€250.00",
    totalReturnPercent: "+1.67%",
    allocationLabel: "On target",
    allocationDetail: "100% allocated",
    lastUpdated: "1 Aug 2026, 16:20",
    icon: "shield",
  },
];
