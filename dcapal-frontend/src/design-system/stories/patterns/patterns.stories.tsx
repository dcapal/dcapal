import type { Meta, StoryObj } from "@storybook/react";
import { Pencil, PieChart, Plus, RefreshCw } from "lucide-react";
import { Button } from "../../components/ui";
import {
  AppShell,
  DriftDiagnostic,
  FloatingActionMenu,
  MetricCard,
  PageHeader,
  PerformanceChart,
  PortfolioAssetCollection,
  TopNavigation,
} from "../../components/patterns";
import { portfolioAssets } from "../../fixtures/portfolio";

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

export const Shell: Story = {
  render: () => (
    <AppShell
      navigation={
        <TopNavigation
          links={[
            { label: "Overview", href: "#overview", active: true },
            { label: "Historical analytics", href: "#history" },
          ]}
        />
      }
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
    </AppShell>
  ),
};

export const ShellMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  render: () => (
    <AppShell
      navigation={
        <TopNavigation
          links={[
            { label: "Overview", href: "#overview", active: true },
            { label: "Historical analytics", href: "#history" },
          ]}
        />
      }
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
    </AppShell>
  ),
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
  render: () => (
    <div className="dcapal-theme" style={{ minHeight: "18rem" }}>
      <FloatingActionMenu
        actions={[
          {
            label: "Add asset",
            icon: <Plus aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Allocate",
            icon: <PieChart aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Rebalance",
            icon: <RefreshCw aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Edit portfolio",
            icon: <Pencil aria-hidden />,
            onClick: () => undefined,
          },
        ]}
      />
    </div>
  ),
  tags: ["!autodocs"],
};

export const FloatingActionsMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  render: () => (
    <div className="dcapal-theme" style={{ minHeight: "18rem" }}>
      <FloatingActionMenu
        actions={[
          {
            label: "Add asset",
            icon: <Plus aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Allocate",
            icon: <PieChart aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Rebalance",
            icon: <RefreshCw aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Edit portfolio",
            icon: <Pencil aria-hidden />,
            onClick: () => undefined,
          },
        ]}
      />
    </div>
  ),
  tags: ["!autodocs"],
};

export const FloatingActionsDesktop: Story = {
  parameters: { viewport: { defaultViewport: "desktop" } },
  render: () => (
    <div className="dcapal-theme" style={{ minHeight: "18rem" }}>
      <FloatingActionMenu
        actions={[
          {
            label: "Add asset",
            icon: <Plus aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Allocate",
            icon: <PieChart aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Rebalance",
            icon: <RefreshCw aria-hidden />,
            onClick: () => undefined,
          },
          {
            label: "Edit portfolio",
            icon: <Pencil aria-hidden />,
            onClick: () => undefined,
          },
        ]}
      />
    </div>
  ),
  tags: ["!autodocs"],
};
