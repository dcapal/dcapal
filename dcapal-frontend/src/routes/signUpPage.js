import React from "react";

import { Auth } from "@supabase/auth-ui-react";
import { ThemeSupa } from "@supabase/auth-ui-shared";
import { supabase } from "@app/config";

const containerStyle = {
  width: "100vw",
  height: "100vh",
  display: "flex",
  justifyContent: "center",
  alignItems: "center",
  backgroundColor: "#F3F4F6", // light gray
};

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

export default function SignUpPage() {
  return (
    <div style={containerStyle}>
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
  );
}
