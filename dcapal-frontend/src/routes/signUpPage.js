import React from "react";

import { Auth } from "@supabase/auth-ui-react";
import { ThemeSupa } from "@supabase/auth-ui-shared";
import { supabase } from "@app/config";

const formContainerStyle = {
  backgroundColor: "#FFFFFF",
  padding: "20px",
  borderRadius: "10px",
  boxShadow: "0 4px 6px rgba(0, 0, 0, 0.1)",
  width: "400px",
  textAlign: "left",
};

const titleStyle = {
  margin: "0",
  marginBottom: "10px",
  fontSize: "24px",
  fontWeight: "bold",
  color: "#333",
};

const subtitleStyle = {
  margin: "0",
  marginBottom: "20px",
  fontSize: "16px",
  color: "#666",
};

const linkStyle = {
  marginTop: "20px",
  textAlign: "center",
  textDecoration: "underline",
  fontSize: "13px",
  fontWeight: "normal",
  lineHeight: "1.5",
  color: "#666",
};

// Media query styles
export default function SignUpPage() {
  return (
    <div className="grid md:grid-cols-2 w-full min-h-screen">
      <div className="bg-primary flex flex-col items-center justify-center p-8 md:p-12 lg:p-16">
        <div className="max-w-md space-y-6">
          <div className="flex items-center space-x-2">
            <span className="text-5xl font-bold text-primary-foreground">
              DcaPal
            </span>
          </div>
          <div className="space-y-2">
            <h2 className="text-3xl font-bold text-primary-foreground">
              Dollar Cost Averaging Made Easy
            </h2>
            <p className="text-primary-foreground/80">
              Signing up for DcaPal provides numerous benefits, including the
              ability to keep your portfolio under control with tax-efficient
              suggestions for your monthly investments.
            </p>
          </div>
          <ul className="space-y-2 text-primary-foreground">
            <li className="flex items-center space-x-2">
              <CheckIcon className="h-5 w-5" />
              <span>
                Access to your investment portfolios across all devices*
              </span>
            </li>
            <li className="flex items-center space-x-2">
              <CheckIcon className="h-5 w-5" />
              <span>Dive into a fully customizable experience*</span>
            </li>
            <li className="flex items-center space-x-2">
              <CheckIcon className="h-5 w-5" />
              <span>Access to the dashboard*</span>
            </li>
            <li className="flex items-center space-x-2">
              <CheckIcon className="h-5 w-5" />
              <span>Set custom alerts for your investments*</span>
            </li>
          </ul>
          <div className="text-primary-foreground/80 text-center">
            *Feature not yet available.
          </div>
        </div>
      </div>
      <div className="flex items-center justify-center p-8 md:p-12 lg:p-16">
        <div style={formContainerStyle}>
          <h1 style={titleStyle}>Sign Up</h1>
          <p style={subtitleStyle}>to continue to DcaPal</p>
          <Auth
            supabaseClient={supabase}
            appearance={{
              theme: ThemeSupa,
              variables: {
                default: {
                  colors: {
                    brand: "#0000FF",
                    brandAccent: "#0000FF",
                  },
                  radii: {
                    button: "5px",
                  },
                },
              },
            }}
            providers={["google", "github"]}
            view="sign_up"
            showLinks={false}
          />
          <div style={linkStyle}>
            <a href="/login" style={linkStyle}>
              Already have an account? Sign in
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}

function CheckIcon(props) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M20 6 9 17l-5-5" />
    </svg>
  );
}
