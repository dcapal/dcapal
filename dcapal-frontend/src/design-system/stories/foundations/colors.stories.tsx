import type { Meta, StoryObj } from "@storybook/react";
import "../../styles/index.css";

const meta = {
  title: "Foundations / Colors",
  tags: ["autodocs"],
  parameters: { layout: "padded" },
} satisfies Meta;
export default meta;
type Story = StoryObj<typeof meta>;

const swatches = [
  ["Canvas · warm ivory", "--dcapal-canvas"],
  ["Surface", "--dcapal-surface"],
  ["Ink", "--dcapal-foreground"],
  ["Muted ink", "--dcapal-foreground-muted"],
  ["Border", "--dcapal-border"],
  ["Primary action · rust", "--dcapal-primary"],
  ["Positive", "--dcapal-success"],
  ["Warning", "--dcapal-warning"],
  ["Error · destructive", "--dcapal-destructive"],
  ["Information", "--dcapal-info"],
  ["Equities", "--dcapal-asset-equities"],
  ["Bonds", "--dcapal-asset-bonds"],
  ["Cash", "--dcapal-asset-cash"],
] as const;

export const Palette: Story = {
  render: () => (
    <div className="dcapal-theme ds-foundation-grid">
      {swatches.map(([label, token]) => (
        <div key={token} className="ds-foundation-swatch">
          <span style={{ background: `var(${token})` }} />
          <strong>{label}</strong>
          <code>{token}</code>
        </div>
      ))}
    </div>
  ),
};
