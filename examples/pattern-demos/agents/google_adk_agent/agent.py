"""
Google ADK Agent: Multi-step Project Setup with Saga Pattern

This agent demonstrates ADK's agent composition features through a saga pattern
implementation for multi-step project setup. The saga pattern ensures that if
any step in the setup process fails, compensating actions are executed to roll
back the changes.

Architecture:
    - Coordinator Agent: Orchestrates the setup workflow
    - Project Init Agent: Creates project structure
    - Config Agent: Sets up configuration files
    - Dependencies Agent: Installs and manages dependencies

Each sub-agent has compensating actions that are executed on failure, ensuring
the system can be restored to a consistent state.
"""

from __future__ import annotations

import logging
import os
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

from google.adk.agents import Agent
from google.adk.tools import FunctionTool

logger = logging.getLogger(__name__)


class StepStatus(Enum):
    """Status of a saga step."""

    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    COMPENSATED = "compensated"


@dataclass
class SagaStep:
    """Represents a step in the saga with its compensating action."""

    name: str
    status: StepStatus = StepStatus.PENDING
    result: dict[str, Any] = field(default_factory=dict)
    error: str | None = None


@dataclass
class SagaContext:
    """Context for tracking saga execution state."""

    project_name: str
    project_path: str
    steps: list[SagaStep] = field(default_factory=list)
    current_step: int = 0

    def add_step(self, name: str) -> SagaStep:
        step = SagaStep(name=name)
        self.steps.append(step)
        return step

    def mark_current_completed(self, result: dict[str, Any]) -> None:
        if self.current_step < len(self.steps):
            self.steps[self.current_step].status = StepStatus.COMPLETED
            self.steps[self.current_step].result = result
            self.current_step += 1

    def mark_current_failed(self, error: str) -> None:
        if self.current_step < len(self.steps):
            self.steps[self.current_step].status = StepStatus.FAILED
            self.steps[self.current_step].error = error

    def get_completed_steps(self) -> list[SagaStep]:
        return [s for s in self.steps if s.status == StepStatus.COMPLETED]


# Global saga context (in production, use proper state management)
_saga_contexts: dict[str, SagaContext] = {}


# =============================================================================
# Project Initialization Tools
# =============================================================================


def create_project_structure(
    project_name: str,
    project_type: str = "python",
    base_path: str = "/tmp/projects",
) -> dict[str, Any]:
    """
    Create the initial project directory structure.

    Args:
        project_name: Name of the project to create
        project_type: Type of project (python, node, rust)
        base_path: Base path where project will be created

    Returns:
        Dictionary with creation status and paths created
    """
    project_path = os.path.join(base_path, project_name)

    # Initialize saga context
    context = SagaContext(project_name=project_name, project_path=project_path)
    context.add_step("create_project_structure")
    context.add_step("initialize_git")
    context.add_step("create_config")
    context.add_step("setup_dependencies")
    _saga_contexts[project_name] = context

    # Define directory structure based on project type
    structures = {
        "python": ["src", "tests", "docs", ".github/workflows"],
        "node": ["src", "test", "docs", ".github/workflows"],
        "rust": ["src", "tests", "benches", ".github/workflows"],
    }

    dirs = structures.get(project_type, structures["python"])
    created_dirs = []

    try:
        os.makedirs(project_path, exist_ok=True)
        for dir_name in dirs:
            dir_path = os.path.join(project_path, dir_name)
            os.makedirs(dir_path, exist_ok=True)
            created_dirs.append(dir_path)

        result = {
            "status": "success",
            "project_path": project_path,
            "created_directories": created_dirs,
            "project_type": project_type,
        }
        context.mark_current_completed(result)
        return result

    except Exception as e:
        context.mark_current_failed(str(e))
        return {
            "status": "error",
            "error": str(e),
            "compensation_required": True,
        }


def compensate_project_structure(project_name: str) -> dict[str, Any]:
    """
    Compensating action: Remove created project structure.

    Args:
        project_name: Name of the project to clean up

    Returns:
        Dictionary with compensation status
    """
    import shutil

    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "error", "error": "No saga context found"}

    try:
        if os.path.exists(context.project_path):
            shutil.rmtree(context.project_path)

        for step in context.get_completed_steps():
            if step.name == "create_project_structure":
                step.status = StepStatus.COMPENSATED

        return {
            "status": "compensated",
            "removed_path": context.project_path,
        }
    except Exception as e:
        return {"status": "error", "error": str(e)}


