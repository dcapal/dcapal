import React, { useEffect } from "react";

import { Auth } from "@supabase/auth-ui-react";
import { ThemeSupa } from "@supabase/auth-ui-shared";
import { supabase } from "@app/config";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

const containerStyle = {
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

// Media query styles
export default function SignUpPage() {
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
      <div className="grid md:grid-cols-2 w-full min-h-screen">
        <div className="bg-primary flex flex-col items-center justify-center p-8 md:p-12 lg:p-16">
          <div className="max-w-md space-y-6">
            <div className="flex items-center space-x-2">
              <span className="text-5xl font-bold text-primary-foreground">
                {t("page.signUp.title")}
              </span>
            </div>
            <div className="space-y-2">
              <h2 className="text-3xl font-bold text-primary-foreground">
                {t("page.signUp.subtitle")}
              </h2>
              <p className="text-primary-foreground/80">
                {t("page.signUp.description")}
              </p>
            </div>
            <ul className="space-y-2 text-primary-foreground">
              <li className="flex items-center space-x-2">
                <CheckIcon className="h-5 w-5" />
                <span>{t("page.signUp.feature1")}</span>
              </li>
              <li className="flex items-center space-x-2">
                <CheckIcon className="h-5 w-5" />
                <span>{t("page.signUp.feature2")}*</span>
              </li>
              <li className="flex items-center space-x-2">
                <CheckIcon className="h-5 w-5" />
                <span>{t("page.signUp.feature3")}*</span>
              </li>
              <li className="flex items-center space-x-2">
                <CheckIcon className="h-5 w-5" />
                <span>{t("page.signUp.feature4")}*</span>
              </li>
            </ul>
            <div className="text-primary-foreground/80 text-center">
              <p>{t("page.signUp.featureNotYetAvailable")}</p>
            </div>
          </div>
        </div>
        <div className="flex items-center justify-center p-8 md:p-12 lg:p-16">
          <div style={formContainerStyle}>
            <h1 style={titleStyle}>Sign Up</h1>
            <p style={subtitleStyle}>to continue to DcaPal</p>
            <Auth
              supabaseClient={supabase}
              localization={{
                variables: {
                  sign_up: {
                    email_label: t("page.signUp.email"),
                    password_label: t("page.signUp.password"),
                    email_input_placeholder: t("page.signUp.emailInput"),
                    password_input_placeholder: t("page.signUp.passwordInput"),
                    button_label: t("page.signUp.signUp"),
                    social_provider_text: t("page.signUp.socialLogin"),
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
              view="sign_up"
              showLinks={false}
              onlyThirdPartyProviders={true}
            />
            <div style={linkStyle}>
              <a href="/login" style={linkStyle}>
                {t("page.signUp.alreadyHaveAnAccount")}
              </a>
            </div>
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
