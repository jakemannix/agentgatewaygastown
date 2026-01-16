"""LangGraph ReAct agent implementation with state management."""

from __future__ import annotations

import operator
from dataclasses import dataclass, field
from typing import Annotated, Any, Literal, Sequence, TypedDict

from langchain_anthropic import ChatAnthropic
from langchain_core.messages import (
    AIMessage,
    BaseMessage,
    HumanMessage,
    SystemMessage,
    ToolMessage,
)
from langchain_core.tools import BaseTool
from langgraph.graph import END, StateGraph
from langgraph.prebuilt import ToolNode


class AgentState(TypedDict):
    """State for the ReAct agent.

    The state tracks the conversation history, tool results, and
    orchestration metadata for visualization and debugging.
    """

    # Core conversation state
    messages: Annotated[Sequence[BaseMessage], operator.add]

    # Orchestration tracking
    step_count: int
    tool_calls_made: list[dict[str, Any]]
    current_phase: str


@dataclass
class ExecutionTrace:
    """Trace of agent execution for visualization."""

    steps: list[dict[str, Any]] = field(default_factory=list)

    def add_step(
        self,
        step_type: str,
        content: str,
        tool_name: str | None = None,
        tool_args: dict[str, Any] | None = None,
        tool_result: str | None = None,
    ) -> None:
        """Add a step to the execution trace."""
        self.steps.append(
            {
                "type": step_type,
                "content": content,
                "tool_name": tool_name,
                "tool_args": tool_args,
                "tool_result": tool_result,
            }
        )


def create_react_agent(
    tools: list[BaseTool],
    model: str = "claude-sonnet-4-20250514",
    system_prompt: str | None = None,
    max_iterations: int = 10,
) -> tuple[StateGraph, ExecutionTrace]:
    """Create a LangGraph ReAct agent with the given tools.

    Args:
        tools: List of LangChain tools to make available to the agent
        model: Anthropic model ID to use
        system_prompt: Optional system prompt for the agent
        max_iterations: Maximum number of reasoning iterations

    Returns:
        A tuple of (compiled graph, execution trace)
    """
    # Initialize the LLM with tools
    llm = ChatAnthropic(model=model, temperature=0)
    llm_with_tools = llm.bind_tools(tools)

    # Create execution trace
    trace = ExecutionTrace()

    # Default system prompt for ReAct pattern
    default_system = """You are a helpful AI assistant that uses tools to accomplish tasks.

When given a task:
1. Think about what needs to be done and break it into steps
2. Use available tools to gather information or take actions
3. Analyze results and determine next steps
4. Continue until the task is complete

Always explain your reasoning before taking actions."""

    system_message = SystemMessage(content=system_prompt or default_system)

    def should_continue(state: AgentState) -> Literal["tools", "end"]:
        """Determine whether to continue with tools or end."""
        messages = state["messages"]
        last_message = messages[-1]

        # Check iteration limit
        if state.get("step_count", 0) >= max_iterations:
            return "end"

        # If the LLM made tool calls, continue to tools
        if isinstance(last_message, AIMessage) and last_message.tool_calls:
            return "tools"

        return "end"

    def call_model(state: AgentState) -> dict[str, Any]:
        """Call the LLM to reason and potentially invoke tools."""
        messages = state["messages"]

        # Ensure system message is first
        if not messages or not isinstance(messages[0], SystemMessage):
            messages = [system_message] + list(messages)

        response = llm_with_tools.invoke(messages)

        # Track the step
        step_count = state.get("step_count", 0) + 1
        tool_calls_made = list(state.get("tool_calls_made", []))

        # Record tool calls for tracing
        if isinstance(response, AIMessage) and response.tool_calls:
            for tc in response.tool_calls:
                tool_calls_made.append(
                    {
                        "step": step_count,
                        "tool": tc["name"],
                        "args": tc["args"],
                    }
                )
                trace.add_step(
                    step_type="tool_call",
                    content=response.content if response.content else "",
                    tool_name=tc["name"],
                    tool_args=tc["args"],
                )
        else:
            trace.add_step(
                step_type="reasoning",
                content=str(response.content),
            )

        # Determine current phase based on context
        current_phase = "reasoning"
        if tool_calls_made:
            last_tool = tool_calls_made[-1]["tool"].lower()
            if "search" in last_tool or "research" in last_tool:
                current_phase = "research"
            elif "summarize" in last_tool:
                current_phase = "summarizing"
            elif "notify" in last_tool or "send" in last_tool or "slack" in last_tool:
                current_phase = "notifying"

        return {
            "messages": [response],
            "step_count": step_count,
            "tool_calls_made": tool_calls_made,
            "current_phase": current_phase,
        }

    def process_tool_results(state: AgentState) -> dict[str, Any]:
        """Process tool results and add to trace."""
        messages = state["messages"]
        for msg in reversed(messages):
            if isinstance(msg, ToolMessage):
                trace.add_step(
                    step_type="tool_result",
                    content=str(msg.content)[:500],
                    tool_name=msg.name,
                    tool_result=str(msg.content)[:500],
                )
                break
        return {}

    # Build the graph
    workflow = StateGraph(AgentState)

    # Add nodes
    workflow.add_node("agent", call_model)
    tool_node = ToolNode(tools)

    def tools_with_trace(state: AgentState) -> dict[str, Any]:
        """Wrapper to capture tool results in trace."""
        result = tool_node.invoke(state)
        # Process results for trace
        for msg in result.get("messages", []):
            if isinstance(msg, ToolMessage):
                trace.add_step(
                    step_type="tool_result",
                    content=str(msg.content)[:500],
                    tool_name=msg.name,
                    tool_result=str(msg.content)[:500],
                )
        return result

    workflow.add_node("tools", tools_with_trace)

    # Set entry point
    workflow.set_entry_point("agent")

    # Add edges
    workflow.add_conditional_edges(
        "agent",
        should_continue,
        {
            "tools": "tools",
            "end": END,
        },
    )
    workflow.add_edge("tools", "agent")

    return workflow.compile(), trace


