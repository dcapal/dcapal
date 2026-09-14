import type { Meta, StoryObj } from "@storybook/react";
import "../../styles/index.css";

const meta = {
  title: "Foundations / Radius",
  tags: ["autodocs"],
  parameters: { layout: "padded" },
} satisfies Meta;
export default meta;
type Story = StoryObj<typeof meta>;

export const Scale: Story = {
  render: () => (
    <div className="dcapal-theme ds-foundation-grid">
      {[
        ["sm", "--dcapal-radius-sm"],
        ["md", "--dcapal-radius-md"],
        ["lg", "--dcapal-radius-lg"],
        ["pill", "--dcapal-radius-pill"],
      ].map(([label, token]) => (
        <div
          key={token}
          style={{
            border: "1px solid var(--dcapal-border)",
            borderRadius: `var(${token})`,
            padding: "var(--dcapal-space-6)",
          }}
        >
          {label}
        </div>
      ))}
    </div>
  ),
};
