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
  },
} satisfies Meta<typeof PortfolioOverviewComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Populated: Story = {};

export const FirstUse: Story = {
  args: { state: "empty" },
  tags: ["!autodocs"],
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
