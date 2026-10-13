import * as React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Button, type PortfolioIconId } from "../../components/ui";
import {
  AppShell,
  DriftDiagnostic,
  MetricCard,
  PageHeader,
  PerformanceChart,
  PortfolioAssetCollection,
  PortfolioFooter,
  PortfolioIndex,
  PortfolioSetupPage,
  PortfolioWorkspace,
  portfolioSummaryFixtures,
  TopNavigation,
} from "../../components/patterns";
import { portfolioAssets } from "../../fixtures/portfolio";
import { PortfolioIconPicker } from "../../components/patterns/portfolio-icon-picker";

const workspacePortfolioFixtures = {
  "Core portfolio": {
    currentValue: "€124,680.75",
    dayChange: "+€612.18 (+0.49%)",
    assets: portfolioAssets.slice(0, 4),
  },
  "Long-term growth": {
    currentValue: "€52,340.12",
    dayChange: "+€342.56 (+0.69%)",
    assets: portfolioAssets.slice(0, 3),
  },
  "Emergency fund": {
    currentValue: "€15,250.00",
    dayChange: "+€12.50 (+0.08%)",
    assets: portfolioAssets.slice(4),
  },
} as const;

const meta = {
  title: "Patterns / Portfolio",
  component: AppShell,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: { story: { inline: false, iframeHeight: 1000 } },
  },
} satisfies Meta<typeof AppShell>;
export default meta;
type Story = StoryObj<typeof meta>;

const portfolioOptions = [
  { label: "Core portfolio", value: "core" },
  { label: "Long-term growth", value: "growth" },
  { label: "Emergency fund", value: "emergency" },
] as const;

function FloatingActionsNavigationPreview() {
  const [portfolioId, setPortfolioId] = React.useState("core");
  const portfolioOption =
    portfolioOptions.find(({ value }) => value === portfolioId) ??
    portfolioOptions[0];
  const portfolio = workspacePortfolioFixtures[portfolioOption.label];

  return (
    <PortfolioWorkspace
      portfolioName={portfolioOption.label}
      portfolioOptions={portfolioOptions}
      onPortfolioChange={setPortfolioId}
      selectedItem="overview"
    >
      <PageHeader
        title={portfolioOption.label}
        description="As of 2 Aug 2026"
        actions={<Button variant="primary">Add asset</Button>}
      />
      <div className="ds-metric-grid">
        <MetricCard
          metric={{
            label: "Current value",
            value: portfolio.currentValue,
            context: "As of 2 Aug 2026",
          }}
        />
        <MetricCard
          metric={{
            label: "Day change",
            value: portfolio.dayChange,
            context: "Since previous close",
            trend: "positive",
          }}
        />
      </div>
    </PortfolioWorkspace>
  );
}

export const Shell: Story = {
  render: () => (
    <PortfolioWorkspace
      portfolioName="Core portfolio"
      portfolioOptions={portfolioOptions}
      selectedItem="overview"
    >
      <PageHeader
        title="Core portfolio"
        description="As of 2 Aug 2026"
        actions={<Button variant="outline">Portfolio settings</Button>}
      />
      <div className="ds-metric-grid">
        <MetricCard
          metric={{
            label: "Current value",
            value: "€124,680.75",
            context: "As of 2 Aug 2026",
          }}
        />
        <MetricCard
          metric={{
            label: "Day change",
            value: "+€612.18 (+0.49%)",
            context: "Since previous close",
            trend: "positive",
          }}
        />
        <MetricCard
          metric={{
            label: "Cash balance",
            value: "€3,842.15",
            context: "Available to invest",
          }}
        />
      </div>
    </PortfolioWorkspace>
  ),
};

export const ShellMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  render: () => (
    <PortfolioWorkspace
      portfolioName="Core portfolio"
      portfolioOptions={portfolioOptions}
      selectedItem="overview"
    >
      <PageHeader title="Core portfolio" description="As of 2 Aug 2026" />
      <div className="ds-metric-grid">
        <MetricCard metric={{ label: "Current value", value: "€124,680.75" }} />
        <MetricCard
          metric={{
            label: "Day change",
            value: "+€612.18 (+0.49%)",
            trend: "positive",
          }}
        />
      </div>
    </PortfolioWorkspace>
  ),
  tags: ["!autodocs"],
};

export const MyPortfoliosEmpty: Story = {
  render: () => (
    <AppShell
      navigation={
        <TopNavigation
          showPortfolioSelector={false}
          links={[
            { label: "My portfolios", href: "/allocate", active: true },
            { label: "Documentation", href: "/docs" },
            { label: "About", href: "/about" },
          ]}
        />
      }
      footer={<PortfolioFooter />}
    >
      <PortfolioIndex
        onCreatePortfolio={() => undefined}
        onImportPortfolio={() => undefined}
      />
    </AppShell>
  ),
};

