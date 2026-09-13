import type { Meta, StoryObj } from "@storybook/react";
import "../../styles/index.css";

const meta = {
  title: "Foundations / Elevation",
  tags: ["autodocs"],
  parameters: { layout: "padded" },
} satisfies Meta;
export default meta;
type Story = StoryObj<typeof meta>;

export const Levels: Story = {
  render: () => (
    <div className="dcapal-theme ds-foundation-grid">
      {[
        ["none", "--dcapal-elevation-none"],
        ["raised", "--dcapal-elevation-raised"],
        ["overlay", "--dcapal-elevation-overlay"],
      ].map(([label, token]) => (
        <div
          key={token}
          style={{
            boxShadow: `var(${token})`,
            padding: "var(--dcapal-space-8)",
            background: "var(--dcapal-surface)",
            borderRadius: "var(--dcapal-radius-lg)",
          }}
        >
          {label}
        </div>
      ))}
    </div>
  ),
};
