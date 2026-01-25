/**
 * Pipeline pattern builder
 */

import type {
  PatternSpec,
  PipelineSpec,
  PipelineStep,
  StepOperation,
  DataBinding,
  ConstructBinding,
} from '../types.js';
import { path } from '../path-builder.js';

/**
 * Builder for pipeline steps
 */
export class StepBuilder {
  private step: Partial<PipelineStep> = {};

  constructor(id: string) {
    this.step.id = id;
  }

  /**
   * Set step to call a tool
   */
  tool(name: string): this {
    this.step.operation = { tool: { name } };
    return this;
  }

  /**
   * Set step to execute a pattern
   */
  pattern(spec: PatternSpec): this {
    this.step.operation = { pattern: spec };
    return this;
  }

  /**
   * Set input from composition input
   */
  fromInput(pathExpr: string = '$'): this {
    this.step.input = { input: { path: path(pathExpr) } };
    return this;
  }

  /**
   * Set input from a previous step
   */
  fromStep(stepId: string, pathExpr: string = '$'): this {
    this.step.input = { step: { stepId, path: path(pathExpr) } };
    return this;
  }

  /**
   * Set input to a constant value
   */
  constant(value: unknown): this {
    this.step.input = { constant: value };
    return this;
  }

  /**
   * Construct input from multiple field bindings
   * Allows building a new object from fields extracted from prior steps or input
   */
  construct(fields: Record<string, DataBinding>): this {
    this.step.input = { construct: { fields } };
    return this;
  }

  /**
   * Build the step
   */
  build(): PipelineStep {
    if (!this.step.id) {
      throw new Error('Step ID is required');
    }
    if (!this.step.operation) {
      throw new Error('Step operation (tool or pattern) is required');
    }
    return this.step as PipelineStep;
  }
}

/**
 * Builder for pipeline patterns
 */
export class PipelineBuilder {
  private steps: PipelineStep[] = [];

  /**
   * Add a step that calls a tool
   */
  step(id: string, toolName: string): this {
    this.steps.push({
      id,
      operation: { tool: { name: toolName } },
    });
    return this;
  }

  /**
   * Add a step with full configuration
   */
  addStep(step: PipelineStep): this {
    this.steps.push(step);
    return this;
  }

  /**
   * Add a step using the step builder
   */
  add(id: string): StepBuilder & { then: () => PipelineBuilder } {
    const builder = new StepBuilder(id) as StepBuilder & { then: () => PipelineBuilder };
    builder.then = () => {
      this.steps.push(builder.build());
      return this;
    };
    return builder;
  }

  /**
   * Build the pipeline pattern spec
   */
  build(): PatternSpec {
    return { pipeline: { steps: this.steps } };
  }

  /**
   * Get the raw pipeline spec
   */
  spec(): PipelineSpec {
    return { steps: this.steps };
  }
}

/**
 * Create a pipeline pattern
 *
 * @example
 * const searchPipeline = pipeline()
 *   .step('search', 'web_search')
 *   .step('summarize', 'summarize_text')
 *   .build();
 *
 * // With step builder
 * const pipeline2 = pipeline()
 *   .add('search').tool('web_search').fromInput('$.query').then()
 *   .add('process').tool('process').fromStep('search', '$.results').then()
 *   .build();
 */
export function pipeline(): PipelineBuilder {
  return new PipelineBuilder();
}

/**
 * Create a step builder
 */
export function step(id: string): StepBuilder {
  return new StepBuilder(id);
}

// =============================================================================
// Binding Helper Functions
// =============================================================================

/**
 * Create an input binding (reference to composition input)
 */
export function fromInput(pathExpr: string = '$'): DataBinding {
  return { input: { path: path(pathExpr) } };
}

/**
 * Create a step binding (reference to prior step output)
 */
export function fromStep(stepId: string, pathExpr: string = '$'): DataBinding {
  return { step: { stepId, path: path(pathExpr) } };
}

/**
 * Create a constant binding
 */
export function constant(value: unknown): DataBinding {
  return { constant: value };
}

/**
 * Create a construct binding (build object from multiple field bindings)
 *
 * @example
 * construct({
 *   product_id: fromStep('alerts', '$.alerts[0].product_id'),
 *   quantity: fromStep('alerts', '$.alerts[0].deficit'),
 * })
 */
export function construct(fields: Record<string, DataBinding>): DataBinding {
  return { construct: { fields } };
}

