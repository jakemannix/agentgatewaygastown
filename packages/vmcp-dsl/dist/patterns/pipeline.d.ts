/**
 * Pipeline pattern builder
 */
import type { PatternSpec, PipelineSpec, PipelineStep, DataBinding } from '../types.js';
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
     * Construct input from multiple field bindings
     * Allows building a new object from fields extracted from prior steps or input
     */
    construct(fields: Record<string, DataBinding>): this;
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
/**
 * Create an input binding (reference to composition input)
 */
export declare function fromInput(pathExpr?: string): DataBinding;
/**
 * Create a step binding (reference to prior step output)
 */
export declare function fromStep(stepId: string, pathExpr?: string): DataBinding;
/**
 * Create a constant binding
 */
export declare function constant(value: unknown): DataBinding;
/**
 * Create a construct binding (build object from multiple field bindings)
 *
 * @example
 * construct({
 *   product_id: fromStep('alerts', '$.alerts[0].product_id'),
 *   quantity: fromStep('alerts', '$.alerts[0].deficit'),
 * })
 */
export declare function construct(fields: Record<string, DataBinding>): DataBinding;
//# sourceMappingURL=pipeline.d.ts.map