# =============================================================================
# Git Initialization Tools
# =============================================================================


def initialize_git(project_name: str, default_branch: str = "main") -> dict[str, Any]:
    """
    Initialize a git repository in the project.

    Args:
        project_name: Name of the project
        default_branch: Default branch name

    Returns:
        Dictionary with git initialization status
    """
    import subprocess

    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "error", "error": "No saga context found"}

    try:
        # Initialize git
        subprocess.run(
            ["git", "init", "-b", default_branch],
            cwd=context.project_path,
            check=True,
            capture_output=True,
        )

        # Create .gitignore
        gitignore_content = """
# Python
__pycache__/
*.py[cod]
.venv/
*.egg-info/

# Node
node_modules/
dist/

# IDE
.idea/
.vscode/
*.swp

# OS
.DS_Store
Thumbs.db
"""
        gitignore_path = os.path.join(context.project_path, ".gitignore")
        with open(gitignore_path, "w") as f:
            f.write(gitignore_content.strip())

        result = {
            "status": "success",
            "git_initialized": True,
            "default_branch": default_branch,
            "gitignore_created": True,
        }
        context.mark_current_completed(result)
        return result

    except Exception as e:
        context.mark_current_failed(str(e))
        return {
            "status": "error",
            "error": str(e),
            "compensation_required": True,
        }


def compensate_git(project_name: str) -> dict[str, Any]:
    """
    Compensating action: Remove git initialization.

    Args:
        project_name: Name of the project

    Returns:
        Dictionary with compensation status
    """
    import shutil

    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "error", "error": "No saga context found"}

    try:
        git_dir = os.path.join(context.project_path, ".git")
        gitignore = os.path.join(context.project_path, ".gitignore")

        if os.path.exists(git_dir):
            shutil.rmtree(git_dir)
        if os.path.exists(gitignore):
            os.remove(gitignore)

        for step in context.get_completed_steps():
            if step.name == "initialize_git":
                step.status = StepStatus.COMPENSATED

        return {
            "status": "compensated",
            "removed_git": True,
        }
    except Exception as e:
        return {"status": "error", "error": str(e)}


# =============================================================================
# Configuration Tools
# =============================================================================


def create_config(
    project_name: str,
    author: str = "Developer",
    description: str = "A new project",
) -> dict[str, Any]:
    """
    Create project configuration files.

    Args:
        project_name: Name of the project
        author: Project author name
        description: Project description

    Returns:
        Dictionary with configuration status
    """
    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "error", "error": "No saga context found"}

    try:
        # Create pyproject.toml
        pyproject_content = f'''[project]
name = "{project_name}"
version = "0.1.0"
description = "{description}"
authors = [{{name = "{author}"}}]
requires-python = ">=3.10"
dependencies = []

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
'''
        pyproject_path = os.path.join(context.project_path, "pyproject.toml")
        with open(pyproject_path, "w") as f:
            f.write(pyproject_content)

        # Create README.md
        readme_content = f"""# {project_name}

{description}

## Installation

```bash
pip install -e .
```

## Usage

```python
import {project_name.replace('-', '_')}
```

## License

MIT
"""
        readme_path = os.path.join(context.project_path, "README.md")
        with open(readme_path, "w") as f:
            f.write(readme_content)

        result = {
            "status": "success",
            "files_created": ["pyproject.toml", "README.md"],
        }
        context.mark_current_completed(result)
        return result

    except Exception as e:
        context.mark_current_failed(str(e))
        return {
            "status": "error",
            "error": str(e),
            "compensation_required": True,
        }


