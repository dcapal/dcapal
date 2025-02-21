import React, { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { Auth } from "@supabase/auth-ui-react";
import { supabase } from "@app/config";
import { useTranslation } from "react-i18next";
import { ThemeSupa } from "@supabase/auth-ui-shared";

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
  marginTop: "25px",
  textAlign: "center",
  textDecoration: "underline",
  fontSize: "13px",
  fontWeight: "normal",
  lineHeight: "1.5",
  color: "#666",
};

export default function AuthPage() {
  const navigate = useNavigate();
  const { t } = useTranslation();

  useEffect(() => {
    const { data: authListener } = supabase.auth.onAuthStateChange(
      (event, session) => {
        if (event === "SIGNED_IN") {
          navigate("/");
        }
      }
    );

    return () => {
      authListener.subscription.unsubscribe();
    };
  }, [navigate]);

  return (
    <div style={containerStyle}>
      <div style={formContainerStyle}>
        <h1 style={titleStyle}>{t("page.login.signIn")}</h1>
        <p style={subtitleStyle}>{t("page.login.subtitle")}</p>
        <Auth
          supabaseClient={supabase}
          localization={{
            variables: {
              sign_in: {
                email_label: t("page.login.email"),
                password_label: t("page.login.password"),
                email_input_placeholder: t("page.login.emailInput"),
                password_input_placeholder: t("page.login.passwordInput"),
                button_label: t("page.login.signIn"),
                social_provider_text: t("page.login.socialLogin"),
              },
            },
          }}
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
          view="sign_in"
          showLinks={false}
          magicLink={false}
          onlyThirdPartyProviders={true}
        />
        <div style={linkStyle}>
          <a href="/signup" style={linkStyle}>
            {t("page.login.signUp")}
          </a>
          <br />
        </div>
      </div>
    </div>
  );
}