export const MyPortfoliosEmptyMobile: Story = {
  parameters: {
    viewport: { defaultViewport: "mobile" },
    docs: { story: { inline: false, iframeHeight: 1200 } },
  },
  render: () => (
    <AppShell
      navigation={
        <TopNavigation showPortfolioSelector={false} showMenu={false} />
      }
      footer={<PortfolioFooter />}
    >
      <PortfolioIndex
        onCreatePortfolio={() => undefined}
        onImportPortfolio={() => undefined}
      />
    </AppShell>
  ),
  tags: ["!autodocs"],
};

export const MyPortfoliosPopulated: Story = {
  render: () => {
    const [portfolios, setPortfolios] = React.useState(
      portfolioSummaryFixtures
    );
    return (
      <AppShell
        navigation={
          <TopNavigation
            showPortfolioSelector={false}
            links={[
              { label: "My portfolios", href: "/allocate", active: true },
              { label: "Documentation", href: "/docs" },
              { label: "About", href: "/about" },
            ]}
          />
        }
        footer={<PortfolioFooter />}
      >
        <PortfolioIndex
          portfolios={portfolios}
          onSelectPortfolio={() => undefined}
          onCreatePortfolio={() => undefined}
          onImportPortfolio={() => undefined}
          onPortfolioIconChange={(portfolio, icon) =>
            setPortfolios((current) =>
              current.map((item) =>
                item.id === portfolio.id ? { ...item, icon } : item
              )
            )
          }
        />
      </AppShell>
    );
  },
};

export const MyPortfoliosPopulatedMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  render: () => (
    <AppShell
      navigation={
        <TopNavigation showPortfolioSelector={false} showMenu={false} />
      }
      footer={<PortfolioFooter />}
    >
      <PortfolioIndex
        portfolios={portfolioSummaryFixtures}
        onSelectPortfolio={() => undefined}
        onCreatePortfolio={() => undefined}
        onImportPortfolio={() => undefined}
        onPortfolioIconChange={() => undefined}
      />
    </AppShell>
  ),
  tags: ["!autodocs"],
};

export const PortfolioIconCustomization: Story = {
  render: () => {
    const [icon, setIcon] = React.useState<PortfolioIconId>("pie-chart");
    return (
      <div
        className="dcapal-theme"
        style={{ padding: "var(--dcapal-space-6)", minHeight: "12rem" }}
      >
        <PortfolioIconPicker value={icon} onValueChange={setIcon} />
      </div>
    );
  },
  tags: ["!autodocs"],
};

export const PortfolioSetup: Story = {
  render: () => (
    <AppShell
      navigation={
        <TopNavigation showPortfolioSelector={false} showMenu={false} />
      }
      footer={<PortfolioFooter />}
    >
      <PortfolioSetupPage />
    </AppShell>
  ),
  parameters: { docs: { story: { inline: false, iframeHeight: 1100 } } },
};

export const PortfolioSetupMobile: Story = {
  parameters: {
    viewport: { defaultViewport: "mobile" },
    docs: { story: { inline: false, iframeHeight: 1200 } },
  },
  render: () => (
    <AppShell
      navigation={
        <TopNavigation showPortfolioSelector={false} showMenu={false} />
      }
      footer={<PortfolioFooter />}
    >
      <PortfolioSetupPage />
    </AppShell>
  ),
  tags: ["!autodocs"],
};

export const PortfolioWorkspaceShell: Story = {
  render: () => {
    const [portfolio, setPortfolio] = React.useState("Core portfolio");
    const [selected, setSelected] = React.useState("overview");
    const metrics =
      workspacePortfolioFixtures[
        portfolio as keyof typeof workspacePortfolioFixtures
      ];
    return (
      <PortfolioWorkspace
        portfolioName={portfolio}
        portfolioOptions={portfolioOptions}
        onPortfolioChange={(value) =>
          setPortfolio(
            portfolioOptions.find((option) => option.value === value)?.label ??
              "Core portfolio"
          )
        }
        selectedItem={selected}
        onSelectItem={(item) => setSelected(item.id)}
      >
        <div id="overview">
          <PageHeader title={portfolio} description="As of 2 Aug 2026" />
          <div className="ds-metric-grid">
            <MetricCard
              metric={{ label: "Current value", value: metrics.currentValue }}
            />
            <MetricCard
              metric={{
                label: "Day change",
                value: metrics.dayChange,
                trend: "positive",
              }}
            />
          </div>
          <PortfolioAssetCollection
            title="Holdings"
            subtitle={null}
            assets={metrics.assets}
            presentation="overview"
          />
        </div>
      </PortfolioWorkspace>
    );
  },
};

