import type { Meta, StoryObj } from "@storybook/react";
import { PortfolioOverviewComposition } from "../../components/patterns/compositions";

const meta = {
  title: "Compositions / Portfolio overview",
  component: PortfolioOverviewComposition,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: { story: { inline: false, iframeHeight: 1100 } },
  },
  argTypes: {
    state: {
      control: { type: "select" },
      options: ["populated", "empty"],
    },
    firstUseStage: {
      control: { type: "select" },
      options: ["empty", "name", "asset", "weights"],
    },
  },
} satisfies Meta<typeof PortfolioOverviewComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Populated: Story = {};

export const FirstUse: Story = {
  args: { state: "empty" },
  tags: ["!autodocs"],
};

export const FirstUseNameComplete: Story = {
  args: { state: "empty", firstUseStage: "name" },
  tags: ["!autodocs"],
};

export const FirstUseAssetAdded: Story = {
  args: { state: "empty", firstUseStage: "asset" },
  tags: ["!autodocs"],
};

export const FirstUseWeightsComplete: Story = {
  args: { state: "empty", firstUseStage: "weights" },
  tags: ["!autodocs"],
};

export const FirstUseAssetAddedMobile: Story = {
  args: { state: "empty", firstUseStage: "asset" },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};

export const PopulatedMobile: Story = {
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
