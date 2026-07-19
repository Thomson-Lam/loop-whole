import { lazy, Suspense, useEffect, useState } from "react";
import Landing from "./Landing";
import Dashboard from "./Dashboard";

const Benchmarks = lazy(() => import("./Benchmarks"));

function currentRoute() {
  const hash = window.location.hash.replace(/^#/, "");
  if (hash.startsWith("/benchmarks")) return "benchmarks";
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

  if (route === "app") return <Dashboard />;
  if (route === "benchmarks") {
    return (
      <Suspense fallback={<div className="dash pane-body">Loading benchmarks…</div>}>
        <Benchmarks />
      </Suspense>
    );
  }
  return <Landing />;
}
