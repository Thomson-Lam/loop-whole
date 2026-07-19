import { useEffect, useState } from "react";
import { getCurrentSession } from "./api";

export default function useLiveSession() {
  const [session, setSession] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    const controller = new AbortController();
    let timer;

    async function refresh() {
      try {
        setSession(await getCurrentSession(controller.signal));
        setError(null);
      } catch (nextError) {
        if (nextError.name !== "AbortError") setError(nextError);
      } finally {
        if (!controller.signal.aborted) timer = setTimeout(refresh, 1500);
      }
    }

    refresh();
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, []);

  return { session, error };
}