def compensate_config(project_name: str) -> dict[str, Any]:
    """
    Compensating action: Remove configuration files.

    Args:
        project_name: Name of the project

    Returns:
        Dictionary with compensation status
    """
    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "error", "error": "No saga context found"}

    config_files = ["pyproject.toml", "README.md"]

    try:
        removed = []
        for filename in config_files:
            filepath = os.path.join(context.project_path, filename)
            if os.path.exists(filepath):
                os.remove(filepath)
                removed.append(filename)

        for step in context.get_completed_steps():
            if step.name == "create_config":
                step.status = StepStatus.COMPENSATED

        return {
            "status": "compensated",
            "removed_files": removed,
        }
    except Exception as e:
        return {"status": "error", "error": str(e)}


# =============================================================================
# Dependencies Tools
# =============================================================================


def setup_dependencies(
    project_name: str,
    dependencies: list[str] | None = None,
) -> dict[str, Any]:
    """
    Set up project dependencies.

    Args:
        project_name: Name of the project
        dependencies: List of dependencies to install

    Returns:
        Dictionary with dependency setup status
    """
    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "error", "error": "No saga context found"}

    deps = dependencies or ["pytest", "ruff"]

    try:
        # Create virtual environment indicator
        venv_marker = os.path.join(context.project_path, ".python-version")
        with open(venv_marker, "w") as f:
            f.write("3.11\n")

        # Update pyproject.toml with dependencies
        pyproject_path = os.path.join(context.project_path, "pyproject.toml")
        if os.path.exists(pyproject_path):
            with open(pyproject_path) as f:
                content = f.read()

            # Add dependencies
            deps_str = ", ".join(f'"{d}"' for d in deps)
            content = content.replace(
                "dependencies = []", f"dependencies = [{deps_str}]"
            )

            with open(pyproject_path, "w") as f:
                f.write(content)

        result = {
            "status": "success",
            "dependencies_added": deps,
            "python_version": "3.11",
        }
        context.mark_current_completed(result)
        return result

    except Exception as e:
        context.mark_current_failed(str(e))
        return {
            "status": "error",
            "error": str(e),
            "compensation_required": True,
        }


def compensate_dependencies(project_name: str) -> dict[str, Any]:
    """
    Compensating action: Remove dependency configuration.

    Args:
        project_name: Name of the project

    Returns:
        Dictionary with compensation status
    """
    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "error", "error": "No saga context found"}

    try:
        venv_marker = os.path.join(context.project_path, ".python-version")
        if os.path.exists(venv_marker):
            os.remove(venv_marker)

        for step in context.get_completed_steps():
            if step.name == "setup_dependencies":
                step.status = StepStatus.COMPENSATED

        return {
            "status": "compensated",
            "removed_dependency_config": True,
        }
    except Exception as e:
        return {"status": "error", "error": str(e)}


# =============================================================================
# Saga Orchestration
# =============================================================================


def execute_saga(project_name: str) -> dict[str, Any]:
    """
    Execute the full project setup saga with automatic compensation on failure.

    Args:
        project_name: Name of the project to set up

    Returns:
        Dictionary with saga execution status
    """
    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "error", "error": "No saga context found"}

    # If all steps completed, return success
    if all(s.status == StepStatus.COMPLETED for s in context.steps):
        return {
            "status": "success",
            "project_name": project_name,
            "project_path": context.project_path,
            "steps_completed": [s.name for s in context.steps],
        }

    # Find failed step and compensate
    failed_idx = None
    for i, step in enumerate(context.steps):
        if step.status == StepStatus.FAILED:
            failed_idx = i
            break

    if failed_idx is not None:
        # Execute compensating actions in reverse order
        compensators = {
            "create_project_structure": compensate_project_structure,
            "initialize_git": compensate_git,
            "create_config": compensate_config,
            "setup_dependencies": compensate_dependencies,
        }

        compensation_results = []
        for i in range(failed_idx - 1, -1, -1):
            step = context.steps[i]
            if step.status == StepStatus.COMPLETED:
                compensator = compensators.get(step.name)
                if compensator:
                    result = compensator(project_name)
                    compensation_results.append({
                        "step": step.name,
                        "result": result,
                    })

        return {
            "status": "rolled_back",
            "failed_step": context.steps[failed_idx].name,
            "error": context.steps[failed_idx].error,
            "compensations": compensation_results,
        }

    return {
        "status": "in_progress",
        "completed_steps": [s.name for s in context.get_completed_steps()],
    }


