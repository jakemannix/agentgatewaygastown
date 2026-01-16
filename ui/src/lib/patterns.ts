/**
 * Stateful composition patterns for resilient operation orchestration.
 *
 * This module provides TypeScript types and builders for stateful patterns
 * like Timeout, Retry, Circuit Breaker, etc. These patterns wrap operations
 * and provide resilience capabilities at the composition level.
 */

/**
 * Timeout pattern specification.
 *
 * Wraps an inner operation with a timeout. If the inner operation doesn't
 * complete within the specified duration, either returns an error or
 * executes a fallback operation.
 */
export interface TimeoutSpec {
  /** The timeout duration in milliseconds. */
  durationMs: number;
  /** The inner operation to execute with a timeout. */
  inner: Operation;
  /** Optional fallback operation to execute if timeout is exceeded. */
  fallback?: Operation;
  /** Optional custom error message for timeout errors. */
  errorMessage?: string;
}

/**
 * Represents an operation that can be composed with patterns.
 */
export type Operation =
  | { type: "constant"; value: unknown }
  | { type: "toolCall"; toolName: string; arguments?: unknown }
  | { type: "timeout"; spec: TimeoutSpec };

/**
 * Builder for creating timeout-wrapped operations.
 *
 * @example
 * ```typescript
 * // Basic timeout
 * const op = timeout(5000, toolCall("slow_api"));
 *
 * // With fallback
 * const op = timeout(5000, toolCall("primary_api"))
 *   .withFallback(constant({ status: "fallback" }));
 *
 * // With custom error message
 * const op = timeout(5000, toolCall("api"))
 *   .withErrorMessage("API call timed out");
 * ```
 */
export class TimeoutBuilder {
  private spec: TimeoutSpec;

  constructor(durationMs: number, inner: Operation) {
    this.spec = {
      durationMs,
      inner,
    };
  }

  /**
   * Add a fallback operation to execute if timeout is exceeded.
   */
  withFallback(fallback: Operation): TimeoutBuilder {
    this.spec.fallback = fallback;
    return this;
  }

  /**
   * Add a custom error message for timeout errors.
   */
  withErrorMessage(message: string): TimeoutBuilder {
    this.spec.errorMessage = message;
    return this;
  }

  /**
   * Build the timeout operation.
   */
  build(): Operation {
    return { type: "timeout", spec: this.spec };
  }

  /**
   * Get the underlying timeout specification.
   */
  getSpec(): TimeoutSpec {
    return this.spec;
  }
}

/**
 * Create a timeout-wrapped operation.
 *
 * @param durationMs - The timeout duration in milliseconds
 * @param inner - The inner operation to wrap with a timeout
 * @returns A TimeoutBuilder for further configuration
 *
 * @example
 * ```typescript
 * const op = timeout(5000, toolCall("my_tool")).build();
 * ```
 */
export function timeout(durationMs: number, inner: Operation): TimeoutBuilder {
  return new TimeoutBuilder(durationMs, inner);
}

/**
 * Create a constant operation that returns a fixed value.
 *
 * @param value - The constant value to return
 * @returns A constant operation
 *
 * @example
 * ```typescript
 * const op = constant({ status: "success", data: [] });
 * ```
 */
export function constant(value: unknown): Operation {
  return { type: "constant", value };
}

/**
 * Create a tool call operation.
 *
 * @param toolName - The name of the tool to call
 * @param args - Optional arguments to pass to the tool
 * @returns A tool call operation
 *
 * @example
 * ```typescript
 * const op = toolCall("weather_api", { city: "Seattle" });
 * ```
 */
export function toolCall(toolName: string, args?: unknown): Operation {
  const op: Operation = { type: "toolCall", toolName };
  if (args !== undefined) {
    (op as { type: "toolCall"; toolName: string; arguments?: unknown }).arguments = args;
  }
  return op;
}

/**
 * Wrap an existing operation with a timeout.
 *
 * @param durationMs - The timeout duration in milliseconds
 * @param op - The operation to wrap
 * @returns A timeout-wrapped operation
 *
 * @example
 * ```typescript
 * const innerOp = toolCall("slow_service");
 * const timedOp = withTimeout(5000, innerOp).build();
 * ```
 */
export function withTimeout(durationMs: number, op: Operation): TimeoutBuilder {
  return timeout(durationMs, op);
}

/**
 * Serialize an operation to JSON format compatible with the Rust backend.
 *
 * @param op - The operation to serialize
 * @returns A JSON-serializable object
 */
export function operationToJson(op: Operation): unknown {
  switch (op.type) {
    case "constant":
      return { type: "constant", value: op.value };
    case "toolCall":
      return {
        type: "toolCall",
        toolName: op.toolName,
        ...(op.arguments !== undefined && { arguments: op.arguments }),
      };
    case "timeout":
      return {
        type: "timeout",
        durationMs: op.spec.durationMs,
        inner: operationToJson(op.spec.inner),
        ...(op.spec.fallback && { fallback: operationToJson(op.spec.fallback) }),
        ...(op.spec.errorMessage && { errorMessage: op.spec.errorMessage }),
      };
  }
}

/**
 * Parse a JSON object into an Operation.
 *
 * @param json - The JSON object to parse
 * @returns An Operation, or undefined if parsing fails
 */
export function operationFromJson(json: unknown): Operation | undefined {
  if (!json || typeof json !== "object") return undefined;

  const obj = json as Record<string, unknown>;

  switch (obj.type) {
    case "constant":
      return { type: "constant", value: obj.value };
    case "toolCall":
      if (typeof obj.toolName !== "string") return undefined;
      return {
        type: "toolCall",
        toolName: obj.toolName,
        ...(obj.arguments !== undefined && { arguments: obj.arguments }),
      };
    case "timeout": {
      if (typeof obj.durationMs !== "number") return undefined;
      const inner = operationFromJson(obj.inner);
      if (!inner) return undefined;
      const spec: TimeoutSpec = {
        durationMs: obj.durationMs,
        inner,
      };
      if (obj.fallback) {
        const fallback = operationFromJson(obj.fallback);
        if (fallback) spec.fallback = fallback;
      }
      if (typeof obj.errorMessage === "string") {
        spec.errorMessage = obj.errorMessage;
      }
      return { type: "timeout", spec };
    }
    default:
      return undefined;
  }
}
