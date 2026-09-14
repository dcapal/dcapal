import type { Meta, StoryObj } from "@storybook/react";
import "../../styles/index.css";

const meta = {
  title: "Foundations / Spacing",
  tags: ["autodocs"],
  parameters: { layout: "padded" },
} satisfies Meta;
export default meta;
type Story = StoryObj<typeof meta>;

export const FourPixelScale: Story = {
  render: () => (
    <div className="dcapal-theme ds-foundation-spacing">
      {["1", "2", "3", "4", "6", "8", "12"].map((space) => (
        <div key={space}>
          <span style={{ width: `var(--dcapal-space-${space})` }} />
          <code>{`--dcapal-space-${space}`}</code>
        </div>
      ))}
    </div>
  ),
};