def get_saga_status(project_name: str) -> dict[str, Any]:
    """
    Get the current status of a saga.

    Args:
        project_name: Name of the project

    Returns:
        Dictionary with saga status
    """
    context = _saga_contexts.get(project_name)
    if not context:
        return {"status": "not_found", "error": "No saga found for this project"}

    return {
        "project_name": project_name,
        "project_path": context.project_path,
        "steps": [
            {
                "name": s.name,
                "status": s.status.value,
                "error": s.error,
            }
            for s in context.steps
        ],
    }


# =============================================================================
# Agent Definitions using ADK's composition features
# =============================================================================


def create_project_init_agent() -> Agent:
    """Create the project initialization sub-agent."""
    return Agent(
        name="project_init_agent",
        model="gemini-2.0-flash",
        description="Handles project structure creation and cleanup",
        instruction="""You are a project initialization specialist. Your role is to:
1. Create project directory structures based on project type
2. Execute compensating actions if errors occur
3. Report detailed status of operations

Always verify operations completed successfully before reporting completion.""",
        tools=[
            FunctionTool(create_project_structure),
            FunctionTool(compensate_project_structure),
        ],
    )


def create_git_agent() -> Agent:
    """Create the git management sub-agent."""
    return Agent(
        name="git_agent",
        model="gemini-2.0-flash",
        description="Handles git repository initialization and management",
        instruction="""You are a git operations specialist. Your role is to:
1. Initialize git repositories with proper configuration
2. Set up .gitignore files based on project type
3. Execute compensating actions if errors occur

Ensure proper branch naming and initial commit setup.""",
        tools=[
            FunctionTool(initialize_git),
            FunctionTool(compensate_git),
        ],
    )


def create_config_agent() -> Agent:
    """Create the configuration management sub-agent."""
    return Agent(
        name="config_agent",
        model="gemini-2.0-flash",
        description="Handles project configuration files",
        instruction="""You are a configuration specialist. Your role is to:
1. Create project manifests (pyproject.toml, package.json, Cargo.toml)
2. Set up README and documentation templates
3. Execute compensating actions if errors occur

Use appropriate configuration standards for each project type.""",
        tools=[
            FunctionTool(create_config),
            FunctionTool(compensate_config),
        ],
    )


def create_dependencies_agent() -> Agent:
    """Create the dependencies management sub-agent."""
    return Agent(
        name="dependencies_agent",
        model="gemini-2.0-flash",
        description="Handles dependency setup and management",
        instruction="""You are a dependency management specialist. Your role is to:
1. Configure dependency specifications
2. Set up virtual environment markers
3. Execute compensating actions if errors occur

Recommend appropriate dependencies based on project type.""",
        tools=[
            FunctionTool(setup_dependencies),
            FunctionTool(compensate_dependencies),
        ],
    )


def create_coordinator_agent() -> Agent:
    """
    Create the main coordinator agent that orchestrates the saga.

    This demonstrates ADK's agent composition features by creating
    a hierarchical agent structure where the coordinator delegates
    to specialized sub-agents.
    """
    # Create sub-agents
    project_init = create_project_init_agent()
    git_agent = create_git_agent()
    config_agent = create_config_agent()
    deps_agent = create_dependencies_agent()

    # Create coordinator with sub-agents
    coordinator = Agent(
        name="project_setup_coordinator",
        model="gemini-2.0-flash",
        description="Coordinates multi-step project setup using saga pattern",
        instruction="""You are a project setup coordinator implementing the saga pattern.

Your workflow:
1. Start project setup by creating the structure (delegate to project_init_agent)
2. Initialize git repository (delegate to git_agent)
3. Create configuration files (delegate to config_agent)
4. Set up dependencies (delegate to dependencies_agent)
5. Monitor saga status and handle failures

SAGA PATTERN RULES:
- Execute steps in order
- If any step fails, execute compensating actions in reverse order
- Report final saga status with all step results

Use get_saga_status to check progress and execute_saga to handle completion/rollback.""",
        sub_agents=[project_init, git_agent, config_agent, deps_agent],
        tools=[
            FunctionTool(execute_saga),
            FunctionTool(get_saga_status),
        ],
    )

    return coordinator


# Export the main agent for ADK CLI
root_agent = create_coordinator_agent()
