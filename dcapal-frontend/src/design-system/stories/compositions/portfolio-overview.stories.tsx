import type { Meta, StoryObj } from "@storybook/react-webpack5";
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
    allocationMethod: {
      control: { type: "select" },
      options: ["simple", "strategic"],
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
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
};

export const PopulatedMobile: Story = {
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
};

/** Strategic: class rows with an Edit {Class} button; the table has no mode switch. */
export const PortfolioOverviewStrategic: Story = {
  args: { allocationMethod: "strategic" },
  tags: ["!autodocs"],
};
export const PortfolioOverviewStrategicMobile: Story = {
  args: { allocationMethod: "strategic" },
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
};
/** The Edit Equities dialog is open over the Strategic overview. */
export const EditClassDialog: Story = {
  args: { allocationMethod: "strategic", defaultEditClass: "equities" },
  tags: ["!autodocs"],
};
export const EditClassDialogMobile: Story = {
  args: { allocationMethod: "strategic", defaultEditClass: "equities" },
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
};
