import type { Meta, StoryObj } from "@storybook/react";
import { AllocateComposition } from "../../components/patterns/compositions";

const meta = {
  title: "Compositions / Allocate and rebalance",
  component: AllocateComposition,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: { story: { inline: false, iframeHeight: 1100 } },
  },
  argTypes: {
    state: {
      control: { type: "select" },
      options: ["options", "result"],
    },
    mode: {
      control: { type: "select" },
      options: ["allocate", "rebalance"],
    },
  },
} satisfies Meta<typeof AllocateComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

export const AllocationOptions: Story = {};
export const AllocationOptionsMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
export const AllocationResult: Story = {
  args: { state: "result" },
  tags: ["!autodocs"],
};
export const AllocationReviewDeltas: Story = {
  args: { state: "result", mode: "rebalance" },
  tags: ["!autodocs"],
};
export const RebalanceOptions: Story = {
  args: { mode: "rebalance" },
  tags: ["!autodocs"],
};
