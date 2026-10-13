import type { Preview } from "@storybook/react-webpack5";
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
      options: {
        light_canvas: {
          name: "Light canvas",
          value: "var(--dcapal-canvas)",
        },
        ink_surface: {
          name: "Ink surface",
          value: "var(--dcapal-chrome)",
        },
      },
    },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    layout: "fullscreen",
    viewport: {
      options: {
        mobile: {
          name: "Mobile",
          styles: { width: "390px", height: "844px" },
        },
        desktop: {
          name: "Desktop",
          styles: { width: "1440px", height: "900px" },
        },
      },
    },
  },

  initialGlobals: {
    viewport: {
      value: "desktop",
      isRotated: false,
    },

    backgrounds: {
      value: "light_canvas",
    },
  },
};

export default preview;
