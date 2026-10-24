import type { Meta, StoryObj } from "@storybook/react";
import { EditRemoveAssetComposition } from "../../components/patterns/compositions";

const meta = {
  title: "Compositions / Edit and remove asset",
  component: EditRemoveAssetComposition,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: { story: { inline: false, iframeHeight: 1100 } },
  },
  argTypes: {
    state: {
      control: { type: "select" },
      options: ["editing", "saved", "error", "remove-confirmation"],
    },
    stage: {
      control: { type: "select" },
      options: ["details", "fees", "remove"],
    },
    open: { control: { type: "boolean" } },
    allocationMethod: {
      control: { type: "select" },
      options: ["simple", "strategic"],
    },
  },
} satisfies Meta<typeof EditRemoveAssetComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

export const EditDetails: Story = {};
export const EditDetailsMobile: Story = {
  args: { open: true },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
export const EditDetailsOpen: Story = {
  args: { open: true },
  tags: ["!autodocs"],
};
export const SaveSuccess: Story = {
  args: { state: "saved" },
  tags: ["!autodocs"],
};
export const SaveError: Story = {
  args: { state: "error" },
  tags: ["!autodocs"],
};
export const TransactionFees: Story = {
  args: { stage: "fees" },
  tags: ["!autodocs"],
};
export const TransactionFeesMobile: Story = {
  args: { stage: "fees" },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
export const RemoveConfirmation: Story = {
  args: { state: "remove-confirmation", stage: "remove" },
  tags: ["!autodocs"],
};

/** Strategic: the asset dialog edits the share of the class and previews siblings. */
export const EditAssetStrategic: Story = {
  args: { open: true, allocationMethod: "strategic" },
  tags: ["!autodocs"],
};
export const EditAssetStrategicMobile: Story = {
  args: { open: true, allocationMethod: "strategic" },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
/** Simple: the asset dialog keeps the single absolute Target weight field. */
export const EditAssetSimple: Story = {
  args: { open: true, allocationMethod: "simple" },
  tags: ["!autodocs"],
};
export const EditAssetSimpleMobile: Story = {
  args: { open: true, allocationMethod: "simple" },
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
