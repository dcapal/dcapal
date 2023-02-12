import React from "react";
import GitHubButton from "react-github-btn";

import { NavBar } from "../components/core/navBar";

import { IMAGEKIT_URL } from "../app/config";
import { IKImage } from "imagekitio-react";
import {
  HEADER_ME_JPG,
  SOCIAL_GITHUB_SVG,
  SOCIAL_LINKEDIN_SVG,
  SOCIAL_TELEGRAM_SVG,
  SOCIAL_TWITTER_SVG,
  SOCIAL_YOUTUBE_SVG,
} from "../app/images";
import { DcaPalHelmet } from "./helmet";

const SocialBar = () => {
  return (
    <div className="flex gap-1 items-center justify-center">
      <a href="https://github.com/leonardoarcari">
        <IKImage
          className="w-full max-w-[1.75rem] mr-[2px]"
          urlEndpoint={IMAGEKIT_URL}
          path={SOCIAL_GITHUB_SVG}
        />
      </a>
      <a href="https://www.linkedin.com/in/leonardoarcari/">
        <IKImage
          className="w-full max-w-[2rem]"
          urlEndpoint={IMAGEKIT_URL}
          path={SOCIAL_LINKEDIN_SVG}
        />
      </a>
      <a href="https://twitter.com/arcari_leonardo">
        <IKImage
          className="w-full max-w-[2rem]"
          urlEndpoint={IMAGEKIT_URL}
          path={SOCIAL_TWITTER_SVG}
        />
      </a>
      <a href="https://www.youtube.com/@leonardoarcari3011">
        <IKImage
          className="w-full max-w-[2rem]"
          urlEndpoint={IMAGEKIT_URL}
          path={SOCIAL_YOUTUBE_SVG}
        />
      </a>
      <a href="https://t.me/leonardoarcari">
        <IKImage
          className="w-full max-w-[2rem]"
          urlEndpoint={IMAGEKIT_URL}
          path={SOCIAL_TELEGRAM_SVG}
        />
      </a>
    </div>
  );
};

export default function AboutPage() {
  return (
    <>
      <DcaPalHelmet title="About" />
      <div
        id="anchor-social-profiles"
        className="w-full h-screen flex flex-col items-center"
      >
        <NavBar />
        <div className="w-full flex flex-col items-center gap-4 px-6 py-6 bg-[#ededed]">
          <h1 className="mb-8 text-3xl sm:text-4xl font-bold">About</h1>
          <IKImage
            className="w-full max-w-[15rem] rounded-full border border-neutral-700/20 shadow-md"
            urlEndpoint={IMAGEKIT_URL}
            path={HEADER_ME_JPG}
            lqip={{ active: true }}
          />
          <SocialBar />
        </div>
        <div className="w-full max-w-[44rem] p-6 flex flex-col items-center gap-6">
          <div className="w-full flex flex-col gap-4 text-justify font-thin text-lg">
            <p>
              <span className="font-normal">Hey there, I'm Leo!</span> 👋 I'm a
              Software Engineer in Finance, working on low-latency backend
              systems, mostly in C++ and Rust🦀. I know, I know, but there are
              worse things I could do.
            </p>
            <p>
              I designed DcaPal as a{" "}
              <span className="font-normal">pragmatic tool</span> for passive
              investors like me: financially-educated people managing their own
              portfolios of not-too-many assets replicating major world indices.
            </p>
            <p>
              I was facing a common problem: it's that time of the month, got
              some savings to invest and have to split them across my portfolio
              assets.{" "}
              <span className="italic">
                How the heck can I do it so that my portfolio stays balanced?
              </span>
            </p>
            <p>
              Hence DcaPal. You come here every week/month/quarter, build your
              portfolio, define asset allocation in percentage, input how much
              you want to invest and{" "}
              <span className="font-normal">
                let the algorithm do the splitting for you
              </span>
              .
            </p>
            <p>
              You can even export your portfolio and import it back next time to
              save precious minutes.
            </p>
          </div>
          <div
            id="privacy-policy"
            className="w-full flex flex-col gap-2 text-justify font-thin text-lg"
          >
            <h2 className="w-full text-3xl font-semibold mt-4 mb-2">
              Privacy policy
            </h2>
            <p>
              Here's our poor man privacy policy. We store your IP location for
              stats purposes. That's it. Nothing more.
            </p>
            <p>
              Since DcaPal is a zero-login service, we don't do user profiling.
              We do not store any user data: portfolio assets, invested amounts,
              nothing ever leaves your browser.
            </p>
            <p>
              This may change in the future. Until then, long live the Far West.
            </p>
          </div>
          <div
            id="feedback"
            className="w-full flex flex-col gap-2 text-justify font-thin text-lg"
          >
            <h2 className="w-full text-3xl font-semibold mt-4 mb-2">
              Feedback
            </h2>
            <p>
              If you find any issue, would like to ask for new features or
              simply leave a feedback,{" "}
              <span className="font-normal">feel free to reach out</span> to me
              on any of{" "}
              <a className="underline" href="#anchor-social-profiles">
                my social profiles
              </a>
              .
            </p>
            <p>
              <span className="align-super">
                If you are a fellow Dev, don't forget to{" "}
              </span>
              <GitHubButton
                href="https://github.com/leonardoarcari/dcapal"
                data-icon="octicon-star"
                data-size="large"
                aria-label="Star leonardoarcari/dcapal on GitHub"
              >
                Star
              </GitHubButton>
              <span className="align-super">
                {" "}
                DcaPal on{" "}
                <a
                  className="underline"
                  href="https://github.com/leonardoarcari/dcapal"
                >
                  Github
                </a>{" "}
                if you like it or drop me an{" "}
              </span>
              <GitHubButton
                href="https://github.com/leonardoarcari/dcapal/issues"
                data-icon="octicon-issue-opened"
                data-size="large"
                aria-label="Issue leonardoarcari/dcapal on GitHub"
              >
                Issue
              </GitHubButton>
              <span className="align-super"> if you want to help.</span>
            </p>
            <p>Every contribution is much appreciated!</p>
          </div>
        </div>
      </div>
    </>
  );
}
