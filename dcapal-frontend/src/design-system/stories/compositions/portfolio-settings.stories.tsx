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
    allocationMethod: {
      control: { type: "select" },
      options: ["simple", "strategic"],
    },
  },
} satisfies Meta<typeof PortfolioSettingsComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

/** The Simple allocation method is saved; Strategic is one click away. */
export const SettingsSimple: Story = {};
export const SettingsSimpleMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
/** Strategic is saved: a note points to the Portfolio page for weights. */
export const SettingsStrategic: Story = {
  args: { allocationMethod: "strategic" },
  tags: ["!autodocs"],
};
export const SettingsStrategicMobile: Story = {
  args: { allocationMethod: "strategic" },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
/** Simple is saved; used by the radio card and fee control tests. */
export const AllocationMethodOpen: Story = {
  tags: ["!autodocs"],
};
/** Saving a move to Strategic opens the dialog with every class weight set. */
export const SetClassWeights: Story = {
  args: { initialSelection: "strategic", classWeightsOpen: true },
  tags: ["!autodocs"],
};
export const SetClassWeightsMobile: Story = {
  args: { initialSelection: "strategic", classWeightsOpen: true },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
/** Class weights that total 95%: Save stays enabled and a note explains. */
export const SetClassWeightsUnbalanced: Story = {
  args: {
    initialSelection: "strategic",
    classWeightsOpen: true,
    classWeightsInitial: {
      equities: 60,
      bonds: 25,
      cash: 10,
      crypto: 0,
      commodities: 0,
      other: 0,
    },
  },
  tags: ["!autodocs"],
};
export const SetClassWeightsUnbalancedMobile: Story = {
  args: SetClassWeightsUnbalanced.args,
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
