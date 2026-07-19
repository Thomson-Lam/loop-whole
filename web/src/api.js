async function getJson(path, signal) {
  const response = await fetch(path, { signal });
  if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
  return response.json();
}

export async function getCurrentSession(signal) {
  const snapshot = await getJson("/api/v1/sessions/current", signal);
  // ponytail: refetch all details for demo-sized sessions; cache by id if timelines grow.
  const details = await Promise.all(
    snapshot.toolCalls.map((call) =>
      getJson(`/api/v1/tool-calls/${call.id}`, signal)
    )
  );

  return {
    ...snapshot,
    toolCalls: snapshot.toolCalls.map((call, index) => ({
      ...call,
      ...details[index],
    })),
  };
}
