import React, { useEffect } from "react";
import { Toaster } from "react-hot-toast";
import { useGetAssetsFiat } from "@dcapal/api-client";

import { AllocationFlow } from "@components/allocationFlow";
import { useAppStore } from "@/state/appStore";
import { SESSION_STALE_TIME } from "@/api/queryClient";
import { ContainerPage } from "@routes/containerPage";

/** Renders the allocation application inside the route-level shell. */
export const App = () => {
  const setCurrencies = useAppStore((state) => state.setCurrencies);
  const fiatAssetsQuery = useGetAssetsFiat({
    query: { staleTime: SESSION_STALE_TIME },
  });

  useEffect(() => {
    const currencies = fiatAssetsQuery.data?.data?.map((asset) => asset.id);
    if (currencies) setCurrencies({ currencies });
  }, [fiatAssetsQuery.data, setCurrencies]);

  return (
    <ContainerPage
      title={"Allocate"}
      content={
        <>
          <Toaster
            position="top-center"
            reverseOrder={false}
            toastOptions={{
              success: {
                duration: 5000,
                iconTheme: { primary: "#166534", secondary: "#f0fdf4" },
              },
            }}
          />
          <div className="absolute bg-[#F3F4F6] w-full h-[50px] top-10 -z-40" />
          <div className="absolute app-bg -z-50" />
          <div
            data-testid="route-allocate"
            className="flex-grow w-full flex flex-col items-center"
          >
            <div className="w-full max-w-[42rem] grow flex px-6 pt-4 pb-6">
              <AllocationFlow />
            </div>
          </div>
        </>
      }
    />
  );
};

/** Default export consumed by the lazy allocation route. */
export default App;
