import { useEffect, useState } from "react";
import Landing from "./Landing";
import Dashboard from "./Dashboard";

function currentRoute() {
  const hash = window.location.hash.replace(/^#/, "");
  return hash.startsWith("/app") ? "app" : "landing";
}

export default function App() {
  const [route, setRoute] = useState(currentRoute());

  useEffect(() => {
    const onHashChange = () => {
      setRoute(currentRoute());
      window.scrollTo(0, 0);
    };
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  return route === "app" ? <Dashboard /> : <Landing />;
}
