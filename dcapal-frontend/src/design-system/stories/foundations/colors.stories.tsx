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
  ["Canvas", "--dcapal-canvas"],
  ["Surface", "--dcapal-surface"],
  ["Foreground", "--dcapal-foreground"],
  ["Primary", "--dcapal-primary"],
  ["Success", "--dcapal-success"],
  ["Warning", "--dcapal-warning"],
  ["Destructive", "--dcapal-destructive"],
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
