# about 

This is a project intended for a hackathon proof of concept; primarily focused on quality over quantity because tracks are diverse and hard. Focusing on delivering a polished and usable proof of concept of a product is the main priority here.

Track - best developer tool: "Build a hack that focuses on improving the developer experience in some meaningful way - this could be tackling any part of the development lifecycle (creating, modifying or testing software)!" It has been confirmed that if the tool improves the developer experience in both efficiency and cost of using coding agents in any way, it is valid.

Personal takeaway: Warp is the sponsor track. They are looking for any good product framing and developer tooling design that can be integrated into their agentic development environment, so the scope and scale of the applicability of this tool is also vital to the project's success in the hackathon. Namely, "How many users can this tool benefit, and is it worth integrating into our own agentic development environment product?"

Previous research and existing brainstorming files have been done already. Refer to them and assist me in iterating a MVP.

# product formulation

Our current strongest product framing is: "Everyone has different workflows. So instead of building one specific dev tool that only improves one person, we built a generalizable tool that can benefit and improve the DX for everyone, from Claude Code to Codex, GH copilot to OpenCode and Roo code users."

We identified the 3 common pain points across 3 different axes present in everyone's coding agents:

1. alignment: when the user has a clear structure and an idea of a system (typically a system diagram of how actors in a system interact with each other in their head), natural language is used to convey this to the agent. But this creates drift, and the user has no way of actually telling whether the coding agent fully got the architecture and system. Refer to @alignment-conversation.md
2. context optimization: AI have evolved from chatbots to agents, yet they have full access and raw primitive actions directly over the codebase, which has states. They are treating coding agents as full conversations and not states, so context for both files and tool calls can be optimized. Refer to the @context-optimization-conversation.md file
3. verification and alignment: the coding agent writes code, tests, and runs tests, which all look and sound right. You could use a subagent to verify the agent's work but this leads to the Ouroboros poblem; the human is kept completely out of the loop, and there are no way to catch silent failures such as:
  - skipped work: did the agent actually make the change needed?
  - out of scope work: did the agent implement something that the user did not need?
  - instruction violation: did the agent implement something that the user explicitly instructed against?
  - integration failure: an error or code edge case happened, but the agent did not handle them accordingly.
  - hallucination: The agent asserts something with no basis in fact. A feature that doesn't exist, a policy never set, a number never established. It's stated as plainly as if it were true.
  - communication failure: did the agent actually interpret and understand the user's implementation instruction and guidance correctly?
Refer to @verification-alignment-conversation.md. 

Note that this is a hackathon project. The intended demo model is through self constructed example codebases, and pre-recorded demo videos.

What the product looks like:

" A single MCP server that proxies read and write tools for the agent with the state management built into it, with 1) a minimal and demo-specific frontend UI for specifically context optimization based on token usage, 3) a custom MCP structure and agent tooling integrated as a tool/MCP resource for the coding agent to use."

The current primrary concern is that while the track is narrow and we are building specifically for this track: "we present a suite of solutions that optimizes pain points we found to be common for everyone's workflows", the product will turn into "a bundle of 3 random things that it is not really great at, and will turn into a master of none product". We wish to harden the current pipeline, and avoid this. Ideally, we wish to keep 1 for its easy demo ability, 2 for its highly effective gains, and 3 for its importance in developing systems that work.
