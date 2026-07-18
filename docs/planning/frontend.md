# what the frontend should look like

Because we need to demonstrate that the concept of tool calling and file state management compaction works, we need to show what gets returned to the model as tool calls.

The frontend UI will have at the top left: token tab (current focus), silent failures tab (deferred)

on the token tab: we split the UI vertically into 2 halves, with the left side showing what the original agent -> tool returns, and on the right, what our wrapper that intercepts the tool returns (compaction/compression/diff on tool calls, which matters).
on the bottom right of the token tab screen UI, left and right arrows are present for cycling through every tool call. 

The frontend polls the server instead of websockets, because tool calls do not require real time low latency.

On the top right of the token tab, we compute a cumulative diff in % of all tokens saved for the entire system, via the before and after interception (this might require new functionality in the backend). 

On the token tab, the key c opens a floating window/popup (the context window UI in claude), which shows a comparison between how much tool calling would have taken without our wrapper and how much context our solution uses. This is for the whole agent session, excluding the system prompt and agents.md (which we can exlcude). Only consider the tool calling and file reads for this context window UI.
