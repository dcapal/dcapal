import type { Meta, StoryObj } from "@storybook/react";
import "../../styles/index.css";

const meta = {
  title: "Foundations / Typography",
  tags: ["autodocs"],
  parameters: { layout: "padded" },
} satisfies Meta;
export default meta;
type Story = StoryObj<typeof meta>;

export const Roles: Story = {
  render: () => (
    <div className="dcapal-theme ds-foundation-type">
      <h1 className="ds-page-header__title">Portfolio overview</h1>
      <p>System sans typography keeps the interface familiar and readable.</p>
      <p className="ds-asset-card__meta">Muted supporting text · 12px</p>
      <p className="ds-metric-card__value">€124,680.75</p>
    </div>
  ),
};
