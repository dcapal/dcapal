/** @type {import("tailwindcss").Config} */
module.exports = {
  darkMode: ["class"],
  content: ["./src/**/*.{html,js,jsx,ts,tsx}"],
  theme: {
    extend: {
      colors: {
        dcapal: {
          canvas: "var(--dcapal-canvas)",
          surface: "var(--dcapal-surface)",
          "surface-subtle": "var(--dcapal-surface-subtle)",
          foreground: "var(--dcapal-foreground)",
          "foreground-muted": "var(--dcapal-foreground-muted)",
          border: "var(--dcapal-border)",
          primary: "var(--dcapal-primary)",
          "primary-soft": "var(--dcapal-primary-soft)",
          success: "var(--dcapal-success)",
          warning: "var(--dcapal-warning)",
          destructive: "var(--dcapal-destructive)",
          info: "var(--dcapal-info)",
          chrome: "var(--dcapal-chrome)",
          "chrome-foreground": "var(--dcapal-chrome-foreground)",
          asset: {
            equities: "var(--dcapal-asset-equities)",
            bonds: "var(--dcapal-asset-bonds)",
            cash: "var(--dcapal-asset-cash)",
            crypto: "var(--dcapal-asset-crypto)",
            commodities: "var(--dcapal-asset-commodities)",
            other: "var(--dcapal-asset-other)",
          },
        },
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
      },
      borderRadius: {
        lg: `var(--radius)`,
        md: `calc(var(--radius) - 2px)`,
        sm: "calc(var(--radius) - 4px)",
        "dcapal-sm": "var(--dcapal-radius-sm)",
        "dcapal-md": "var(--dcapal-radius-md)",
        "dcapal-lg": "var(--dcapal-radius-lg)",
        "dcapal-pill": "var(--dcapal-radius-pill)",
      },
      spacing: {
        "dcapal-1": "var(--dcapal-space-1)",
        "dcapal-2": "var(--dcapal-space-2)",
        "dcapal-3": "var(--dcapal-space-3)",
        "dcapal-4": "var(--dcapal-space-4)",
        "dcapal-5": "var(--dcapal-space-5)",
        "dcapal-6": "var(--dcapal-space-6)",
        "dcapal-8": "var(--dcapal-space-8)",
        "dcapal-10": "var(--dcapal-space-10)",
        "dcapal-12": "var(--dcapal-space-12)",
        "dcapal-16": "var(--dcapal-space-16)",
        "dcapal-gutter": "var(--dcapal-page-gutter)",
      },
      fontFamily: {
        dcapal: "var(--dcapal-font-sans)",
      },
      boxShadow: {
        "dcapal-none": "var(--dcapal-elevation-none)",
        "dcapal-raised": "var(--dcapal-elevation-raised)",
        "dcapal-overlay": "var(--dcapal-elevation-overlay)",
      },
      maxWidth: {
        "dcapal-page": "var(--dcapal-page-width)",
        "dcapal-form": "var(--dcapal-form-width)",
      },
      minHeight: {
        "dcapal-header": "var(--dcapal-header-height)",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};
