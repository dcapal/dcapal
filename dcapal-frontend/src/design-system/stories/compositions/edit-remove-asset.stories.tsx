import type { Meta, StoryObj } from "@storybook/react-webpack5";
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
  },
} satisfies Meta<typeof EditRemoveAssetComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

export const EditDetails: Story = {};
export const EditDetailsMobile: Story = {
  args: { open: true },
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
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
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
};
export const RemoveConfirmation: Story = {
  args: { state: "remove-confirmation", stage: "remove" },
  tags: ["!autodocs"],
};
