import { PersistGate } from "redux-persist/integration/react";
import { BrowserRouter } from "react-router-dom";
import { Provider } from "react-redux";
import { store } from "./app/store"
// import { Router } from "./src/routes/router";
import { Router } from "./routes/router"


export const App = () => {
  return (
    <Provider store={store}>
      {/* <PersistGate persistor={persistor} loading={null}> */}
        <BrowserRouter>
          <Router />
        </BrowserRouter>
      {/* </PersistGate> */}
    </Provider>
  )
}