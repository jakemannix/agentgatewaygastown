/**
 * Pipeline pattern builder
 */
import type { PatternSpec, PipelineSpec, PipelineStep } from '../types.js';
/**
 * Builder for pipeline steps
 */
export declare class StepBuilder {
    private step;
    constructor(id: string);
    /**
     * Set step to call a tool
     */
    tool(name: string): this;
    /**
     * Set step to execute a pattern
     */
    pattern(spec: PatternSpec): this;
    /**
     * Set input from composition input
     */
    fromInput(pathExpr?: string): this;
    /**
     * Set input from a previous step
     */
    fromStep(stepId: string, pathExpr?: string): this;
    /**
     * Set input to a constant value
     */
    constant(value: unknown): this;
    /**
     * Build the step
     */
    build(): PipelineStep;
}
/**
 * Builder for pipeline patterns
 */
export declare class PipelineBuilder {
    private steps;
    /**
     * Add a step that calls a tool
     */
    step(id: string, toolName: string): this;
    /**
     * Add a step with full configuration
     */
    addStep(step: PipelineStep): this;
    /**
     * Add a step using the step builder
     */
    add(id: string): StepBuilder & {
        then: () => PipelineBuilder;
    };
    /**
     * Build the pipeline pattern spec
     */
    build(): PatternSpec;
    /**
     * Get the raw pipeline spec
     */
    spec(): PipelineSpec;
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
export declare function pipeline(): PipelineBuilder;
/**
 * Create a step builder
 */
export declare function step(id: string): StepBuilder;
//# sourceMappingURL=pipeline.d.ts.map