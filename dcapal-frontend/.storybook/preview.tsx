import type { Preview } from "@storybook/react";
import "../src/style.css";
import "../src/design-system/styles/index.css";

const preview: Preview = {
  decorators: [
    (Story) => (
      <div className="dcapal-theme" style={{ minHeight: "100vh" }}>
        <Story />
      </div>
    ),
  ],
  parameters: {
    a11y: {
      test: "error",
    },
    backgrounds: {
      default: "Light canvas",
      values: [
        {
          name: "Light canvas",
          value: "var(--dcapal-canvas)",
        },
        {
          name: "Dark chrome",
          value: "var(--dcapal-chrome)",
        },
      ],
    },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    layout: "fullscreen",
    viewport: {
      viewports: {
        mobile: {
          name: "Mobile",
          styles: { width: "390px", height: "844px" },
        },
        desktop: {
          name: "Desktop",
          styles: { width: "1440px", height: "900px" },
        },
      },
      defaultViewport: "desktop",
    },
  },
};

export default preview;