def run_agent(
    graph: StateGraph,
    user_input: str,
    trace: ExecutionTrace | None = None,
) -> tuple[str, AgentState]:
    """Run the agent with a user input.

    Args:
        graph: The compiled LangGraph
        user_input: The user's input message
        trace: Optional execution trace to record steps

    Returns:
        A tuple of (final response text, final state)
    """
    initial_state: AgentState = {
        "messages": [HumanMessage(content=user_input)],
        "step_count": 0,
        "tool_calls_made": [],
        "current_phase": "starting",
    }

    if trace:
        trace.add_step(
            step_type="input",
            content=user_input,
        )

    # Run the graph
    final_state = None
    for state in graph.stream(initial_state):
        final_state = state

    # Extract final response
    if final_state and "agent" in final_state:
        agent_state = final_state["agent"]
        messages = agent_state.get("messages", [])
        if messages:
            last_msg = messages[-1]
            if isinstance(last_msg, AIMessage):
                return str(last_msg.content), agent_state

    return "No response generated", initial_state


async def arun_agent(
    graph: StateGraph,
    user_input: str,
    trace: ExecutionTrace | None = None,
) -> tuple[str, AgentState]:
    """Run the agent asynchronously with a user input.

    Args:
        graph: The compiled LangGraph
        user_input: The user's input message
        trace: Optional execution trace to record steps

    Returns:
        A tuple of (final response text, final state)
    """
    initial_state: AgentState = {
        "messages": [HumanMessage(content=user_input)],
        "step_count": 0,
        "tool_calls_made": [],
        "current_phase": "starting",
    }

    if trace:
        trace.add_step(
            step_type="input",
            content=user_input,
        )

    # Run the graph
    final_state = None
    async for state in graph.astream(initial_state):
        final_state = state

    # Extract final response
    if final_state and "agent" in final_state:
        agent_state = final_state["agent"]
        messages = agent_state.get("messages", [])
        if messages:
            last_msg = messages[-1]
            if isinstance(last_msg, AIMessage):
                return str(last_msg.content), agent_state

    return "No response generated", initial_state
