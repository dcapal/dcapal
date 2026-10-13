import type { Meta, StoryObj } from "@storybook/react";
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
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
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
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
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
  parameters: { viewport: { defaultViewport: "mobile" } },
  tags: ["!autodocs"],
};
