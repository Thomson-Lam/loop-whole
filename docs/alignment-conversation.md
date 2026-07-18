# 1) alignment: problem statement  

"I have a clear idea of a system and want the agent to implement it, my system has multiple actors that interact with each other, with different data flows and conditionals. I convey this to the agent in natural language, but I cannot confirm that the agent's interpretation of my system is actually correct, or whether my natural language description of the system is translating what my mind has correctly. Despite prompting being the most efficient and quickest manner of conveying idea and intent to the agent, There is a lot of drift when using natural language and is hard to verify and control cheaply that the agent is actually aligned with your idea and intent."

The core drift is here: a human thinks in concepts, typically with a "system diagram" in their heads, but the model reasons in token space. A picture is worth a thousand words for the human, but LLMs struggle with understanding complex systems and diagrams fully correctly and are instead very good at outputting JSON. And typically, drifts go unnoticed until the code is implemented, which leads to worse token burn.

>  What if there is a bridge between the human and the agent, that provides a cheap and easily verifiable, non intrusive workflow to verify agent and human alignment before implementation?

1. human plans and reasons normally with the agent 
2. instead of the agent outputting text responses, the agent calls this tool and generates JSON: the JSON is then parsed and an ASCII diagram detailing the agent's plan is converted into ASCII for the human (the agent does not see the ASCII)
3. this JSON --> ASCII is for human review, but after the implementation of the code, static deterministic checks can also be done; using the existing pre-implementation JSON as a quick check (not sure how, either using AST, treesitter, LSP, or anything needed)
