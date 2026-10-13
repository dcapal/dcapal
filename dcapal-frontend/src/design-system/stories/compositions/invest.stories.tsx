import type { Meta, StoryObj } from "@storybook/react";
import { InvestComposition } from "../../components/patterns/invest-composition";

const meta = {
  title: "Compositions / Invest",
  component: InvestComposition,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: { story: { inline: false, iframeHeight: 1100 } },
  },
  argTypes: {
    screen: {
      control: { type: "select" },
      options: ["drift", "plan", "proposal", "updated"],
    },
    strategy: {
      control: { type: "select" },
      options: ["add-money", "rebalance"],
    },
    proposalState: {
      control: { type: "select" },
      options: [
        "ready",
        "loading",
        "blocked",
        "needs-amount",
        "on-target",
        "fees-too-high",
        "price-too-high",
        "error",
      ],
    },
  },
} satisfies Meta<typeof InvestComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

const mobile = { viewport: { defaultViewport: "mobile" } } as const;
const laptop = { viewport: { defaultViewport: "laptop" } } as const;
const compact = { viewport: { defaultViewport: "compact" } } as const;

/** Starts at the drift view; the whole journey can be walked in the Canvas. */
export const DriftOverview: Story = {};
export const DriftOverviewMobile: Story = {
  parameters: mobile,
  tags: ["!autodocs"],
};

export const PlanAddMoney: Story = {
  args: { screen: "plan", strategy: "add-money" },
  tags: ["!autodocs"],
};
export const PlanAddMoneyMobile: Story = {
  args: { screen: "plan", strategy: "add-money" },
  parameters: mobile,
  tags: ["!autodocs"],
};

export const PlanRebalance: Story = {
  args: { screen: "plan", strategy: "rebalance" },
  tags: ["!autodocs"],
};

/** The commissions editor is open, so this story is for the Canvas only. */
export const CommissionsDialog: Story = {
  args: { screen: "plan", strategy: "add-money", feeDialogOpen: true },
  tags: ["!autodocs"],
};

export const ProposalAddMoney: Story = {
  args: { screen: "proposal", strategy: "add-money" },
  tags: ["!autodocs"],
};
export const ProposalAddMoneyMobile: Story = {
  args: { screen: "proposal", strategy: "add-money" },
  parameters: mobile,
  tags: ["!autodocs"],
};

export const ProposalRebalance: Story = {
  args: { screen: "proposal", strategy: "rebalance" },
  tags: ["!autodocs"],
};
export const ProposalRebalanceMobile: Story = {
  args: { screen: "proposal", strategy: "rebalance" },
  parameters: mobile,
  tags: ["!autodocs"],
};

export const Updated: Story = {
  args: { screen: "updated", strategy: "add-money" },
  tags: ["!autodocs"],
};
export const UpdatedMobile: Story = {
  args: { screen: "updated", strategy: "add-money" },
  parameters: mobile,
  tags: ["!autodocs"],
};

/**
 * Main-column responsiveness: with the portfolio sidebar the content column is
 * far narrower than the viewport, so these frames check the 1180px and 1024px
 * widths that sit between the mobile and 1440px layouts.
 */
export const DriftOverviewLaptop: Story = {
  parameters: laptop,
  tags: ["!autodocs"],
};
export const DriftOverviewCompact: Story = {
  parameters: compact,
  tags: ["!autodocs"],
};
export const PlanAddMoneyLaptop: Story = {
  args: { screen: "plan", strategy: "add-money" },
  parameters: laptop,
  tags: ["!autodocs"],
};
export const PlanAddMoneyCompact: Story = {
  args: { screen: "plan", strategy: "add-money" },
  parameters: compact,
  tags: ["!autodocs"],
};
export const ProposalAddMoneyLaptop: Story = {
  args: { screen: "proposal", strategy: "add-money" },
  parameters: laptop,
  tags: ["!autodocs"],
};
export const ProposalAddMoneyCompact: Story = {
  args: { screen: "proposal", strategy: "add-money" },
  parameters: compact,
  tags: ["!autodocs"],
};
export const ProposalRebalanceLaptop: Story = {
  args: { screen: "proposal", strategy: "rebalance" },
  parameters: laptop,
  tags: ["!autodocs"],
};
export const ProposalRebalanceCompact: Story = {
  args: { screen: "proposal", strategy: "rebalance" },
  parameters: compact,
  tags: ["!autodocs"],
};

export const ProposalLoading: Story = {
  args: { screen: "proposal", proposalState: "loading" },
  tags: ["!autodocs"],
};
export const ProposalBlocked: Story = {
  args: { screen: "proposal", proposalState: "blocked" },
  tags: ["!autodocs"],
};
export const ProposalNeedsAmount: Story = {
  args: { screen: "proposal", proposalState: "needs-amount" },
  tags: ["!autodocs"],
};
export const ProposalOnTarget: Story = {
  args: { screen: "proposal", proposalState: "on-target" },
  tags: ["!autodocs"],
};
export const ProposalFeesTooHigh: Story = {
  args: { screen: "proposal", proposalState: "fees-too-high" },
  tags: ["!autodocs"],
};
export const ProposalPriceTooHigh: Story = {
  args: { screen: "proposal", proposalState: "price-too-high" },
  tags: ["!autodocs"],
};
export const ProposalError: Story = {
  args: { screen: "proposal", proposalState: "error" },
  tags: ["!autodocs"],
};
