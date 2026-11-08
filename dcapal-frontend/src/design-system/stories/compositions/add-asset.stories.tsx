import type { Meta, StoryObj } from "@storybook/react-webpack5";
import { AddAssetComposition } from "../../components/patterns/compositions";

const meta = {
  title: "Compositions / Add asset",
  component: AddAssetComposition,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: { story: { inline: false, iframeHeight: 1100 } },
  },
  argTypes: {
    open: { control: "boolean" },
    state: {
      control: { type: "select" },
      options: [
        "entry",
        "loading",
        "search-results",
        "empty",
        "error",
        "unpriced",
        "edit",
      ],
    },
  },
} satisfies Meta<typeof AddAssetComposition>;
export default meta;
type Story = StoryObj<typeof meta>;

export const EntryPoint: Story = {};

export const EntryPointMobile: Story = {
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
};

export const SearchLoading: Story = {
  args: { state: "loading", open: true },
  tags: ["!autodocs"],
};
export const SearchResults: Story = {
  args: { state: "search-results", open: true },
  tags: ["!autodocs"],
};
export const SearchResultsMobile: Story = {
  args: { state: "search-results", open: true },
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
};
export const EmptyResult: Story = {
  args: { state: "empty", open: true },
  tags: ["!autodocs"],
};
export const SearchError: Story = {
  args: { state: "error", open: true },
  tags: ["!autodocs"],
};
export const UnpricedResult: Story = {
  args: { state: "unpriced", open: true },
  tags: ["!autodocs"],
};
export const NewAssetEdit: Story = {
  args: { state: "edit", open: true },
  tags: ["!autodocs"],
};
export const NewAssetEditMobile: Story = {
  args: { state: "edit", open: true },
  tags: ["!autodocs"],
  globals: {
    viewport: {
      value: "mobile",
      isRotated: false,
    },
  },
};