export const PortfolioWorkspaceShellMobile: Story = {
  parameters: {
    viewport: { defaultViewport: "mobile" },
    docs: { story: { inline: false, iframeHeight: 1200 } },
  },
  render: () => {
    const [portfolio, setPortfolio] = React.useState("Core portfolio");
    const [selected, setSelected] = React.useState("overview");
    const metrics =
      workspacePortfolioFixtures[
        portfolio as keyof typeof workspacePortfolioFixtures
      ];
    return (
      <PortfolioWorkspace
        portfolioName={portfolio}
        portfolioOptions={portfolioOptions}
        onPortfolioChange={(value) =>
          setPortfolio(
            portfolioOptions.find((option) => option.value === value)?.label ??
              "Core portfolio"
          )
        }
        selectedItem={selected}
        onSelectItem={(item) => setSelected(item.id)}
      >
        <div id="overview">
          <PageHeader title={portfolio} description="As of 2 Aug 2026" />
          <div className="ds-metric-grid">
            <MetricCard
              metric={{ label: "Current value", value: metrics.currentValue }}
            />
            <MetricCard
              metric={{
                label: "Day change",
                value: metrics.dayChange,
                trend: "positive",
              }}
            />
          </div>
          <PortfolioAssetCollection
            title="Holdings"
            subtitle={null}
            assets={metrics.assets}
          />
        </div>
      </PortfolioWorkspace>
    );
  },
  tags: ["!autodocs"],
};

export const PortfolioFooterStory: Story = {
  render: () => <PortfolioFooter />,
};

export const PortfolioFooterMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  render: () => <PortfolioFooter />,
  tags: ["!autodocs"],
};

export const Drift: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <DriftDiagnostic
        assetName="Equities"
        driftPp={4.8}
        status="over"
        onReview={() => undefined}
      />
    </div>
  ),
};

export const DriftUnder: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <DriftDiagnostic assetName="Bonds" driftPp={-2.4} status="under" />
    </div>
  ),
  tags: ["!autodocs"],
};

export const DriftWithin: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <DriftDiagnostic assetName="Cash" driftPp={0.2} status="within" />
    </div>
  ),
  tags: ["!autodocs"],
};

export const DriftOver: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <DriftDiagnostic assetName="Equities" driftPp={4.8} status="over" />
    </div>
  ),
  tags: ["!autodocs"],
};

export const Assets: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PortfolioAssetCollection assets={portfolioAssets} />
    </div>
  ),
};

export const AssetsMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PortfolioAssetCollection
        assets={portfolioAssets}
        onEdit={() => undefined}
        onMenu={() => undefined}
      />
    </div>
  ),
  tags: ["!autodocs"],
};

export const AssetsDesktop: Story = {
  parameters: { viewport: { defaultViewport: "desktop" } },
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PortfolioAssetCollection
        assets={portfolioAssets}
        onEdit={() => undefined}
        onMenu={() => undefined}
      />
    </div>
  ),
  tags: ["!autodocs"],
};

export const AssetsOverviewDesktop: Story = {
  parameters: { viewport: { defaultViewport: "desktop" } },
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PortfolioAssetCollection
        title="Holdings"
        subtitle={null}
        presentation="overview"
        assets={portfolioAssets}
        onEdit={() => undefined}
        onMenu={() => undefined}
      />
    </div>
  ),
  tags: ["!autodocs"],
};

export const AssetsLongName: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PortfolioAssetCollection
        assets={[
          {
            ...portfolioAssets[0],
            id: "long-name",
            name: "Vanguard FTSE All-World UCITS ETF Accumulating EUR Hedged",
          },
        ]}
      />
    </div>
  ),
  tags: ["!autodocs"],
};

export const ChartReady: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PerformanceChart />
    </div>
  ),
};

export const ChartLoading: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PerformanceChart state="loading" />
    </div>
  ),
  tags: ["!autodocs"],
};

export const ChartEmpty: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PerformanceChart state="empty" />
    </div>
  ),
  tags: ["!autodocs"],
};

export const ChartGated: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ padding: "var(--dcapal-space-6)" }}>
      <PerformanceChart state="gated" />
    </div>
  ),
  tags: ["!autodocs"],
};

export const FloatingActions: Story = {
  render: () => <FloatingActionsNavigationPreview />,
  tags: ["!autodocs"],
};

export const FloatingActionsMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  render: () => <FloatingActionsNavigationPreview />,
  tags: ["!autodocs"],
};

export const FloatingActionsDesktop: Story = {
  parameters: { viewport: { defaultViewport: "desktop" } },
  render: () => <FloatingActionsNavigationPreview />,
  tags: ["!autodocs"],
};
