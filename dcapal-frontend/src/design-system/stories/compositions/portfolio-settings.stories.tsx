import type { Meta, StoryObj } from "@storybook/react";
import { PortfolioSettingsComposition } from "../../components/patterns/compositions";

const meta = {
  title: "Compositions / Portfolio settings",
  component: PortfolioSettingsComposition,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: { story: { inline: false, iframeHeight: 1100 } },
  },
  argTypes: {
    saveState: {
      control: { type: "select" },
      options: ["idle", "success", "error"],
    },
    view: {
      control: { type: "select" },
      options: ["settings", "strategic"],
    },
    open: { control: { type: "boolean" } },
  },
} satisfies Meta<typeof PortfolioSettingsComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

export const AllocationMethod: Story = {};
export const AllocationMethodMobile: Story = {
  args: { open: true },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
export const AllocationMethodOpen: Story = {
  args: { open: true },
  tags: ["!autodocs"],
};
export const StrategicAllocation: Story = {
  args: { view: "strategic", open: true },
  tags: ["!autodocs"],
};
export const StrategicAllocationMobile: Story = {
  args: { view: "strategic", open: true },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
export const SaveSuccess: Story = {
  args: { saveState: "success" },
  tags: ["!autodocs"],
};
export const SaveError: Story = {
  args: { saveState: "error" },
  tags: ["!autodocs"],
};
