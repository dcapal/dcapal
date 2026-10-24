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
      <p>
        Instrument Sans keeps headings, labels, and numbers clear at every size.
      </p>
      <p className="ds-label">Control label · Medium</p>
      <p className="ds-asset-card__meta">Supporting text · Muted</p>
      <p className="ds-metric-card__value">€124,680.75</p>
    </div>
  ),
};
